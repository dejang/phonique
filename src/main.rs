mod app_state;
mod audio_scanner;
mod fonts;
mod icons;
mod menu_bar;
mod player;
mod screen;
mod sidebar;
mod storage;
mod theme;
mod util;
mod view_types;
mod widgets;

use iced::{Element, Settings, Subscription, Task, Theme as IcedTheme};
use theme::Theme;

pub fn main() -> iced::Result {
    env_logger::init();
    fonts::set();
    iced::application(Phoniq::default, Phoniq::update, Phoniq::view)
        .theme(Phoniq::theme)
        .title("Phoniq")
        .subscription(Phoniq::subscription)
        .settings(Settings {
            default_font: fonts::SANS.clone().into(),
            fonts: fonts::load(),
            antialiasing: true,
            ..Default::default()
        })
        .run()
}

enum Phoniq {
    Main(Box<screen::main_screen::MainScreen>),
}

#[derive(Debug, Clone)]
enum Message {
    Main(screen::main_screen::Message),
}

impl Default for Phoniq {
    fn default() -> Self {
        Self::Main(Box::default())
    }
}

impl Phoniq {
    fn update(&mut self, message: Message) -> Task<Message> {
        match self {
            Self::Main(screen) => match message {
                Message::Main(message) => screen.update(message).map(Message::Main),
                _ => Task::none(),
            },
        }
    }

    fn view(&self) -> Element<Message> {
        match self {
            Phoniq::Main(main) => main.view().map(Message::Main),
        }
    }

    fn theme(&self) -> IcedTheme {
        crate::Theme::Light.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if let Self::Main(mainscreen) = self {
            mainscreen.subscription().map(Message::Main)
        } else {
            Subscription::none()
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use iced_test::selector::text;
    // use iced_test::{Error, simulator};

    // #[test]
    // fn it_counts() -> Result<(), Error> {
    // let mut counter = Counter { value: 0 };
    // let mut ui = simulator(counter.view());

    // let _ = ui.click(text("Increment"))?;
    // let _ = ui.click(text("Increment"))?;
    // let _ = ui.click(text("Decrement"))?;

    // for message in ui.into_messages() {
    //     counter.update(message);
    // }

    // assert_eq!(counter.value, 1);

    // let mut ui = simulator(counter.view());
    // assert!(ui.find(text("1")).is_ok(), "Counter should display 1!");

    // Ok(())
    // }
}
