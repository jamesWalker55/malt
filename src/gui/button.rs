use nih_plug_egui::egui::{
    vec2, Align2, Color32, Image, ImageSource, Response, Sense, TextStyle, Ui, Widget,
};

pub(crate) enum ButtonContent {
    Text(&'static str),
    Image(ImageSource<'static>),
}

pub(crate) struct Button {
    content: ButtonContent,
    x: f32,
    y: f32,
    text_inactive: Color32,
    text_hover: Color32,
    text_active: Color32,
    bg_inactive: Color32,
    bg_hover: Color32,
    bg_active: Color32,
}

impl Button {
    pub(crate) fn new(
        content: ButtonContent,
        x: f32,
        y: f32,
        text_inactive: Color32,
        text_hover: Color32,
        text_active: Color32,
        bg_inactive: Color32,
        bg_hover: Color32,
        bg_active: Color32,
    ) -> Self {
        Self {
            content,
            x,
            y,
            text_inactive,
            text_hover,
            text_active,
            bg_inactive,
            bg_hover,
            bg_active,
        }
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired_size = vec2(self.x, self.y);
        let response = ui.allocate_response(desired_size, Sense::click());

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
                ButtonContent::Text(text) => {
                    // default font if not set in styles
                    let font_id = TextStyle::Button.resolve(ui.style());
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
