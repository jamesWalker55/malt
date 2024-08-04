// Ardura 2024 update - ui_knob.rs - egui + nih-plug parameter widget with customization
//  this ui_knob.rs is built off a2aaron's knob base as part of nyasynth and Robbert's ParamSlider code
// https://github.com/a2aaron/nyasynth/blob/canon/src/ui_knob.rs

use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    self,
    epaint::{CircleShape, PathShape},
    pos2, Align2, Color32, FontId, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, Ui, Vec2,
    Widget,
};
use once_cell::sync::Lazy;
use std::{
    f32::consts::TAU,
    ops::{Add, Mul, Sub},
};

/// When shift+dragging a parameter, one pixel dragged corresponds to this much change in the
/// noramlized parameter.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.001;
const NORMAL_DRAG_MULTIPLIER: f32 = 0.005;

static DRAG_NORMALIZED_START_VALUE_MEMORY_ID: Lazy<egui::Id> =
    Lazy::new(|| egui::Id::new((file!(), 0)));
static DRAG_AMOUNT_MEMORY_ID: Lazy<egui::Id> = Lazy::new(|| egui::Id::new((file!(), 1)));

struct SliderRegion<'a, P: Param> {
    param: &'a P,
    param_setter: &'a ParamSetter<'a>,
}

impl<'a, P: Param> SliderRegion<'a, P> {
    fn new(param: &'a P, param_setter: &'a ParamSetter) -> Self {
        SliderRegion {
            param,
            param_setter,
        }
    }

    fn set_normalized_value(&self, normalized: f32) {
        // This snaps to the nearest plain value if the parameter is stepped in some way.
        // TODO: As an optimization, we could add a `const CONTINUOUS: bool` to the parameter to
        //       avoid this normalized->plain->normalized conversion for parameters that don't need
        //       it
        let value = self.param.preview_plain(normalized);
        if value != self.plain_value() {
            self.param_setter.set_parameter(self.param, value);
        }
    }

    fn plain_value(&self) -> P::Plain {
        self.param.modulated_plain_value()
    }

    fn normalized_value(&self) -> f32 {
        self.param.modulated_normalized_value()
    }

    fn get_drag_normalized_start_value_memory(ui: &Ui) -> f32 {
        ui.memory(|mem| mem.data.get_temp(*DRAG_NORMALIZED_START_VALUE_MEMORY_ID))
            .unwrap_or(0.5)
    }

    fn set_drag_normalized_start_value_memory(ui: &Ui, amount: f32) {
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(*DRAG_NORMALIZED_START_VALUE_MEMORY_ID, amount)
        });
    }

    fn get_drag_amount_memory(ui: &Ui) -> f32 {
        ui.memory(|mem| mem.data.get_temp(*DRAG_AMOUNT_MEMORY_ID))
            .unwrap_or(0.0)
    }

    fn set_drag_amount_memory(ui: &Ui, amount: f32) {
        ui.memory_mut(|mem| mem.data.insert_temp(*DRAG_AMOUNT_MEMORY_ID, amount));
    }

    /// Begin and end drag still need to be called when using this..
    fn reset_param(&self) {
        self.param_setter
            .set_parameter(self.param, self.param.default_plain_value());
    }

    fn granular_drag(&self, ui: &Ui, drag_delta: Vec2) {
        // Remember the intial position when we started with the granular drag. This value gets
        // reset whenever we have a normal itneraction with the slider.
        let start_value = if Self::get_drag_amount_memory(ui) == 0.0 {
            Self::set_drag_normalized_start_value_memory(ui, self.normalized_value());
            self.normalized_value()
        } else {
            Self::get_drag_normalized_start_value_memory(ui)
        };

        let total_drag_distance = -drag_delta.y + Self::get_drag_amount_memory(ui);
        Self::set_drag_amount_memory(ui, total_drag_distance);

        self.set_normalized_value(
            (start_value + (total_drag_distance * GRANULAR_DRAG_MULTIPLIER)).clamp(0.0, 1.0),
        );
    }

    // Copied this to modify the normal drag behavior to not match a slider
    fn normal_drag(&self, ui: &Ui, drag_delta: Vec2) {
        let start_value = if Self::get_drag_amount_memory(ui) == 0.0 {
            Self::set_drag_normalized_start_value_memory(ui, self.normalized_value());
            self.normalized_value()
        } else {
            Self::get_drag_normalized_start_value_memory(ui)
        };

        let total_drag_distance = -drag_delta.y + Self::get_drag_amount_memory(ui);
        Self::set_drag_amount_memory(ui, total_drag_distance);

        self.set_normalized_value(
            (start_value + (total_drag_distance * NORMAL_DRAG_MULTIPLIER)).clamp(0.0, 1.0),
        );
    }

    // Handle the input for a given response. Returns an f32 containing the normalized value of
    // the parameter.
    fn handle_response(&self, ui: &Ui, response: &mut Response) -> f32 {
        // This has been replaced with the ParamSlider/CustomParamSlider structure and supporting
        // functions (above) since that was still working in egui 0.22

        if response.drag_started() {
            // When beginning a drag or dragging normally, reset the memory used to keep track of
            // our granular drag
            self.param_setter.begin_set_parameter(self.param);
            Self::set_drag_amount_memory(ui, 0.0);
        }
        if let Some(_clicked_pos) = response.interact_pointer_pos() {
            if ui.input(|mem| mem.modifiers.command) {
                // Like double clicking, Ctrl+Click should reset the parameter
                self.reset_param();
                response.mark_changed();
            } else if ui.input(|mem| mem.modifiers.shift) {
                // And shift dragging should switch to a more granular input method
                self.granular_drag(ui, response.drag_delta());
                response.mark_changed();
            } else {
                self.normal_drag(ui, response.drag_delta());
                response.mark_changed();
                //Self::set_drag_amount_memory(ui, 0.0);
            }
        }
        if response.double_clicked() {
            self.reset_param();
            response.mark_changed();
        }
        if response.drag_stopped() {
            self.param_setter.end_set_parameter(self.param);
            Self::set_drag_amount_memory(ui, 0.0);
        }
        self.normalized_value()
    }

    fn get_string(&self) -> String {
        self.param.to_string()
    }
}

pub(crate) struct ArcKnob<'a, P: Param> {
    size: f32,
    line_width: f32,
    slider_region: SliderRegion<'a, P>,
    line_color: Color32,
    fill_color: Color32,
    hover_text: bool,
    hover_text_content: String,
    arc_start: f32,
    arc_end: f32,
}

impl<'a, P: Param> ArcKnob<'a, P> {
    pub(crate) fn for_param(
        param: &'a P,
        param_setter: &'a ParamSetter,
        size: f32,
        line_width: f32,
    ) -> Self {
        ArcKnob {
            size,
            line_width,
            slider_region: SliderRegion::new(param, param_setter),
            line_color: Color32::from_rgb(48, 200, 48),
            fill_color: Color32::from_rgb(70, 48, 48),
            hover_text: true,
            hover_text_content: "Gain reduction".into(),
            arc_start: 0.625,
            arc_end: -0.75,
        }
    }
}

impl<'a, P: Param> Widget for ArcKnob<'a, P> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        // Figure out the size to reserve on screen for widget
        let desired_size = Vec2::splat(self.size);
        let mut response = ui.allocate_response(desired_size, Sense::click_and_drag());

        let value = self.slider_region.handle_response(&ui, &mut response);

        ui.vertical(|ui| {
            let painter = ui.painter_at(response.rect);
            let center = response.rect.center();

            // Draw the inactive arc behind the highlight line
            {
                let outline_stroke = Stroke::new(1.0, self.fill_color.linear_multiply(0.7));
                let outline_shape = Shape::Path(PathShape {
                    points: get_arc_points(
                        self.arc_start,
                        self.arc_end,
                        center,
                        self.size / 2.0 * 0.7
                            + self.size / 2.0 * 0.012
                            + (self.size / 2.0 * 0.3 / 2.0),
                        1.0,
                        0.03,
                    ),
                    closed: false,
                    fill: self.fill_color.linear_multiply(0.7),
                    stroke: outline_stroke,
                });
                painter.add(outline_shape);
            }

            // Draw the highlight line
            let arc_radius = self.size / 2.0 * 0.7 + self.size / 2.0 * 0.012;
            {
                let arc_stroke = Stroke::new(self.line_width, self.line_color);
                let shape = Shape::Path(PathShape {
                    points: get_arc_points(
                        self.arc_start,
                        self.arc_end,
                        center,
                        arc_radius,
                        value,
                        0.03,
                    ),
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke: arc_stroke,
                });
                painter.add(shape);
            }

            // Center of Knob
            {
                let circle_shape = Shape::Circle(CircleShape {
                    center: center,
                    radius: self.size / 2.0 * 0.7,
                    stroke: Stroke::new(0.0, Color32::TRANSPARENT),
                    fill: self.fill_color,
                });
                painter.add(circle_shape);
            }

            // Draw the knob center line
            {
                // "balls" are end caps for the stroke
                let ball_width = self.line_width / 5.0;
                let ball_line_stroke = Stroke::new(ball_width, self.line_color);

                let line_shape = Shape::Path(PathShape {
                    points: get_pointer_points(
                        self.arc_start,
                        self.arc_end,
                        center,
                        arc_radius + ball_width,
                        value,
                    ),
                    closed: false,
                    fill: self.line_color,
                    stroke: Stroke::new(ball_width * 3.0, self.line_color),
                });
                painter.add(line_shape);

                // Draw circles at ends of highlight line to mimic a rounded stroke
                let end_ball = Shape::Circle(CircleShape {
                    center: get_end_point(
                        self.arc_start,
                        self.arc_end,
                        center,
                        arc_radius + ball_width,
                        value,
                    ),
                    radius: ball_width,
                    fill: self.line_color,
                    stroke: ball_line_stroke,
                });
                painter.add(end_ball);
                let center_ball = Shape::Circle(CircleShape {
                    center: center,
                    radius: ball_width,
                    fill: self.line_color,
                    stroke: ball_line_stroke,
                });
                painter.add(center_ball);
            }

            // Show hover text
            if self.hover_text {
                if self.hover_text_content.is_empty() {
                    self.hover_text_content = self.slider_region.get_string();
                }
                // check for hover within knob region
                ui.allocate_rect(
                    Rect::from_center_size(
                        center,
                        Vec2::new(self.size / 2.0 * 2.0, self.size / 2.0 * 2.0),
                    ),
                    Sense::hover(),
                )
                .on_hover_text_at_pointer(self.hover_text_content);
            }
        });
        response
    }
}

fn get_end_point(start: f32, end: f32, center: Pos2, radius: f32, value: f32) -> Pos2 {
    let start_turns: f32 = start;
    let arc_length = lerp(0.0, end, value);
    let end_turns = start_turns + arc_length;

    let angle = end_turns * TAU;
    let x = center.x + radius * angle.cos();
    let y = center.y + -radius * angle.sin();
    pos2(x, y)
}

fn get_pointer_points(start: f32, end: f32, center: Pos2, radius: f32, value: f32) -> Vec<Pos2> {
    let start_turns: f32 = start;
    let arc_length = lerp(0.0, end, value);
    let end_turns = start_turns + arc_length;

    let angle = end_turns * TAU;
    let x = center.x + radius * angle.cos();
    let y = center.y + -radius * angle.sin();
    let short_x = center.x + (radius * 0.04) * angle.cos();
    let short_y = center.y + (-radius * 0.04) * angle.sin();
    vec![pos2(short_x, short_y), pos2(x, y)]
}

fn get_arc_points(
    start: f32,
    end: f32,
    center: Pos2,
    radius: f32,
    value: f32,
    max_arc_distance: f32,
) -> Vec<Pos2> {
    let start_turns: f32 = start;
    let arc_length = lerp(0.0, end, value);
    let end_turns = start_turns + arc_length;

    let points = (arc_length.abs() / max_arc_distance).ceil() as usize;
    let points = points.max(1);
    (0..=points)
        .map(|i| {
            let t = i as f32 / (points - 1) as f32;
            let angle = lerp(start_turns * TAU, end_turns * TAU, t);
            let x = radius * angle.cos();
            let y = -radius * angle.sin();
            pos2(x, y) + center.to_vec2()
        })
        .collect()
}

// Moved lerp to this file to reduce dependencies - Ardura
pub(crate) fn lerp<T>(start: T, end: T, t: f32) -> T
where
    T: Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + Copy,
{
    (end - start) * t.clamp(0.0, 1.0) + start
}
