use super::knob::{Knob, KnobStyle};
use crate::{
    gui::{
        button::{custom_block_button, BlockButton, ButtonContent},
        knob::KnobDonutText,
        knobtext::KnobText,
        palette::{self as C},
    },
    MIDIProcessingMode, Malt,
};
use nih_plug::prelude::*;
use nih_plug_egui::{
    create_egui_editor,
    egui::{
        self,
        style::ScrollStyle,
        text::{LayoutJob, TextWrapping},
        vec2, Align, CentralPanel, Color32, Context, FontFamily, FontId, Id, Label, Layout,
        Painter, Pos2, Rect, Response, RichText, ScrollArea, Spacing, Style, TextStyle, Ui,
        UiBuilder, Vec2,
    },
    resizable_window::ResizableWindow,
    widgets::{self, ParamSlider},
};

// the DPI-independent size of the window
// pub(crate) const GUI_DEFAULT_WIDTH: u32 = 651;
// pub(crate) const GUI_DEFAULT_HEIGHT: u32 = 391;
// pub(crate) const GUI_MINIMUM_WIDTH: u32 = 128;
// pub(crate) const GUI_MINIMUM_HEIGHT: u32 = 128;

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

fn simple_block_button(
    ui: &mut Ui,
    active: bool,
    content: ButtonContent,
    size: Vec2,
    active_color: Color32,
    fg_color: Color32,
    bg_color: Color32,
) -> Response {
    if active {
        ui.add(BlockButton::new(
            content,
            size,
            bg_color,
            bg_color,
            bg_color,
            active_color,
            active_color.lerp_to_gamma(C::FG_WHITE, 0.2),
            active_color.lerp_to_gamma(C::BG_DARK, 0.2),
        ))
    } else {
        ui.add(BlockButton::new(
            content,
            size,
            fg_color,
            fg_color,
            fg_color.gamma_multiply(0.5),
            Color32::TRANSPARENT,
            C::FG_WHITE.gamma_multiply(0.1),
            Color32::TRANSPARENT,
        ))
    }
}

fn panel_band<'a, P: Param>(
    ui: &mut Ui,
    name: &'static str,
    precomp: &'a P,
    decay: &'a P,
    reduction: &'a P,
) {
    const BUTTON_SIZE: Vec2 = Vec2::splat(22.0);

    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
        let res = simple_block_button(
            ui,
            true, // TODO
            ButtonContent::Image(egui::include_image!("res/power.svg")),
            BUTTON_SIZE,
            C::FG_ORANGE,
            C::FG_GREY,
            C::BG_NORMAL,
        );
        if res.clicked() {
            nih_log!("Power!");
        }

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            let res = simple_block_button(
                ui,
                true, // TODO
                ButtonContent::Text("S", FontId::new(C::TEXT_BASE, C::FONT_BOLD.clone())),
                BUTTON_SIZE,
                C::FG_BLUE,
                C::FG_GREY,
                C::BG_NORMAL,
            );
            if res.clicked() {
                nih_log!("Solo!");
            }
            let res = simple_block_button(
                ui,
                true, // TODO
                ButtonContent::Text("M", FontId::new(C::TEXT_BASE, C::FONT_BOLD.clone())),
                BUTTON_SIZE,
                C::FG_RED,
                C::FG_GREY,
                C::BG_NORMAL,
            );
            if res.clicked() {
                nih_log!("Mute!");
            }
        });
    });
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.add(BlockButton::new(
            ButtonContent::Text("?", FontId::new(C::TEXT_BASE, C::FONT_BOLD.clone())),
            BUTTON_SIZE,
            C::BG_DARK,
            C::BG_DARK,
            C::BG_DARK,
            C::FG_GREEN,
            C::FG_GREEN.lerp_to_gamma(C::FG_WHITE, 0.1),
            C::FG_GREEN.lerp_to_gamma(C::BG_DARK, 0.2),
        ));
    });
}

// TEMP SIZES
pub(crate) const GUI_DEFAULT_WIDTH: u32 = 560;
pub(crate) const GUI_DEFAULT_HEIGHT: u32 = 350;
pub(crate) const GUI_MINIMUM_WIDTH: u32 = 560;
pub(crate) const GUI_MINIMUM_HEIGHT: u32 = 350;
/// TEMP GUI
pub(crate) fn create_gui(
    plugin: &mut Malt,
    _async_executor: AsyncExecutor<Malt>,
) -> Option<Box<dyn Editor>> {
    let params = plugin.params.clone();
    let egui_state = plugin.params.editor_state.clone();
    create_egui_editor(
        plugin.params.editor_state.clone(),
        (),
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
                use egui::TextStyle;

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
                    egui::SidePanel::left("left panel")
                        .exact_width(250.0)
                        .resizable(false)
                        .frame(egui::Frame::none().fill(C::BG_NORMAL))
                        .show(ctx, |ui| {
                            fn blockbutton_param<'a>(
                                ui: &mut Ui,
                                param: &BoolParam,
                                param_setter: &'a ParamSetter,
                                content: ButtonContent,
                                size: Vec2,
                                active_color: Color32,
                                fg_color: Color32,
                                bg_color: Color32,
                            ) -> Response {
                                let old_active = param.value();

                                let res = simple_block_button(
                                    ui,
                                    old_active,
                                    content,
                                    size,
                                    active_color,
                                    fg_color,
                                    bg_color,
                                );

                                if res.clicked() {
                                    param_setter.begin_set_parameter(param);
                                    param_setter.set_parameter(param, !old_active);
                                    param_setter.end_set_parameter(param);
                                }
                                res
                            }

                            // top bypass button
                            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                                blockbutton_param(
                                    ui,
                                    &params.bypass,
                                    setter,
                                    ButtonContent::Text(
                                        "Bypass",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(52.0, 22.0),
                                    C::FG_ORANGE,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                            });

                            // options section
                            rt(ui, "Options", &C::FONT_NORMAL, C::TEXT_BASE, C::FG_GREY);
                            blockbutton_param(
                                ui,
                                &params.smoothing,
                                setter,
                                ButtonContent::Text(
                                    "Smooth",
                                    FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                ),
                                vec2(52.0, 22.0),
                                C::FG_BLUE,
                                C::FG_WHITE,
                                C::BG_NORMAL,
                            );

                            ui.horizontal(|ui| {
                                rt(ui, "Lookahead", &C::FONT_NORMAL, C::TEXT_SM, C::FG_GREY);
                                ui.add(Knob::for_param(
                                    &params.lookahead,
                                    setter,
                                    24.0,
                                    KnobStyle::Analog {
                                        highlight_color: C::FG_YELLOW,
                                        line_width: 2.0,
                                    },
                                ));
                                ui.add(KnobText::for_param(
                                    &params.lookahead,
                                    setter,
                                    vec2(60.0, 24.0),
                                    FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                    C::FG_GREY,
                                    true,
                                    true,
                                    false,
                                ));
                            });

                            ui.horizontal(|ui| {
                                rt(ui, "Mix", &C::FONT_NORMAL, C::TEXT_SM, C::FG_GREY);
                                ui.add(Knob::for_param(
                                    &params.mix,
                                    setter,
                                    24.0,
                                    KnobStyle::Analog {
                                        highlight_color: C::FG_WHITE,
                                        line_width: 2.0,
                                    },
                                ));
                                ui.add(KnobText::for_param(
                                    &params.mix,
                                    setter,
                                    vec2(60.0, 24.0),
                                    FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                    C::FG_GREY,
                                    true,
                                    true,
                                    false,
                                ));
                            });
                            ui.horizontal(|ui| {
                                rt(ui, "MIDI Mode", &C::FONT_NORMAL, C::TEXT_SM, C::FG_GREY);
                                ui.add(ParamSlider::for_param(&params.midi_mode, setter));
                            });
                            if matches!(params.midi_mode.value(), MIDIProcessingMode::Pitch) {
                                ui.horizontal(|ui| {
                                    rt(ui, "Root note", &C::FONT_NORMAL, C::TEXT_SM, C::FG_GREY);
                                    ui.add(Knob::for_param(
                                        &params.midi_root_note,
                                        setter,
                                        24.0,
                                        KnobStyle::Analog {
                                            highlight_color: C::FG_WHITE,
                                            line_width: 2.0,
                                        },
                                    ));
                                    ui.add(KnobText::for_param(
                                        &params.midi_root_note,
                                        setter,
                                        vec2(60.0, 24.0),
                                        FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                        C::FG_GREY,
                                        true,
                                        true,
                                        false,
                                    ));
                                });
                            }

                            ui.separator();

                            // band splits section
                            rt(ui, "Band splits", &C::FONT_NORMAL, C::TEXT_BASE, C::FG_GREY);
                            ui.horizontal(|ui| {
                                rt(ui, "Slope", &C::FONT_NORMAL, C::TEXT_SM, C::FG_GREY);
                                ui.add(ParamSlider::for_param(&params.crossover_slope, setter));
                            });

                            ui.horizontal(|ui| {
                                ui.add(Knob::for_param(
                                    &params.high_crossover,
                                    setter,
                                    15.0,
                                    KnobStyle::Donut { line_width: 4.0 },
                                ));
                                ui.add(KnobText::for_param(
                                    &params.high_crossover,
                                    setter,
                                    vec2(70.0, 15.0),
                                    FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                    C::FG_WHITE,
                                    true,
                                    true,
                                    false,
                                ));
                            });

                            ui.horizontal(|ui| {
                                ui.add(Knob::for_param(
                                    &params.low_crossover,
                                    setter,
                                    15.0,
                                    KnobStyle::Donut { line_width: 4.0 },
                                ));
                                ui.add(KnobText::for_param(
                                    &params.low_crossover,
                                    setter,
                                    vec2(70.0, 15.0),
                                    FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                    C::FG_WHITE,
                                    true,
                                    true,
                                    false,
                                ));
                            });

                            ui.horizontal(|ui| {
                                rt(ui, "HIGH", &C::FONT_NORMAL, C::TEXT_SM, C::FG_GREY);

                                blockbutton_param(
                                    ui,
                                    &params.solo_high,
                                    setter,
                                    ButtonContent::Text(
                                        "S",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_BLUE,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                                blockbutton_param(
                                    ui,
                                    &params.mute_high,
                                    setter,
                                    ButtonContent::Text(
                                        "M",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_RED,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                                blockbutton_param(
                                    ui,
                                    &params.bypass_high,
                                    setter,
                                    ButtonContent::Text(
                                        "X",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_ORANGE,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                            });
                            ui.horizontal(|ui| {
                                rt(ui, "MID", &C::FONT_NORMAL, C::TEXT_SM, C::FG_GREY);

                                blockbutton_param(
                                    ui,
                                    &params.solo_mid,
                                    setter,
                                    ButtonContent::Text(
                                        "S",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_BLUE,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                                blockbutton_param(
                                    ui,
                                    &params.mute_mid,
                                    setter,
                                    ButtonContent::Text(
                                        "M",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_RED,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                                blockbutton_param(
                                    ui,
                                    &params.bypass_mid,
                                    setter,
                                    ButtonContent::Text(
                                        "X",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_ORANGE,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                            });
                            ui.horizontal(|ui| {
                                rt(ui, "LOW", &C::FONT_NORMAL, C::TEXT_SM, C::FG_GREY);

                                blockbutton_param(
                                    ui,
                                    &params.solo_low,
                                    setter,
                                    ButtonContent::Text(
                                        "S",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_BLUE,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                                blockbutton_param(
                                    ui,
                                    &params.mute_low,
                                    setter,
                                    ButtonContent::Text(
                                        "M",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_RED,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                                blockbutton_param(
                                    ui,
                                    &params.bypass_low,
                                    setter,
                                    ButtonContent::Text(
                                        "X",
                                        FontId::new(C::TEXT_BASE, C::FONT_NORMAL),
                                    ),
                                    vec2(22.0, 22.0),
                                    C::FG_ORANGE,
                                    C::FG_WHITE,
                                    C::BG_NORMAL,
                                );
                            });
                        });

                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(C::BG_NORMAL))
                        .show(ctx, |ui| {
                            ui.style_mut().spacing.scroll = ScrollStyle::solid();
                            ScrollArea::vertical().show(ui, |ui| {
                                // channels
                                let channel_count =
                                    if matches!(params.midi_mode.value(), MIDIProcessingMode::Omni)
                                    {
                                        1
                                    } else {
                                        16
                                    };

                                for i in 0..channel_count {
                                    let ch = &params.channels[i];

                                    rt(
                                        ui,
                                        format!("Channel {}", i),
                                        &C::FONT_NORMAL,
                                        C::TEXT_BASE,
                                        C::FG_GREY,
                                    );
                                    ui.horizontal(|ui| {
                                        ui.add(Knob::for_param(
                                            &ch.high_precomp,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_YELLOW,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.high_precomp,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));

                                        ui.add(Knob::for_param(
                                            &ch.high_decay,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_YELLOW,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.high_decay,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));

                                        ui.add(Knob::for_param(
                                            &ch.high_db,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_WHITE,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.high_db,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.add(Knob::for_param(
                                            &ch.mid_precomp,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_PURPLE,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.mid_precomp,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));

                                        ui.add(Knob::for_param(
                                            &ch.mid_decay,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_PURPLE,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.mid_decay,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));

                                        ui.add(Knob::for_param(
                                            &ch.mid_db,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_WHITE,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.mid_db,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.add(Knob::for_param(
                                            &ch.low_precomp,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_BLUE,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.low_precomp,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));

                                        ui.add(Knob::for_param(
                                            &ch.low_decay,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_BLUE,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.low_decay,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));

                                        ui.add(Knob::for_param(
                                            &ch.low_db,
                                            setter,
                                            24.0,
                                            KnobStyle::Analog {
                                                highlight_color: C::FG_WHITE,
                                                line_width: 2.0,
                                            },
                                        ));
                                        ui.add(KnobText::for_param(
                                            &ch.low_db,
                                            setter,
                                            vec2(60.0, 24.0),
                                            FontId::new(C::TEXT_SM, C::FONT_NORMAL),
                                            C::FG_GREY,
                                            true,
                                            true,
                                            false,
                                        ));
                                    });
                                }
                            });
                        });
                });
        },
    )
}

// pub(crate) fn create_gui(
//     plugin: &mut Malt,
//     _async_executor: AsyncExecutor<Malt>,
// ) -> Option<Box<dyn Editor>> {
//     let params = plugin.params.clone();
//     let egui_state = plugin.params.editor_state.clone();
//     create_egui_editor(
//         plugin.params.editor_state.clone(),
//         UIState::new(),
//         |ctx, state| {
//             // Load new fonts
//             {
//                 use egui::{FontData, FontDefinitions, FontFamily};

//                 let mut fonts = FontDefinitions::empty();

//                 // Load font data
//                 fonts.font_data.insert(
//                     "Inter".into(),
//                     FontData::from_static(include_bytes!("../../fonts/Inter-Regular.ttf")),
//                 );
//                 fonts.font_data.insert(
//                     "Inter Bold".into(),
//                     FontData::from_static(include_bytes!("../../fonts/Inter-Bold.ttf")),
//                 );

//                 // Define font priority
//                 fonts
//                     .families
//                     .entry(FontFamily::Proportional)
//                     .or_insert(Default::default())
//                     .push("Inter".into());
//                 fonts
//                     .families
//                     .entry(FontFamily::Name("bold".into()))
//                     .or_insert(Default::default())
//                     .push("Inter Bold".into());

//                 ctx.set_fonts(fonts)
//             }

//             // Override GUI styling
//             {
//                 use egui::FontFamily::Proportional;
//                 use egui::FontId;
//                 use egui::Style;
//                 use egui::TextStyle;
//                 use egui::Visuals;

//                 let mut style = (*ctx.style()).clone();

//                 // font sizes
//                 style.text_styles = [
//                     (TextStyle::Heading, FontId::new(16.0, Proportional)),
//                     (TextStyle::Body, FontId::new(11.0, Proportional)),
//                     (TextStyle::Small, FontId::new(10.0, Proportional)),
//                     (TextStyle::Button, FontId::new(12.0, Proportional)),
//                     // nih-plug's ParamSlider uses monospace for some reason,
//                     // need to add this or else ParamSlider will panic
//                     (TextStyle::Monospace, FontId::new(11.0, Proportional)),
//                 ]
//                 .into();

//                 // make background red to help identify places with no background
//                 style.visuals.panel_fill = Color32::RED;

//                 // disable item spacing, do everything manually
//                 style.spacing.item_spacing = Vec2::ZERO;

//                 style.interaction.selectable_labels = false;

//                 ctx.set_style(style);
//             }

//             // Enable loading image resources
//             egui_extras::install_image_loaders(ctx);
//         },
//         move |ctx, setter, state| {
//             ResizableWindow::new("resizable-window")
//                 .min_size(vec2(GUI_MINIMUM_WIDTH as f32, GUI_MINIMUM_HEIGHT as f32))
//                 .show(ctx, &egui_state, |ui| {
//                     let header_frame = egui::Frame::none().fill(C::BG_DARK);
//                     const HEADER_HEIGHT: f32 = 25.0;

//                     // Header
//                     egui::TopBottomPanel::top("header_panel")
//                         .show_separator_line(false)
//                         .exact_height(HEADER_HEIGHT)
//                         .frame(header_frame.clone())
//                         .show(ctx, |ui| {
//                             ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
//                                 // Left side
//                                 let left_side = |ui: &mut Ui| {
//                                     ui.add_space(12.0);
//                                     rt(ui, "sai audio", &C::FONT_NORMAL, C::TEXT_LARGE, C::FG_GREY);
//                                     ui.add_space(10.0);
//                                     rt(ui, "Malt", &C::FONT_BOLD, C::TEXT_LARGE, C::FG_WHITE);
//                                 };
//                                 // Right side
//                                 // Widgets must be inserted in reverse order here
//                                 let right_side = |ui: &mut Ui| {
//                                     let res = simple_block_button(
//                                         ui,
//                                         state.help_enabled,
//                                         ButtonContent::Text(
//                                             "?",
//                                             FontId::new(C::TEXT_BASE, C::FONT_BOLD.clone()),
//                                         ),
//                                         vec2(22.0, 25.0),
//                                         C::FG_GREEN,
//                                         C::FG_GREY,
//                                         C::BG_DARK,
//                                     );
//                                     if res.clicked() {
//                                         state.help_enabled = !state.help_enabled;
//                                     }
//                                 };

//                                 ui.with_layout(
//                                     egui::Layout::left_to_right(egui::Align::Center),
//                                     left_side,
//                                 );
//                                 ui.with_layout(
//                                     egui::Layout::right_to_left(egui::Align::Center),
//                                     right_side,
//                                 );
//                             })
//                         });

//                     // Footer
//                     egui::TopBottomPanel::bottom("footer_panel")
//                         .show_separator_line(false)
//                         .exact_height(HEADER_HEIGHT)
//                         .frame(header_frame.clone())
//                         .show(ctx, |ui| {
//                             ui.columns(5, |cols| {
//                                 let available_size = cols[0].available_size();

//                                 cols[0].add_sized(available_size, |ui: &mut Ui| -> Response {
//                                     custom_block_button(
//                                         ui,
//                                         vec2(22.0, 22.0),
//                                         Color32::TRANSPARENT,
//                                         C::FG_WHITE.gamma_multiply(0.1),
//                                         Color32::TRANSPARENT,
//                                         |ui, res, painter, state| {
//                                             draw_texts(
//                                                 &painter,
//                                                 &ui.style().clone(),
//                                                 available_size.x,
//                                                 res.rect.center(),
//                                                 [
//                                                     rt_obj(
//                                                         ui,
//                                                         "Overlap: ",
//                                                         &C::FONT_NORMAL,
//                                                         C::TEXT_SM,
//                                                         C::FG_GREY,
//                                                     ),
//                                                     rt_obj(
//                                                         ui,
//                                                         "Replace",
//                                                         &C::FONT_NORMAL,
//                                                         C::TEXT_SM,
//                                                         C::FG_WHITE,
//                                                     ),
//                                                 ],
//                                             );
//                                         },
//                                     )
//                                 });
//                                 cols[1].add_sized(available_size, |ui: &mut Ui| -> Response {
//                                     custom_block_button(
//                                         ui,
//                                         vec2(22.0, 22.0),
//                                         Color32::TRANSPARENT,
//                                         C::FG_WHITE.gamma_multiply(0.1),
//                                         Color32::TRANSPARENT,
//                                         |ui, res, painter, state| {
//                                             draw_texts(
//                                                 &painter,
//                                                 &ui.style().clone(),
//                                                 available_size.x,
//                                                 res.rect.center(),
//                                                 [
//                                                     rt_obj(
//                                                         ui,
//                                                         "Lookahead: ",
//                                                         &C::FONT_NORMAL,
//                                                         C::TEXT_SM,
//                                                         C::FG_GREY,
//                                                     ),
//                                                     rt_obj(
//                                                         ui,
//                                                         "10ms",
//                                                         &C::FONT_NORMAL,
//                                                         C::TEXT_SM,
//                                                         C::FG_WHITE,
//                                                     ),
//                                                 ],
//                                             );
//                                         },
//                                     )
//                                 });
//                                 cols[2].add_sized(available_size, |ui: &mut Ui| -> Response {
//                                     custom_block_button(
//                                         ui,
//                                         vec2(22.0, 22.0),
//                                         Color32::TRANSPARENT,
//                                         C::FG_WHITE.gamma_multiply(0.1),
//                                         Color32::TRANSPARENT,
//                                         |ui, res, painter, state| {
//                                             draw_texts(
//                                                 &painter,
//                                                 &ui.style().clone(),
//                                                 available_size.x,
//                                                 res.rect.center(),
//                                                 [
//                                                     rt_obj(
//                                                         ui,
//                                                         "Smooth: ",
//                                                         &C::FONT_NORMAL,
//                                                         C::TEXT_SM,
//                                                         C::FG_GREY,
//                                                     ),
//                                                     rt_obj(
//                                                         ui,
//                                                         "On",
//                                                         &C::FONT_NORMAL,
//                                                         C::TEXT_SM,
//                                                         C::FG_WHITE,
//                                                     ),
//                                                 ],
//                                             );
//                                         },
//                                     )
//                                 });
//                                 cols[3].add_sized(available_size, |ui: &mut Ui| -> Response {
//                                     custom_block_button(
//                                         ui,
//                                         vec2(22.0, 22.0),
//                                         Color32::TRANSPARENT,
//                                         C::FG_WHITE.gamma_multiply(0.1),
//                                         Color32::TRANSPARENT,
//                                         |ui, res, painter, state| {
//                                             draw_texts(
//                                                 &painter,
//                                                 &ui.style().clone(),
//                                                 available_size.x,
//                                                 res.rect.center(),
//                                                 [rt_obj(
//                                                     ui,
//                                                     "Bypass",
//                                                     &C::FONT_NORMAL,
//                                                     C::TEXT_SM,
//                                                     C::FG_WHITE,
//                                                 )],
//                                             );
//                                         },
//                                     )
//                                 });
//                                 cols[4].add_sized(available_size, |ui: &mut Ui| -> Response {
//                                     custom_block_button(
//                                         ui,
//                                         vec2(22.0, 22.0),
//                                         Color32::TRANSPARENT,
//                                         C::FG_WHITE.gamma_multiply(0.1),
//                                         Color32::TRANSPARENT,
//                                         |ui, res, painter, state| {
//                                             draw_texts(
//                                                 &painter,
//                                                 &ui.style().clone(),
//                                                 available_size.x,
//                                                 res.rect.center(),
//                                                 [
//                                                     rt_obj(
//                                                         ui,
//                                                         "Mix: ",
//                                                         &C::FONT_NORMAL,
//                                                         C::TEXT_SM,
//                                                         C::FG_GREY,
//                                                     ),
//                                                     rt_obj(
//                                                         ui,
//                                                         "100%",
//                                                         &C::FONT_NORMAL,
//                                                         C::TEXT_SM,
//                                                         C::FG_WHITE,
//                                                     ),
//                                                 ],
//                                             );
//                                         },
//                                     )
//                                 });
//                             })
//                         });

//                     const BAND_WIDGET_WIDTH: f32 = 248.0;
//                     const BAND_WIDGET_HEIGHT: f32 = 113.0;

//                     // right-side controls (fixed width, variable height)
//                     egui::SidePanel::right("controls_panel")
//                         .exact_width(BAND_WIDGET_WIDTH)
//                         .show_separator_line(false)
//                         .resizable(false)
//                         .frame(egui::Frame::none().fill(C::BG_LIGHT))
//                         .show(ctx, |ui| {
//                             // TODO: Handle 2-band or 1-band scenario

//                             let rect = ui.max_rect();

//                             // subtract 2 pixels (1px per divider line)
//                             let band_height = (rect.height() - 2.0) / 3.0;

//                             ui.allocate_ui_with_layout(
//                                 vec2(BAND_WIDGET_WIDTH, band_height),
//                                 Layout::left_to_right(Align::Center),
//                                 |ui| {
//                                     panel_band(
//                                         ui,
//                                         "HIGH",
//                                         &params.channels[0].high_precomp,
//                                         &params.channels[0].high_decay,
//                                         &params.channels[0].high_db,
//                                     )
//                                 },
//                             );

//                             ui.label(format!("band_height: {:?}", band_height));

//                             let knob = Knob::for_param(
//                                 &params.channels[0].low_db,
//                                 setter,
//                                 34.0,
//                                 KnobStyle::Analog {
//                                     highlight_color: C::FG_YELLOW,
//                                     line_width: 2.0,
//                                 },
//                             );
//                             ui.add(knob);

//                             let knob = Knob::for_param(
//                                 &params.channels[0].low_decay,
//                                 setter,
//                                 44.0,
//                                 KnobStyle::Analog {
//                                     highlight_color: C::FG_PURPLE,
//                                     line_width: 2.0,
//                                 },
//                             );
//                             ui.add(knob);

//                             let knob = Knob::for_param(
//                                 &params.channels[0].low_decay,
//                                 setter,
//                                 15.0,
//                                 KnobStyle::Donut {
//                                     line_width: 4.0,
//                                     text: Some(KnobDonutText {
//                                         spacing: 0.0,
//                                         width: 70.0,
//                                         font_id: FontId::new(C::TEXT_XS, C::FONT_NORMAL),
//                                         color: C::FG_GREY,
//                                     }),
//                                 },
//                             );
//                             ui.add(knob);

//                             let knob = KnobText::for_param(
//                                 &params.low_crossover,
//                                 setter,
//                                 vec2(100.0, 30.0),
//                                 FontId::new(C::TEXT_XS, C::FONT_NORMAL),
//                                 C::FG_GREY,
//                                 true,
//                                 true,
//                                 false,
//                             );
//                             ui.add(knob);

//                             ui.add(BlockButton::new(
//                                 ButtonContent::Text(
//                                     "M",
//                                     FontId::new(12.0, FontFamily::Name("bold".into())),
//                                 ),
//                                 vec2(22.0, 22.0),
//                                 Color32::WHITE,
//                                 Color32::WHITE,
//                                 Color32::from_white_alpha(128),
//                                 Color32::TRANSPARENT,
//                                 Color32::from_white_alpha(26),
//                                 Color32::from_white_alpha(26),
//                             ));

//                             ui.add(BlockButton::new(
//                                 ButtonContent::Text(
//                                     "S",
//                                     FontId::new(12.0, FontFamily::Name("bold".into())),
//                                 ),
//                                 vec2(22.0, 22.0),
//                                 Color32::WHITE,
//                                 Color32::WHITE,
//                                 Color32::from_white_alpha(128),
//                                 Color32::TRANSPARENT,
//                                 Color32::from_white_alpha(26),
//                                 Color32::from_white_alpha(26),
//                             ));

//                             if ui.button("Hello").clicked() {
//                                 nih_log!("Hello");
//                             }
//                         });

//                     // left-side analyser (variable size)
//                     egui::CentralPanel::default().show(ctx, |ui| {
//                         // TODO: Add a proper custom widget instead of reusing a progress bar
//                         // let peak_meter =
//                         //     util::gain_to_db(peak_meter.load(std::sync::atomic::Ordering::Relaxed));
//                         // let peak_meter_text = if peak_meter > util::MINUS_INFINITY_DB {
//                         //     format!("{peak_meter:.1} dBFS")
//                         // } else {
//                         //     String::from("-inf dBFS")
//                         // };

//                         // let peak_meter_normalized = (peak_meter + 60.0) / 60.0;
//                         // ui.allocate_space(egui::Vec2::splat(2.0));
//                         // ui.add(
//                         //     egui::widgets::ProgressBar::new(peak_meter_normalized).text(peak_meter_text),
//                         // );

//                         // This is a fancy widget that can get all the information it needs to properly
//                         // display and modify the parameter from the parametr itself
//                         // It's not yet fully implemented, as the text is missing.
//                         ui.label("gain_reduction");
//                         ui.add(widgets::ParamSlider::for_param(
//                             &params.channels[0].low_db,
//                             setter,
//                         ));
//                         ui.label("precomp");
//                         ui.add(widgets::ParamSlider::for_param(
//                             &params.channels[0].low_precomp,
//                             setter,
//                         ));
//                         ui.label("release");
//                         ui.add(widgets::ParamSlider::for_param(
//                             &params.channels[0].low_decay,
//                             setter,
//                         ));
//                         // ui.label("low_crossover");
//                         // ui.add(widgets::ParamSlider::for_param(
//                         //     &params.low_crossover,
//                         //     setter,
//                         // ));
//                         // ui.label("high_crossover");
//                         // ui.add(widgets::ParamSlider::for_param(
//                         //     &params.high_crossover,
//                         //     setter,
//                         // ));
//                         // ui.label("low_gain");
//                         // ui.add(widgets::ParamSlider::for_param(&params.low_gain, setter));
//                         // ui.label("mid_gain");
//                         // ui.add(widgets::ParamSlider::for_param(&params.mid_gain, setter));
//                         // ui.label("high_gain");
//                         // ui.add(widgets::ParamSlider::for_param(&params.high_gain, setter));

//                         // ui.label(
//                         //     "Also gain, but with a lame widget. Can't even render the value correctly!",
//                         // );
//                         // // This is a simple naieve version of a parameter slider that's not aware of how
//                         // // the parameters work
//                         // ui.add(
//                         //     egui::widgets::Slider::from_get_set(-30.0..=30.0, |new_value| {
//                         //         match new_value {
//                         //             Some(new_value_db) => {
//                         //                 let new_value = util::gain_to_db(new_value_db as f32);

//                         //                 setter.begin_set_parameter(&params.gain);
//                         //                 setter.set_parameter(&params.gain, new_value);
//                         //                 setter.end_set_parameter(&params.gain);

//                         //                 new_value_db
//                         //             }
//                         //             None => util::gain_to_db(params.gain.value()) as f64,
//                         //         }
//                         //     })
//                         //     .suffix(" dB"),
//                         // );
//                     });
//                 });
//         },
//     )
// }
