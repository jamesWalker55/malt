use std::sync::LazyLock;

use nih_plug_egui::egui::{Color32, FontFamily};

// thanks chatgpt
macro_rules! define_colors {
    ($($name:ident = $value:expr;)*) => {
        $(pub(crate) const $name: Color32 = $value;)*
    };
}

define_colors! {
    // basic color definitions
    BG_DARK = Color32::from_rgb(17, 17, 17);
    BG_NORMAL = Color32::from_rgb(33, 33, 33);
    BG_LIGHT = Color32::from_rgb(48, 48, 48); // for the knobs panel

    FG_WHITE = Color32::from_rgb(245, 245, 245);
    FG_GREY = Color32::from_rgb(158, 158, 158);
    FG_DARK_GREY = Color32::from_rgb(97, 97, 97);
    FG_RED = Color32::from_rgb(239, 154, 154);
    FG_PURPLE = Color32::from_rgb(206, 147, 216);
    FG_ORANGE = Color32::from_rgb(255, 204, 128);
    FG_YELLOW = Color32::from_rgb(255, 245, 157);
    FG_BLUE = Color32::from_rgb(129, 212, 250);
    FG_GREEN = Color32::from_rgb(165, 214, 167);

    // // program colors
    // PANEL_BORDER = BG_DARK;

    // TITLEBAR_BG = BG_DARK;
    // TITLEBAR_COMPANY_TEXT = TEXT_DIMMED;
    // TITLEBAR_NAME_TEXT = TEXT_NORMAL;
    // TITLEBAR_BUTTON = TEXT_DIMMED;
    // TITLEBAR_HELP_ACTIVE = TEXT_GREEN;

    // BAND_LOW = TEXT_BLUE;
    // BAND_MID = TEXT_PURPLE;
    // BAND_HIGH = TEXT_YELLOW;

    // ANALYZER_BG = BG_NORMAL;

    // // for the knobs panel
    // PANEL_BG = BG_LIGHT;
    // PANEL_TEXT = TEXT_DIMMED;

    // PANEL_KNOB_BASE = BG_NORMAL;
    // PANEL_KNOB_MARKER = TEXT_NORMAL;
    // PANEL_KNOB_RIM_BG = Color32::from_rgb(64, 64, 64);
    // PANEL_KNOB_TEXT = Color32::from_rgb(64, 64, 64);
}

pub(crate) const TEXT_LARGE: f32 = 16.0;
pub(crate) const TEXT_BASE: f32 = 12.0;
pub(crate) const TEXT_SM: f32 = 11.0;
pub(crate) const TEXT_XS: f32 = 10.0;

pub(crate) const FONT_NORMAL: FontFamily = FontFamily::Proportional;
pub(crate) static FONT_BOLD: LazyLock<FontFamily> =
    LazyLock::new(|| FontFamily::Name("bold".into()));
