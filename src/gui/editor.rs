use super::knob::{Knob, KnobStyle};
use crate::{
    gui::button::{Button, ButtonContent},
    Malt,
};
use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{self, vec2, Color32, FontFamily, FontId, TextStyle},
    resizable_window::ResizableWindow,
    widgets,
};

// the DPI-independent size of the window
pub(crate) const GUI_DEFAULT_WIDTH: u32 = 651;
pub(crate) const GUI_DEFAULT_HEIGHT: u32 = 391;
pub(crate) const GUI_MINIMUM_WIDTH: u32 = 128;
pub(crate) const GUI_MINIMUM_HEIGHT: u32 = 128;

pub(crate) fn create_gui(
    plugin: &mut Malt,
    _async_executor: AsyncExecutor<Malt>,
) -> Option<Box<dyn Editor>> {
    let params = plugin.params.clone();
    let egui_state = plugin.params.editor_state.clone();
    create_egui_editor(
        plugin.params.editor_state.clone(),
        (),
        |ctx, _| {
            // Load new fonts
            {
                use egui::{FontData, FontDefinitions, FontFamily};

                let mut fonts = FontDefinitions::empty();

                // Load font data
                fonts.font_data.insert(
                    "Inter".into(),
                    FontData::from_static(include_bytes!("../../fonts/Inter-Regular.ttf")),
                );
                fonts.font_data.insert(
                    "Inter Bold".into(),
                    FontData::from_static(include_bytes!("../../fonts/Inter-Bold.ttf")),
                );

                // Define font priority
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_insert(Default::default())
                    .push("Inter".into());
                fonts
                    .families
                    .entry(FontFamily::Name("bold".into()))
                    .or_insert(Default::default())
                    .push("Inter Bold".into());

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
                    // nih-plug's ParamSlider uses monospace for some reason,
                    // need to add this or else ParamSlider will panic
                    (TextStyle::Monospace, FontId::new(11.0, Proportional)),
                ]
                .into();

                // color styling
                style.visuals.panel_fill = Color32::from_rgb(48, 48, 48);

                style.interaction.selectable_labels = false;

                ctx.set_style(style);
            }

            // Enable loading image resources
            egui_extras::install_image_loaders(ctx);
        },
        move |ctx, setter, _state| {
            ResizableWindow::new("resizable-window")
                .min_size(vec2(GUI_MINIMUM_WIDTH as f32, GUI_MINIMUM_HEIGHT as f32))
                .show(ctx, &egui_state, |ui| {
                    let header_frame =
                        egui::Frame::none().fill(egui::Color32::from_rgb(17, 17, 17));
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
                                        let help_btn = Button::new(
                                            ButtonContent::Text(
                                                "?",
                                                FontId::new(12.0, FontFamily::Name("bold".into())),
                                            ),
                                            22.0,
                                            22.0,
                                            Color32::WHITE,
                                            Color32::WHITE,
                                            Color32::from_white_alpha(128),
                                            Color32::TRANSPARENT,
                                            Color32::from_white_alpha(26),
                                            Color32::from_white_alpha(26),
                                        );
                                        if ui.add(help_btn).clicked() {
                                            nih_log!("Help!");
                                        }
                                        ui.label("right side:");
                                    },
                                );
                                // Left side
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        {
                                            let style = ui.style_mut();
                                            style.override_text_style = Some(TextStyle::Heading);

                                            ui.label("sai audio Malt");

                                            ui.reset_style();
                                        }
                                        ui.label("This is the header again!");
                                    },
                                );
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

                            let knob = Knob::for_param(
                                &params.channels[0].low_db,
                                setter,
                                34.0,
                                KnobStyle::Analog {
                                    highlight_color: Color32::from_rgb(255, 245, 157),
                                    line_width: 2.0,
                                },
                            );
                            ui.add(knob);

                            let knob = Knob::for_param(
                                &params.channels[0].low_decay,
                                setter,
                                44.0,
                                KnobStyle::Analog {
                                    highlight_color: Color32::from_rgb(206, 147, 216),
                                    line_width: 2.0,
                                },
                            );
                            ui.add(knob);

                            let knob = Knob::for_param(
                                &params.channels[0].low_decay,
                                setter,
                                15.0,
                                KnobStyle::Donut { line_width: 4.0 },
                            );
                            ui.add(knob);

                            ui.add(Button::new(
                                ButtonContent::Image(egui::include_image!("res/power.svg")),
                                22.0,
                                22.0,
                                Color32::WHITE,
                                Color32::WHITE,
                                Color32::from_white_alpha(128),
                                Color32::TRANSPARENT,
                                Color32::from_white_alpha(26),
                                Color32::from_white_alpha(26),
                            ));

                            ui.add(Button::new(
                                ButtonContent::Text(
                                    "M",
                                    FontId::new(12.0, FontFamily::Name("bold".into())),
                                ),
                                22.0,
                                22.0,
                                Color32::WHITE,
                                Color32::WHITE,
                                Color32::from_white_alpha(128),
                                Color32::TRANSPARENT,
                                Color32::from_white_alpha(26),
                                Color32::from_white_alpha(26),
                            ));

                            ui.add(Button::new(
                                ButtonContent::Text(
                                    "S",
                                    FontId::new(12.0, FontFamily::Name("bold".into())),
                                ),
                                22.0,
                                22.0,
                                Color32::WHITE,
                                Color32::WHITE,
                                Color32::from_white_alpha(128),
                                Color32::TRANSPARENT,
                                Color32::from_white_alpha(26),
                                Color32::from_white_alpha(26),
                            ));

                            if ui.button("Hello").clicked() {
                                nih_log!("Hello");
                            }
                        });

                    // left-side analyser (variable size)
                    egui::CentralPanel::default().show(ctx, |ui| {
                        // TODO: Add a proper custom widget instead of reusing a progress bar
                        // let peak_meter =
                        //     util::gain_to_db(peak_meter.load(std::sync::atomic::Ordering::Relaxed));
                        // let peak_meter_text = if peak_meter > util::MINUS_INFINITY_DB {
                        //     format!("{peak_meter:.1} dBFS")
                        // } else {
                        //     String::from("-inf dBFS")
                        // };

                        // let peak_meter_normalized = (peak_meter + 60.0) / 60.0;
                        // ui.allocate_space(egui::Vec2::splat(2.0));
                        // ui.add(
                        //     egui::widgets::ProgressBar::new(peak_meter_normalized).text(peak_meter_text),
                        // );

                        // This is a fancy widget that can get all the information it needs to properly
                        // display and modify the parameter from the parametr itself
                        // It's not yet fully implemented, as the text is missing.
                        ui.label("gain_reduction");
                        ui.add(widgets::ParamSlider::for_param(
                            &params.channels[0].low_db,
                            setter,
                        ));
                        ui.label("precomp");
                        ui.add(widgets::ParamSlider::for_param(
                            &params.channels[0].low_precomp,
                            setter,
                        ));
                        ui.label("release");
                        ui.add(widgets::ParamSlider::for_param(
                            &params.channels[0].low_decay,
                            setter,
                        ));
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
                });
        },
    )
}
