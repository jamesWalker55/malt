use super::palette as C;
use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    epaint::{CircleShape, PathShape, PathStroke},
    pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
    Widget,
};
use std::{
    f32::consts::TAU,
    ops::{Add, Mul, Sub},
};

/// When shift+dragging a parameter, one pixel dragged corresponds to this much change in the
/// noramlized parameter.
const GRANULAR_DRAG_MULTIPLIER: f32 = 0.0002;
const NORMAL_DRAG_MULTIPLIER: f32 = 0.001;

pub(crate) struct KnobDonutText {
    pub(crate) spacing: f32,
    pub(crate) width: f32,
    pub(crate) font_id: FontId,
    pub(crate) color: Color32,
}

pub(crate) enum KnobStyle {
    Analog {
        highlight_color: Color32,
        line_width: f32,
    },
    Donut {
        line_width: f32,
        text: Option<KnobDonutText>,
    },
}

pub(crate) struct Knob<'a, P: Param> {
    size: f32,
    style: KnobStyle,
    param: &'a P,
    param_setter: &'a ParamSetter<'a>,
}

impl<'a, P: Param> Knob<'a, P> {
    // negative length to rotate clockwise
    // https://www.desmos.com/calculator/cctb9rqruw
    const ARC_START: f32 = -3.0 / 8.0 * TAU;
    const ARC_END: f32 = -9.0 / 8.0 * TAU;

    const LINE_COLOR: Color32 = C::FG_WHITE;
    const BG_COLOR: Color32 = C::PANEL_KNOB_RIM_BG;
    const KNOB_COLOR: Color32 = C::BG_NORMAL;

    pub(crate) fn for_param(
        param: &'a P,
        param_setter: &'a ParamSetter,
        size: f32,
        style: KnobStyle,
    ) -> Self {
        Knob {
            size,
            style,
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
        if value != self.param.modulated_plain_value() {
            self.param_setter.set_parameter(self.param, value);
        }
    }

    fn normalized_value(&self) -> f32 {
        self.param.modulated_normalized_value()
    }

    /// NOTE: You need to call begin and end drag when using this
    fn reset_param(&self) {
        self.param_setter
            .set_parameter(self.param, self.param.default_plain_value());
    }
}

impl<'a, P: Param> Widget for Knob<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Figure out the size to reserve on screen for widget
        let mut response = {
            // minimum bounding box
            let bounding_box = if let KnobStyle::Donut {
                text: Some(KnobDonutText { spacing, width, .. }),
                ..
            } = self.style
            {
                vec2(self.size + spacing + width, self.size)
            } else {
                Vec2::splat(self.size)
            };

            ui.allocate_response(bounding_box, Sense::click_and_drag())
        };
        let rect = response.rect;

        // handle mouse click/drag events
        //
        // drag only occurs after (1) holding down mouse, then (2) moving mouse
        // therefore `drag_started()` and `clicked()` cannot BOTH be true at the same frame
        //
        // when ctrl+clicking on the knob, reset the parameter
        if response.clicked() && ui.input(|x| x.modifiers.command) {
            self.param_setter.begin_set_parameter(self.param);
            self.reset_param();
            self.param_setter.end_set_parameter(self.param);
            response.mark_changed();
        } else {
            // otherwise, check if user is dragging

            // This executes on first frame of drag only
            if response.drag_started() {
                self.param_setter.begin_set_parameter(self.param);
            }

            // this checks when knob is clicked or dragged:
            if let Some(_clicked_pos) = response.interact_pointer_pos() {
                let drag_distance = -response.drag_delta().y;

                // check drag_delta to make sure we are actually dragging
                if drag_distance != 0.0 {
                    let value = self.normalized_value();

                    // Shift dragging switches to a more granular input method
                    let new_value = if ui.input(|mem| mem.modifiers.shift) {
                        value + (drag_distance * GRANULAR_DRAG_MULTIPLIER)
                    } else {
                        value + (drag_distance * NORMAL_DRAG_MULTIPLIER)
                    }
                    .clamp(0.0, 1.0);

                    self.set_normalized_value(new_value);
                    response.mark_changed();
                }
            }

            if response.drag_stopped() {
                self.param_setter.end_set_parameter(self.param);
            }
        }

        let value = self.normalized_value();

        match &self.style {
            KnobStyle::Analog {
                highlight_color,
                line_width,
            } => {
                let painter = ui.painter_at(rect);
                let center = rect.center();

                // since we will draw a stroke, we need to account for the line's width
                // outline radius should be: size / 2.0 - line_width / 2.0
                // this formula is the same, simplified:
                let outline_radius = (self.size - line_width) / 2.0;
                let center_radius = self.size / 2.0 - line_width;

                // Draw the inactive arc behind the highlight line
                {
                    let shape = Shape::Path(PathShape {
                        points: get_arc_points(
                            1.0,
                            Self::ARC_START,
                            Self::ARC_END,
                            center,
                            // improve rendering by making outline overlap with knob center a bit
                            outline_radius - 1.0,
                            0.2,
                        ),
                        closed: false,
                        fill: Default::default(),
                        // improve rendering by making outline overlap with knob center a bit
                        stroke: PathStroke::new(line_width + 2.0, Self::BG_COLOR),
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
                            // improve rendering by making outline overlap with knob center a bit
                            outline_radius - 1.0,
                            0.2,
                        ),
                        closed: false,
                        fill: Default::default(),
                        // improve rendering by making outline overlap with knob center a bit
                        stroke: PathStroke::new(line_width + 2.0, *highlight_color),
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
                    let inner_radius = line_width;
                    // make the marker line terminate just before it reaches the highlight ring
                    // also add 0.25 pixel to make it look nicer
                    let outer_radius = outline_radius - line_width + 0.25;

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
                        fill: Default::default(),
                        stroke: PathStroke::new(*line_width, Self::LINE_COLOR),
                    });
                    painter.add(line_shape);

                    if *line_width > 3.0 {
                        // Draw circles ("balls") at ends of highlight line to mimic a rounded stroke
                        let ball_radius = line_width / 2.0 - 1.0;
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
            }
            KnobStyle::Donut { line_width, text } => {
                let painter = ui.painter_at(rect);
                // center of knob
                let center = {
                    let mut rv = rect.center();
                    if text.is_some() {
                        // align knob to the left if there is text
                        rv.x = rect.left() + self.size / 2.0;
                    }
                    rv
                };

                // since we will draw a stroke, we need to account for the line's width
                // outline radius should be: size / 2.0 - line_width / 2.0
                // this formula is the same, simplified:
                let line_radius = (self.size - line_width) / 2.0;

                // Draw the inactive arc behind the highlight line
                {
                    let shape = Shape::Path(PathShape {
                        points: get_arc_points(
                            1.0,
                            Self::ARC_START,
                            Self::ARC_END,
                            center,
                            line_radius,
                            0.2,
                        ),
                        closed: false,
                        fill: Default::default(),
                        stroke: PathStroke::new(*line_width, Self::BG_COLOR),
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
                            line_radius,
                            0.2,
                        ),
                        closed: false,
                        fill: Default::default(),
                        // improve rendering by making outline overlap with knob center a bit
                        stroke: PathStroke::new(*line_width, Self::LINE_COLOR),
                    });
                    painter.add(shape);
                }

                // draw text label
                if let Some(text) = text {
                    let mut text_rect = rect;
                    text_rect.set_left(text_rect.left() + self.size + text.spacing);

                    // clip text to the bounds
                    let painter = ui.painter_at(text_rect);

                    painter.text(
                        response.rect.center(),
                        Align2::CENTER_CENTER,
                        self.param.to_string(),
                        text.font_id.clone(),
                        text.color,
                    );
                }
            }
        }

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
