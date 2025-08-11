use iced::{
    Element, Length, Padding, Task,
    alignment::Vertical,
    widget::{self, text_input::focus},
};

use crate::{
    app_state::{Section, state_impl::State},
    fonts::ICON,
    icons::{ICON_BOOK_MARKED, ICON_BOOKMARK, ICON_PLUS},
    sidebar::{ITEM_PADDING_LEFT_RIGHT, ITEM_PADDING_TOP_BOTTOM, header, item_with_icon},
    storage::Tag,
    widgets::container::MenuState,
};

static NEW_TAG_INPUT_ID: &str = "adding_playlist";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuOptions {
    Rename,
    Delete,
    Clear,
}

impl std::fmt::Display for MenuOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MenuOptions::Rename => f.write_str("Rename Tag"),
            MenuOptions::Delete => f.write_str("Delete Tag"),
            MenuOptions::Clear => f.write_str("Clear Tag"),
        }
    }
}

static CONTEXT_MENU: &[MenuOptions] =
    &[MenuOptions::Rename, MenuOptions::Delete, MenuOptions::Clear];

#[derive(Debug, Clone)]
pub enum Message {
    Adding,
    TypingName(String),
    Created(Option<i64>, String),
    ContextAction(MenuOptions, i64, String),
    Selected(Section),
    ContextMenuHover(Option<usize>),
    ContextHide,
}

pub struct Tags {
    new_tag: bool,
    editing: Option<i64>,
    name: String,
    menu_state: MenuState<'static, MenuOptions>,
}

impl Default for Tags {
    fn default() -> Self {
        Tags {
            new_tag: false,
            editing: None,
            name: String::new(),
            menu_state: MenuState::new(CONTEXT_MENU),
        }
    }
}

impl Tags {
    fn tag_element<'a>(&'a self, tag: &'a Tag, selected: bool) -> Element<'a, Message> {
        item_with_icon(
            &tag.name,
            ICON_BOOKMARK,
            selected,
            Some(self.menu_state.clone()),
        )
        .on_select(|_| Message::Selected(Section::Tag(tag.id)))
        .on_menu_select(|_, option| Message::ContextAction(option, tag.id, tag.name.clone()))
        .on_menu_hover(|option| {
            Message::ContextMenuHover(CONTEXT_MENU.iter().position(|t| t.eq(&option)))
        })
        .on_menu_close(Message::ContextHide)
        .into()
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Adding => {
                self.name = String::new();
                self.new_tag = true;
                self.editing = None;
                return focus(NEW_TAG_INPUT_ID);
            }
            Message::TypingName(value) => {
                self.name = value;
            }
            Message::Created(_, _) => {
                self.editing = None;
                self.new_tag = false;
            }
            Message::ContextAction(option, id, name) => {
                if option == MenuOptions::Rename {
                    self.new_tag = false;
                    self.editing = Some(id);
                    self.name = name;
                    return focus(NEW_TAG_INPUT_ID);
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
            widget::TextInput::new("Name your tag", &self.name)
                .id(NEW_TAG_INPUT_ID)
                .on_input(Message::TypingName)
                .on_submit(Message::Created(id, self.name.to_string())),
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
            header("Tags").width(Length::Fill).into(),
            widget::mouse_area(widget::Text::new(ICON_PLUS).font(ICON).size(20))
                .interaction(iced::mouse::Interaction::Pointer)
                .on_press(Message::Adding)
                .into(),
        ])
        .align_y(Vertical::Center)
        .padding(Padding::default().right(ITEM_PADDING_LEFT_RIGHT / 2.0))
        .width(Length::Fill);

        let mut elements: Vec<Element<'a, Message>> = vec![header_row.into()];
        if self.new_tag {
            elements.push(self.editing_field(None));
        }

        state.tags().iter().for_each(|t| {
            let element = if let Some(id) = self.editing
                && id == t.id
            {
                self.editing_field(Some(id))
            } else {
                self.tag_element(&t, section.eq(&Section::Tag(t.id)))
            };
            elements.push(element);
        });

        widget::Column::from_vec(elements)
            .width(Length::Fill)
            .into()
    }
}
