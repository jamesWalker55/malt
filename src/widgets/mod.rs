// Ardura 2024 update - ui_knob.rs - egui + nih-plug parameter widget with customization
//  this ui_knob.rs is built off a2aaron's knob base as part of nyasynth and Robbert's ParamSlider code
// https://github.com/a2aaron/nyasynth/blob/canon/src/ui_knob.rs

use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    self,
    epaint::{CircleShape, PathShape},
    pos2, Color32, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2, Widget,
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
    highlight_color: Color32,
}

impl<'a, P: Param> ArcKnob<'a, P> {
    // negative length to rotate clockwise
    // https://www.desmos.com/calculator/cctb9rqruw
    const ARC_START: f32 = -3.0 / 8.0 * TAU;
    const ARC_END: f32 = -9.0 / 8.0 * TAU;

    const LINE_COLOR: Color32 = Color32::from_rgb(245, 245, 245);
    const BG_COLOR: Color32 = Color32::from_rgb(64, 64, 64);
    const KNOB_COLOR: Color32 = Color32::from_rgb(33, 33, 33);

    pub(crate) fn for_param(
        param: &'a P,
        param_setter: &'a ParamSetter,
        size: f32,
        highlight_color: Color32,
        line_width: f32,
    ) -> Self {
        ArcKnob {
            size,
            line_width,
            slider_region: SliderRegion::new(param, param_setter),
            highlight_color,
        }
    }
}

impl<'a, P: Param> Widget for ArcKnob<'a, P> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        // Figure out the size to reserve on screen for widget
        let bounding_box = Vec2::splat(self.size);
        let mut response = ui.allocate_response(bounding_box, Sense::click_and_drag());

        let value = self.slider_region.handle_response(&ui, &mut response);

        ui.vertical(|ui| {
            let painter = ui.painter_at(response.rect);
            let center = response.rect.center();

            // since we will draw a stroke, we need to account for the line's width
            // outline radius should be: size / 2.0 - line_width / 2.0
            // this formula is the same, simplified:
            let outline_radius = (self.size - self.line_width) / 2.0;
            let center_radius = self.size / 2.0 - self.line_width;

            // Draw the inactive arc behind the highlight line
            {
                let shape = Shape::Path(PathShape {
                    points: get_arc_points(
                        1.0,
                        Self::ARC_START,
                        Self::ARC_END,
                        center,
                        outline_radius,
                        0.2,
                    ),
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke: Stroke::new(self.line_width, Self::BG_COLOR),
                });
                painter.add(shape);
            }

            // Draw the highlight line
            {
                let shape = Shape::Path(PathShape {
                    points: get_arc_points(
                        value,
                        Self::ARC_START,
                        Self::ARC_END,
                        center,
                        outline_radius,
                        0.2,
                    ),
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke: Stroke::new(self.line_width, self.highlight_color),
                });
                painter.add(shape);
            }

            // Center of Knob
            {
                let shape = Shape::Circle(CircleShape {
                    center,
                    radius: center_radius,
                    stroke: Stroke::NONE,
                    fill: Self::KNOB_COLOR,
                });
                painter.add(shape);
            }

            // Draw the knob marker line
            {
                // make the marker line begin off-center
                let inner_radius = center_radius * 0.35;
                // make the marker line terminate just before it reaches the highlight ring
                let outer_radius = outline_radius - self.line_width;

                // find start and end point
                let angle = lerp(Self::ARC_START, Self::ARC_END, value);
                let start_point = {
                    let x = center.x + inner_radius * angle.cos();
                    let y = center.y + inner_radius * -angle.sin();
                    pos2(x, y)
                };
                let end_point = {
                    let x = center.x + outer_radius * angle.cos();
                    let y = center.y + outer_radius * -angle.sin();
                    pos2(x, y)
                };

                let line_shape = Shape::Path(PathShape {
                    points: vec![start_point, end_point],
                    closed: false,
                    fill: Self::LINE_COLOR,
                    stroke: Stroke::new(self.line_width, Self::LINE_COLOR),
                });
                painter.add(line_shape);

                if self.line_width > 3.0 {
                    // Draw circles ("balls") at ends of highlight line to mimic a rounded stroke
                    let ball_radius = self.line_width / 2.0 - 1.0;
                    let end_ball = Shape::Circle(CircleShape {
                        center: end_point,
                        radius: ball_radius,
                        fill: Self::LINE_COLOR,
                        stroke: Stroke::NONE,
                    });
                    painter.add(end_ball);
                    let center_ball = Shape::Circle(CircleShape {
                        center: start_point,
                        radius: ball_radius,
                        fill: Self::LINE_COLOR,
                        stroke: Stroke::NONE,
                    });
                    painter.add(center_ball);
                }
            }

            // Show hover text
            response
                .clone()
                .on_hover_text_at_pointer(self.slider_region.get_string());
        });
        response
    }
}

/// Return a bunch of points that lie on an arc.
///
/// Radian measurements start from the right and lie on the x-axis, see this
/// visualizer: https://www.desmos.com/calculator/cctb9rqruw
///
/// * `value` - A scalar between 0.0 and 1.0 to interpolate between `start` and `end`
/// * `start` - Starting angle in radians
/// * `end` - Ending angle in radians
/// * `center` - Center point of the circle which the arc is based on
/// * `radius` - radius
/// * `max_arc_distance` - How precise the curve should be, measured in radians
fn get_arc_points(
    value: f32,
    start: f32,
    mut end: f32,
    center: Pos2,
    radius: f32,
    max_arc_distance: f32,
) -> Vec<Pos2> {
    end = lerp(start, end, value);
    let length = (end - start).abs();

    let points = (length / max_arc_distance).ceil() as usize;
    let points = points.max(2);
    (0..=points)
        .map(|i| {
            let t = i as f32 / (points - 1) as f32;
            let angle = lerp(start, end, t);
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
