mod biquad;
mod filters;
mod oscillator;
mod voice;

use filters::{ButterworthLPF, LinkwitzRileyHPF, LinkwitzRileyLPF};
use nih_plug::prelude::*;
use std::sync::Arc;

struct SaiSampler {
    params: Arc<SaiSamplerParams>,
    lpf_l: LinkwitzRileyLPF,
    lpf_r: LinkwitzRileyLPF,
    hpf_l: LinkwitzRileyHPF,
    hpf_r: LinkwitzRileyHPF,
}

#[derive(Params)]
struct SaiSamplerParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for SaiSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(SaiSamplerParams::default()),
            // - Samplerate: 44100 Hz
            // - Freq: 600 Hz
            // - Q: 0.707 (Fixed)
            lpf_l: LinkwitzRileyLPF::new(1000.0, 44100.0),
            lpf_r: LinkwitzRileyLPF::new(1000.0, 44100.0),
            hpf_l: LinkwitzRileyHPF::new(1000.0, 44100.0),
            hpf_r: LinkwitzRileyHPF::new(1000.0, 44100.0),
        }
    }
}

impl Default for SaiSamplerParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                600.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            // .with_smoother(SmoothingStyle::Exponential(10.0))
            .with_unit(" Hz"),
            // .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            // .with_string_to_value(formatters::s2v_f32_gain_to_db()),
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
        debug_assert_eq!(buffer.channels(), 2);

        for mut channel_samples in buffer.iter_samples() {
            // update params
            let gain = self.params.gain.smoothed.next();
            self.lpf_l.set_frequency(gain as f64);
            self.lpf_r.set_frequency(gain as f64);
            self.hpf_l.set_frequency(gain as f64);
            self.hpf_r.set_frequency(gain as f64);

            // left channel
            {
                let sample = channel_samples.get_mut(0).unwrap();
                let hpf_sample = self.hpf_l.process_sample(*sample as f64);
                let lpf_sample = self.lpf_l.process_sample(*sample as f64);
                *sample = (lpf_sample - hpf_sample) as f32;
            }

            // right channel
            {
                let sample = channel_samples.get_mut(1).unwrap();
                let hpf_sample = self.hpf_r.process_sample(*sample as f64);
                let lpf_sample = self.lpf_r.process_sample(*sample as f64);
                *sample = (lpf_sample - hpf_sample) as f32;
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
