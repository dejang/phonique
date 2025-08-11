use iced::{
    Element, Length,
    alignment::{
        Horizontal::{self},
        Vertical,
    },
    widget::{button, column, container, text, text_input},
};

use crate::icons;

#[derive(Debug, Default)]
pub struct Login {
    username: String,
    password: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    UsernameChanged(String),
    PasswordChanged(String),
    LoginPressed,
}

impl Login {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::UsernameChanged(username) => self.username = username,
            Message::PasswordChanged(password) => self.password = password,
            Message::LoginPressed => {
                // TODO: Implement login logic
                println!("Login pressed with username: {}", self.username);
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        let content = column![
            text("Login").size(32),
            text_input("Username", &self.username)
                .on_input(Message::UsernameChanged)
                .padding(10)
                .size(20)
                .icon(icons::input_icon(icons::ICON_USER)),
            text_input("Password", &self.password)
                .on_input(Message::PasswordChanged)
                .secure(true)
                .padding(10)
                .size(20)
                .icon(icons::input_icon(icons::ICON_LOCK)),
            button("Login").on_press(Message::LoginPressed).padding(10),
        ]
        .spacing(20)
        .padding(20)
        .width(Length::Fixed(320.0))
        .align_x(Horizontal::Center);

        container(content)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
