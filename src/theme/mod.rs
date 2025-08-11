pub mod button;
// pub mod container;

use std::sync::Arc;

use iced::{
    Background, Color, Degrees, Gradient, Radians, Theme as IcedTheme, color,
    gradient::{ColorStop, Linear},
    theme::palette::Pair,
};

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl From<Theme> for IcedTheme {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Light => IcedTheme::Custom(Arc::new(theme.to_theme())),
            Theme::Dark => IcedTheme::Custom(Arc::new(iced::theme::Custom::new(
                "Dark".to_string(),
                theme.to_palette(),
            ))),
        }
    }
}

impl Theme {
    pub fn to_palette(&self) -> iced::theme::Palette {
        let colors = self.colors();
        iced::theme::Palette {
            background: colors.background_base,
            text: colors.foreground_base,
            primary: color!(0x3026F1),
            success: color!(0x3026F1),
            warning: color!(0xFF9292),
            danger: color!(0xE71E7D),
        }
    }

    pub fn to_theme(&self) -> iced::theme::Custom {
        let colors = self.colors();
        let name = match self {
            Theme::Light => "PhoniqueLight",
            Theme::Dark => "PhoniqueDark",
        };
        let is_dark = matches!(self, Theme::Dark);
        iced::theme::Custom::new("PhoniqueLight".into(), self.to_palette())
    }

    pub fn colors(&self) -> &Palette {
        &Palette::LIGHT
    }
}

pub struct Palette {
    pub primary_base_gradient: iced::Background,
    pub primary_base_text: Color,
    pub primary_strong_gradient: iced::Background,
    pub primary_strong_text: Color,
    pub secondary_background: Color,
    pub secondary_text: Color,
    pub background_base: Color,
    pub foreground_base: Color,
    pub background_weak: Color,
    pub foreground_weak: Color,
    pub background_strong: Color,
    pub foreground_strong: Color,
    pub background_alt: Color,
    pub foreground_alt: Color,
    pub background_highlight: Color,
    pub border_light: Color,
    pub border_dark: Color,
    pub scroller: Color,
}

mod light {
    use iced::{Color, color};

    pub const BACKGROUND: Color = color!(0xFFFFFF);
    pub const FOREGROUND: Color = color!(0x09090B);
    pub const PRIMARY: Color = color!(0x18181B);
    pub const SECONDARY: Color = color!(0xF4F4F5);
    pub const MUTED: Color = color!(0xF4F4F5);
    pub const MUTED_FOREGROUND: Color = color!(0x71717B);
    pub const DESTRUCTIVE: Color = color!(0xE7000B);
    pub const BORDER: Color = color!(0xE4E4E7);
    pub const SIDEBAR: Color = color!(0xFAFAFA);
}

impl Palette {
    pub const LIGHT: Palette = Palette {
        primary_base_gradient: Background::Gradient(Gradient::Linear(Linear {
            angle: iced::Radians(2.355),
            stops: [
                Some(ColorStop {
                    offset: 0.0,
                    color: Color {
                        r: 0.188,
                        g: 0.149,
                        b: 0.945,
                        a: 0.4,
                    },
                }),
                Some(ColorStop {
                    offset: 1.0,
                    color: color!(0xFF9292),
                }),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
        })),
        primary_base_text: color!(0xFFFFFF),
        primary_strong_gradient: Background::Gradient(Gradient::Linear(Linear {
            angle: iced::Radians(2.355),
            stops: [
                Some(ColorStop {
                    offset: 0.0,
                    color: color!(0x3026F1),
                }),
                Some(ColorStop {
                    offset: 1.0,
                    color: color!(0xFF9292),
                }),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
        })),
        primary_strong_text: color!(0xFFFFFF),
        secondary_background: color!(0xE71E7D),
        secondary_text: color!(0xFFFFFF),
        background_base: color!(0xFFFFFF),
        foreground_base: color!(0x000000),
        background_weak: color!(0xF2EFEF),
        foreground_weak: color!(0x868686),
        background_strong: color!(0xF6F3F3),
        foreground_strong: color!(0x000000),
        background_alt: color!(0xDCDCDC),
        foreground_alt: color!(0x5E5E5E),
        background_highlight: color!(0x868686),
        border_light: color!(0xE7E7E7),
        border_dark: color!(0xE6E3E3),
        scroller: color!(0xD3D0D0),
    };
}
