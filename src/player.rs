use std::{
    ops::Deref,
    sync::{Arc, mpsc::RecvTimeoutError},
    time::Duration,
};

use iced::{
    Border, Color, Element, Length, Padding, Shadow, Subscription, Task,
    alignment::{Horizontal, Vertical},
    event,
    futures::{SinkExt, channel::mpsc::Sender},
    keyboard::{self, key::Named},
    widget::{
        Button, Column, Row, Space, Text, button::Status, column, container, horizontal_rule, row,
        slider, text,
    },
};
use log::{debug, error, info};
use rodio::{Decoder, OutputStream, Sink, Source};

use crate::{
    app_state::{AudioPlayable, PlayableId, state_impl::State},
    fonts,
    icons::{
        ICON_CIRCLE_PAUSE, ICON_CIRCLE_PLAY, ICON_FAST_FORWARD, ICON_HEART, ICON_REWIND,
        ICON_SHUFFLE, ICON_SKIP_BACK, ICON_SKIP_FORWARD, ICON_VOLUME, ICON_VOLUME_1, ICON_VOLUME_2,
        ICON_VOLUME_OFF,
    },
    util::{duration_to_str, playable_artwork},
};

#[derive(Debug, Clone)]
pub enum Message {
    ProgressChanged(u64, u64),
    VolumeChanged(f32),
    AudioReady(Sender<Message>),
    Paused,
    Resume,
    Rewind,
    FastForward,
    Play(Arc<dyn AudioPlayable>),
    EndPlay,
    Seek(u64),
    ProgressUpdate(u64, u64),
    Next,
    Prev,
    ShuffleToggle,
    ToggleVolume,
    TogglePlay,
    Like(PlayableId),
}

pub struct Player {
    sender: Option<iced::futures::channel::mpsc::Sender<Message>>,
    volume_level: f32,
    is_playing: bool,
    is_paused: bool,
    shuffle_enabled: bool,
    duration: (u64, u64),
    current_playable: Option<Arc<dyn AudioPlayable>>,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            sender: None,
            volume_level: 100.0,
            is_playing: false,
            is_paused: false,
            shuffle_enabled: false,
            duration: (0, 0),
            current_playable: None,
        }
    }
}

impl Player {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ProgressChanged(current, total) => {
                info!("Sending progress changed message to sender");
                if let Some(sender) = &mut self.sender {
                    let _ = sender.try_send(Message::Seek(current));
                }
                self.duration = (current, total);
            }
            Message::Play(playable) => {
                info!("Sending play message to sender");
                if let Some(sender) = &mut self.sender {
                    let _ = sender.try_send(Message::Play(playable.clone()));
                }
                self.current_playable = Some(playable);
                self.is_playing = true;
                self.is_paused = false;
            }
            Message::TogglePlay => {
                if self.is_playing {
                    return Task::done(Message::Paused);
                } else {
                    return Task::done(Message::Resume);
                }
            }
            Message::Paused => {
                if let Some(sender) = &mut self.sender {
                    let _ = sender.try_send(Message::Paused);
                }
                self.is_playing = false;
                self.is_paused = true;
            }
            Message::Resume => {
                if let Some(sender) = &mut self.sender {
                    let _ = sender.try_send(Message::Resume);
                }
                self.is_playing = true;
                self.is_paused = false;
            }
            Message::Rewind => {
                if (self.is_playing || self.is_paused)
                    && let Some(sender) = &mut self.sender
                {
                    let (current, _) = self.duration;
                    let diff = (current as i64) - 30;
                    let pos = if diff < 0 { 0 } else { diff };
                    let _ = sender.try_send(Message::Seek(pos as u64));
                }
            }
            Message::FastForward => {
                if (self.is_playing || self.is_paused)
                    && let Some(sender) = &mut self.sender
                {
                    let (current, total) = self.duration;
                    let sum = current + 30;
                    let pos = if sum > total { total - 1 } else { sum };
                    let _ = sender.try_send(Message::Seek(pos));
                }
            }
            Message::ShuffleToggle => {
                self.shuffle_enabled = !self.shuffle_enabled;
            }
            Message::ToggleVolume => {
                if let Some(sender) = &mut self.sender {
                    if self.volume_level == 0.0 {
                        self.volume_level = 50.0;
                    } else {
                        self.volume_level = 0.0;
                    }
                    let _ = sender.try_send(Message::VolumeChanged(self.volume_level / 100.0));
                }
            }
            Message::VolumeChanged(v) => {
                if let Some(sender) = &mut self.sender {
                    info!("Sending volume changed message to sender");
                    self.volume_level = v;
                    let v = v / 100.0;
                    let _ = sender.try_send(Message::VolumeChanged(v));
                }
            }
            Message::ProgressUpdate(current, total) => {
                self.duration = (current, total);
            }
            Message::AudioReady(sender) => {
                info!("Got sender: {sender:?}");
                self.sender.replace(sender);
            }
            Message::EndPlay => {
                self.is_playing = false;
                self.is_paused = false;
                return Task::done(Message::Next);
            }
            _ => {}
        };
        Task::none()
    }

    pub fn view<'a>(&'a self, state: &'a State) -> Element<'a, Message> {
        let song_info: Element<Message> = if let Some(song_info) = self.song_info(state) {
            song_info.into()
        } else {
            Space::with_width(Length::Fixed(10.0)).into()
        };

        let song_info = container(song_info)
            .align_x(Horizontal::Right)
            .align_y(Vertical::Center)
            .width(Length::Fixed(300.0))
            .max_width(400.0);
        let player_controls = container(self.player_controls())
            .align_x(Horizontal::Right)
            .align_y(Vertical::Center)
            .width(Length::FillPortion(9));
        let misc_controls = container(self.misc_controls())
            .align_x(Horizontal::Right)
            .align_y(Vertical::Center)
            .width(Length::Fixed(200.0));

        column![
            horizontal_rule(1),
            row![song_info, player_controls, misc_controls]
                .spacing(50)
                .align_y(Vertical::Center)
                .height(100)
                .width(Length::Fill)
                .padding(16)
        ]
        .width(Length::Fill)
        .into()
    }
    fn player_controls(&self) -> Column<Message> {
        let (play_icon, play_message) = if self.is_playing {
            (ICON_CIRCLE_PAUSE, Message::Paused)
        } else {
            (ICON_CIRCLE_PLAY, Message::Resume)
        };
        let buttons = row![
            player_button(ICON_SKIP_BACK, None).on_press(Message::Prev),
            player_button(ICON_REWIND, None).on_press(Message::Rewind),
            player_button(play_icon, Some(30)).on_press(play_message),
            player_button(ICON_FAST_FORWARD, None).on_press(Message::FastForward),
            player_button(ICON_SKIP_FORWARD, None).on_press(Message::Next),
        ]
        .spacing(12)
        .align_y(Vertical::Center);

        let (current, total) = self.duration;
        let slider = row![
            text(duration_to_str(current)).size(12),
            slider(0.0..=total as f64, current as f64, move |v| {
                info!("{v}");
                Message::ProgressChanged(v as u64, total)
            })
            .step(0.1),
            text(duration_to_str(total)).size(12)
        ]
        .spacing(8);
        column![buttons, slider]
            .align_x(Horizontal::Center)
            .spacing(12)
    }
    fn misc_controls(&self) -> Row<Message> {
        let shuffle_button = Button::new(text(ICON_SHUFFLE).font(fonts::ICON).size(20))
            .padding(0)
            .style(|theme: &iced::Theme, status| {
                let palette = theme.palette();
                let text_color = if self.shuffle_enabled {
                    palette.text
                } else {
                    palette.text.scale_alpha(0.7)
                };
                iced::widget::button::Style {
                    text_color,
                    background: Some(iced::Background::Color(Color::TRANSPARENT)),
                    ..button_style(theme, status)
                }
            })
            .on_press(Message::ShuffleToggle);
        let volume_icon = if self.volume_level == 0.0 {
            ICON_VOLUME_OFF
        } else if self.volume_level > 0.0 && self.volume_level < 50.0 {
            ICON_VOLUME
        } else if self.volume_level >= 50.0 && self.volume_level < 100.0 {
            ICON_VOLUME_1
        } else {
            ICON_VOLUME_2
        };
        row![
            shuffle_button,
            // player_button(ICON_LIST_MUSIC, None),
            player_button(volume_icon, None).on_press(Message::ToggleVolume),
            slider(0.0..=100.0, self.volume_level, |v| {
                Message::VolumeChanged(v)
            })
            .width(100)
        ]
        .align_y(Vertical::Center)
        .spacing(8)
    }
    fn song_info<'a>(&'a self, state: &'a State) -> Option<Row<'a, Message>> {
        if let Some(current_playable) = &self.current_playable {
            let artwork: iced::Element<'a, Message> =
                playable_artwork(current_playable.deref(), 56, 56);
            let title_str = current_playable.get_title();
            let title_and_artist = Column::new()
                .push(
                    Text::new(title_str)
                        .font(fonts::SANS_BOLD)
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .size(14),
                )
                .push(
                    Text::new(current_playable.get_artist())
                        .size(14)
                        .wrapping(text::Wrapping::WordOrGlyph),
                )
                .spacing(8)
                .width(Length::Fill);

            let like_button = Button::new(text(ICON_HEART).font(fonts::ICON).size(13))
                .width(Length::Fixed(30.0))
                .on_press(Message::Like(current_playable.get_id()))
                .style(|theme: &iced::Theme, _| {
                    let palette = theme.palette();
                    let color = if state.is_liked(&current_playable.get_id()) {
                        palette.danger
                    } else {
                        palette.text
                    };
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(Color::TRANSPARENT)),
                        text_color: color,
                        border: Border::default().width(0),
                        ..Default::default()
                    }
                });
            return Some(
                row![artwork, title_and_artist, like_button]
                    .align_y(Vertical::Center)
                    .spacing(10),
            );
        }

        None
    }
    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_events = event::listen_with(|event, status, _| {
            if let iced::event::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = event
                && status == iced::event::Status::Ignored
            {
                match key {
                    keyboard::Key::Named(Named::Space) => Some(Message::TogglePlay),
                    keyboard::Key::Named(Named::ArrowLeft) => Some(Message::Rewind),
                    keyboard::Key::Named(Named::ArrowRight) => Some(Message::FastForward),
                    _ => None,
                }
            } else {
                None
            }
        });
        Subscription::batch([Subscription::run(start_audio), keyboard_events])
    }
}

fn player_button<'a>(icon: char, size: Option<u8>) -> Button<'a, Message> {
    let size = size.unwrap_or(20);
    Button::new(text(icon).font(fonts::ICON).size(size as f32))
        .padding(Padding {
            top: 1.0,
            right: 5.0,
            bottom: 1.0,
            left: 5.0,
        })
        .style(button_style)
}

fn button_style(theme: &iced::Theme, status: Status) -> iced::widget::button::Style {
    let palette = theme.palette();
    let extended_palette = theme.extended_palette();

    let background_color = match status {
        Status::Active => Color::TRANSPARENT,
        Status::Hovered => Color::TRANSPARENT,
        Status::Pressed => extended_palette.primary.weak.color.scale_alpha(0.3),
        Status::Disabled => Color::TRANSPARENT,
    };

    let text_color = match status {
        Status::Active => palette.text,
        Status::Hovered => palette.text,
        Status::Pressed => palette.text,
        Status::Disabled => palette.text.scale_alpha(0.7),
    };

    iced::widget::button::Style {
        text_color,
        background: Some(iced::Background::Color(background_color)),
        border: Border {
            color: extended_palette.secondary.weak.color,
            width: 0.0,
            radius: 20.into(),
        },
        ..iced::widget::button::Style::default()
    }
}

fn spawn_audio_worker(
    sender: iced::futures::channel::mpsc::Sender<Message>,
) -> (
    std::thread::JoinHandle<Result<(), String>>,
    std::sync::mpsc::Sender<Message>,
) {
    let (audio_tx, audio_rx) = std::sync::mpsc::channel::<Message>();
    let handle = std::thread::spawn(move || {
        audio_worker(sender, audio_rx).map_err(|e| {
            error!("{e}");
            "Player died".to_string()
        })
    });
    (handle, audio_tx)
}

fn start_audio() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(100, |mut output: Sender<Message>| async move {
        info!("Starting audio stream...");
        let (sender, mut receiver) = iced::futures::channel::mpsc::channel::<Message>(100);

        match output.send(Message::AudioReady(sender.clone())).await {
            Ok(_) => info!("Successfully sent AudioReady message"),
            Err(e) => error!("Failed to send AudioReady message: {e:?}"),
        }

        let (mut handle, mut audio_tx) = spawn_audio_worker(sender.clone());
        loop {
            use iced::futures::StreamExt;

            // our audio worker has died because of an error. We revive it.
            if handle.is_finished() {
                (handle, audio_tx) = spawn_audio_worker(sender.clone());
            }
            let input = receiver.select_next_some().await;
            if let Err(e) = audio_tx.send(input.clone()) {
                info!("Failed to send message to audio worker: {e:?}");
            }
            match input {
                Message::ProgressUpdate(current, total) => {
                    let _ = output.send(Message::ProgressUpdate(current, total)).await;
                }
                Message::EndPlay => {
                    let _ = output.send(Message::EndPlay).await;
                }
                _ => {}
            }
        }
    })
}

fn load_and_play_audio(
    playable: Arc<dyn AudioPlayable>,
    sink: &Sink,
) -> Result<u64, Box<dyn std::error::Error>> {
    let source = Decoder::new(playable.stream()?)?;

    let duration = source.total_duration().map(|d| d.as_secs()).unwrap_or(0);

    sink.append(source);
    sink.play();

    Ok(duration)
}

// Helper function to handle the timeout case (send progress updates)
fn handle_timeout(
    sink: &Sink,
    sender: &mut Sender<Message>,
    total_duration: &mut u64, // Pass mutably in case we need to reset it
) {
    if !sink.empty() {
        if sink.is_paused() {
            // Send Paused state periodically if needed by UI
            let _ = sender.try_send(Message::Paused);
        } else {
            // Send ProgressUpdate periodically
            let current_pos = sink.get_pos().as_secs();
            // Ensure current_pos doesn't exceed total_duration visually
            let display_pos = current_pos.min(*total_duration);
            let _ = sender.try_send(Message::ProgressUpdate(display_pos, *total_duration));
        }
    } else {
        // If sink is empty but we thought we had a duration, reset it
        if *total_duration != 0 {
            debug!("[AudioWorker] Sink empty, resetting duration.");
            *total_duration = 0;
            let _ = sender.try_send(Message::ProgressUpdate(0, 0));
        }
    }
}

// Main audio worker function, now using helper functions
fn audio_worker(
    mut sender: Sender<Message>, // No longer needs to be explicitly futures::channel::mpsc::Sender
    receiver: std::sync::mpsc::Receiver<Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;
    let mut total_duration: u64 = 0;
    let progress_update_interval = Duration::from_millis(500);
    let mut is_playing = false;
    info!("[AudioWorker] Started");

    loop {
        if is_playing && sink.empty() {
            is_playing = false;
            let _ = sender.try_send(Message::EndPlay);
        }
        match receiver.recv_timeout(progress_update_interval) {
            Ok(message) => match message {
                Message::Play(playable) => {
                    sink.stop();
                    sink.clear();

                    let duration = load_and_play_audio(playable.clone(), &sink)?;
                    total_duration = duration;
                    is_playing = true;
                    let _ = sender.try_send(Message::ProgressUpdate(0, total_duration));
                }
                Message::Paused => {
                    sink.pause();
                    is_playing = false;
                    let current_progress = sink.get_pos().as_secs();
                    let _ =
                        sender.try_send(Message::ProgressUpdate(current_progress, total_duration));
                }
                Message::Resume => {
                    sink.play();
                    is_playing = true;
                    let current_progress = sink.get_pos().as_secs();
                    let _ =
                        sender.try_send(Message::ProgressUpdate(current_progress, total_duration));
                }
                Message::Seek(pos) => {
                    if !sink.empty() {
                        let seek_duration = Duration::from_secs(pos);
                        sink.try_seek(seek_duration)?;
                    }
                }
                Message::VolumeChanged(vol) => {
                    sink.set_volume(vol);
                }
                _ => {}
            },
            Err(RecvTimeoutError::Timeout) => {
                handle_timeout(&sink, &mut sender, &mut total_duration);
            }
            Err(RecvTimeoutError::Disconnected) => {
                info!("[AudioWorker] Channel disconnected. Exiting.");
                break; // Exit the loop cleanly
            }
        }
    }

    info!("[AudioWorker] Exited loop.");
    Ok(())
}
