mod oscillator;
mod voice;

use nih_plug::prelude::*;
use oscillator as osc;
use std::sync::Arc;
use voice::Voice;

struct SaiSampler {
    sample_rate: f32,
    params: Arc<SaiSamplerParams>,
    // 1 voice for each note
    voices: [Option<Voice<osc::Saw>>; 128],
}

#[derive(Params)]
struct SaiSamplerParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

const EMPTY_VOICE: Option<Voice<osc::Saw>> = None;

impl Default for SaiSampler {
    fn default() -> Self {
        Self {
            sample_rate: 1.0,
            params: Arc::new(SaiSamplerParams::default()),
            voices: [EMPTY_VOICE; 128],
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
        self.sample_rate = _buffer_config.sample_rate;

        true
    }

    fn reset(&mut self) {
        self.voices = [EMPTY_VOICE; 128];
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        while let Some(evt) = ctx.next_event() {
            match evt {
                NoteEvent::NoteOn {
                    timing,
                    voice_id,
                    channel,
                    note,
                    velocity,
                } => {
                    let f = 55.0 * 2.0_f64.powf((note - 33) as f64 / 12.0);
                    self.voices[note as usize] = Some(Voice::new(
                        osc::Saw::new(true),
                        self.sample_rate,
                        f as f32,
                        None,
                    ))
                }
                NoteEvent::NoteOff {
                    timing,
                    voice_id,
                    channel,
                    note,
                    velocity,
                } => self.voices[note as usize] = None,
                NoteEvent::Choke {
                    timing,
                    voice_id,
                    channel,
                    note,
                } => self.voices[note as usize] = None,
                // NoteEvent::MidiPitchBend {
                //     timing,
                //     channel,
                //     value,
                // } => (),
                _ => (),
            }
        }

        for channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();

            let x: f32 = self
                .voices
                .iter_mut()
                .filter_map(|x| x.as_mut().map(|voice| voice.tick()))
                .sum();
            for sample in channel_samples {
                *sample += x * gain;
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
