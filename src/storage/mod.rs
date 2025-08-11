pub mod local;

use serde::Deserialize;
use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFileKind {
    LocalFile = 0,
    GoogleDrive = 1,
    Dropbox = 2,
    Youtube = 3,
    Stream = 4,
}

impl TryFrom<i64> for AudioFileKind {
    type Error = StorageError;
    fn try_from(value: i64) -> Result<Self> {
        match value {
            0 => Ok(AudioFileKind::LocalFile),
            1 => Ok(AudioFileKind::GoogleDrive),
            2 => Ok(AudioFileKind::Dropbox),
            3 => Ok(AudioFileKind::Youtube),
            4 => Ok(AudioFileKind::Stream),
            _ => Err(StorageError::InvalidPlayableKind),
        }
    }
}

impl From<AudioFileKind> for i64 {
    fn from(kind: AudioFileKind) -> Self {
        kind as i64
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Playable {
    pub id: i64,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub genre_name: Option<String>,
    pub duration: i64,
    pub source_url: String,
    pub type_id: AudioFileKind,
    pub date_added: i64,
    pub artwork: Option<Vec<u8>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Artist {
    pub id: i64,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Album {
    pub id: i64,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Genre {
    pub id: i64,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Like {
    pub playable_id: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub enum PlaylistKind {
    #[default]
    Static,
    Dynamic,
    Folder,
}

impl fmt::Display for PlaylistKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaylistKind::Static => write!(f, "static"),
            PlaylistKind::Dynamic => write!(f, "dynamic"),
            PlaylistKind::Folder => write!(f, "folder"),
        }
    }
}

impl From<&str> for PlaylistKind {
    fn from(s: &str) -> Self {
        match s {
            "static" => PlaylistKind::Static,
            "dynamic" => PlaylistKind::Dynamic,
            "folder" => PlaylistKind::Folder,
            _ => PlaylistKind::Static, // default fallback
        }
    }
}

impl From<Option<String>> for PlaylistKind {
    fn from(s: Option<String>) -> Self {
        match s {
            Some(ref kind_str) => PlaylistKind::from(kind_str.as_str()),
            None => PlaylistKind::Static, // default
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Playlist {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub kind: PlaylistKind,
    pub position: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlaylistPlayable {
    pub playlist_id: i64,
    pub playable_id: i64,
    pub position: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlayableTag {
    pub tag_id: i64,
    pub playable_id: i64,
}

#[derive(Debug, Clone)]
pub struct AudioFileDescriptor {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: u16,
    pub genre: String,
    pub duration: u64,
    pub path: String,
    pub artwork: Option<Vec<u8>>,
    pub kind: AudioFileKind,
}

pub trait Storage {
    fn read_library(&self) -> Result<Vec<Playable>>;
    fn read_library_from_ids(&self, ids: &[i64]) -> Result<Vec<Playable>>;
    fn read_likes(&self) -> Result<Vec<Playable>>;
    fn read_playlist(&self, playlist_id: i64) -> Result<Vec<Playable>>;
    fn read_tag(&self, tag_id: i64) -> Result<Vec<Playable>>;

    fn create_playlist(
        &mut self,
        name: &str,
        kind: Option<PlaylistKind>,
        parent_id: Option<i64>,
    ) -> Result<i64>;
    fn delete_playlist(&mut self, playlist_id: i64) -> Result<()>;
    fn rename_playlist(&mut self, playlist_id: i64, name: &str) -> Result<()>;
    fn read_playlists(&self) -> Result<Vec<Playlist>>;

    fn create_tag(&mut self, name: &str) -> Result<i64>;
    fn delete_tag(&mut self, tag_id: i64) -> Result<()>;
    fn rename_tag(&mut self, tag_id: i64, name: &str) -> Result<()>;
    fn read_tags(&self) -> Result<Vec<Tag>>;

    fn clear_playlist(&mut self, id: i64) -> Result<()>;

    fn append_to_library(&mut self, arg: &AudioFileDescriptor) -> Result<i64>;
    fn append_to_playlist(&mut self, playlist_id: i64, playable_id: i64) -> Result<()>;
    fn append_to_tag(&mut self, tag_id: i64, playable_id: i64) -> Result<()>;
    fn append_like(&mut self, playable_id: i64) -> Result<()>;

    fn remove_from_library(&mut self, id: i64) -> Result<()>;
    fn remove_from_likes(&mut self, playable_id: i64) -> Result<()>;
    fn remove_from_playlist(&mut self, playlist_id: i64, playable_id: i64) -> Result<()>;
    fn remove_from_tag(&mut self, tag_id: i64, playable_id: i64) -> Result<()>;

    fn bulk_append_to_library(&mut self, playables: &[AudioFileDescriptor]) -> Result<Vec<i64>>;
    fn bulk_append_to_playlist(
        &mut self,
        playlist_id: i64,
        playables: &[AudioFileDescriptor],
    ) -> Result<()>;
    fn bulk_remove_from_library(&mut self, playable_ids: &[i64]) -> Result<()>;
    fn bulk_remove_from_playlist(&mut self, playlist_id: i64, indexes: &[i64]) -> Result<()>;

    // fn query_library(&self, )

    fn is_liked(&self, playable_id: i64) -> Result<bool>;
    fn filter_library_by_paths(&self, paths: &[String]) -> Result<Vec<Playable>>;
}

pub struct DummyStorage;

impl Storage for DummyStorage {
    fn read_library(&self) -> Result<Vec<Playable>> {
        Ok(vec![])
    }

    fn read_library_from_ids(&self, _ids: &[i64]) -> Result<Vec<Playable>> {
        Ok(vec![])
    }

    fn read_likes(&self) -> Result<Vec<Playable>> {
        Ok(vec![])
    }

    fn read_playlist(&self, _playlist_id: i64) -> Result<Vec<Playable>> {
        Ok(vec![])
    }

    fn read_tag(&self, _tag_id: i64) -> Result<Vec<Playable>> {
        Ok(vec![])
    }

    fn create_playlist(
        &mut self,
        _name: &str,
        _kind: Option<PlaylistKind>,
        _parent_id: Option<i64>,
    ) -> Result<i64> {
        Ok(0)
    }

    fn delete_playlist(&mut self, _playlist_id: i64) -> Result<()> {
        Ok(())
    }

    fn rename_playlist(&mut self, _playlist_id: i64, _name: &str) -> Result<()> {
        Ok(())
    }

    fn read_playlists(&self) -> Result<Vec<Playlist>> {
        Ok(vec![])
    }

    fn create_tag(&mut self, _name: &str) -> Result<i64> {
        Ok(0)
    }

    fn delete_tag(&mut self, _tag_id: i64) -> Result<()> {
        Ok(())
    }

    fn rename_tag(&mut self, _tag_id: i64, _name: &str) -> Result<()> {
        Ok(())
    }

    fn read_tags(&self) -> Result<Vec<Tag>> {
        Ok(vec![])
    }

    fn append_to_library(&mut self, _arg: &AudioFileDescriptor) -> Result<i64> {
        Ok(0)
    }

    fn append_to_playlist(&mut self, _playlist_id: i64, _playable_id: i64) -> Result<()> {
        Ok(())
    }

    fn append_to_tag(&mut self, _tag_id: i64, _playable_id: i64) -> Result<()> {
        Ok(())
    }

    fn append_like(&mut self, _playable_id: i64) -> Result<()> {
        Ok(())
    }

    fn remove_from_library(&mut self, _id: i64) -> Result<()> {
        Ok(())
    }

    fn remove_from_likes(&mut self, _playable_id: i64) -> Result<()> {
        Ok(())
    }

    fn remove_from_playlist(&mut self, _playlist_id: i64, _playable_id: i64) -> Result<()> {
        Ok(())
    }

    fn remove_from_tag(&mut self, _tag_id: i64, _playable_id: i64) -> Result<()> {
        Ok(())
    }

    fn bulk_append_to_library(&mut self, _playables: &[AudioFileDescriptor]) -> Result<Vec<i64>> {
        Ok(vec![])
    }

    fn bulk_remove_from_library(&mut self, _playable_ids: &[i64]) -> Result<()> {
        Ok(())
    }

    fn bulk_remove_from_playlist(&mut self, _playlist_id: i64, _indexes: &[i64]) -> Result<()> {
        Ok(())
    }

    fn is_liked(&self, _playable_id: i64) -> Result<bool> {
        Ok(false)
    }

    fn filter_library_by_paths(&self, _paths: &[String]) -> Result<Vec<Playable>> {
        Ok(vec![])
    }

    fn bulk_append_to_playlist(
        &mut self,
        _playlist_id: i64,
        _playables: &[AudioFileDescriptor],
    ) -> Result<()> {
        Ok(())
    }

    fn clear_playlist(&mut self, _id: i64) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    LibsqlError(#[from] rusqlite::Error),
    #[error("Invalid playable kind")]
    InvalidPlayableKind,
    #[error("Query Error")]
    QueryError,
    #[error("Playlist already exists")]
    PlaylistExists,
    #[error("Duplicate entry")]
    DuplicateEntry,
}
