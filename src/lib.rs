mod biquad;
mod oscillator;
mod voice;

use biquad::Biquad;
use nih_plug::prelude::*;
use oscillator as osc;
use std::sync::Arc;
use voice::Voice;

struct SaiSampler {
    params: Arc<SaiSamplerParams>,
    filter_l: Biquad,
    filter_r: Biquad,
}

#[derive(Params)]
struct SaiSamplerParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

const EMPTY_VOICE: Option<Voice<osc::Sine>> = None;

impl Default for SaiSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(SaiSamplerParams::default()),
            // Coefficients calculated from https://arachnoid.com/BiQuadDesigner/index.html
            // - Samplerate: 44100 Hz
            // - Freq: 600 Hz
            // - Q: 0.707
            filter_l: Biquad::new(0.00172186, 0.00344372, 0.00172186, -1.87922368, 0.88611112),
            filter_r: Biquad::new(0.00172186, 0.00344372, 0.00172186, -1.87922368, 0.88611112),
        }
    }
}

impl Default for SaiSamplerParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-60.0),
                    max: util::db_to_gain(0.0),
                    factor: FloatRange::gain_skew_factor(-60.0, 0.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(20.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
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
        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        assert_eq!(buffer.channels(), 2);

        for (channel_idx, channel_samples) in buffer.iter_samples().enumerate() {
            // update params
            let gain = self.params.gain.smoothed.next();

            match channel_idx {
                0 => {
                    for sample in channel_samples {
                        *sample = self.filter_l.process_sample(*sample as f64) as f32;
                    }
                }
                1 => {
                    for sample in channel_samples {
                        *sample = self.filter_l.process_sample(*sample as f64) as f32;
                    }
                }
                _ => unreachable!("only 2 channels as input is supported"),
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
