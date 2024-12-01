use super::knob::{Knob, KnobStyle};
use crate::{
    gui::{
        button::{custom_block_button, BlockButton, ButtonContent},
        knob::KnobDonutText,
        palette::{self as C},
    },
    Malt,
};
use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{
        self,
        text::{LayoutJob, TextWrapping},
        vec2, Align, CentralPanel, Color32, FontFamily, FontId, Painter, Pos2, Response, RichText,
        Spacing, Style, TextStyle, Ui, Vec2,
    },
    resizable_window::ResizableWindow,
    widgets,
};

// the DPI-independent size of the window
pub(crate) const GUI_DEFAULT_WIDTH: u32 = 651;
pub(crate) const GUI_DEFAULT_HEIGHT: u32 = 391;
pub(crate) const GUI_MINIMUM_WIDTH: u32 = 128;
pub(crate) const GUI_MINIMUM_HEIGHT: u32 = 128;

/// Rich text
fn rt(ui: &mut egui::Ui, text: impl Into<String>, family: &FontFamily, size: f32, color: Color32) {
    ui.label(
        egui::RichText::new(text)
            .family(family.clone())
            .size(size)
            .color(color),
    );
}

fn rt_obj(
    ui: &mut egui::Ui,
    text: impl Into<String>,
    family: &FontFamily,
    size: f32,
    color: Color32,
) -> egui::RichText {
    egui::RichText::new(text)
        .family(family.clone())
        .size(size)
        .color(color)
}

fn draw_texts(
    painter: &Painter,
    style: &Style,
    available_width: f32,
    mut position: Pos2,
    richtexts: impl IntoIterator<Item = RichText>,
) {
    let mut layout_job = LayoutJob::default();
    layout_job.wrap =
        TextWrapping::from_wrap_mode_and_width(egui::TextWrapMode::Truncate, available_width);
    layout_job.halign = Align::Center;

    for rt in richtexts {
        rt.append_to(
            &mut layout_job,
            &style,
            egui::FontSelection::Default,
            Align::Center,
        );
    }

    let galley = painter.layout_job(layout_job);

    position.y -= galley.rect.bottom() / 2.0;

    painter.galley(position, galley, Color32::RED);
}

struct UIState {
    help_enabled: bool,
}

impl UIState {
    fn new() -> Self {
        Self {
            help_enabled: false,
        }
    }
}

pub(crate) fn create_gui(
    plugin: &mut Malt,
    _async_executor: AsyncExecutor<Malt>,
) -> Option<Box<dyn Editor>> {
    let params = plugin.params.clone();
    let egui_state = plugin.params.editor_state.clone();
    create_egui_editor(
        plugin.params.editor_state.clone(),
        UIState::new(),
        |ctx, state| {
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

                // make background red to help identify places with no background
                style.visuals.panel_fill = Color32::RED;

                // disable item spacing, do everything manually
                style.spacing.item_spacing = Vec2::ZERO;

                style.interaction.selectable_labels = false;

                ctx.set_style(style);
            }

            // Enable loading image resources
            egui_extras::install_image_loaders(ctx);
        },
        move |ctx, setter, state| {
            ResizableWindow::new("resizable-window")
                .min_size(vec2(GUI_MINIMUM_WIDTH as f32, GUI_MINIMUM_HEIGHT as f32))
                .show(ctx, &egui_state, |ui| {
                    let header_frame = egui::Frame::none().fill(C::BG_DARK);
                    const HEADER_HEIGHT: f32 = 25.0;

                    // Header
                    egui::TopBottomPanel::top("header_panel")
                        .show_separator_line(false)
                        .exact_height(HEADER_HEIGHT)
                        .frame(header_frame.clone())
                        .show(ctx, |ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Left side
                                let left_side = |ui: &mut Ui| {
                                    ui.add_space(12.0);
                                    rt(ui, "sai audio", &C::FONT_NORMAL, C::TEXT_LARGE, C::FG_GREY);
                                    ui.add_space(10.0);
                                    rt(ui, "Malt", &C::FONT_BOLD, C::TEXT_LARGE, C::FG_WHITE);
                                };
                                // Right side
                                // Widgets must be inserted in reverse order here
                                let right_side = |ui: &mut Ui| {
                                    let res = if state.help_enabled {
                                        ui.add(BlockButton::new(
                                            ButtonContent::Text(
                                                "?",
                                                FontId::new(C::TEXT_BASE, C::FONT_BOLD.clone()),
                                            ),
                                            vec2(22.0, 25.0),
                                            C::BG_DARK,
                                            C::BG_DARK,
                                            C::BG_DARK,
                                            C::FG_GREEN,
                                            C::FG_GREEN.lerp_to_gamma(C::FG_WHITE, 0.1),
                                            C::FG_GREEN.lerp_to_gamma(C::BG_DARK, 0.2),
                                        ))
                                    } else {
                                        ui.add(BlockButton::new(
                                            ButtonContent::Text(
                                                "?",
                                                FontId::new(C::TEXT_BASE, C::FONT_BOLD.clone()),
                                            ),
                                            vec2(22.0, 25.0),
                                            C::FG_GREY,
                                            C::FG_GREY,
                                            C::FG_GREY.gamma_multiply(0.5),
                                            Color32::TRANSPARENT,
                                            C::FG_WHITE.gamma_multiply(0.1),
                                            Color32::TRANSPARENT,
                                        ))
                                    };
                                    if res.clicked() {
                                        state.help_enabled = !state.help_enabled;
                                    }
                                };

                                ui.allocate_ui_with_layout(
                                    egui::Vec2::new(0.0, HEADER_HEIGHT),
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    right_side,
                                );
                                ui.with_layout(
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    left_side,
                                );
                            })
                        });

                    // Footer
                    egui::TopBottomPanel::bottom("footer_panel")
                        .show_separator_line(false)
                        .exact_height(HEADER_HEIGHT)
                        .frame(header_frame.clone())
                        .show(ctx, |ui| {
                            ui.columns(5, |cols| {
                                let available_size = cols[0].available_size();

                                cols[0].add_sized(available_size, |ui: &mut Ui| -> Response {
                                    custom_block_button(
                                        ui,
                                        vec2(22.0, 22.0),
                                        Color32::TRANSPARENT,
                                        C::FG_WHITE.gamma_multiply(0.1),
                                        Color32::TRANSPARENT,
                                        |ui, res, painter, state| {
                                            draw_texts(
                                                &painter,
                                                &ui.style().clone(),
                                                available_size.x,
                                                res.rect.center(),
                                                [
                                                    rt_obj(
                                                        ui,
                                                        "Overlap: ",
                                                        &C::FONT_NORMAL,
                                                        C::TEXT_SM,
                                                        C::FG_GREY,
                                                    ),
                                                    rt_obj(
                                                        ui,
                                                        "Replace",
                                                        &C::FONT_NORMAL,
                                                        C::TEXT_SM,
                                                        C::FG_WHITE,
                                                    ),
                                                ],
                                            );
                                        },
                                    )
                                });
                                cols[1].add_sized(available_size, |ui: &mut Ui| -> Response {
                                    custom_block_button(
                                        ui,
                                        vec2(22.0, 22.0),
                                        Color32::TRANSPARENT,
                                        C::FG_WHITE.gamma_multiply(0.1),
                                        Color32::TRANSPARENT,
                                        |ui, res, painter, state| {
                                            draw_texts(
                                                &painter,
                                                &ui.style().clone(),
                                                available_size.x,
                                                res.rect.center(),
                                                [
                                                    rt_obj(
                                                        ui,
                                                        "Lookahead: ",
                                                        &C::FONT_NORMAL,
                                                        C::TEXT_SM,
                                                        C::FG_GREY,
                                                    ),
                                                    rt_obj(
                                                        ui,
                                                        "10ms",
                                                        &C::FONT_NORMAL,
                                                        C::TEXT_SM,
                                                        C::FG_WHITE,
                                                    ),
                                                ],
                                            );
                                        },
                                    )
                                });
                                cols[2].add_sized(available_size, |ui: &mut Ui| -> Response {
                                    custom_block_button(
                                        ui,
                                        vec2(22.0, 22.0),
                                        Color32::TRANSPARENT,
                                        C::FG_WHITE.gamma_multiply(0.1),
                                        Color32::TRANSPARENT,
                                        |ui, res, painter, state| {
                                            draw_texts(
                                                &painter,
                                                &ui.style().clone(),
                                                available_size.x,
                                                res.rect.center(),
                                                [
                                                    rt_obj(
                                                        ui,
                                                        "Smooth: ",
                                                        &C::FONT_NORMAL,
                                                        C::TEXT_SM,
                                                        C::FG_GREY,
                                                    ),
                                                    rt_obj(
                                                        ui,
                                                        "On",
                                                        &C::FONT_NORMAL,
                                                        C::TEXT_SM,
                                                        C::FG_WHITE,
                                                    ),
                                                ],
                                            );
                                        },
                                    )
                                });
                                cols[3].add_sized(available_size, |ui: &mut Ui| -> Response {
                                    custom_block_button(
                                        ui,
                                        vec2(22.0, 22.0),
                                        Color32::TRANSPARENT,
                                        C::FG_WHITE.gamma_multiply(0.1),
                                        Color32::TRANSPARENT,
                                        |ui, res, painter, state| {
                                            draw_texts(
                                                &painter,
                                                &ui.style().clone(),
                                                available_size.x,
                                                res.rect.center(),
                                                [rt_obj(
                                                    ui,
                                                    "Bypass",
                                                    &C::FONT_NORMAL,
                                                    C::TEXT_SM,
                                                    C::FG_WHITE,
                                                )],
                                            );
                                        },
                                    )
                                });
                                cols[4].add_sized(available_size, |ui: &mut Ui| -> Response {
                                    custom_block_button(
                                        ui,
                                        vec2(22.0, 22.0),
                                        Color32::TRANSPARENT,
                                        C::FG_WHITE.gamma_multiply(0.1),
                                        Color32::TRANSPARENT,
                                        |ui, res, painter, state| {
                                            draw_texts(
                                                &painter,
                                                &ui.style().clone(),
                                                available_size.x,
                                                res.rect.center(),
                                                [
                                                    rt_obj(
                                                        ui,
                                                        "Mix: ",
                                                        &C::FONT_NORMAL,
                                                        C::TEXT_SM,
                                                        C::FG_GREY,
                                                    ),
                                                    rt_obj(
                                                        ui,
                                                        "100%",
                                                        &C::FONT_NORMAL,
                                                        C::TEXT_SM,
                                                        C::FG_WHITE,
                                                    ),
                                                ],
                                            );
                                        },
                                    )
                                });
                            })
                        });

                    const BAND_WIDGET_WIDTH: f32 = 341.0;
                    const BAND_WIDGET_HEIGHT: f32 = 113.0;

                    // right-side controls (fixed width, variable height)
                    egui::SidePanel::right("controls_panel")
                        .exact_width(BAND_WIDGET_WIDTH)
                        .show_separator_line(false)
                        .resizable(false)
                        .frame(egui::Frame::none().fill(C::BG_LIGHT))
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
                                    highlight_color: C::FG_YELLOW,
                                    line_width: 2.0,
                                },
                            );
                            ui.add(knob);

                            let knob = Knob::for_param(
                                &params.channels[0].low_decay,
                                setter,
                                44.0,
                                KnobStyle::Analog {
                                    highlight_color: C::FG_PURPLE,
                                    line_width: 2.0,
                                },
                            );
                            ui.add(knob);

                            let knob = Knob::for_param(
                                &params.channels[0].low_decay,
                                setter,
                                15.0,
                                KnobStyle::Donut {
                                    line_width: 4.0,
                                    text: Some(KnobDonutText {
                                        spacing: 0.0,
                                        width: 70.0,
                                        font_id: FontId::new(C::TEXT_XS, C::FONT_NORMAL),
                                        color: C::FG_GREY,
                                    }),
                                },
                            );
                            ui.add(knob);

                            ui.add(BlockButton::new(
                                ButtonContent::Image(egui::include_image!("res/power.svg")),
                                vec2(22.0, 22.0),
                                Color32::WHITE,
                                Color32::WHITE,
                                Color32::from_white_alpha(128),
                                Color32::TRANSPARENT,
                                Color32::from_white_alpha(26),
                                Color32::from_white_alpha(26),
                            ));

                            ui.add(BlockButton::new(
                                ButtonContent::Text(
                                    "M",
                                    FontId::new(12.0, FontFamily::Name("bold".into())),
                                ),
                                vec2(22.0, 22.0),
                                Color32::WHITE,
                                Color32::WHITE,
                                Color32::from_white_alpha(128),
                                Color32::TRANSPARENT,
                                Color32::from_white_alpha(26),
                                Color32::from_white_alpha(26),
                            ));

                            ui.add(BlockButton::new(
                                ButtonContent::Text(
                                    "S",
                                    FontId::new(12.0, FontFamily::Name("bold".into())),
                                ),
                                vec2(22.0, 22.0),
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
