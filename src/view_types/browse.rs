use iced::{Element, Task, widget};
use log::debug;

use crate::discogs;

#[derive(Debug, Clone)]
pub enum Message {
    Discogs(discogs::ui::Message),
    GoogleCloud,
    Dropbox,
}

#[derive(Debug)]
pub enum BrowseView {
    Discogs(discogs::ui::DiscogsUI),
    GoogleCloud,
    Dropbox,
}

impl Default for BrowseView {
    fn default() -> Self {
        Self::Discogs(discogs::ui::DiscogsUI::default())
    }
}

impl BrowseView {
    pub fn view(&self) -> Element<Message> {
        debug!("Currently set to: {:?}", &self);
        match self {
            BrowseView::Discogs(ui) => ui.view().map(Message::Discogs),
            BrowseView::GoogleCloud => widget::text("Google Cloud").into(),
            BrowseView::Dropbox => widget::text("Dropbox").into(),
        }
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match self {
            BrowseView::Discogs(discogs_ui) => {
                if let Message::Discogs(msg) = message {
                    return discogs_ui.update(msg).map(Message::Discogs);
                }
            }
            BrowseView::GoogleCloud => todo!(),
            BrowseView::Dropbox => todo!(),
        };
        Task::none()
    }
}
