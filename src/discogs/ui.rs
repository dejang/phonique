use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use iced::{
    Background, Color, Element, Length, Padding, Shadow, Task,
    advanced::graphics::futures::MaybeSend,
    alignment::{Horizontal, Vertical},
    widget::{self, image::Handle, text::Wrapping, vertical_rule},
};
use log::{error, trace};

use crate::{
    app_state::AudioPlayable,
    fonts::{ICON, SANS_BOLD},
    icons::ICON_DELETE,
    theme::{Theme, button::primary},
    widgets::container::{Container, MenuState, Status},
};

use super::{
    DiscogsClient,
    crawler::parser::parse_track,
    models::{SearchParams, SearchResponse, SearchResult},
    playlist::{self, DiscogsPlaylist},
};

const DEFAULT_THUMB: &[u8] = include_bytes!("../../images/default_vinyl.png");

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextMenuOptions {
    BrowseLabel,
    BrowseArtist,
    AppendToPlaylist,
}

impl std::fmt::Display for ContextMenuOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextMenuOptions::BrowseLabel => f.write_str("Browse Label"),
            ContextMenuOptions::BrowseArtist => f.write_str("Browse Artist"),
            ContextMenuOptions::AppendToPlaylist => f.write_str("Append to Playlist"),
        }
    }
}

#[derive(Debug, Default)]
struct Filter {
    name: String,
    value: String,
}

#[derive(Debug, Clone)]
pub struct CardState {
    pub title: String,
    pub cat_no: String,
    pub style: String,
    pub id: i32,
    pub label: Vec<String>,
}

#[derive(Debug, Default)]
struct State {
    results: Vec<CardState>,
    images: Vec<Handle>,
}

impl State {
    pub fn mock() -> Self {
        Self {
            results: vec![CardState {
                style: "Minimal".into(),
                title: "Artist Name - Album Name".into(),
                cat_no: "AL001".into(),
                id: 123455,
                label: vec!["Label".into()],
            }],
            images: vec![Handle::from_bytes(DEFAULT_THUMB)],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Search,
    EditingSearchTerm(String),
    AddFilter(String),
    RemoveFilter(usize),
    UpdateFilter(usize, String),
    SearchResults(String, Vec<CardState>, Vec<Option<String>>),
    SearchError(String),
    SearchEnded,
    ImageLoaded(String, usize, Handle),
    ImageError(String, usize, String),
    NewTab(String),
    SelectTab(String),
    CloseTab(String),
    Playlist(Box<playlist::Message>),
    DiscogsError(String),
    Play(Arc<dyn AudioPlayable>),
    ContextMenuSelect(i32, ContextMenuOptions),
    ContextMenuOpen,
    ConextMenuClose,
    ContextMenuHover(usize),
}

#[derive(Debug)]
pub struct DiscogsUI {
    tabs: HashMap<String, State>,
    tabs_order: Vec<String>,
    active_tab: Option<String>,
    search_term: String,
    filters: Vec<Filter>,
    is_searching: bool,
    playlist: DiscogsPlaylist,
    context_menu: MenuState<'static, ContextMenuOptions>,
}

impl Default for DiscogsUI {
    fn default() -> Self {
        Self {
            context_menu: MenuState::new(&[
                ContextMenuOptions::AppendToPlaylist,
                ContextMenuOptions::BrowseLabel,
                ContextMenuOptions::BrowseArtist,
            ]),
            tabs: HashMap::new(),
            tabs_order: vec![],
            active_tab: None,
            search_term: String::new(),
            filters: vec![],
            is_searching: false,
            playlist: DiscogsPlaylist::default(),
        }
    }
}

impl DiscogsUI {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConextMenuClose => {
                self.context_menu.selected = None;
            }
            Message::ContextMenuHover(usize) => {
                self.context_menu.selected = Some(usize);
            }
            Message::NewTab(name) => {
                self.tabs.insert(name.clone(), State::default());
                self.tabs_order.push(name);
            }
            Message::SelectTab(name) => {
                self.active_tab = Some(name);
            }
            Message::CloseTab(name) => {
                self.tabs_order = self
                    .tabs_order
                    .iter()
                    .filter(|t| **t != name)
                    .cloned()
                    .collect();
                self.tabs.remove(&name);
                if let Some(active_tab) = &self.active_tab
                    && active_tab.eq(&name)
                {
                    if self.tabs_order.is_empty() {
                        self.active_tab = None;
                    } else {
                        self.active_tab = Some(self.tabs_order.first().cloned().unwrap());
                    }
                }
            }
            Message::Search => {
                self.is_searching = true;
                let query = self.search_term.clone();
                return discogs_query(
                    query.clone(),
                    self.tabs.contains_key(&self.search_term),
                    async move {
                        DiscogsClient::default()
                            .search(SearchParams::default().query(query))
                            .await
                    },
                );
            }
            Message::SearchEnded => {
                self.is_searching = false;
            }
            Message::SearchResults(tab_name, results, image_urls) => {
                trace!("Received Search Results {results:?}");
                let state = self.tabs.get_mut(&tab_name).unwrap();
                state.results = results;
                state.images = vec![Handle::from_bytes(DEFAULT_THUMB); state.results.len()];
                let fetch_images_batch = load_images_for_tab(tab_name, &image_urls);
                return Task::none()
                    .chain(fetch_images_batch)
                    .chain(Task::done(Message::SearchEnded));
            }
            Message::EditingSearchTerm(term) => {
                self.search_term = term;
            }
            Message::SearchError(err) => {
                self.is_searching = false;
                error!("Search error: {err:?}");
            }
            Message::ImageLoaded(tab_name, index, bytes) => {
                if let Some(state) = self.tabs.get_mut(&tab_name) {
                    state.images[index] = bytes;
                }
            }
            Message::ImageError(tab_name, index, error) => {
                let state = self.tabs.get_mut(&tab_name).unwrap();
                state.images[index] = Handle::from_bytes(DEFAULT_THUMB);
                log::error!("Failed to retrieve image for result at index {index}: {error}");
            }
            Message::ContextMenuSelect(id, option) => match option {
                ContextMenuOptions::BrowseLabel => {
                    self.is_searching = true;
                    let (_, card) = self.find_card_by_id(id).unwrap();
                    if let Some(label) = card.label.first() {
                        let query = format!("label: {label}");
                        let tab_exists = self.tabs.contains_key(&query);
                        let label = label.clone();
                        return discogs_query(query.clone(), tab_exists, async move {
                            let params = SearchParams::default().label(label);
                            DiscogsClient::default().search(params).await
                        });
                    }
                }
                ContextMenuOptions::AppendToPlaylist => {
                    return Task::perform(
                        async move { DiscogsClient::default().release(id).await },
                        |result| match result {
                            Ok(release) => Message::Playlist(Box::new(
                                playlist::Message::AppendRelease(Box::new(release)),
                            )),
                            Err(error) => Message::DiscogsError(error.to_string()),
                        },
                    );
                }
                ContextMenuOptions::BrowseArtist => {
                    self.is_searching = true;
                    let (_, card) = self.find_card_by_id(id).unwrap();
                    // we parse this as a track title but really we only need to get the artists.
                    // it just so happens that a track and a album title have the same format.
                    // this means we can reuse our parser to extract what we need from this format.
                    let parsed = parse_track(&card.title);
                    let tasks = parsed.artists.iter().map(|a| {
                        let query = format!("artist: {a}");
                        let tab_exists = self.tabs.contains_key(&query);
                        let artist = a.to_string();
                        discogs_query(query.clone(), tab_exists, async move {
                            DiscogsClient::default()
                                .search(SearchParams::default().artist(artist))
                                .await
                        })
                    });
                    return Task::batch(tasks);
                }
            },

            Message::DiscogsError(err) => {
                log::error!("Error retrieving release: {err}");
            }

            Message::AddFilter(_) => {}
            Message::Playlist(msg) => {
                let task = self
                    .playlist
                    .update(*msg.clone())
                    .map(|msg| Message::Playlist(Box::new(msg)));
                if let playlist::Message::PlayAction(playable) = *msg {
                    return task.chain(Task::done(Message::Play(playable)));
                }
                return task;
            }
            _ => {}
        };

        Task::none()
    }
    pub fn view(&self) -> Element<Message> {
        let mut text_input = widget::TextInput::new("Search on Discogs..", &self.search_term);
        if !self.is_searching {
            text_input = text_input
                .on_input(Message::EditingSearchTerm)
                .on_submit(Message::Search);
        }

        let mut submit_btn = widget::Button::new("Search").style(primary);
        if !self.is_searching {
            submit_btn = submit_btn.on_press(Message::Search);
        }

        let search_bar = widget::row![text_input, submit_btn];
        let tabs: Vec<Element<Message>> = self
            .tabs_order
            .iter()
            .map(|k| tab_header(k, self.active_tab.eq(&Some(k.to_owned()))))
            .collect();
        let results: Element<Message> = if let Some(tab_name) = &self.active_tab {
            let state = self.tabs.get(tab_name).unwrap();
            widget::Scrollable::new(
                widget::Row::from_vec(
                    state
                        .results
                        .iter()
                        .zip(state.images.iter())
                        .map(|(r, i)| result_card(r, i, self.context_menu.clone()))
                        .collect(),
                )
                .width(Length::Fill)
                .spacing(20)
                .padding(5)
                .wrap(),
            )
            .into()
        } else {
            widget::Text::new("Start a new search...").into()
        };

        let left_side = widget::Column::from_vec(vec![
            search_bar.into(),
            widget::Column::from_vec(vec![
                widget::Row::from_vec(tabs).spacing(5).into(),
                widget::horizontal_rule(1).into(),
            ])
            .width(Length::Fill)
            .into(),
            results,
        ])
        .spacing(5.0)
        .width(Length::FillPortion(3));

        widget::Row::new()
            .push(left_side)
            .push(vertical_rule(1.0))
            .push(
                widget::container(
                    self.playlist
                        .view()
                        .map(|msg| Message::Playlist(Box::new(msg))),
                )
                .width(Length::FillPortion(2)),
            )
            .spacing(5.0)
            .into()
    }

    fn find_card_by_id(&self, id: i32) -> Option<(&String, &CardState)> {
        for (tab_name, state) in &self.tabs {
            if let Some(card) = state.results.iter().find(|card| card.id == id) {
                return Some((tab_name, card));
            }
        }
        None
    }
}

fn result_card<'a>(
    result: &'a CardState,
    handle: &'a widget::image::Handle,
    menu_state: MenuState<'a, ContextMenuOptions>,
) -> Element<'a, Message> {
    let card_content = widget::column![
        widget::Image::new(handle)
            .width(Length::Fixed(210.0))
            .height(Length::Fixed(210.0)),
        widget::text(&result.title)
            .font(SANS_BOLD)
            .size(15)
            .wrapping(Wrapping::WordOrGlyph),
        widget::text(&result.style).size(13),
        widget::text(&result.cat_no).size(13)
    ]
    .spacing(15)
    .clip(true)
    .align_x(Horizontal::Center);
    Container::<ContextMenuOptions, Message, iced::Theme>::new(card_content, Some(menu_state))
        .id(&result.id)
        .on_menu_select(|_, option| Message::ContextMenuSelect(result.id, option))
        .on_menu_open(Message::ContextMenuOpen)
        .on_menu_close(Message::ConextMenuClose)
        .on_menu_hover(|option| match option {
            ContextMenuOptions::BrowseLabel => Message::ContextMenuHover(1),
            ContextMenuOptions::BrowseArtist => Message::ContextMenuHover(2),
            ContextMenuOptions::AppendToPlaylist => Message::ContextMenuHover(0),
        })
        .align_x(Horizontal::Center)
        .align_y(Vertical::Top)
        .padding(iced::Padding {
            top: 0.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        })
        .border(|theme: &iced::Theme, status: &Status| match status {
            Status::Selected => iced::Border {
                color: theme.extended_palette().background.strongest.color,
                width: 1.0,
                radius: iced::border::Radius::new(5),
            },
            Status::Default => iced::Border {
                color: theme.extended_palette().background.weakest.color,
                width: 1.0,
                radius: iced::border::Radius::new(5),
            },
        })
        .width(210.0)
        .into()
}

impl From<&SearchResult> for CardState {
    fn from(value: &SearchResult) -> Self {
        Self {
            title: value.title.clone(),
            cat_no: value.catno.clone(),
            style: value.style.join(", "),
            id: value.id,
            label: value.label.clone(),
        }
    }
}

fn load_images_for_tab(tab_name: String, img_urls: &[Option<String>]) -> Task<Message> {
    let tasks: Vec<Task<Message>> = img_urls
        .iter()
        .enumerate()
        .map(|(index, url)| {
            if let Some(image_url) = url {
                let image_url = image_url.clone();
                let tab_name = tab_name.clone();
                Task::perform(async move { fetch_image(image_url).await }, move |result| {
                    match result {
                        Ok(bytes) => {
                            Message::ImageLoaded(tab_name.clone(), index, Handle::from_bytes(bytes))
                        }
                        Err(e) => Message::ImageError(tab_name.clone(), index, e.to_string()),
                    }
                })
            } else {
                Task::done(Message::ImageLoaded(
                    tab_name.clone(),
                    index,
                    Handle::from_bytes(DEFAULT_THUMB),
                ))
            }
        })
        .collect();

    Task::batch(tasks)
}

async fn fetch_image(url: String) -> Result<Vec<u8>, reqwest::Error> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

trait DiscogsResults {
    fn unique(self) -> Self;
    fn get_results(&self) -> Vec<CardState>;
    fn get_image_urls(&self) -> Vec<Option<String>>;
}

impl DiscogsResults for SearchResponse {
    fn unique(mut self) -> Self {
        let mut catno_unique: HashSet<String> = HashSet::new();
        let mut results: Vec<SearchResult> = vec![];
        for r in self.results {
            if catno_unique.contains(&r.catno) {
                continue;
            }
            catno_unique.insert(r.catno.clone());
            results.push(r);
        }
        self.results = results;
        self
    }
    fn get_results(&self) -> Vec<CardState> {
        self.results.iter().map(|r| r.into()).collect()
    }
    fn get_image_urls(&self) -> Vec<Option<String>> {
        self.results.iter().map(|r| r.thumb.clone()).collect()
    }
}

fn discogs_query<T: DiscogsResults + MaybeSend + 'static, E: ToString + MaybeSend + 'static>(
    search_term: String,
    tab_exists: bool,
    future: impl Future<Output = Result<T, E>> + Send + 'static,
) -> Task<Message> {
    let tab_task = if tab_exists {
        Task::done(Message::SelectTab(search_term.clone()))
    } else {
        Task::done(Message::NewTab(search_term.clone()))
            .chain(Task::done(Message::SelectTab(search_term.clone())))
    };
    let search_term = search_term.clone();
    tab_task.chain(Task::perform(
        async move { (future.await, search_term.clone()) },
        |(response, search_term)| match response {
            Ok(response) => {
                let response = response.unique();
                Message::SearchResults(
                    search_term,
                    response.get_results(),
                    response.get_image_urls(),
                )
            }
            Err(e) => Message::SearchError(e.to_string()),
        },
    ))
}

fn tab_header<'a>(key: &'a str, is_selected: bool) -> Element<'a, Message> {
    log::debug!("Tab Header: {key} is_selected: {is_selected}");
    let close_btn = widget::Button::new(widget::Text::new(ICON_DELETE).font(ICON))
        .style(
            |theme: &iced::Theme, _status: widget::button::Status| widget::button::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                text_color: theme.palette().text,
                ..Default::default()
            },
        )
        .padding(0)
        .on_press(Message::CloseTab(key.to_string()));

    let tab_name = widget::mouse_area(widget::Text::new(key).font(SANS_BOLD).size(14))
        .on_press(Message::SelectTab(key.to_string()));

    let tab_header = widget::container(
        widget::Row::from_vec(vec![tab_name.into(), close_btn.into()]).spacing(5),
    )
    .style(|theme: &iced::Theme| widget::container::Style {
        text_color: Some(theme.palette().text),
        background: Some(Background::Color(theme.palette().background)),
        ..Default::default()
    })
    .padding(Padding {
        top: 4.0,
        right: 4.0,
        bottom: 0.0,
        left: 4.0,
    });

    widget::container(tab_header)
        .padding(Padding::default().bottom(2.0))
        .style(move |theme: &iced::Theme| -> widget::container::Style {
            let bg_color = if is_selected {
                theme.extended_palette().background.strongest.color
            } else {
                theme.palette().background
            };
            let shadow = if is_selected {
                iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.1),
                    offset: iced::Vector::new(0.0, 5.0),
                    blur_radius: 8.0,
                }
            } else {
                Shadow::default()
            };
            widget::container::Style {
                background: Some(Background::Color(bg_color)),
                shadow,
                ..Default::default()
            }
        })
        .into()
}
