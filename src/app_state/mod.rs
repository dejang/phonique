use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
};

use crate::storage;

pub mod state_impl;

pub type PlayableId = i64;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Section {
    ListenNow,
    Browse,
    Library,
    Favorites,
    RecentlyPlayed,
    Playlist(i64),
    Tag(i64),
}

impl Default for Section {
    fn default() -> Self {
        Self::Library
    }
}

impl std::fmt::Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Section::ListenNow => f.write_str("Listen Now"),
            Section::Browse => f.write_str("Browse"),
            Section::Library => f.write_str("Library"),
            Section::Favorites => f.write_str("Favorites"),
            Section::RecentlyPlayed => f.write_str("Recently Played"),
            Section::Playlist(id) => f.write_fmt(format_args!("Playlist {id}")),
            Section::Tag(id) => f.write_fmt(format_args!("Tag {id}")),
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum PlayableKind {
    LocalFile,
    GoogleDrive,
    Dropbox,
    Youtube,
    Stream,
}

impl From<storage::AudioFileKind> for PlayableKind {
    fn from(kind: storage::AudioFileKind) -> Self {
        match kind {
            storage::AudioFileKind::LocalFile => PlayableKind::LocalFile,
            storage::AudioFileKind::GoogleDrive => PlayableKind::GoogleDrive,
            storage::AudioFileKind::Dropbox => PlayableKind::Dropbox,
            storage::AudioFileKind::Youtube => PlayableKind::Youtube,
            storage::AudioFileKind::Stream => PlayableKind::Stream,
        }
    }
}

#[allow(dead_code)]
pub trait AudioPlayable: std::fmt::Debug + Send + Sync {
    // Set to -1 to mark that the playable is not in the library yet
    fn get_id(&self) -> PlayableId;
    fn get_title(&self) -> &str;
    fn get_album(&self) -> &str;
    fn get_artist(&self) -> &str;
    fn get_date_added(&self) -> &i64;
    fn get_genre(&self) -> &str;
    fn get_duration(&self) -> u64;
    fn get_path(&self) -> &str;
    fn get_album_art(&self) -> &Option<Vec<u8>>;
    fn get_kind(&self) -> PlayableKind;
    fn stream(&self) -> Result<Cursor<Vec<u8>>, std::io::Error>;
}

impl AudioPlayable for storage::Playable {
    fn get_id(&self) -> PlayableId {
        self.id
    }
    fn get_title(&self) -> &str {
        self.title.as_str()
    }

    fn get_album(&self) -> &str {
        if let Some(name) = &self.album_name {
            name.as_str()
        } else {
            ""
        }
    }

    fn get_artist(&self) -> &str {
        if let Some(name) = &self.artist_name {
            name.as_str()
        } else {
            ""
        }
    }

    fn stream(&self) -> Result<Cursor<Vec<u8>>, std::io::Error> {
        let file = File::open(self.get_path())?;
        let mut reader = BufReader::new(file);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;

        Ok(Cursor::new(buffer))
    }

    fn get_date_added(&self) -> &i64 {
        &self.date_added
    }

    fn get_genre(&self) -> &str {
        if let Some(name) = &self.genre_name {
            name.as_str()
        } else {
            ""
        }
    }

    fn get_path(&self) -> &str {
        &self.source_url
    }

    fn get_duration(&self) -> u64 {
        let v: u64 = self.duration.try_into().unwrap();
        v
    }

    fn get_album_art(&self) -> &Option<Vec<u8>> {
        &self.artwork
    }

    fn get_kind(&self) -> PlayableKind {
        match self.type_id {
            storage::AudioFileKind::LocalFile => PlayableKind::LocalFile,
            storage::AudioFileKind::GoogleDrive => PlayableKind::GoogleDrive,
            storage::AudioFileKind::Dropbox => PlayableKind::Dropbox,
            storage::AudioFileKind::Youtube => PlayableKind::Youtube,
            storage::AudioFileKind::Stream => PlayableKind::Stream,
        }
    }
}
