use iced::{
    Color, Element, Length, Task,
    widget::{self, Row},
};
use std::io::{Error as IOError, ErrorKind};
use std::{collections::HashMap, io::Cursor, sync::Arc};

use crate::{
    app_state::{AudioPlayable, PlayableId, PlayableKind},
    fonts::{ICON, SANS_BOLD},
    icons::ICON_PLAY,
    player,
    widgets::compact_row::CompactRow,
};

use super::{
    crawler::{Stream, crawl_release},
    models::Release,
};

#[derive(Clone, Debug)]
pub enum Message {
    AppendRelease(Box<Release>),
    UpdateReleaseTracks(i32, Box<Vec<Vec<Option<Stream>>>>),
    CrawlerError(i32, String),
    PlayAction(Arc<dyn AudioPlayable>),
    OpenUrl(String),
    Play(player::Message),
}

#[derive(Debug, Clone)]
pub enum DiscogsTrackStatus {
    Loading,
    Complete,
    Failed,
}

#[derive(Default, Debug)]
struct PlaylistState {
    playables: Vec<DiscogsTrack>,
    lookup_mapping: HashMap<i32, Vec<usize>>,
}

#[derive(Default, Debug)]
pub struct DiscogsPlaylist {
    state: PlaylistState,
}

impl DiscogsPlaylist {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppendRelease(release) => {
                let clone = *release.clone();
                let id = release.id;
                let task = Task::perform(
                    async move { (crawl_release(clone).await, id) },
                    |(result, id)| match result {
                        Ok(streams) => Message::UpdateReleaseTracks(id, Box::new(streams)),
                        Err(err) => Message::CrawlerError(id, err.to_string()),
                    },
                );
                let tracks: Vec<DiscogsTrack> = (*release).into();
                self.state.append(id, tracks);
                return task;
            }
            Message::CrawlerError(release_id, err) => {
                log::error!("{err}");
                self.state
                    .status_update(release_id, DiscogsTrackStatus::Failed);
            }
            Message::UpdateReleaseTracks(release_id, streams) => {
                self.state.update_streams(release_id, *streams);
                self.state
                    .status_update(release_id, DiscogsTrackStatus::Complete);
            }
            Message::PlayAction(playable) => {
                return Task::done(Message::Play(player::Message::Play(playable)));
            }
            Message::OpenUrl(url) => {
                if let Err(error) = open::that(url) {
                    log::error!("{error}");
                }
            }
            _ => {}
        };
        Task::none()
    }
    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let col_names = ["", "#", "Artist", "Title", "#Cat", "Sources"];
        let col_widths = [
            Length::Fixed(25.0),
            Length::Fixed(25.0),
            Length::FillPortion(2),
            Length::FillPortion(2),
            Length::FillPortion(1),
            Length::FillPortion(1),
        ];

        let header_cells: Vec<Element<'a, Message>> = col_names
            .iter()
            .zip(col_widths.iter())
            .map(|(name, width)| {
                widget::Text::new(*name)
                    .font(SANS_BOLD)
                    .size(15)
                    .width(*width)
                    .into()
            })
            .collect();
        let header = Row::from_vec(header_cells).width(Length::Fill);

        let rows: Vec<Element<'a, Message>> = self
            .state
            .playables
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let text_color = |theme: &iced::Theme| match p.status {
                    DiscogsTrackStatus::Loading => widget::text::Style {
                        color: Some(theme.extended_palette().secondary.weak.color),
                    },
                    DiscogsTrackStatus::Complete => widget::text::Style {
                        color: Some(theme.extended_palette().primary.strong.color),
                    },
                    DiscogsTrackStatus::Failed => widget::text::Style {
                        color: Some(theme.extended_palette().warning.strong.color),
                    },
                };
                let mut compact_row = CompactRow::new(false);
                if !p.streams.is_empty() {
                    compact_row = compact_row.push(
                        widget::button(widget::Text::new(ICON_PLAY).font(ICON).size(14))
                            .width(col_widths[0]),
                    );
                } else {
                    compact_row = compact_row.push(widget::Space::new(col_widths[0], 12.0));
                }

                compact_row
                    .push(
                        widget::Text::new((i + 1).to_string())
                            .style(text_color)
                            .width(col_widths[1])
                            .size(13),
                    )
                    .push(
                        widget::Text::new(p.get_artist())
                            .style(text_color)
                            .width(col_widths[2])
                            .size(13),
                    )
                    .push(
                        widget::Text::new(p.get_title())
                            .style(text_color)
                            .width(col_widths[3])
                            .size(13),
                    )
                    .push(
                        widget::Text::new(&p.catno)
                            .style(text_color)
                            .width(col_widths[4])
                            .size(13),
                    )
                    .push(
                        Row::from_vec(
                            p.streams
                                .iter()
                                .map(|s| {
                                    widget::Button::new(
                                        widget::Text::new(s.craler_id.to_string()).size(13),
                                    )
                                    .on_press(Message::OpenUrl(s.page_url.clone()))
                                    .into()
                                })
                                .collect::<Vec<Element<Message>>>(),
                        )
                        .width(col_widths[5]),
                    )
                    .width(Length::Fill)
                    .on_dbl_click(Message::PlayAction(Arc::new(p.to_owned())))
                    .into()
            })
            .collect();

        widget::Column::new()
            .width(Length::Fill)
            .push(header)
            .push(
                widget::Scrollable::new(widget::Column::from_vec(rows).width(Length::Fill))
                    .width(Length::Fill),
            )
            .into()
    }
}

impl PlaylistState {
    pub fn append(&mut self, id: i32, tracks: Vec<DiscogsTrack>) {
        if self.lookup_mapping.contains_key(&id) {
            return;
        }

        let indexes: Vec<usize> = tracks
            .into_iter()
            .map(|t| {
                self.playables.push(t);
                self.playables.len() - 1
            })
            .collect();
        self.lookup_mapping.insert(id, indexes);
    }

    pub fn update_streams(&mut self, release_id: i32, streams: Vec<Vec<Option<Stream>>>) {
        let indexes = self.lookup_mapping.get(&release_id).unwrap();
        streams
            .into_iter()
            .zip(indexes.iter())
            .for_each(|(track_streams, track_index)| {
                let playable = self.playables.get_mut(*track_index).unwrap();
                let streams = track_streams.into_iter().flatten().collect();
                playable.streams = streams;
            });
    }

    pub fn status_update(&mut self, release_id: i32, status: DiscogsTrackStatus) {
        let indexes = self.lookup_mapping.get(&release_id).unwrap();
        indexes.iter().for_each(|i| {
            self.playables.get_mut(*i).unwrap().status = status.clone();
        });
    }
}

#[derive(Debug, Clone)]
pub struct DiscogsTrack {
    artist: String,
    title: String,
    catno: String,
    streams: Vec<Stream>,
    album: String,
    album_art: Option<Vec<u8>>,
    date_added: i64,
    pub status: DiscogsTrackStatus,
    web_path: String,
    genre: String,
}

impl From<Release> for Vec<DiscogsTrack> {
    fn from(value: Release) -> Self {
        value
            .tracklist
            .iter()
            .map(|t| DiscogsTrack {
                album: value.title.clone(),
                artist: t.artists_clean().unwrap_or(vec![value.artist()]).join(","),
                title: t.title.clone(),
                catno: value.cat_no(),
                streams: vec![],
                status: DiscogsTrackStatus::Loading,
                album_art: None,
                date_added: 0,
                web_path: value.resource_url.clone(),
                genre: value.genres.join(","),
            })
            .collect()
    }
}

impl AudioPlayable for DiscogsTrack {
    fn get_id(&self) -> PlayableId {
        // A way to mark that this playable is not part of the library yet
        -1
    }

    fn get_title(&self) -> &str {
        &self.title
    }

    fn get_album(&self) -> &str {
        &self.album
    }

    fn get_artist(&self) -> &str {
        &self.artist
    }

    fn get_date_added(&self) -> &i64 {
        &self.date_added
    }

    fn get_genre(&self) -> &str {
        &self.genre
    }

    fn get_duration(&self) -> u64 {
        0
    }

    fn get_path(&self) -> &str {
        &self.web_path
    }

    fn get_album_art(&self) -> &Option<Vec<u8>> {
        &self.album_art
    }

    fn get_kind(&self) -> PlayableKind {
        PlayableKind::Stream
    }

    fn stream(&self) -> Result<Cursor<Vec<u8>>, std::io::Error> {
        let stream = self.streams.first();
        if stream.is_none() {
            return Err(IOError::from(ErrorKind::NotFound));
        }

        let stream = stream.unwrap();
        let response = reqwest::blocking::get(&stream.audio_url).map_err(|err| {
            log::error!("{err}");
            IOError::new(ErrorKind::ConnectionRefused, err.to_string())
        })?;

        if !response.status().is_success() {
            let status_code = response.status();
            log::error!("HTTP Error: {}", response.text().unwrap());
            return Err(IOError::new(
                ErrorKind::ResourceBusy,
                status_code.to_string(),
            ));
        }

        if let Some(content_type) = response.headers().get("content-type")
            && let Ok(content_type_str) = content_type.to_str()
            && !content_type_str.starts_with("audio/")
        {
            log::error!("Unexpected content type: {content_type_str}");
            return Err(IOError::from(ErrorKind::InvalidData));
        }

        let bytes = response.bytes().map_err(IOError::other)?;
        Ok(Cursor::new(bytes.to_vec()))
    }
}
