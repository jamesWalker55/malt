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
use envelope::Envelope;
use nih_plug::{buffer::ChannelSamples, prelude::*};
use nih_plug_egui::{create_egui_editor, egui, widgets, EguiState};
use parameter_formatters::{s2v_f32_ms_then_s, v2s_f32_ms_then_s};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use splitter::MinimumTwoBand24Slope;
use std::sync::Arc;
use util::{db_to_gain, gain_to_db};

pub struct SaiSampler {
    params: Arc<SaiSamplerParams>,
    sr: f32,
    latency_seconds: f32,
    latency_samples: u32,
    splitter_l: MinimumTwoBand24Slope,
    splitter_r: MinimumTwoBand24Slope,
    buf: AllocRingBuffer<f32>,
    env: Option<Envelope>,
    env_filter: FixedQFilter<FirstOrderLP>,

    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    // #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
    /// The current data for the peak meter. This is stored as an [`Arc`] so we can share it between
    /// the GUI and the audio processing parts. If you have more state to share, then it's a good
    /// idea to put all of that in a struct behind a single `Arc`.
    ///
    /// This is stored as voltage gain.
    peak_meter: Arc<AtomicF32>,
}

impl Default for SaiSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(SaiSamplerParams::default()),
            // these fields are not initialised here, see `initialize()` for the actual values
            sr: 0.0,
            latency_seconds: 0.0,
            latency_samples: 0,
            splitter_l: MinimumTwoBand24Slope::new(0.0, 0.0),
            splitter_r: MinimumTwoBand24Slope::new(0.0, 0.0),
            buf: AllocRingBuffer::new(1),
            env: None,
            env_filter: FixedQFilter::new(0.0, 0.0),
            // TEMP
            peak_meter: Arc::new(AtomicF32::new(util::MINUS_INFINITY_DB)),
            editor_state: EguiState::from_size(300, 400),
        }
    }
}

#[derive(Params)]
struct SaiSamplerParams {
    #[id = "gain_reduction"]
    pub gain_reduction: FloatParam,
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
            gain_reduction: FloatParam::new(
                "Gain Reduction",
                db_to_gain(-30.0),
                FloatRange::Skewed {
                    min: 0.0,
                    max: 1.0,
                    factor: FloatRange::skew_factor(-1.2),
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
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
        self.splitter_l = MinimumTwoBand24Slope::new(1000.0, self.sr.into());
        self.splitter_r = MinimumTwoBand24Slope::new(1000.0, self.sr.into());

        // clear envelope
        self.env = None;
        // a filter to smooth the envelope
        // at 600Hz it settles in about 2ms
        // switch to 1000Hz to settle in about 1ms
        self.env_filter = FixedQFilter::new(1000.0, self.sr.into());

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
            // GUI-specific variables
            let mut amplitude = 0.0;

            // update params
            let gain_reduction_db = gain_to_db(self.params.gain_reduction.smoothed.next());
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
            self.splitter_l.set_frequency(low_crossover.into());
            self.splitter_r.set_frequency(low_crossover.into());

            // left channel
            {
                let sample = channel_samples.get_mut(0).unwrap();

                // the sample from eons ago (the latency)
                let delayed_sample = *self.buf.get(0).unwrap();
                // push sample to buffer queue
                self.buf.push(*sample);
                // *sample = delayed_sample;

                // process delayed sample
                *sample =
                    self.splitter_l
                        .apply_gain(delayed_sample as f64, &[1.0, 1.0]) as f32;
            }

            // right channel
            {
                let sample = channel_samples.get_mut(1).unwrap();

                // the sample from eons ago (the latency)
                let delayed_sample = *self.buf.get(0).unwrap();
                // push sample to buffer queue
                self.buf.push(*sample);
                // *sample = delayed_sample;

                // process delayed sample
                *sample =
                    self.splitter_r
                        .apply_gain(delayed_sample as f64, &[1.0, 1.0]) as f32;
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
                *sample = *sample * db_to_gain((env_val as f32) * gain_reduction_db);
                amplitude += sample.abs();
                // *sample = *sample * env_val as f32;
            }

            // GUI-specific code
            if self.editor_state.is_open() {
                // divide by 2 channels
                amplitude = amplitude / 2.0;
                let current_peak_meter = self.peak_meter.load(std::sync::atomic::Ordering::Relaxed);
                const PEAK_METER_DECAY_MS: f64 = 150.0;
                let peak_meter_decay_weight =
                    0.25f64.powf((self.sr as f64 * PEAK_METER_DECAY_MS / 1000.0).recip()) as f32;
                let new_peak_meter = if amplitude > current_peak_meter {
                    amplitude
                } else {
                    current_peak_meter * peak_meter_decay_weight
                        + amplitude * (1.0 - peak_meter_decay_weight)
                };

                self.peak_meter
                    .store(new_peak_meter, std::sync::atomic::Ordering::Relaxed)
            }
        }

        ProcessStatus::Normal
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let peak_meter = self.peak_meter.clone();
        create_egui_editor(
            self.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    // NOTE: See `plugins/diopser/src/editor.rs` for an example using the generic UI widget

                    // TODO: Add a proper custom widget instead of reusing a progress bar
                    let peak_meter =
                        util::gain_to_db(peak_meter.load(std::sync::atomic::Ordering::Relaxed));
                    let peak_meter_text = if peak_meter > util::MINUS_INFINITY_DB {
                        format!("{peak_meter:.1} dBFS")
                    } else {
                        String::from("-inf dBFS")
                    };

                    let peak_meter_normalized = (peak_meter + 60.0) / 60.0;
                    ui.allocate_space(egui::Vec2::splat(2.0));
                    ui.add(
                        egui::widgets::ProgressBar::new(peak_meter_normalized)
                            .text(peak_meter_text),
                    );

                    // This is a fancy widget that can get all the information it needs to properly
                    // display and modify the parameter from the parametr itself
                    // It's not yet fully implemented, as the text is missing.
                    ui.label("gain_reduction");
                    ui.add(widgets::ParamSlider::for_param(
                        &params.gain_reduction,
                        setter,
                    ));
                    ui.label("precomp");
                    ui.add(widgets::ParamSlider::for_param(&params.precomp, setter));
                    ui.label("release");
                    ui.add(widgets::ParamSlider::for_param(&params.release, setter));
                    ui.label("low_crossover");
                    ui.add(widgets::ParamSlider::for_param(
                        &params.low_crossover,
                        setter,
                    ));
                    ui.label("high_crossover");
                    ui.add(widgets::ParamSlider::for_param(
                        &params.high_crossover,
                        setter,
                    ));

                    // ui.label(
                    //     "Also gain, but with a lame widget. Can't even render the value correctly!",
                    // );
                    // // This is a simple naieve version of a parameter slider that's not aware of how
                    // // the parameters work
                    // ui.add(
                    //     egui::widgets::Slider::from_get_set(-30.0..=30.0, |new_value| {
                    //         match new_value {
                    //             Some(new_value_db) => {
                    //                 let new_value = util::gain_to_db(new_value_db as f32);

                    //                 setter.begin_set_parameter(&params.gain);
                    //                 setter.set_parameter(&params.gain, new_value);
                    //                 setter.end_set_parameter(&params.gain);

                    //                 new_value_db
                    //             }
                    //             None => util::gain_to_db(params.gain.value()) as f64,
                    //         }
                    //     })
                    //     .suffix(" dB"),
                    // );
                });
            },
        )
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
