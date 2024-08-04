use crate::SaiSampler;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets};

// the DPI-independent size of the window
pub(crate) const GUI_WIDTH: u32 = 700;
pub(crate) const GUI_HEIGHT: u32 = 700;

pub(crate) fn create_gui(
    plugin: &mut SaiSampler,
    _async_executor: AsyncExecutor<SaiSampler>,
) -> Option<Box<dyn Editor>> {
    let params = plugin.params.clone();
    let peak_meter = plugin.peak_meter.clone();
    create_egui_editor(
        plugin.editor_state.clone(),
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
                    egui::widgets::ProgressBar::new(peak_meter_normalized).text(peak_meter_text),
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
                ui.label("low_gain");
                ui.add(widgets::ParamSlider::for_param(&params.low_gain, setter));
                ui.label("mid_gain");
                ui.add(widgets::ParamSlider::for_param(&params.mid_gain, setter));
                ui.label("high_gain");
                ui.add(widgets::ParamSlider::for_param(&params.high_gain, setter));

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
