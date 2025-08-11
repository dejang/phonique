use crate::theme::Theme;

use iced::{
    Background, Border, Color, Shadow, Theme as IcedTheme,
    widget::button::{Catalog, Status, Style, StyleFn},
};

pub fn primary(theme: &IcedTheme, status: Status) -> Style {
    let palette = theme.extended_palette();
    match status {
        Status::Active => Style {
            background: Some(Background::Color(palette.primary.base.color)),
            text_color: palette.primary.base.text,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        },
        Status::Hovered => Style {
            background: Some(Background::Color(palette.primary.strong.color)),
            text_color: palette.primary.strong.text,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        },
        Status::Pressed => Style {
            background: Some(Background::Color(palette.primary.strong.color)),
            text_color: palette.primary.strong.text,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        },
        Status::Disabled => Style {
            background: Some(Background::Color(palette.primary.weak.color)),
            text_color: palette.primary.weak.text,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        },
    }
}
