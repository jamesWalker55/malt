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
use splitter::DynamicThreeBand24Slope;
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

pub struct SaiSampler {
    params: Arc<SaiSamplerParams>,
    sr: f32,
    latency_seconds: f32,
    latency_samples: u32,
    splitter_l: MultibandGainApplier,
    splitter_r: MultibandGainApplier,
    latency_buf: AllocRingBuffer<f32>,
    env: Option<Envelope<EaseInSine, EaseInOutSine>>,
    env_filter: FixedQFilter<FirstOrderLP>,
}

impl Default for SaiSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(SaiSamplerParams::default()),
            // these fields are not initialised here, see `initialize()` for the actual values
            sr: 0.0,
            latency_seconds: 0.0,
            latency_samples: 0,
            splitter_l: MultibandGainApplier::ThreeBand24(MinimumThreeBand24Slope::new(
                0.0, 0.0, 0.0,
            )),
            splitter_r: MultibandGainApplier::ThreeBand24(MinimumThreeBand24Slope::new(
                0.0, 0.0, 0.0,
            )),
            latency_buf: AllocRingBuffer::new(1),
            env: None,
            env_filter: FixedQFilter::new(0.0, 0.0),
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
            mix: FloatParam::new(
                "Mix",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            ),
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
        self.env = None;
        // a filter to smooth the envelope
        // at 600Hz it settles in about 2ms
        // switch to 1000Hz to settle in about 1ms
        self.env_filter = FixedQFilter::new(1000.0, self.sr.into());
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
            let low_gain = self.params.low_gain.smoothed.next() as f64;
            let mid_gain = self.params.mid_gain.smoothed.next() as f64;
            let high_gain = self.params.high_gain.smoothed.next() as f64;
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
            let crossover_slope = self.params.crossover_slope.value();
            let smoothing = self.params.smoothing.value();
            let lookahead = self.params.lookahead.smoothed.next();
            let bypass = self.params.bypass.value();
            let mix = self.params.mix.smoothed.next();

            // handle MIDI events
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }

                match event {
                    NoteEvent::NoteOn { note, .. } => {
                        self.env = Some(Envelope::new(
                            self.sr,
                            self.latency_seconds - low_precomp,
                            low_precomp,
                            low_decay,
                            EaseInSine,
                            EaseInOutSine,
                        ));
                    }
                    // NoteEvent::NoteOff { note, .. } => (),
                    // NoteEvent::Choke { note, .. } => (),
                    // NoteEvent::MidiPitchBend { value, .. } => (),
                    _ => (),
                }

                next_event = ctx.next_event();
            }

            // update existing envelopes (if any)
            if let Some(env) = &mut self.env {
                env.set_release(low_decay);
            }

            // update filter frequency
            self.splitter_l
                .set_frequencies(low_crossover.into(), high_crossover.into());
            self.splitter_r
                .set_frequencies(low_crossover.into(), high_crossover.into());

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

            // test process envelope
            let mut env_val = if let Some(env) = &mut self.env {
                let x = env.tick();
                if let Some(x) = x {
                    x
                } else {
                    // envelope has ended
                    self.env = None;
                    0.0
                }
            } else {
                0.0
            };
            env_val = self.env_filter.process_sample(env_val.into()) as f32;
            for sample in channel_samples {
                *sample = *sample * db_to_gain(env_val as f32);
                // *sample = *sample * env_val as f32;
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
