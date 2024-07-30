mod biquad;
mod envelope;
mod filters;
mod oscillator;
mod parameter_formatters;
mod voice;

use biquad::Precision;
use envelope::Envelope;
use filters::{ButterworthLPF, FirstOrderLPF, LinkwitzRileyHPF, LinkwitzRileyLPF};
use nih_plug::{buffer::ChannelSamples, prelude::*};
use parameter_formatters::{s2v_f32_ms_then_s, v2s_f32_ms_then_s};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use std::sync::Arc;
use util::db_to_gain;

struct SaiSampler {
    params: Arc<SaiSamplerParams>,
    sr: f32,
    latency_seconds: f32,
    latency_samples: u32,
    lpf_l: LinkwitzRileyLPF,
    lpf_r: LinkwitzRileyLPF,
    hpf_l: LinkwitzRileyHPF,
    hpf_r: LinkwitzRileyHPF,
    buf: AllocRingBuffer<f32>,
    env: Option<Envelope>,
    env_filter: FirstOrderLPF,
}

impl Default for SaiSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(SaiSamplerParams::default()),
            // these fields are not initialised here, see `initialize()` for the actual values
            sr: 0.0,
            latency_seconds: 0.0,
            latency_samples: 0,
            lpf_l: LinkwitzRileyLPF::new(0.0, 0.0),
            lpf_r: LinkwitzRileyLPF::new(0.0, 0.0),
            hpf_l: LinkwitzRileyHPF::new(0.0, 0.0),
            hpf_r: LinkwitzRileyHPF::new(0.0, 0.0),
            buf: AllocRingBuffer::new(1),
            env: None,
            env_filter: FirstOrderLPF::new(0.0, 0.0),
        }
    }
}

#[derive(Params)]
struct SaiSamplerParams {
    #[id = "precomp"]
    pub precomp: FloatParam,
    #[id = "release"]
    pub release: FloatParam,
    #[id = "low_crossover"]
    pub low_crossover: FloatParam,
    #[id = "high_crossover"]
    pub high_crossover: FloatParam,
}

impl Default for SaiSamplerParams {
    fn default() -> Self {
        Self {
            precomp: FloatParam::new(
                "Precomp",
                10.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_value_to_string(v2s_f32_ms_then_s(3))
            .with_string_to_value(s2v_f32_ms_then_s()),
            release: FloatParam::new(
                "Release",
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
            low_crossover: FloatParam::new(
                "Low Crossover",
                120.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(3))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            high_crossover: FloatParam::new(
                "High Crossover",
                2500.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(3))
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
        const LATENCY_SECONDS: f32 = 0.01;
        self.latency_seconds = LATENCY_SECONDS;
        self.latency_samples = (LATENCY_SECONDS * self.sr).round() as u32;
        _context.set_latency_samples(self.latency_samples);

        // times 2 for 2 channels
        self.buf = {
            let mut buf = AllocRingBuffer::new((self.latency_samples * 2).try_into().unwrap());
            buf.fill(0.0);
            buf
        };

        // setup filters
        self.lpf_l = LinkwitzRileyLPF::new(1000.0, self.sr.into());
        self.lpf_r = LinkwitzRileyLPF::new(1000.0, self.sr.into());
        self.hpf_l = LinkwitzRileyHPF::new(1000.0, self.sr.into());
        self.hpf_r = LinkwitzRileyHPF::new(1000.0, self.sr.into());

        // clear envelope
        self.env = None;
        // a filter to smooth the envelope
        // at 600Hz it settles in about 2ms
        // switch to 1000Hz to settle in about 1ms
        self.env_filter = FirstOrderLPF::new(1000.0, self.sr.into());

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

        let mut next_event = ctx.next_event();

        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            // update params
            let precomp = self.params.precomp.smoothed.next() / 1000.0;
            let release = self.params.release.smoothed.next() / 1000.0;
            let low_crossover = self.params.low_crossover.smoothed.next();
            let high_crossover = self.params.high_crossover.smoothed.next();

            debug_assert!(precomp <= self.latency_seconds);

            // handle MIDI events
            while let Some(event) = next_event {
                if event.timing() != sample_id as u32 {
                    break;
                }

                match event {
                    NoteEvent::NoteOn { note, .. } => {
                        self.env = Some(Envelope::new(
                            self.sr,
                            self.latency_seconds - precomp,
                            precomp,
                            release,
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
                env.set_release(release);
            }

            // update filter frequency
            self.lpf_l.set_frequency(low_crossover as Precision);
            self.lpf_r.set_frequency(low_crossover as Precision);
            self.hpf_l.set_frequency(low_crossover as Precision);
            self.hpf_r.set_frequency(low_crossover as Precision);

            // left channel
            {
                let sample = channel_samples.get_mut(0).unwrap();

                // the sample from eons ago (the latency)
                let delayed_sample = *self.buf.get(0).unwrap();
                // push sample to buffer queue
                self.buf.push(*sample);

                // process delayed sample
                let hpf_sample = self.hpf_l.process_sample(delayed_sample as Precision);
                let lpf_sample = self.lpf_l.process_sample(delayed_sample as Precision);

                // return to sender
                *sample = (lpf_sample - hpf_sample) as f32;
            }

            // right channel
            {
                let sample = channel_samples.get_mut(1).unwrap();

                // the sample from eons ago (the latency)
                let delayed_sample = *self.buf.get(0).unwrap();
                // push sample to buffer queue
                self.buf.push(*sample);

                // process delayed sample
                let hpf_sample = self.hpf_r.process_sample(delayed_sample as Precision);
                let lpf_sample = self.lpf_r.process_sample(delayed_sample as Precision);

                // return to sender
                *sample = (lpf_sample - hpf_sample) as f32;
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
            env_val = self.env_filter.process_sample(env_val as Precision) as f32;
            for sample in channel_samples {
                *sample = *sample * env_val as f32;
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
