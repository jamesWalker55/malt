mod biquad;
mod envelope;
mod oscillator;
mod parameter_formatters;
mod pattern;
mod splitter;
mod svf;
mod voice;

use biquad::{
    ButterworthLP, FirstOrderAP, FirstOrderLP, FixedQFilter, LinkwitzRileyHP, LinkwitzRileyLP,
};
use envelope::EaseInOutSine;
use envelope::EaseInSine;
use envelope::Envelope;
use nih_plug::prelude::*;
use parameter_formatters::{s2v_f32_ms_then_s, v2s_f32_ms_then_s};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use splitter::MinimumThreeBand12Slope;
use splitter::MinimumThreeBand24Slope;
use std::sync::Arc;
use util::{db_to_gain, gain_to_db};

const CROSSOVER_MIN_HZ: f32 = 10.0;
const CROSSOVER_MAX_HZ: f32 = 20000.0;

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

struct EnvelopeLane<A: envelope::Curve, R: envelope::Curve, const VOICES: usize> {
    sr: f64,
    latency_seconds: f32,
    voices: [Option<Envelope<A, R>>; VOICES],
    filter: FixedQFilter<FirstOrderLP>,
    smooth: bool,
}

impl<A: envelope::Curve, R: envelope::Curve, const VOICES: usize> EnvelopeLane<A, R, VOICES> {
    const EMPTY_VOICE: Option<Envelope<A, R>> = None;

    fn default_filter(sr: f64) -> FixedQFilter<FirstOrderLP> {
        FixedQFilter::new(1000.0, sr)
    }

    fn new(sr: f64, latency_seconds: f32, smooth: bool) -> Self {
        Self {
            sr,
            latency_seconds,
            voices: [Self::EMPTY_VOICE; VOICES],
            filter: Self::default_filter(sr),
            smooth,
        }
    }

    fn add(&mut self, precomp: f32, decay: f32, attack_curve: A, release_curve: R) {
        let env = Envelope::new(
            self.sr as f32,
            self.latency_seconds - precomp,
            precomp,
            decay,
            attack_curve,
            release_curve,
        );

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

        self.voices[insertion_idx] = Some(env);
    }

    fn set_release(&mut self, release: f32) {
        for voice in &mut self.voices {
            let Some(voice) = voice else {
                continue;
            };

            voice.set_release(release);
        }
    }

    fn set_smooth(&mut self, smooth: bool) {
        if smooth == self.smooth {
            return;
        }

        self.smooth = smooth;
        // reset filter to avoid pops and clicks
        self.filter = Self::default_filter(self.sr);
    }

    fn tick(&mut self) -> f32 {
        // collect all envelope values into a single value
        let result = self
            .voices
            .iter_mut()
            .filter_map(|x| match x {
                Some(voice) => voice.tick(),
                None => None,
            })
            // use `max_by()` to get the highest envelope at this point.
            // if you want the envelopes to stack, use `sum()` instead.
            .max_by(|a, b| a.total_cmp(b))
            .unwrap_or(0.0);

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

        if !self.smooth {
            return result;
        }

        self.filter.process_sample(result as f64) as f32
    }
}

pub struct SaiSampler {
    params: Arc<SaiSamplerParams>,
    sr: f32,
    latency_seconds: f32,
    latency_samples: u32,
    current_slope: Slope,
    splitter_l: MultibandGainApplier,
    splitter_r: MultibandGainApplier,
    env_low: EnvelopeLane<EaseInSine, EaseInOutSine, 8>,
    env_mid: EnvelopeLane<EaseInSine, EaseInOutSine, 8>,
    env_high: EnvelopeLane<EaseInSine, EaseInOutSine, 8>,
    latency_buf: AllocRingBuffer<f32>,
}

impl Default for SaiSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(SaiSamplerParams::default()),
            // these fields are not initialised here, see `initialize()` for the actual values
            sr: 0.0,
            latency_seconds: 0.0,
            latency_samples: 0,
            current_slope: Slope::F24,
            splitter_l: MultibandGainApplier::ThreeBand24(MinimumThreeBand24Slope::new(
                0.0, 0.0, 0.0,
            )),
            splitter_r: MultibandGainApplier::ThreeBand24(MinimumThreeBand24Slope::new(
                0.0, 0.0, 0.0,
            )),
            latency_buf: AllocRingBuffer::new(1),
            env_low: EnvelopeLane::new(0.0, 0.0, false),
            env_mid: EnvelopeLane::new(0.0, 0.0, false),
            env_high: EnvelopeLane::new(0.0, 0.0, false),
        }
    }
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
struct SaiSamplerParams {
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

    // gain is scalar, 0.0 -- 1.0
    #[id = "low_gain"]
    pub(crate) low_gain: FloatParam,
    #[id = "mid_gain"]
    pub(crate) mid_gain: FloatParam,
    #[id = "high_gain"]
    pub(crate) high_gain: FloatParam,

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

impl Default for SaiSamplerParams {
    fn default() -> Self {
        Self {
            low_precomp: FloatParam::new(
                "Low precomp",
                10.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),
            mid_precomp: FloatParam::new(
                "Mid precomp",
                10.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),
            high_precomp: FloatParam::new(
                "High precomp",
                10.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
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

            low_gain: FloatParam::new(
                "Low gain reduction",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.2),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            mid_gain: FloatParam::new(
                "Mid gain reduction",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.2),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
            high_gain: FloatParam::new(
                "High gain reduction",
                db_to_gain(0.0),
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.2),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

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
                    max: 10.0,
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),

            bypass: BoolParam::new("Bypass", false),
            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(3))
                .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}

impl Plugin for SaiSampler {
    const NAME: &'static str = "SAI Sampler";
    const VENDOR: &'static str = "James Walker";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "your@email.com";

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
        self.sr = _buffer_config.sample_rate;

        // report latency
        const MAX_LATENCY_SECONDS: f32 = 0.01;
        let max_latency_samples = (MAX_LATENCY_SECONDS * self.sr).round() as usize;
        self.latency_buf = {
            // times 2 for 2 channels
            let mut buf = AllocRingBuffer::new(max_latency_samples * 2);
            buf.fill(0.0);
            buf
        };

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
        self.env_low = EnvelopeLane::new(
            self.sr.into(),
            self.latency_seconds,
            self.params.smoothing.value(),
        );
        self.env_mid = EnvelopeLane::new(
            self.sr.into(),
            self.latency_seconds,
            self.params.smoothing.value(),
        );
        self.env_high = EnvelopeLane::new(
            self.sr.into(),
            self.latency_seconds,
            self.params.smoothing.value(),
        );
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        debug_assert_eq!(buffer.channels(), 2);
        // handle if latency has changed
        {
            let new_lookahread = self.params.lookahead.value() / 1000.0;
            if new_lookahread != self.latency_seconds {
                self.latency_seconds = new_lookahread;
                self.latency_samples = (new_lookahread * self.sr).round() as u32;
                ctx.set_latency_samples(self.latency_samples);
            }
        }
        // handle crossover slope change
        {
            let new_slope = self.params.crossover_slope.value();
            if new_slope != self.current_slope {
                // replace splitters with new slopes
                self.current_slope = new_slope;
                match self.current_slope {
                    Slope::F24 => {
                        self.splitter_l = MultibandGainApplier::ThreeBand24(
                            MinimumThreeBand24Slope::new(1000.0, 2000.0, self.sr.into()),
                        );
                        self.splitter_r = MultibandGainApplier::ThreeBand24(
                            MinimumThreeBand24Slope::new(1000.0, 2000.0, self.sr.into()),
                        );
                    }
                    Slope::F12 => {
                        self.splitter_l = MultibandGainApplier::ThreeBand12(
                            MinimumThreeBand12Slope::new(1000.0, 2000.0, self.sr.into()),
                        );
                        self.splitter_r = MultibandGainApplier::ThreeBand12(
                            MinimumThreeBand12Slope::new(1000.0, 2000.0, self.sr.into()),
                        );
                    }
                }
            }
        }
        // handle smoothing change
        {
            let smoothing = self.params.smoothing.value();
            self.env_low.set_smooth(smoothing);
            self.env_mid.set_smooth(smoothing);
            self.env_high.set_smooth(smoothing);
        }

        let bypass = self.params.bypass.value();

        let mut next_event = ctx.next_event();

        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // update params
            let low_precomp = {
                let value = self.params.low_precomp.smoothed.next() / 1000.0;
                value.min(self.latency_seconds)
            };
            let mid_precomp = {
                let value = self.params.mid_precomp.smoothed.next() / 1000.0;
                value.min(self.latency_seconds)
            };
            let high_precomp = {
                let value = self.params.high_precomp.smoothed.next() / 1000.0;
                value.min(self.latency_seconds)
            };
            let low_decay = self.params.low_decay.smoothed.next() / 1000.0;
            let mid_decay = self.params.mid_decay.smoothed.next() / 1000.0;
            let high_decay = self.params.high_decay.smoothed.next() / 1000.0;
            let low_max_gain_db = -gain_to_db(self.params.low_gain.smoothed.next());
            let mid_max_gain_db = -gain_to_db(self.params.mid_gain.smoothed.next());
            let high_max_gain_db = -gain_to_db(self.params.high_gain.smoothed.next());
            let low_crossover = {
                let value = self.params.low_crossover.smoothed.next();
                // since high crossover will be 1 octave above this, this cannot be too high
                value.min(CROSSOVER_MAX_HZ / 2.0)
            };
            let high_crossover = {
                // limit high crossover to be 1 octave above low crossover
                // (this is pro-mb's behaviour)
                let value = self.params.high_crossover.smoothed.next();
                let min_value = low_crossover * 2.0;
                value.max(min_value)
            };
            let mix = self.params.mix.smoothed.next();

            // handle MIDI events
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }

                match event {
                    NoteEvent::NoteOn { note, .. } => {
                        self.env_low
                            .add(low_precomp, low_decay, EaseInSine, EaseInOutSine);
                        self.env_mid
                            .add(mid_precomp, mid_decay, EaseInSine, EaseInOutSine);
                        self.env_high
                            .add(high_precomp, high_decay, EaseInSine, EaseInOutSine);
                    }
                    _ => (),
                }

                next_event = ctx.next_event();
            }

            // update existing envelopes (if any)
            self.env_low.set_release(low_decay);
            self.env_mid.set_release(mid_decay);
            self.env_high.set_release(high_decay);

            // update filter frequency
            self.splitter_l
                .set_frequencies(low_crossover.into(), high_crossover.into());
            self.splitter_r
                .set_frequencies(low_crossover.into(), high_crossover.into());

            #[inline(always)]
            fn calculate_final_gain(env_val: f64, max_gain_db: f64, mix: f64) -> f32 {
                db_to_gain(-(max_gain_db * env_val * mix) as f32)
            }

            // tick envelopes and get gain value
            let low_gain;
            let mid_gain;
            let high_gain;
            if bypass {
                low_gain = 1.0;
                mid_gain = 1.0;
                high_gain = 1.0;
            } else {
                low_gain = calculate_final_gain(
                    self.env_low.tick() as f64,
                    low_max_gain_db as f64,
                    mix as f64,
                ) as f64;
                mid_gain = calculate_final_gain(
                    self.env_mid.tick() as f64,
                    mid_max_gain_db as f64,
                    mix as f64,
                ) as f64;
                high_gain = calculate_final_gain(
                    self.env_high.tick() as f64,
                    high_max_gain_db as f64,
                    mix as f64,
                ) as f64;
            }

            // left channel
            {
                let sample = channel_samples.get_mut(0).unwrap();

                // the sample from eons ago (the latency)
                let delayed_sample = *self.latency_buf.get(0).unwrap();
                // push sample to buffer queue
                self.latency_buf.push(*sample);
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
                let delayed_sample = *self.latency_buf.get(0).unwrap();
                // push sample to buffer queue
                self.latency_buf.push(*sample);
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
}

impl ClapPlugin for SaiSampler {
    const CLAP_ID: &'static str = "com.sai-audio.sai-sampler";
    const CLAP_DESCRIPTION: Option<&'static str> = None;
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for SaiSampler {
    const VST3_CLASS_ID: [u8; 16] = *b"WMbSpkNDqN0uignG";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(SaiSampler);
nih_export_vst3!(SaiSampler);
