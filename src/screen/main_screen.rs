use crate::{
    app_state::{Section, state_impl::State as AppState},
    audio_scanner::{ScannedFile, scan_file, scan_folder},
    menu_bar::{self, MenuBar},
    player::{self, Player},
    sidebar::{self, Sidebar, playlists::MenuOptions},
    view_types::{
        browse::{self, BrowseView},
        compact_view::{self, CompactView},
    },
};
use iced::{
    Element, Length, Subscription, Task,
    advanced::Overlay,
    event,
    futures::{SinkExt, Stream, channel::mpsc::Sender},
    widget::{Column, Container, PaneGrid, container, pane_grid, text, vertical_rule},
    window::Event as WindowEvent,
};
use log::error;
use std::{path::PathBuf, sync::Arc};

use std::pin::Pin;

const MIN_SIDEBAR_WIDTH: f32 = 200.0;
const MAX_SIDEBAR_WIDTH: f32 = 275.0;

#[derive(Clone, Debug)]
enum Panes {
    Sidebar,
    Central,
}

#[derive(Debug, Clone)]
pub enum Message {
    PaneResize(pane_grid::ResizeEvent),
    // only interested in the width for now
    WindowResize(f32),
    Sidebar(sidebar::Message),
    Player(player::Message),
    CompactView(compact_view::Message),
    MetadataScanResult(ScannedFile),
    MetadataScanningStarted(Option<PathBuf>),
    MetadataScanningEnded,
    MenuBar(menu_bar::Message),
    Browse(browse::Message),
    Error(String),
}

pub struct MainScreen {
    pane_state: pane_grid::State<Panes>,
    pane_ratio: f32,
    compact_view: CompactView,
    browse_view: BrowseView,
    player: Player,
    state: AppState,
    // we use this both as a flag and something to hold the value in when the files are dropped on the main window
    // when no scanning is in progress, it should be set to None
    scanning_files: Option<PathBuf>,
    // cleared after each scan
    scannned_files: Vec<ScannedFile>,
    menubar: MenuBar,
    sidebar: Sidebar,
}

impl Default for MainScreen {
    fn default() -> Self {
        let ratio = 0.25;
        let pane_state = pane_grid::State::with_configuration(pane_grid::Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio,
            a: Box::new(pane_grid::Configuration::Pane(Panes::Sidebar)),
            b: Box::new(pane_grid::Configuration::Pane(Panes::Central)),
        });

        let state = AppState::default();

        Self {
            pane_state,
            pane_ratio: ratio,
            player: Player::default(),
            state,
            scanning_files: None,
            scannned_files: Vec::new(),
            compact_view: CompactView::default(),
            browse_view: BrowseView::default(),
            menubar: MenuBar::default(),
            sidebar: Sidebar::default(),
        }
    }
}

impl MainScreen {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowResize(width) => {
                let sidebar_width = self.pane_ratio * width;
                let sidebar_width = sidebar_width.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
                let new_ratio = sidebar_width / width;
                self.pane_ratio = new_ratio;
                self.pane_state =
                    pane_grid::State::with_configuration(pane_grid::Configuration::Split {
                        axis: pane_grid::Axis::Vertical,
                        ratio: self.pane_ratio,
                        a: Box::new(pane_grid::Configuration::Pane(Panes::Sidebar)),
                        b: Box::new(pane_grid::Configuration::Pane(Panes::Central)),
                    });
            }
            Message::MenuBar(msg) => {
                match msg.clone() {
                    menu_bar::Message::Search(val) => {
                        self.state.search(val);
                    }
                    menu_bar::Message::MetadataScanningStarted(path_buf) => {
                        self.scanning_files = path_buf.clone();
                    }
                    _ => {}
                };
                return self.menubar.update(msg).map(Message::MenuBar);
            }
            Message::PaneResize(event) => {
                self.pane_state.resize(event.split, event.ratio);
            }
            Message::Sidebar(msg) => {
                match &msg {
                    sidebar::Message::Selected(section) => {
                        if let Err(error) = self.state.set_section(section.to_owned()) {
                            return Task::done(Message::Error(error.to_string()));
                        }
                    }
                    sidebar::Message::Playlists(msg) => match msg {
                        sidebar::playlists::Message::CreatedPlaylist(maybe_id, value, kind) => {
                            if let Some(id) = maybe_id {
                                if let Err(error) = self.state.rename_playlist(*id, &value) {
                                    return Task::done(Message::Error(error.to_string()));
                                }
                            } else {
                                let result = self.state.create_playlist(value, kind.clone());
                                if let Err(error) = result {
                                    return Task::done(Message::Error(error.to_string()));
                                }
                                let id = result.unwrap();
                                if let Err(error) = self.state.set_section(Section::Playlist(id)) {
                                    return Task::done(Message::Error(error.to_string()));
                                }
                            }
                        }
                        sidebar::playlists::Message::ContextAction(option, id, _) => {
                            match option {
                                MenuOptions::Rename => {
                                    // self.state.set_section(Section::RenamePlaylist);
                                }
                                MenuOptions::Delete => {
                                    if let Err(error) = self.state.delete_playlist(*id) {
                                        return Task::done(Message::Error(error.to_string()));
                                    }
                                }
                                MenuOptions::Clear => {
                                    if let Err(error) = self.state.clear_playlist(*id) {
                                        return Task::done(Message::Error(error.to_string()));
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    sidebar::Message::Tags(msg) => match msg {
                        sidebar::tags::Message::Created(_maybe_id, name) => {
                            if let Err(error) = self.state.create_tag(name) {
                                return Task::done(Message::Error(error.to_string()));
                            }
                        }
                        sidebar::tags::Message::ContextAction(menu_option, id, _) => {
                            match menu_option {
                                sidebar::tags::MenuOptions::Rename => todo!(),
                                sidebar::tags::MenuOptions::Delete => {
                                    if let Err(error) = self.state.delete_tag(*id) {
                                        return Task::done(Message::Error(error.to_string()));
                                    }
                                }
                                sidebar::tags::MenuOptions::Clear => todo!(),
                            }
                        }
                        _ => {}
                    },
                };
                return self.sidebar.update(msg).map(Message::Sidebar);
            }
            Message::CompactView(compact_view_msg) => {
                let task = self.compact_view.update(compact_view_msg.clone());
                let main_task = match compact_view_msg {
                    compact_view::Message::RemovePlayables(indexes, to_trash) => {
                        self.state.bulk_remove(&indexes, to_trash);
                        Task::none()
                    }
                    compact_view::Message::DblClick(index, id) => {
                        self.state.player.current_index = Some(index);
                        self.state.player.current_playable = Some(id);
                        let playable = self.state.playables().nth(index).unwrap();
                        Task::done(player::Message::Play(Arc::new(playable.clone())))
                            .map(Message::Player)
                    }
                    _ => Task::none(),
                };
                return Task::batch([task.map(Message::CompactView), main_task]);
            }
            Message::Browse(msg) => {
                if let browse::Message::Discogs(crate::discogs::ui::Message::Play(playable)) = msg {
                    return self
                        .player
                        .update(player::Message::Play(playable))
                        .map(Message::Player);
                }
                return self.browse_view.update(msg).map(Message::Browse);
            }
            Message::Player(msg) => match msg {
                player::Message::Next => {
                    self.state.next_playable();
                    if let Some(next_index) = self.state.player.current_index
                        && let Some(next_id) = self.state.player.current_playable
                    {
                        return Task::batch([
                            Task::done(Message::CompactView(compact_view::Message::ScrollTo(
                                next_index,
                            ))),
                            Task::done(Message::CompactView(compact_view::Message::DblClick(
                                next_index, next_id,
                            ))),
                        ]);
                    }
                }
                player::Message::Prev => {
                    self.state.previous_playable();
                    if let Some(prev_index) = self.state.player.current_index
                        && let Some(prev_id) = self.state.player.current_playable
                    {
                        return Task::batch([
                            Task::done(Message::CompactView(compact_view::Message::ScrollTo(
                                prev_index,
                            ))),
                            Task::done(Message::CompactView(compact_view::Message::DblClick(
                                prev_index, prev_id,
                            ))),
                        ]);
                    }
                }
                player::Message::Like(id) => {
                    if self.state.is_liked(&id) {
                        self.state.remove_from_likes(&id);
                    } else {
                        self.state.add_to_likes(&id);
                    }
                }
                _ => return self.player.update(msg).map(Message::Player),
            },
            Message::MetadataScanningStarted(path) => {
                self.scanning_files = path;
            }
            Message::MetadataScanningEnded => {
                self.scanning_files = None;
                let files = self.scannned_files.clone();
                self.scannned_files.clear();
                if let Err(error) = self.state.append_bulk(files) {
                    return Task::done(Message::Error(error.to_string()));
                }
            }
            Message::MetadataScanResult(metadata) => {
                self.scannned_files.push(metadata);
            }
            Message::Error(message) => {
                log::error!("{message}");
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let pane_grid = PaneGrid::new(
            &self.pane_state,
            |_pane, state, _is_maximized| match state {
                Panes::Sidebar => pane_grid::Content::new(container(
                    self.sidebar
                        .view(self.state.section(), &self.state)
                        .map(Message::Sidebar),
                )),
                Panes::Central => {
                    let central_element = match self.state.section() {
                        Section::Library
                        | Section::Favorites
                        | Section::RecentlyPlayed
                        | Section::Playlist(_)
                        | Section::Tag(_) => self
                            .compact_view
                            .view(&self.state)
                            .map(Message::CompactView),
                        Section::Browse => self.browse_view.view().map(Message::Browse),
                        _ => text("Empty").into(),
                    };

                    let content = container(central_element).width(Length::Fill).padding(16);
                    pane_grid::Content::new(
                        iced::widget::row![vertical_rule(1), content]
                            .width(Length::Fill)
                            .height(Length::Fill),
                    )
                }
            },
        )
        .on_resize(10, Message::PaneResize);

        let main_layout = Column::new()
            .push(self.menubar.view().map(Message::MenuBar))
            .push(pane_grid.height(Length::Fill))
            .push(self.player.view(&self.state).map(Message::Player));

        Container::new(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let scanning_subscription = if let Some(path) = self.scanning_files.clone() {
            Subscription::run_with(path, scan_files)
        } else {
            Subscription::none()
        };

        let file_drop_subscription = event::listen_with(|ev, _, _| match ev {
            event::Event::Window(WindowEvent::FileDropped(path_buf)) => {
                Some(Message::MetadataScanningStarted(Some(path_buf)))
            }
            event::Event::Window(WindowEvent::Resized(size)) => {
                Some(Message::WindowResize(size.width))
            }
            _ => None,
        });

        Subscription::batch([
            self.compact_view.subscription().map(Message::CompactView),
            file_drop_subscription,
            scanning_subscription,
            self.player.subscription().map(Message::Player),
        ])
    }
}

fn scan_files(path: &PathBuf) -> Pin<Box<dyn Stream<Item = Message> + Send>> {
    let path = path.clone();
    Box::pin(iced::stream::channel(
        100,
        |mut output: Sender<Message>| async move {
            if path.is_file() {
                match scan_file(&path).map_err(|e| format!("{e}")) {
                    Ok(metadata) => {
                        let _ = output.send(Message::MetadataScanResult(metadata)).await;
                        let _ = output.send(Message::MetadataScanningEnded).await;
                    }
                    Err(e) => {
                        error!("scan_files: failed to scan file {path:?}\n{e:?}");
                    }
                }
            } else if path.is_dir() {
                let files = scan_folder(&path);
                for file in files {
                    let _ = output.send(Message::MetadataScanResult(file)).await;
                }
                let _ = output.send(Message::MetadataScanningEnded).await;
            }
        },
    ))
}
