use super::palette as C;
use nih_plug::{
    nih_debug_assert_eq,
    prelude::{Param, ParamSetter},
};
use nih_plug_egui::egui::{
    Align, Align2, Color32, FontId, Id, Key, Layout, Response, Sense, TextEdit, Ui, Vec2, Widget,
};
use parking_lot::Mutex;
use std::sync::Arc;

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
    /// Whether or not to snap to nearest value when using keyboard input
    keyboard_snap: bool,
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
        keyboard_snap: bool,
    ) -> Self {
        KnobText {
            param,
            param_setter,
            size,
            font_id,
            color,
            allow_drag,
            allow_keyboard,
            keyboard_snap,
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

    /// The UI when not in keyboard mode
    fn ui_normal(self, ui: &mut Ui, current_id: Id) -> Response {
        // Figure out the size to reserve on screen for widget
        // TODO: Force this to use `current_id`
        let mut response = ui.allocate_response(self.size, Sense::click_and_drag());
        nih_debug_assert_eq!(response.id, current_id);
        let rect = response.rect;

        // handle mouse click/drag events
        if self.allow_keyboard
            // if drag is enabled, only start keyboard when double clicking
            // if no drag allowed, start keyboard on single click
            && ((self.allow_drag && response.double_clicked())
                || (!self.allow_drag && response.clicked()))
        {
            // start keyboard editing
            ui.memory_mut(|mem| {
                // request keyboard focus on this widget
                mem.request_focus(response.id);
                // make it select everything on the next frame
                mem.data.insert_temp::<bool>(current_id, true);
            });
            // set the text buffer of the widget
            {
                let text_buf_mutex = ui.memory_mut(|mem| {
                    mem.data
                        .get_temp_mut_or_default::<Arc<Mutex<String>>>(current_id)
                        .clone()
                });
                *text_buf_mutex.lock() = self.param.to_string();
            }
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

    /// The UI when typing in text
    fn ui_keyboard(self, ui: &mut Ui, current_id: Id) -> Response {
        let should_select_everything = ui.memory_mut(|mem| {
            let rv = mem.data.get_temp::<bool>(current_id).unwrap_or(false);
            mem.data.insert_temp::<bool>(current_id, false);
            rv
        });

        let text_buf_mutex = ui.memory_mut(|mem| {
            mem.data
                .get_temp_mut_or_default::<Arc<Mutex<String>>>(current_id)
                .clone()
        });
        let mut text_buf = text_buf_mutex.lock();

        let mut output = ui
            .allocate_ui_with_layout(
                self.size,
                Layout::centered_and_justified(ui.layout().main_dir()),
                |ui| {
                    TextEdit::singleline(&mut *text_buf)
                        .font(self.font_id)
                        .text_color(C::FG_WHITE)
                        .desired_width(self.size.x)
                        .id(current_id)
                        .vertical_align(Align::Center)
                        .horizontal_align(Align::Center)
                        .show(ui)
                },
            )
            .inner;

        // select everything if first frame
        if should_select_everything {
            use nih_plug_egui::egui::text::{CCursor, CCursorRange};

            // https://stackoverflow.com/questions/74324236/select-the-text-of-a-textedit-object-in-egui
            output.state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(0),
                CCursor::new(text_buf.len()),
            )));
            output.state.store(ui.ctx(), output.response.id);
        }

        // only change value when Enter is pressed
        if ui.input(|i| i.key_pressed(Key::Enter)) {
            // And try to set the value by string when pressing enter
            self.param_setter.begin_set_parameter(self.param);
            match self.param.string_to_normalized_value(&text_buf) {
                Some(normalized_value) => {
                    if self.keyboard_snap {
                        // convert to "plain" before setting to snap to closest value
                        let value = self.param.preview_plain(normalized_value);
                        if value != self.param.modulated_plain_value() {
                            self.param_setter.set_parameter(self.param, value);
                        }
                    } else {
                        // just set directly without snapping
                        self.param_setter
                            .set_parameter_normalized(self.param, normalized_value);
                    }
                }
                None => (),
            }
            self.param_setter.end_set_parameter(self.param);

            ui.memory_mut(|mem| mem.surrender_focus(current_id));
        } else if ui.input(|i| i.key_pressed(Key::Escape)) {
            // Cancel when pressing escape
            ui.memory_mut(|mem| mem.surrender_focus(current_id));
        }

        output.response
    }
}

impl<'a, P: Param> Widget for KnobText<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        let next_id = ui.next_auto_id();
        // find the id that has keyboard focus
        let focused_id = ui.memory(|mem| mem.focused()).unwrap_or(Id::NULL);
        if focused_id == next_id {
            self.ui_keyboard(ui, next_id)
        } else {
            self.ui_normal(ui, next_id)
        }
    }
}
