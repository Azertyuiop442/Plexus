
use ratatui::style::Color;

pub fn effective_bg() -> Color {
    BG
}

pub const BG: Color = Color::Rgb(0x00, 0x00, 0x00);

pub const FG: Color = Color::Rgb(0xCE, 0xCD, 0xC3);

#[allow(dead_code)]
pub const MUTED: Color = Color::Rgb(0x57, 0x56, 0x53);

pub const ACCENT: Color = Color::Rgb(0x43, 0x85, 0xBE);

pub const GREEN: Color = Color::Rgb(0x87, 0x9A, 0x39);

pub const YELLOW: Color = Color::Rgb(0xD0, 0xA2, 0x15);

pub const RED: Color = Color::Rgb(0xD1, 0x4D, 0x41);

pub const BLUE: Color = Color::Rgb(0x43, 0x85, 0xBE);

pub const PURPLE: Color = Color::Rgb(0x8B, 0x7E, 0xC8);

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Palette {

    pub accent: Color,

    pub panel_bg: Color,

    pub sidebar_bg: Color,

    pub surface0: Color,

    pub surface1: Color,

    pub surface_dim: Color,

    pub overlay0: Color,

    pub overlay1: Color,

    pub text: Color,

    pub subtext0: Color,

    pub mauve: Color,

    pub green: Color,

    pub yellow: Color,

    pub red: Color,

    pub blue: Color,

    pub teal: Color,

    pub peach: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self::dark()
    }
}

impl Palette {

    pub const fn dark() -> Self {
        Self {
            accent: ACCENT,

            panel_bg: Color::Rgb(0x1C, 0x1B, 0x1A),
            sidebar_bg: BG,

            surface0: Color::Rgb(0x28, 0x27, 0x26),
            surface1: Color::Rgb(0x34, 0x33, 0x31),
            surface_dim: Color::Rgb(0x1C, 0x1B, 0x1A),

            overlay0: Color::Rgb(0x57, 0x56, 0x53),
            overlay1: Color::Rgb(0x6F, 0x6E, 0x69),
            text: FG,

            subtext0: Color::Rgb(0x87, 0x85, 0x80),
            mauve: PURPLE,
            green: GREEN,
            yellow: YELLOW,
            red: RED,
            blue: BLUE,
            teal: ACCENT,

            peach: Color::Rgb(0xDA, 0x70, 0x2C),
        }
    }
}

