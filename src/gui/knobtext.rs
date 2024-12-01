use nih_plug::{
    nih_log,
    prelude::{Param, ParamSetter},
};
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

pub(crate) struct KnobText<'a, P: Param> {
    param: &'a P,
    param_setter: &'a ParamSetter<'a>,

    size: Vec2,
    font_id: FontId,
    color: Color32,
    allow_drag: bool,
    allow_keyboard: bool,
}

impl<'a, P: Param> KnobText<'a, P> {
    pub(crate) fn for_param(
        param: &'a P,
        param_setter: &'a ParamSetter,
        size: Vec2,
        font_id: FontId,
        color: Color32,
        allow_drag: bool,
        allow_keyboard: bool,
    ) -> Self {
        KnobText {
            param,
            param_setter,
            size,
            font_id,
            color,
            allow_drag,
            allow_keyboard,
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

impl<'a, P: Param> Widget for KnobText<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Figure out the size to reserve on screen for widget
        let mut response = ui.allocate_response(self.size, Sense::click_and_drag());
        let rect = response.rect;

        // handle mouse click/drag events
        if self.allow_keyboard
            // if drag is enabled, only start keyboard when double clicking
            // if no drag allowed, start keyboard on single click
            && ((self.allow_drag && response.double_clicked())
                || (!self.allow_drag && response.clicked()))
        {
            // start keyboard editing
            nih_log!("it's keyboard time")
        } else if self.allow_drag {
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
        }

        // draw the text
        {
            let painter = ui.painter_at(rect);

            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                self.param.to_string(),
                self.font_id,
                self.color,
            );
        }

        response
    }
}
