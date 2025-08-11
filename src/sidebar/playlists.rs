use iced::{
    Element, Length, Padding, Task,
    alignment::Vertical,
    widget::{self, mouse_area, text_input::focus},
};

use crate::{
    app_state::{Section, state_impl::State},
    fonts::ICON,
    icons::{ICON_LIST_MUSIC, ICON_PLUS},
    sidebar::{ITEM_PADDING_LEFT_RIGHT, ITEM_PADDING_TOP_BOTTOM, header, item_with_icon},
    storage::{Playlist, PlaylistKind},
    widgets::container::MenuState,
};

static PLAYLIST_CONTEXT_MENU: &[MenuOptions] =
    &[MenuOptions::Rename, MenuOptions::Delete, MenuOptions::Clear];

static NEW_PLAYLIST_INPUT_ID: &str = "adding_playlist";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuOptions {
    Rename,
    Delete,
    Clear,
}

impl std::fmt::Display for MenuOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MenuOptions::Rename => f.write_str("Rename Playlist"),
            MenuOptions::Delete => f.write_str("Delete Playlist"),
            MenuOptions::Clear => f.write_str("Clear Playlist"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    AddingPlaylist,
    TypingPlaylistName(String),
    CreatedPlaylist(Option<i64>, String, Option<PlaylistKind>),
    ContextAction(MenuOptions, i64, String),
    Selected(Section),
    ContextMenuHover(Option<usize>),
    ContextHide,
}

pub struct Playlists {
    new_playlist: bool,
    editing_playlist: Option<i64>,
    name: String,
    menu_state: MenuState<'static, MenuOptions>,
}

impl Default for Playlists {
    fn default() -> Self {
        Self {
            new_playlist: false,
            editing_playlist: None,
            name: String::new(),
            menu_state: MenuState::new(PLAYLIST_CONTEXT_MENU),
        }
    }
}

impl Playlists {
    fn playlist_element<'a>(
        &'a self,
        playlist: &'a Playlist,
        selected: bool,
    ) -> Element<'a, Message> {
        item_with_icon(
            &playlist.name,
            ICON_LIST_MUSIC,
            selected,
            Some(self.menu_state.clone()),
        )
        .on_select(|_| Message::Selected(Section::Playlist(playlist.id)))
        .on_menu_select(|_, option| {
            Message::ContextAction(option, playlist.id, playlist.name.clone())
        })
        .on_menu_hover(|option| {
            Message::ContextMenuHover(PLAYLIST_CONTEXT_MENU.iter().position(|p| p.eq(&option)))
        })
        .on_menu_close(Message::ContextHide)
        .into()
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddingPlaylist => {
                self.name = String::new();
                self.new_playlist = true;
                self.editing_playlist = None;
                return focus(NEW_PLAYLIST_INPUT_ID);
            }
            Message::TypingPlaylistName(value) => {
                self.name = value;
            }
            Message::CreatedPlaylist(_, _, _) => {
                self.editing_playlist = None;
                self.new_playlist = false;
            }
            Message::ContextAction(option, id, name) => {
                if option == MenuOptions::Rename {
                    self.new_playlist = false;
                    self.editing_playlist = Some(id);
                    self.name = name;
                    return focus(NEW_PLAYLIST_INPUT_ID);
                }
            }
            Message::ContextMenuHover(index) => {
                self.menu_state.selected = index;
            }
            Message::ContextHide => {
                self.menu_state.selected = None;
            }
            _ => {}
        };
        Task::none()
    }

    fn editing_field<'a>(&'a self, id: Option<i64>) -> Element<'a, Message> {
        widget::container(
            widget::TextInput::new("Name your playlist", &self.name)
                .id(NEW_PLAYLIST_INPUT_ID)
                .on_input(Message::TypingPlaylistName)
                .on_submit(Message::CreatedPlaylist(id, self.name.to_string(), None)),
        )
        .padding(Padding {
            top: ITEM_PADDING_TOP_BOTTOM,
            bottom: ITEM_PADDING_TOP_BOTTOM,
            left: ITEM_PADDING_LEFT_RIGHT / 2.0,
            right: ITEM_PADDING_LEFT_RIGHT / 2.0,
        })
        .into()
    }
    pub fn view<'a>(&'a self, state: &'a State, section: &'a Section) -> Element<'a, Message> {
        let header_row = widget::Row::from_vec(vec![
            header("Playlists").width(Length::Fill).into(),
            mouse_area(widget::Text::new(ICON_PLUS).font(ICON).size(20))
                .interaction(iced::mouse::Interaction::Pointer)
                .on_press(Message::AddingPlaylist)
                .into(),
        ])
        .align_y(Vertical::Center)
        .padding(Padding::default().right(ITEM_PADDING_LEFT_RIGHT / 2.0))
        .width(Length::Fill);

        let mut elements = vec![header_row.into()];
        if self.new_playlist {
            elements.push(self.editing_field(None));
        }
        state.playlists().iter().for_each(|p| {
            let element = if let Some(id) = self.editing_playlist
                && id == p.value.id
            {
                self.editing_field(Some(id))
            } else {
                self.playlist_element(&p.value, section.eq(&Section::Playlist(p.value.id)))
            };
            elements.push(element);
        });

        widget::Column::from_vec(elements)
            .width(Length::Fill)
            .into()
    }
}
