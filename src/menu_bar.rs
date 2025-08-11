use std::path::PathBuf;

use iced::{
    Border, Element, Length, Padding, Task,
    advanced::widget::Id,
    alignment::{Horizontal, Vertical},
    widget::{Column, Row, container, horizontal_rule, text_input},
};

use crate::{
    icons,
    widgets::button_with_menu::{ButtonWithMenu, clicked_overlay},
};

#[derive(Clone, Debug)]
pub enum Message {
    FileOptionSelected(usize),
    OpenFile,
    OpenFolder,
    SearchTypeIn(String),
    Search(String),
    MetadataScanningStarted(Option<PathBuf>),
}

pub struct MenuBar {
    search_string: String,
    file_button_menu_id: Id,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self {
            search_string: Default::default(),
            file_button_menu_id: Id::unique(),
        }
    }
}

impl MenuBar {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FileOptionSelected(option) => {
                clicked_overlay(self.file_button_menu_id.clone()).map(move |_| {
                    if option == 0 {
                        return Message::OpenFile;
                    } else {
                        return Message::OpenFolder;
                    }
                })
            }
            Message::OpenFile => {
                let open_file_task = Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .pick_file()
                            .await
                            .map(|file| file.path().to_path_buf())
                    },
                    Message::MetadataScanningStarted,
                );
                Task::batch([open_file_task])
            }
            Message::OpenFolder => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_folder()
                        .await
                        .map(|file| file.path().to_path_buf())
                },
                Message::MetadataScanningStarted,
            ),
            Message::SearchTypeIn(v) => {
                self.search_string = v;
                if self.search_string.is_empty() {
                    return Task::done(Message::Search(String::new()));
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }
    pub fn view(&self) -> Element<Message> {
        let search = container(
            text_input("Search", &self.search_string)
                .width(Length::Fixed(200.0))
                .icon(icons::input_icon(icons::ICON_SEARCH))
                .on_input(Message::SearchTypeIn)
                .on_submit(Message::Search(self.search_string.clone())),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Right);
        let file_menu = ButtonWithMenu::new("File", &["Add File", "Add Folder"])
            .set_id(&self.file_button_menu_id)
            .on_option_select(Message::FileOptionSelected);
        let menubar = Row::new()
            .push(file_menu)
            .push(search)
            .padding(Padding {
                top: 5.0,
                right: 16.0,
                bottom: 5.0,
                left: 16.0,
            })
            .align_y(Vertical::Center)
            .spacing(16);
        container(Column::new().push(menubar).push(horizontal_rule(1)))
            .style(|theme: &iced::Theme| iced::widget::container::Style {
                text_color: Some(theme.palette().text),
                background: Some(iced::Background::Color(
                    theme.palette().primary.scale_alpha(0.1),
                )),
                border: Border::default().width(0),
                ..Default::default()
            })
            .into()
    }
}
