use crate::{widgets::ArcKnob, SaiSampler};
use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, Color32, Pos2, TextStyle},
    widgets,
};

// the DPI-independent size of the window
pub(crate) const GUI_WIDTH: u32 = 651;
pub(crate) const GUI_HEIGHT: u32 = 391;

pub(crate) fn create_gui(
    plugin: &mut SaiSampler,
    _async_executor: AsyncExecutor<SaiSampler>,
) -> Option<Box<dyn Editor>> {
    let params = plugin.params.clone();
    let peak_meter = plugin.peak_meter.clone();
    create_egui_editor(
        plugin.editor_state.clone(),
        (),
        |ctx, _| {
            // Load new fonts
            {
                use egui::{FontData, FontDefinitions, FontFamily};

                let mut fonts = FontDefinitions::empty();

                // Load font data
                fonts.font_data.insert(
                    "Inter".into(),
                    FontData::from_static(include_bytes!("../fonts/Inter-Regular.ttf")),
                );

                // Define font priority
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .push("Inter".into());

                ctx.set_fonts(fonts)
            }

            // Override GUI styling
            {
                use egui::FontFamily::Proportional;
                use egui::FontId;
                use egui::Style;
                use egui::TextStyle;
                use egui::Visuals;

                let mut style = (*ctx.style()).clone();

                // font sizes
                style.text_styles = [
                    (TextStyle::Heading, FontId::new(16.0, Proportional)),
                    (TextStyle::Body, FontId::new(11.0, Proportional)),
                    (TextStyle::Small, FontId::new(10.0, Proportional)),
                    (TextStyle::Button, FontId::new(12.0, Proportional)),
                ]
                .into();

                // color styling
                style.visuals.panel_fill = Color32::from_rgb(48, 48, 48);
                // style.wid

                ctx.set_style(style);
            }
        },
        move |ctx, setter, _state| {
            let header_frame = egui::Frame::none().fill(egui::Color32::from_rgb(17, 17, 17));
            const HEADER_HEIGHT: f32 = 25.0;

            // Header
            egui::TopBottomPanel::top("header_panel")
                .show_separator_line(false)
                .exact_height(HEADER_HEIGHT)
                .frame(header_frame.clone())
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Right side
                        ui.allocate_ui_with_layout(
                            egui::Vec2::new(0.0, HEADER_HEIGHT),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.label("?");
                                ui.label("right side:");
                            },
                        );
                        // Left side
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            {
                                let style = ui.style_mut();
                                style.override_text_style = Some(TextStyle::Heading);

                                ui.label("sai audio Malt");

                                ui.reset_style();
                            }
                            ui.label("This is the header again!");
                        });
                    })
                });

            // Footer
            egui::TopBottomPanel::bottom("footer_panel")
                .show_separator_line(false)
                .exact_height(HEADER_HEIGHT)
                .frame(header_frame.clone())
                .show(ctx, |ui| {
                    // Un-comment the surrounding code when you're able to implement window resizing

                    // ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    //     // Right side
                    //     ui.allocate_ui_with_layout(
                    //         egui::Vec2::new(0.0, HEADER_HEIGHT),
                    //         egui::Layout::right_to_left(egui::Align::Center),
                    //         |ui| {
                    //             ui.label("///");
                    //         },
                    //     );
                    //     // Left side
                    //     ui.with_layout(
                    //         egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    //         |ui| {
                    ui.columns(5, |cols| {
                        cols[0].centered_and_justified(|ui| ui.label("Trigger: MIDI"));
                        cols[1].centered_and_justified(|ui| ui.label("Lookahead: 10ms"));
                        cols[2].centered_and_justified(|ui| ui.label("Smooth: On"));
                        cols[3].centered_and_justified(|ui| ui.label("Bypass"));
                        cols[4].centered_and_justified(|ui| ui.label("Mix: 100%"));
                    })
                    //         },
                    //     );
                    // })
                });

            const BAND_WIDGET_WIDTH: f32 = 341.0;
            const BAND_WIDGET_HEIGHT: f32 = 113.0;

            // right-side controls (fixed width, variable height)
            egui::SidePanel::right("controls_panel")
                .exact_width(BAND_WIDGET_WIDTH)
                .show_separator_line(false)
                .resizable(false)
                .frame(egui::Frame::none().fill(Color32::from_rgb(48, 48, 48)))
                .show(ctx, |ui| {
                    // TODO: Handle 2-band or 1-band scenario

                    let rect = ui.max_rect();

                    // subtract 2 pixels (1px per divider line)
                    let band_height = (rect.height() - 2.0) / 3.0;
                    ui.label(format!("band_height: {:?}", band_height));

                    // let audio_module_3_knob = ArcKnob::for_param(
                    //     &params.gain_reduction,
                    //     setter,
                    //     28.0,
                    //     KnobLayout::Vertical,
                    // )
                    // .preset_style(KnobStyle::Preset1)
                    // .set_fill_color(Color32::from_rgb(70, 48, 48))
                    // .set_line_color(Color32::from_rgb(48, 200, 48))
                    // .set_text_size(11.0)
                    // .set_hover_text("Gain reduction".into());
                    // ui.add(audio_module_3_knob);
                    ui.add(ArcKnob::for_param(
                        &params.gain_reduction,
                        setter,
                        140.0 / 2.0 / 4.0,
                        Pos2::new(1029.0 / 4.0, 990.0 / 4.0),
                    ));
                });

            // left-side analyser (variable size)
            egui::CentralPanel::default().show(ctx, |ui| {
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
                // ui.label("low_crossover");
                // ui.add(widgets::ParamSlider::for_param(
                //     &params.low_crossover,
                //     setter,
                // ));
                // ui.label("high_crossover");
                // ui.add(widgets::ParamSlider::for_param(
                //     &params.high_crossover,
                //     setter,
                // ));
                // ui.label("low_gain");
                // ui.add(widgets::ParamSlider::for_param(&params.low_gain, setter));
                // ui.label("mid_gain");
                // ui.add(widgets::ParamSlider::for_param(&params.mid_gain, setter));
                // ui.label("high_gain");
                // ui.add(widgets::ParamSlider::for_param(&params.high_gain, setter));

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
