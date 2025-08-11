use std::collections::VecDeque;

use crate::{
    app_state::{AudioPlayable, PlayableId, state_impl::State},
    fonts::{ICON, SANS_BOLD},
    icons::ICON_SQUARE_SPLIT_HORIZONTAL,
    util::playable_artwork,
    widgets::{
        column::{Column, find_position},
        compact_row,
    },
};
use iced::{
    Element, Length, Padding, Subscription, Task,
    advanced::widget::Id,
    alignment::{Horizontal, Vertical},
    event,
    keyboard::{Key, key},
    widget::{
        Row, Scrollable, container, horizontal_rule, mouse_area,
        scrollable::{self, scroll_to},
        text, text_input,
    },
};

static COL_ID: &str = "compact_col";
static SCROLLABLE_ID: &str = "compact_scrollable";

#[derive(Debug, Clone)]
pub enum Message {
    Selected(usize),
    DblClick(usize, PlayableId),
    RightClick(usize),
    SelectionModifierKey(Option<Key>),
    DeleteSelection,
    RemovePlayables(Vec<usize>, bool),
    ScrollTo(usize),
    ScrollEnd(usize),
    ToggleDetails,
}

#[derive(Default)]
pub struct CompactView {
    currently_selected_index: VecDeque<usize>,
    selection_modifier_key: Option<Key>,
    details: bool,
}

impl CompactView {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ScrollTo(index) => {
                return find_position(Id::new(COL_ID), index)
                    .and_then(|y_pos| {
                        scroll_to(
                            scrollable::Id::new(SCROLLABLE_ID),
                            scrollable::AbsoluteOffset { x: 0.0, y: y_pos },
                        )
                    })
                    .map(Message::ScrollEnd);
            }
            Message::Selected(index) => {
                compute_selection(
                    index,
                    &mut self.currently_selected_index,
                    &self.selection_modifier_key,
                );
            }
            Message::DblClick(index, _) => {
                self.currently_selected_index.clear();
                self.currently_selected_index.push_front(index);
            }
            Message::SelectionModifierKey(modifier) => {
                self.selection_modifier_key = modifier;
            }
            Message::DeleteSelection => {
                let to_trash = matches!(
                    self.selection_modifier_key,
                    Some(Key::Named(key::Named::Shift))
                );
                return Task::done(Message::RemovePlayables(
                    self.currently_selected_index.clone().into(),
                    to_trash,
                ));
            }
            Message::ToggleDetails => {
                self.details = !self.details;
            }
            _ => {}
        }

        Task::none()
    }

    pub fn view<'a>(&'a self, state: &'a State) -> Element<'a, Message> {
        let row_sizes = (
            Length::Fixed(50.),
            Length::FillPortion(5),
            Length::FillPortion(4),
            Length::FillPortion(2),
            Length::FillPortion(2),
        );

        let header: Element<Message> = std::convert::Into::<Element<Message>>::into(
            iced::widget::row![
                text("#").font(SANS_BOLD).size(16).width(row_sizes.0),
                text("Title").font(SANS_BOLD).size(16).width(row_sizes.1),
                text("Album").font(SANS_BOLD).size(16).width(row_sizes.2),
                text("Genre").font(SANS_BOLD).size(16).width(row_sizes.3),
                container(text("Duration").font(SANS_BOLD).size(16),)
                    .align_x(Horizontal::Right)
                    .width(row_sizes.4)
            ]
            .width(Length::Fill)
            .padding(Padding {
                left: 20.0,
                right: 20.0,
                bottom: 20.0,
                top: 0.0,
            })
            .spacing(40),
        );

        let compact_column = Column::new().push(header);

        let (_, count) = state.playables().size_hint();
        let mut rows = Column::new().id(iced::advanced::widget::Id::new(COL_ID));
        for (i, playable) in state.playables().enumerate() {
            let is_selected = self.currently_selected_index.contains(&i);
            let row = compact_row::compact_row(playable, i, is_selected, &row_sizes)
                .on_select(Message::Selected(i))
                .on_dbl_click(Message::DblClick(i, playable.get_id()));
            rows = rows.push(row);
        }

        let mut details_bar: Row<Message> = iced::widget::Row::new().align_y(Vertical::Center);
        let count = count.unwrap_or(0);
        details_bar = details_bar
            .push(text(format!("{count} entries")).size(15))
            .push(
                mouse_area(text(ICON_SQUARE_SPLIT_HORIZONTAL).font(ICON).size(18))
                    .on_press(Message::ToggleDetails),
            )
            .spacing(5);

        let compact_column = compact_column
            .push(
                Scrollable::new(rows)
                    .id(scrollable::Id::new(SCROLLABLE_ID))
                    .height(Length::Fill),
            )
            .push(
                container(details_bar)
                    .align_right(Length::Fill)
                    .height(Length::Fixed(22.0)),
            );
        let mut split_view = Row::new().width(Length::Fill).push(compact_column);
        if self.details {
            split_view = split_view.push(self.details_panel(state).width(Length::Fixed(400.0)));
        }
        split_view.into()
    }
    fn details_panel<'a>(&self, state: &'a State) -> container::Container<'a, Message> {
        let content: Element<Message> = if self.currently_selected_index.is_empty() {
            text("No song selected").into()
        } else if self.currently_selected_index.len() == 1 {
            let playable = state
                .playables()
                .nth(*self.currently_selected_index.front().unwrap())
                .unwrap();

            playable_details(playable).into()
        } else {
            text("Many elements").into()
        };
        container(content).padding(Padding::default().left(16).right(16))
    }
    pub fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status, _| {
            if status.eq(&event::Status::Captured) {
                return None;
            }
            match event {
                iced::Event::Keyboard(event) => match event {
                    iced::keyboard::Event::KeyPressed {
                        key,
                        modified_key,
                        physical_key: _,
                        location: _,
                        modifiers: _,
                        text: _,
                    } => {
                        if Key::Named(key::Named::Delete) == key {
                            return Some(Message::DeleteSelection);
                        }
                        match modified_key {
                            Key::Named(key::Named::Shift) => {
                                Some(Message::SelectionModifierKey(Some(modified_key)))
                            }
                            Key::Named(key::Named::Control) => {
                                Some(Message::SelectionModifierKey(Some(modified_key)))
                            }
                            _ => None,
                        }
                    }
                    iced::keyboard::Event::KeyReleased {
                        key: _,
                        modified_key,
                        physical_key: _,
                        location: _,
                        modifiers: _,
                    } => match modified_key {
                        Key::Named(key::Named::Shift) => Some(Message::SelectionModifierKey(None)),
                        Key::Named(key::Named::Control) => {
                            Some(Message::SelectionModifierKey(None))
                        }
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            }
        })
    }
}

fn playable_details<'a>(playable: &'a impl AudioPlayable) -> Column<'a, Message> {
    let (label_width, input_width) = (Length::FillPortion(1), Length::FillPortion(3));
    let header = Row::new()
        .width(Length::Fill)
        .spacing(10)
        .push(playable_artwork(playable, 120, 120))
        .push(text(playable.get_title()).font(SANS_BOLD).size(20));

    let track_title = Row::new()
        .width(Length::Fill)
        .spacing(10)
        .align_y(Vertical::Center)
        .push(text("Title").width(label_width))
        .push(text_input("Edit track title", playable.get_title()).width(input_width));

    let artist = Row::new()
        .width(Length::Fill)
        .spacing(10)
        .align_y(Vertical::Center)
        .push(text("Artist").width(label_width))
        .push(text_input("Edit artist name", playable.get_artist()).width(input_width));

    let genre = Row::new()
        .width(Length::Fill)
        .spacing(10)
        .align_y(Vertical::Center)
        .push(text("Genre").width(label_width))
        .push(text_input("Edit genre", playable.get_genre()).width(input_width));

    let album = Row::new()
        .width(Length::Fill)
        .spacing(10)
        .align_y(Vertical::Center)
        .push(text("Album").width(label_width))
        .push(text_input("Edit album", playable.get_album()).width(input_width));
    let path = Row::new()
        .width(Length::Fill)
        .spacing(10)
        .align_y(Vertical::Center)
        .push(text("Source Path").width(label_width))
        .push(text(playable.get_path()).width(input_width));

    Column::new()
        .spacing(5)
        .width(Length::Fill)
        .push(header)
        .push(horizontal_rule(1))
        .push(track_title)
        .push(horizontal_rule(1))
        .push(artist)
        .push(horizontal_rule(1))
        .push(album)
        .push(horizontal_rule(1))
        .push(genre)
        .push(horizontal_rule(1))
        .push(path)
}

fn compute_selection(index: usize, indexes: &mut VecDeque<usize>, modifier_key: &Option<Key>) {
    if modifier_key.is_none() {
        indexes.clear();
        indexes.push_front(index);
        return;
    }
    let modifier_key = modifier_key.as_ref().unwrap();
    if let Key::Named(key::Named::Shift) = modifier_key {
        if indexes.is_empty() {
            indexes.push_front(index);
        } else if index < indexes[0] {
            let start = indexes.pop_front().unwrap();
            *indexes = (index..=start).rev().collect();
        } else if index > indexes[0] {
            let start = indexes.pop_front().unwrap();
            *indexes = (start..=index).collect();
        }
    } else if let Key::Named(key::Named::Control) = modifier_key {
        indexes.push_front(index);
    }
}
#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use iced::keyboard::{Key, key};

    use crate::view_types::compact_view::compute_selection;

    #[test]
    fn test_compute_single_selection() {
        let mut indexes: VecDeque<usize> = VecDeque::new();
        compute_selection(1, &mut indexes, &None);
        assert_eq!(indexes, VecDeque::from(vec![1]));
    }

    #[test]
    fn test_compute_shift_selection() {
        let mut indexes: VecDeque<usize> = VecDeque::new();
        compute_selection(5, &mut indexes, &Some(Key::Named(key::Named::Shift)));
        assert_eq!(indexes, VecDeque::from(vec![5]));
    }

    #[test]
    fn test_compute_shift_selection_merge_up() {
        let mut indexes: VecDeque<usize> = VecDeque::from(vec![5, 6, 7]);
        compute_selection(2, &mut indexes, &Some(Key::Named(key::Named::Shift)));
        assert_eq!(indexes, VecDeque::from(vec![5, 4, 3, 2]));
    }
    #[test]
    fn test_compute_shift_selection_merge_down() {
        let mut indexes: VecDeque<usize> = VecDeque::from(vec![5, 6, 7, 8]);
        compute_selection(7, &mut indexes, &Some(Key::Named(key::Named::Shift)));
        assert_eq!(indexes, VecDeque::from(vec![5, 6, 7]));

        compute_selection(10, &mut indexes, &Some(Key::Named(key::Named::Shift)));
        assert_eq!(indexes, VecDeque::from(vec![5, 6, 7, 8, 9, 10]));
    }

    #[test]
    fn test_compute_shift_selection_merge_up_down() {
        let mut indexes: VecDeque<usize> = VecDeque::from(vec![5, 6, 7]);
        compute_selection(10, &mut indexes, &Some(Key::Named(key::Named::Shift)));
        compute_selection(3, &mut indexes, &Some(Key::Named(key::Named::Shift)));
        assert_eq!(indexes, VecDeque::from(vec![5, 4, 3]));
    }

    #[test]
    fn test_compute_ctrl_selection_merge() {
        let mut indexes: VecDeque<usize> = VecDeque::from(vec![5, 6, 7]);
        compute_selection(10, &mut indexes, &Some(Key::Named(key::Named::Control)));
        assert_eq!(indexes, VecDeque::from(vec![10, 5, 6, 7]));
    }
}
