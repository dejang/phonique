use std::borrow::Cow;
use std::sync::OnceLock;

use iced::font;

pub static SANS: Font = Font::new(false);
pub static SANS_BOLD: iced::Font = iced::Font{ weight: font::Weight::Bold, ..iced::Font::with_name("inter_bold") };
#[allow(dead_code)]
pub static MONOSPACE: iced::Font = iced::Font::MONOSPACE;
pub static ICON: iced::Font = iced::Font::with_name("lucide");

#[derive(Debug, Clone)]
pub struct Font {
    bold: bool,
    inner: OnceLock<iced::Font>,
}

impl Font {
    const fn new(bold: bool) -> Self {
        Self {
            bold,
            inner: OnceLock::new(),
        }
    }

    fn set(&self, name: String) {
        let name = Box::leak(name.into_boxed_str());
        let weight = if self.bold {
            font::Weight::Bold
        } else {
            font::Weight::Normal
        };

        let _ = self.inner.set(iced::Font {
            weight,
            ..iced::Font::with_name(name)
        });
    }
}

impl From<Font> for iced::Font {
    fn from(value: Font) -> Self {
        value.inner.get().copied().expect("font is set on startup")
    }
}

pub fn set() {
    SANS.set("inter_regular".to_string());
}

pub fn load() -> Vec<Cow<'static, [u8]>> {
    vec![
        include_bytes!("../fonts/inter_regular.ttf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/inter_bold.ttf").as_slice().into(),
        include_bytes!("../fonts/inter_italic.ttf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/inter_bold_italic.ttf")
            .as_slice()
            .into(),
        include_bytes!("../fonts/lucide.ttf").as_slice().into(),
    ]
}
