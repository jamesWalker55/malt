use nih_plug_egui::egui::{
    Align2, Color32, FontId, Image, ImageSource, Painter, Response, Sense, Ui, Vec2, Widget,
};

pub(crate) enum ButtonContent {
    Text(&'static str, FontId),
    Image(ImageSource<'static>),
}

pub(crate) struct BlockButton {
    content: ButtonContent,
    size: Vec2,
    text_inactive: Color32,
    text_hover: Color32,
    text_active: Color32,
    bg_inactive: Color32,
    bg_hover: Color32,
    bg_active: Color32,
}

impl BlockButton {
    pub(crate) fn new(
        content: ButtonContent,
        size: Vec2,
        text_inactive: Color32,
        text_hover: Color32,
        text_active: Color32,
        bg_inactive: Color32,
        bg_hover: Color32,
        bg_active: Color32,
    ) -> Self {
        Self {
            content,
            size,
            text_inactive,
            text_hover,
            text_active,
            bg_inactive,
            bg_hover,
            bg_active,
        }
    }
}

impl Widget for BlockButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let response = ui.allocate_response(self.size, Sense::click());

        let painter = ui.painter_at(response.rect);

        // bg fill
        {
            let fill_color = if response.is_pointer_button_down_on() {
                self.bg_active
            } else if response.hovered() {
                self.bg_hover
            } else {
                self.bg_inactive
            };
            painter.rect_filled(response.rect, 0.0, fill_color);
        }

        // content text/image
        {
            let text_color = if response.is_pointer_button_down_on() {
                self.text_active
            } else if response.hovered() {
                self.text_hover
            } else {
                self.text_inactive
            };
            match self.content {
                ButtonContent::Text(text, font_id) => {
                    painter.text(
                        response.rect.center(),
                        Align2::CENTER_CENTER,
                        text,
                        font_id,
                        text_color,
                    );
                }
                ButtonContent::Image(src) => {
                    let img = Image::new(src).tint(text_color);
                    img.paint_at(ui, response.rect);
                }
            }
        }

        response
    }
}

pub(crate) enum BlockButtonState {
    Inactive,
    Hover,
    Active,
}

pub(crate) fn custom_block_button(
    ui: &mut Ui,
    size: Vec2,
    bg_inactive: Color32,
    bg_hover: Color32,
    bg_active: Color32,
    mut add_contents: impl FnMut(&mut Ui, &Response, Painter, BlockButtonState),
) -> Response {
    let response = ui.allocate_response(size, Sense::click());

    let painter = ui.painter_at(response.rect);

    let state: BlockButtonState;

    // bg fill
    {
        let fill_color = if response.is_pointer_button_down_on() {
            state = BlockButtonState::Active;
            bg_active
        } else if response.hovered() {
            state = BlockButtonState::Hover;
            bg_hover
        } else {
            state = BlockButtonState::Inactive;
            bg_inactive
        };
        painter.rect_filled(response.rect, 0.0, fill_color);
    }

    // content text/image
    add_contents(ui, &response, painter, state);

    response
}
