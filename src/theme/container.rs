use crate::theme::Theme;
use iced::{
    Background, Border, Color, Shadow,
    widget::container::{Catalog, Style, StyleFn},
};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(default)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

fn container(background: Color, foreground: Color) -> Style {
    Style {
        text_color: Some(foreground),
        background: Some(Background::Color(background)),
        border: Border::default(),
        shadow: Shadow::default(),
    }
}

pub fn default(theme: &Theme) -> Style {
    let palette = theme.palette();
    container(palette.background, palette.foreground)
}
