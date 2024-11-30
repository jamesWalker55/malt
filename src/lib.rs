mod biquad;
mod envelope;
mod gui;
mod parameter_formatters;
mod pattern;
mod splitter;
mod svf;

use biquad::{FirstOrderLP, FixedQFilter};
use envelope::Curve;
use envelope::Envelope;
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use parameter_formatters::{s2v_f32_ms_then_s, v2s_f32_ms_then_s};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use splitter::MinimumThreeBand12Slope;
use splitter::MinimumThreeBand24Slope;
use std::sync::Arc;
use util::db_to_gain;

const CROSSOVER_MIN_HZ: f32 = 10.0;
const CROSSOVER_MAX_HZ: f32 = 20000.0;
const MAX_LATENCY_SECONDS: f32 = 0.01;

enum MultibandGainApplier {
    ThreeBand24(splitter::MinimumThreeBand24Slope),
    ThreeBand12(splitter::MinimumThreeBand12Slope),
}

impl MultibandGainApplier {
    /// Gain is scalar, 0.0 to 1.0 and beyond
    fn apply_gain(&mut self, sample: f64, gains: &[f64; 3]) -> f64 {
        match self {
            MultibandGainApplier::ThreeBand24(splitter) => splitter.apply_gain(sample, gains),
            MultibandGainApplier::ThreeBand12(splitter) => splitter.apply_gain(sample, gains),
        }
    }

    pub(crate) fn set_frequencies(&mut self, f1: f64, f2: f64) {
        nih_debug_assert!(f1 < f2, "f1 must be less than f2");

        match self {
            MultibandGainApplier::ThreeBand24(splitter) => {
                splitter.set_frequencies(f1, f2);
            }
            MultibandGainApplier::ThreeBand12(splitter) => {
                splitter.set_frequencies(f1, f2);
            }
        }
    }
}

enum EnvelopeOverlapMode {
    Sum,
    Max,
}

struct BandLinkedVoice {
    channel: u8,
    low: Envelope,
    mid: Envelope,
    high: Envelope,
    low_db: f32,
    mid_db: f32,
    high_db: f32,
}

impl BandLinkedVoice {
    fn set_release(&mut self, low_release: f32, mid_release: f32, high_release: f32) {
        self.low.set_release(low_release);
        self.mid.set_release(mid_release);
        self.high.set_release(high_release);
    }

    /// Returns the lowest progress of all the envelopes
    fn progress(&self) -> f32 {
        self.low
            .progress()
            .min(self.mid.progress())
            .min(self.high.progress())
    }

    fn is_complete(&self) -> bool {
        self.low.is_complete() && self.mid.is_complete() && self.high.is_complete()
    }

    /// This expects scalars 0.0 -- 1.0 as input.
    fn set_db_gains(&mut self, low_db: f32, mid_db: f32, high_db: f32) {
        self.low_db = low_db;
        self.mid_db = mid_db;
        self.high_db = high_db;
    }

    // /// Return the gain in dB of each envelope (is negative, e.g. -24dB)
    // fn tick(&mut self) -> [Option<f32>; 3] {
    //     [
    //         self.low.tick().map(|x| x * self.low_gain),
    //         self.mid.tick().map(|x| x * self.mid_gain),
    //         self.high.tick().map(|x| x * self.high_gain),
    //     ]
    // }
}

struct BandLinkedEnvelopeLanes<const VOICES: usize> {
    // sr: f64,
    // latency_seconds: f32,
    voices: [Option<BandLinkedVoice>; VOICES],
    filter_l: FixedQFilter<FirstOrderLP>,
    filter_m: FixedQFilter<FirstOrderLP>,
    filter_h: FixedQFilter<FirstOrderLP>,
    smooth: bool,
    overlap_mode: EnvelopeOverlapMode,
}

impl<const VOICES: usize> BandLinkedEnvelopeLanes<VOICES> {
    const EMPTY_VOICE: Option<BandLinkedVoice> = None;

    fn default_filter(sr: f64) -> FixedQFilter<FirstOrderLP> {
        FixedQFilter::new(1000.0, sr)
    }

    fn new(sr: f64, smooth: bool, overlap_mode: EnvelopeOverlapMode) -> Self {
        Self {
            voices: [Self::EMPTY_VOICE; VOICES],
            filter_l: Self::default_filter(sr),
            filter_m: Self::default_filter(sr),
            filter_h: Self::default_filter(sr),
            smooth,
            overlap_mode,
        }
    }

    fn add(&mut self, voice: BandLinkedVoice) {
        let insertion_idx = {
            // insert into an empty cell
            match self.voices.iter().position(|x| x.is_none()) {
                Some(idx) => idx,
                // if no empty cells, find the envelope that's closest to finishing
                None => {
                    self.voices
                        .iter()
                        .enumerate()
                        .max_by(|(_, opt1), (_, opt2)| match (opt1, opt2) {
                            (None, None) => unreachable!(),
                            (None, Some(_)) => unreachable!(),
                            (Some(_), None) => unreachable!(),
                            (Some(voice1), Some(voice2)) => {
                                voice1.progress().total_cmp(&voice2.progress())
                            }
                        })
                        .expect("envelope lane must have size of at least 1")
                        .0
                }
            }
        };

        self.voices[insertion_idx] = Some(voice);
    }

    fn set_release(&mut self, channel: u8, low_release: f32, mid_release: f32, high_release: f32) {
        for voice in &mut self.voices {
            let Some(voice) = voice else {
                continue;
            };
            if voice.channel != channel {
                continue;
            }

            voice.set_release(low_release, mid_release, high_release);
        }
    }

    fn set_smooth(&mut self, sr: f64, smooth: bool) {
        if smooth == self.smooth {
            return;
        }

        self.smooth = smooth;
        // reset filter to avoid pops and clicks
        self.filter_l = Self::default_filter(sr);
        self.filter_m = Self::default_filter(sr);
        self.filter_h = Self::default_filter(sr);
    }

    /// Return the gain as scalars for each envelope (in range 0.0 -- 1.0)
    fn tick(&mut self) -> [f32; 3] {
        let low_db: f32 = {
            let iter = self.voices.iter_mut().filter_map(|voice| {
                voice
                    .as_mut()
                    .map(|voice| voice.low.tick().unwrap_or_default() * voice.low_db)
            });

            match self.overlap_mode {
                EnvelopeOverlapMode::Sum => iter.sum(),
                EnvelopeOverlapMode::Max => iter.max_by(|a, b| a.total_cmp(b)).unwrap_or(0.0),
            }
        };
        let mid_db: f32 = {
            let iter = self.voices.iter_mut().filter_map(|voice| {
                voice
                    .as_mut()
                    .map(|voice| voice.mid.tick().unwrap_or_default() * voice.mid_db)
            });

            match self.overlap_mode {
                EnvelopeOverlapMode::Sum => iter.sum(),
                EnvelopeOverlapMode::Max => iter.max_by(|a, b| a.total_cmp(b)).unwrap_or(0.0),
            }
        };
        let high_db: f32 = {
            let iter = self.voices.iter_mut().filter_map(|voice| {
                voice
                    .as_mut()
                    .map(|voice| voice.high.tick().unwrap_or_default() * voice.high_db)
            });

            match self.overlap_mode {
                EnvelopeOverlapMode::Sum => iter.sum(),
                EnvelopeOverlapMode::Max => iter.max_by(|a, b| a.total_cmp(b)).unwrap_or(0.0),
            }
        };

        // remove inactive envelopes
        for cell in &mut self.voices {
            {
                let Some(voice) = cell else {
                    continue;
                };

                if !voice.is_complete() {
                    continue;
                }
            };

            // the cell is filled, and the voice is complete
            // clear it now
            *cell = None;
        }

        let low = db_to_gain(-low_db);
        let mid = db_to_gain(-mid_db);
        let high = db_to_gain(-high_db);

        if !self.smooth {
            [low, mid, high]
        } else {
            let low = self.filter_l.process_sample(low as f64) as f32;
            let mid = self.filter_m.process_sample(mid as f64) as f32;
            let high = self.filter_h.process_sample(high as f64) as f32;

            [low, mid, high]
        }
    }
}

pub struct Malt {
    params: Arc<MaltParams>,
    // fixed variables (per session)
    sr: f32,
    max_latency_samples: usize,
    // audio processing stuff:
    splitter_l: MultibandGainApplier,
    splitter_r: MultibandGainApplier,
    env: BandLinkedEnvelopeLanes<64>,
    latency_buf_l: AllocRingBuffer<f32>,
    latency_buf_r: AllocRingBuffer<f32>,
    // keep track of when parameters get changed:
    latency_seconds: f32,
    latency_samples: usize,
    current_slope: Slope,

    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    // #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
}

#[derive(Enum, PartialEq, Eq)]
enum Slope {
    #[id = "fixed_24"]
    #[name = "24 dB/octave"]
    F24,
    #[id = "fixed_12"]
    #[name = "12 dB/octave"]
    F12,
}

#[derive(Params)]
struct MaltParams {
    #[nested(array, group = "channels")]
    pub channels: [ChannelParams; 16],

    #[id = "low_crossover"]
    pub(crate) low_crossover: FloatParam,
    #[id = "high_crossover"]
    pub(crate) high_crossover: FloatParam,

    #[id = "crossover_slope"]
    pub(crate) crossover_slope: EnumParam<Slope>,

    #[id = "smoothing"]
    pub(crate) smoothing: BoolParam,
    #[id = "lookahead"]
    pub(crate) lookahead: FloatParam,

    #[id = "bypass"]
    pub(crate) bypass: BoolParam,
    #[id = "mix"]
    pub(crate) mix: FloatParam,
}

impl Default for MaltParams {
    fn default() -> Self {
        Self {
            channels: Default::default(),

            low_crossover: FloatParam::new(
                "Low crossover",
                120.0,
                FloatRange::Skewed {
                    min: CROSSOVER_MIN_HZ,
                    max: CROSSOVER_MAX_HZ,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(3))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            high_crossover: FloatParam::new(
                "High crossover",
                2500.0,
                FloatRange::Skewed {
                    min: CROSSOVER_MIN_HZ,
                    max: CROSSOVER_MAX_HZ,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(3))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),

            crossover_slope: EnumParam::new("Crossover slope", Slope::F24),
            smoothing: BoolParam::new("Smoothing", true),
            lookahead: FloatParam::new(
                "Lookahead",
                10.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: MAX_LATENCY_SECONDS * 1000.0,
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s())
            .non_automatable(),

            bypass: BoolParam::new("Bypass", false),
            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(3))
                .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}

impl MaltParams {
    fn next(&self) -> MaltParamsValues {
        let low_crossover = {
            let value = self.low_crossover.smoothed.next();
            // since high crossover will be 1 octave above this, this cannot be too high
            value.min(CROSSOVER_MAX_HZ / 2.0)
        };
        let high_crossover = {
            // limit high crossover to be 1 octave above low crossover
            // (this is pro-mb's behaviour)
            let value = self.high_crossover.smoothed.next();
            let min_value = low_crossover * 2.0;
            value.max(min_value)
        };

        let crossover_slope = self.crossover_slope.value();
        let smoothing = self.smoothing.value();
        let lookahead = self.lookahead.value() / 1000.0; // convert to seconds
        let bypass = self.bypass.value();
        let mix = self.mix.smoothed.next();

        let channels: [ChannelParamValues; 16] =
            self.channels.each_ref().map(|param| param.next(lookahead));

        MaltParamsValues {
            channels,
            low_crossover,
            high_crossover,
            crossover_slope,
            smoothing,
            lookahead,
            bypass,
            mix,
        }
    }
}

struct MaltParamsValues {
    channels: [ChannelParamValues; 16],
    low_crossover: f32,
    high_crossover: f32,
    crossover_slope: Slope,
    smoothing: bool,
    /// in seconds
    lookahead: f32,
    bypass: bool,
    mix: f32,
}

#[derive(Params)]
struct ChannelParams {
    #[id = "low_precomp"]
    pub(crate) low_precomp: FloatParam,
    #[id = "mid_precomp"]
    pub(crate) mid_precomp: FloatParam,
    #[id = "high_precomp"]
    pub(crate) high_precomp: FloatParam,

    #[id = "low_decay"]
    pub(crate) low_decay: FloatParam,
    #[id = "mid_decay"]
    pub(crate) mid_decay: FloatParam,
    #[id = "high_decay"]
    pub(crate) high_decay: FloatParam,

    // gain, 0.0 -- 90.0
    #[id = "low_db"]
    pub(crate) low_db: FloatParam,
    #[id = "mid_db"]
    pub(crate) mid_db: FloatParam,
    #[id = "high_db"]
    pub(crate) high_db: FloatParam,
}

impl Default for ChannelParams {
    fn default() -> Self {
        Self {
            low_precomp: FloatParam::new(
                "Low precomp",
                10.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: MAX_LATENCY_SECONDS * 1000.0,
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),
            mid_precomp: FloatParam::new(
                "Mid precomp",
                10.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: MAX_LATENCY_SECONDS * 1000.0,
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),
            high_precomp: FloatParam::new(
                "High precomp",
                10.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: MAX_LATENCY_SECONDS * 1000.0,
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),

            low_decay: FloatParam::new(
                "Low decay",
                100.0,
                // these settings are similar to FabFilter Pro-C's release
                FloatRange::Skewed {
                    min: 10.0,
                    max: 2500.0,
                    factor: FloatRange::skew_factor(-1.6),
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),
            mid_decay: FloatParam::new(
                "Mid decay",
                100.0,
                // these settings are similar to FabFilter Pro-C's release
                FloatRange::Skewed {
                    min: 10.0,
                    max: 2500.0,
                    factor: FloatRange::skew_factor(-1.6),
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),
            high_decay: FloatParam::new(
                "High decay",
                100.0,
                // these settings are similar to FabFilter Pro-C's release
                FloatRange::Skewed {
                    min: 10.0,
                    max: 2500.0,
                    factor: FloatRange::skew_factor(-1.6),
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),

            low_db: FloatParam::new(
                "Low gain reduction",
                db_to_gain(-30.0),
                FloatRange::Linear {
                    min: 0.0,
                    max: 90.0,
                },
            )
            .with_unit(" dB"),
            mid_db: FloatParam::new(
                "Mid gain reduction",
                db_to_gain(-30.0),
                FloatRange::Linear {
                    min: 0.0,
                    max: 90.0,
                },
            )
            .with_unit(" dB"),
            high_db: FloatParam::new(
                "High gain reduction",
                db_to_gain(-30.0),
                FloatRange::Linear {
                    min: 0.0,
                    max: 90.0,
                },
            )
            .with_unit(" dB"),
        }
    }
}

impl ChannelParams {
    fn next(&self, latency_seconds: f32) -> ChannelParamValues {
        let low_precomp = {
            let value = self.low_precomp.smoothed.next() / 1000.0;
            value.min(latency_seconds)
        };
        let mid_precomp = {
            let value = self.mid_precomp.smoothed.next() / 1000.0;
            value.min(latency_seconds)
        };
        let high_precomp = {
            let value = self.high_precomp.smoothed.next() / 1000.0;
            value.min(latency_seconds)
        };
        let low_decay = self.low_decay.smoothed.next() / 1000.0;
        let mid_decay = self.mid_decay.smoothed.next() / 1000.0;
        let high_decay = self.high_decay.smoothed.next() / 1000.0;
        let low_db = self.low_db.smoothed.next();
        let mid_db = self.mid_db.smoothed.next();
        let high_db = self.high_db.smoothed.next();

        ChannelParamValues {
            low_precomp,
            mid_precomp,
            high_precomp,
            low_decay,
            mid_decay,
            high_decay,
            low_db,
            mid_db,
            high_db,
        }
    }
}

pub(crate) struct ChannelParamValues {
    /// Precomp is in seconds
    pub(crate) low_precomp: f32,
    /// Precomp is in seconds
    pub(crate) mid_precomp: f32,
    /// Precomp is in seconds
    pub(crate) high_precomp: f32,

    /// Decay is in seconds
    pub(crate) low_decay: f32,
    /// Decay is in seconds
    pub(crate) mid_decay: f32,
    /// Decay is in seconds
    pub(crate) high_decay: f32,

    /// Gain in dB, 0.0 -- +90.0
    pub(crate) low_db: f32,
    /// Gain in dB, 0.0 -- +90.0
    pub(crate) mid_db: f32,
    /// Gain in dB, 0.0 -- +90.0
    pub(crate) high_db: f32,
}

impl Default for Malt {
    fn default() -> Self {
        Self {
            params: Arc::new(MaltParams::default()),
            // these fields are not initialised here, see `initialize()` for the actual values
            sr: 0.0,
            latency_seconds: 0.0,
            latency_samples: 0,
            max_latency_samples: 0,
            current_slope: Slope::F24,
            splitter_l: MultibandGainApplier::ThreeBand24(MinimumThreeBand24Slope::new(
                0.0, 0.0, 0.0,
            )),
            splitter_r: MultibandGainApplier::ThreeBand24(MinimumThreeBand24Slope::new(
                0.0, 0.0, 0.0,
            )),
            latency_buf_l: AllocRingBuffer::new(1),
            latency_buf_r: AllocRingBuffer::new(1),
            env: BandLinkedEnvelopeLanes::new(0.0, false, EnvelopeOverlapMode::Max),
            editor_state: EguiState::from_size(gui::GUI_WIDTH, gui::GUI_HEIGHT),
        }
    }
}

impl Plugin for Malt {
    const NAME: &'static str = "Malt";
    const VENDOR: &'static str = "SAI Audio";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "hello@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // constants per session
        self.sr = _buffer_config.sample_rate;
        self.max_latency_samples = (MAX_LATENCY_SECONDS * self.sr).round() as usize;

        // allocate buffers for storing old samples
        // buffer length should be `self.max_latency_samples`
        self.latency_buf_l = {
            let mut buf = AllocRingBuffer::new(self.max_latency_samples);
            buf.fill(0.0);
            buf
        };
        self.latency_buf_r = self.latency_buf_l.clone();

        true
    }

    fn reset(&mut self) {
        // setup filters
        self.current_slope = self.params.crossover_slope.value();
        self.splitter_l = MultibandGainApplier::ThreeBand24(MinimumThreeBand24Slope::new(
            1000.0,
            2000.0,
            self.sr.into(),
        ));
        self.splitter_r = MultibandGainApplier::ThreeBand24(MinimumThreeBand24Slope::new(
            1000.0,
            2000.0,
            self.sr.into(),
        ));

        // clear envelope
        self.env = BandLinkedEnvelopeLanes::new(
            self.sr.into(),
            self.params.smoothing.value(),
            EnvelopeOverlapMode::Max,
        );
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        debug_assert_eq!(buffer.channels(), 2);

        let mut next_event = ctx.next_event();

        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            let params = self.params.next();
            let sample_rate = ctx.transport().sample_rate;

            // handle if latency has changed
            {
                let lookahead_samples = params.lookahead * ctx.transport().sample_rate;
                // update latency for daw, is no-op if value is same
                ctx.set_latency_samples(lookahead_samples.round() as u32);
            }

            // handle crossover slope change
            {
                if params.crossover_slope != self.current_slope {
                    // replace splitters with new slopes
                    self.current_slope = params.crossover_slope;
                    match self.current_slope {
                        Slope::F24 => {
                            self.splitter_l = MultibandGainApplier::ThreeBand24(
                                MinimumThreeBand24Slope::new(1000.0, 2000.0, sample_rate.into()),
                            );
                            self.splitter_r = MultibandGainApplier::ThreeBand24(
                                MinimumThreeBand24Slope::new(1000.0, 2000.0, sample_rate.into()),
                            );
                        }
                        Slope::F12 => {
                            self.splitter_l = MultibandGainApplier::ThreeBand12(
                                MinimumThreeBand12Slope::new(1000.0, 2000.0, sample_rate.into()),
                            );
                            self.splitter_r = MultibandGainApplier::ThreeBand12(
                                MinimumThreeBand12Slope::new(1000.0, 2000.0, sample_rate.into()),
                            );
                        }
                    }
                }
            }

            // handle smoothing change
            self.env.set_smooth(sample_rate as f64, params.smoothing);

            // handle MIDI events
            let mut channel_triggered: [bool; 16] = [false; 16];
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }

                if let NoteEvent::NoteOn { channel, .. } = event {
                    channel_triggered[channel as usize] = true;
                }

                next_event = ctx.next_event();
            }

            // update existing envelopes (if any)
            for (channel, values) in params.channels.iter().enumerate() {
                self.env.set_release(
                    channel as u8,
                    values.low_decay,
                    values.mid_decay,
                    values.high_decay,
                );
            }

            // trigger notes in envelope
            for (channel, triggered) in channel_triggered.iter().enumerate() {
                if !triggered {
                    continue;
                }

                self.env.add(BandLinkedVoice {
                    channel: channel as u8,
                    low: Envelope::from_latency(
                        sample_rate,
                        params.lookahead,
                        params.channels[channel].low_precomp,
                        params.channels[channel].low_decay,
                        Curve::EaseInSine,
                        Curve::EaseInOutSine,
                    ),
                    mid: Envelope::from_latency(
                        sample_rate,
                        params.lookahead,
                        params.channels[channel].mid_precomp,
                        params.channels[channel].mid_decay,
                        Curve::EaseInSine,
                        Curve::EaseInOutSine,
                    ),
                    high: Envelope::from_latency(
                        sample_rate,
                        params.lookahead,
                        params.channels[channel].high_precomp,
                        params.channels[channel].high_decay,
                        Curve::EaseInSine,
                        Curve::EaseInOutSine,
                    ),
                    low_db: params.channels[channel].low_db,
                    mid_db: params.channels[channel].mid_db,
                    high_db: params.channels[channel].high_db,
                });
            }

            // update filter frequency
            self.splitter_l
                .set_frequencies(params.low_crossover.into(), params.high_crossover.into());
            self.splitter_r
                .set_frequencies(params.low_crossover.into(), params.high_crossover.into());

            #[inline(always)]
            fn calculate_final_gain(gain: f32, mix: f32, bypass: bool) -> f64 {
                if bypass {
                    1.0
                } else {
                    // mix should operate scalar-wise, not in dB units
                    // i.e. don't put `mix` inside the `db_to_gain` function
                    mix as f64 * (gain as f64 - 1.0) + 1.0
                }
            }

            // tick envelopes and get gain value
            // we intentionally always call envelope's `tick()` even when bypassed:
            let [low_gain, mid_gain, high_gain] = self.env.tick();
            let low_gain = calculate_final_gain(low_gain, params.mix, params.bypass);
            let mid_gain = calculate_final_gain(mid_gain, params.mix, params.bypass);
            let high_gain = calculate_final_gain(high_gain, params.mix, params.bypass);

            let latency_buf_offset = self.max_latency_samples - self.latency_samples;

            // left channel
            {
                let sample = channel_samples.get_mut(0).unwrap();

                // the sample from eons ago (the latency)
                let delayed_sample = *self.latency_buf_l.get(latency_buf_offset).unwrap();
                // push sample to buffer queue
                self.latency_buf_l.push(*sample);
                // *sample = delayed_sample;

                // process delayed sample
                *sample = self
                    .splitter_l
                    .apply_gain(delayed_sample as f64, &[low_gain, mid_gain, high_gain])
                    as f32;
            }

            // right channel
            {
                let sample = channel_samples.get_mut(1).unwrap();

                // the sample from eons ago (the latency)
                let delayed_sample = *self.latency_buf_r.get(latency_buf_offset).unwrap();
                // push sample to buffer queue
                self.latency_buf_r.push(*sample);
                // *sample = delayed_sample;

                // process delayed sample
                *sample = self
                    .splitter_r
                    .apply_gain(delayed_sample as f64, &[low_gain, mid_gain, high_gain])
                    as f32;
            }
        }

        ProcessStatus::Normal
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        gui::create_gui(self, async_executor)
    }
}

impl ClapPlugin for Malt {
    const CLAP_ID: &'static str = "com.sai-audio.malt.v1";
    const CLAP_DESCRIPTION: Option<&'static str> = None;
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Malt {
    const VST3_CLASS_ID: [u8; 16] = *b"saiaudiomalt1___";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(Malt);
nih_export_vst3!(Malt);
