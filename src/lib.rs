mod voice;

use nih_plug::prelude::*;
use std::sync::Arc;
use voice::{Sine, Voice};

struct SaiSampler {
    params: Arc<SaiSamplerParams>,
    voice: Voice<Sine>,
}

#[derive(Params)]
struct SaiSamplerParams {
    #[id = "gain"]
    pub gain: FloatParam,
    #[id = "freq"]
    pub freq: FloatParam,
}

impl Default for SaiSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(SaiSamplerParams::default()),
            voice: Voice::new(Sine, 100.0, 50.0, None),
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
            freq: FloatParam::new(
                "Frequency",
                440.0,
                FloatRange::Skewed {
                    min: 100.0,
                    max: 20_000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_smoother(SmoothingStyle::Exponential(10.0))
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(2))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
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
        main_input_channels: None,
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
        self.voice.set_samplerate(_buffer_config.sample_rate);
        self.voice.set_frequency(10_000.0);

        true
    }

    fn reset(&mut self) {
        self.voice.reset();
    }

    fn process(
        &mut self,
        mut buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let gain = self.params.gain.smoothed.next();
        let freq = self.params.freq.smoothed.next();
        self.voice.set_frequency(freq);
        self.voice.fill(&mut buffer);

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
