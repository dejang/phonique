pub mod playlists;
pub mod tags;
use std::fmt::Debug;

use iced::{Background, Element, Length, Padding, Task, alignment::Vertical, widget};

use crate::{
    app_state::{Section, state_impl::State},
    fonts::{ICON, SANS_BOLD},
    icons::{ICON_CLOCK, ICON_HEART, ICON_HOUSE, ICON_LIBRARY, ICON_SEARCH},
    sidebar::{playlists::Playlists, tags::Tags},
    widgets::container::{Container, MenuState, Style},
};

pub const HEADER_FONT_SIZE: f32 = 20.0;
pub const ITEM_FONT_SIZE: f32 = 16.0;
pub const ITEM_SPACING: f32 = 4.0;
pub const SECTION_SPACING: f32 = 16.0;
pub const ITEM_PADDING_TOP_BOTTOM: f32 = 8.0;
pub const ITEM_PADDING_LEFT_RIGHT: f32 = 16.0;

#[derive(Debug, Clone)]
pub enum Message {
    Selected(Section),
    Playlists(playlists::Message),
    Tags(tags::Message),
}

#[derive(Clone, Eq, PartialEq)]
enum NoMenu {}
impl std::fmt::Display for NoMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("")
    }
}

pub fn header<'a, M>(text: &'a str) -> widget::Container<'a, M>
where
    M: 'a + Clone,
{
    widget::container(
        widget::Text::new(text)
            .font(SANS_BOLD)
            .size(HEADER_FONT_SIZE),
    )
    .padding(
        Padding::default()
            .left(ITEM_PADDING_LEFT_RIGHT)
            .bottom(ITEM_PADDING_TOP_BOTTOM)
            .top(ITEM_PADDING_TOP_BOTTOM),
    )
}

pub fn item<'a, T, M>(
    content: impl Into<Element<'a, M>>,
    selected: bool,
    menu_state: Option<MenuState<'a, T>>,
) -> Container<'a, T, M, iced::Theme>
where
    T: 'a + Clone + std::fmt::Display + Eq,
    M: 'a + Clone,
{
    Container::new(
        widget::container(content)
            .width(Length::Fill)
            .padding(Padding {
                top: ITEM_PADDING_TOP_BOTTOM,
                bottom: ITEM_PADDING_TOP_BOTTOM,
                left: ITEM_PADDING_LEFT_RIGHT / 2.0,
                right: ITEM_PADDING_LEFT_RIGHT,
            })
            .style(move |theme: &iced::Theme| {
                let color = if selected {
                    theme.palette().background.scale_alpha(0.7)
                } else {
                    theme.palette().background
                };
                iced::widget::container::Style {
                    background: Some(Background::Color(color)),
                    ..Default::default()
                }
            }),
        menu_state,
    )
    .width(Length::Fill)
    .padding(Padding::default().left(ITEM_PADDING_LEFT_RIGHT / 2.0))
    .style(move |theme: &iced::Theme, _| {
        let bg_color = if selected {
            theme.palette().primary.scale_alpha(0.7)
        } else {
            theme.palette().background
        };
        Style {
            background_color: bg_color,
            ..Default::default()
        }
    })
}

pub fn item_with_icon<'a, T, M>(
    text: &'a str,
    icon: char,
    selected: bool,
    menu_state: Option<MenuState<'a, T>>,
) -> Container<'a, T, M, iced::Theme>
where
    T: 'a + Clone + std::fmt::Display + Eq,
    M: 'a + Clone,
{
    let content = widget::Row::from_vec(vec![
        widget::Text::new(icon)
            .font(ICON)
            .size(ITEM_FONT_SIZE - 1.0)
            .into(),
        widget::Text::new(text).size(ITEM_FONT_SIZE).into(),
    ])
    .spacing(ITEM_SPACING)
    .width(Length::Fill)
    .align_y(Vertical::Center);
    item(content, selected, menu_state)
}

pub fn static_content<'a>(selected_section: &Section) -> Vec<Element<'a, Message>> {
    vec![
        // Music section
        widget::Column::from_vec(vec![
            header("Music").width(Length::Fill).into(),
            item_with_icon(
                "Listen Now",
                ICON_HOUSE,
                selected_section.eq(&Section::ListenNow),
                None::<MenuState<'a, NoMenu>>,
            )
            .into(),
            item_with_icon(
                "Browse",
                ICON_SEARCH,
                selected_section.eq(&Section::Browse),
                None::<MenuState<'a, NoMenu>>,
            )
            .on_select(|_| Message::Selected(Section::Browse))
            .into(),
        ])
        .width(Length::Fill)
        .into(),
        // Your Music section
        widget::Column::from_vec(vec![
            header("Your Music").width(Length::Fill).into(),
            item_with_icon(
                "Library",
                ICON_LIBRARY,
                selected_section.eq(&Section::Library),
                None::<MenuState<'a, NoMenu>>,
            )
            .on_select(|_| Message::Selected(Section::Library))
            .into(),
            item_with_icon(
                "Favorites",
                ICON_HEART,
                selected_section.eq(&Section::Favorites),
                None::<MenuState<'a, NoMenu>>,
            )
            .on_select(|_| Message::Selected(Section::Favorites))
            .into(),
            item_with_icon(
                "Recently Played",
                ICON_CLOCK,
                selected_section.eq(&Section::RecentlyPlayed),
                None::<MenuState<'a, NoMenu>>,
            )
            .on_select(|_| Message::Selected(Section::RecentlyPlayed))
            .into(),
        ])
        .width(Length::Fill)
        .into(),
    ]
}

#[derive(Default)]
pub struct Sidebar {
    playlists: Playlists,
    tags: Tags,
}
impl Sidebar {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Playlists(msg) => {
                if let playlists::Message::Selected(v) = msg {
                    return Task::done(Message::Selected(v));
                }
                return self.playlists.update(msg).map(Message::Playlists);
            }
            Message::Tags(msg) => {
                if let tags::Message::Selected(v) = msg {
                    return Task::done(Message::Selected(v));
                }
                return self.tags.update(msg).map(Message::Tags);
            }
            _ => {}
        };
        Task::none()
    }
    pub fn view<'a>(
        &'a self,
        selected_section: &'a Section,
        state: &'a State,
    ) -> Element<'a, Message> {
        let mut elements = static_content(selected_section);
        elements.push(
            self.playlists
                .view(state, selected_section)
                .map(Message::Playlists),
        );
        elements.push(self.tags.view(state, selected_section).map(Message::Tags));
        widget::Column::from_vec(elements)
            .spacing(SECTION_SPACING)
            .width(Length::Fill)
            .into()
    }
}
