use nih_plug::prelude::{Param, ParamSetter};
use nih_plug_egui::egui::{
    epaint::{CircleShape, PathShape},
    pos2, Color32, Id, Pos2, Response, Sense, Shape, Stroke, TextEdit, Ui, Vec2, Widget,
};
use once_cell::sync::Lazy;
use std::{
    f32::consts::TAU,
    ops::{Add, Mul, Sub},
};

static VALUE_ENTRY_BUFFER_ID: Lazy<Id> = Lazy::new(|| Id::new((file!(), 0)));

pub(crate) struct ValueText<'a, P: Param> {
    param: &'a P,
    param_setter: &'a ParamSetter<'a>,
}

impl<'a, P: Param> ValueText<'a, P> {
    // // negative length to rotate clockwise
    // // https://www.desmos.com/calculator/cctb9rqruw
    // const ARC_START: f32 = -3.0 / 8.0 * TAU;
    // const ARC_END: f32 = -9.0 / 8.0 * TAU;

    // const LINE_COLOR: Color32 = Color32::from_rgb(245, 245, 245);
    // const BG_COLOR: Color32 = Color32::from_rgb(64, 64, 64);
    // const KNOB_COLOR: Color32 = Color32::from_rgb(33, 33, 33);

    pub(crate) fn for_param(param: &'a P, param_setter: &'a ParamSetter) -> Self {
        ValueText {
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

impl<'a, P: Param> Widget for ValueText<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        let edit = TextEdit::singleline(&mut *value_entry).id(*VALUE_ENTRY_BUFFER_ID);

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

        response
    }
}

// Moved lerp to this file to reduce dependencies - Ardura
pub(crate) fn lerp<T>(start: T, end: T, t: f32) -> T
where
    T: Add<T, Output = T> + Sub<T, Output = T> + Mul<f32, Output = T> + Copy,
{
    (end - start) * t.clamp(0.0, 1.0) + start
}
