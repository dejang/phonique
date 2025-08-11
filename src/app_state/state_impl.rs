use directories::UserDirs;
use log::{error, info};
use rand::Rng;
use thiserror::Error;

use crate::{
    app_state::{AudioPlayable, PlayableKind, Section},
    audio_scanner::{ScannedFile, ScannedKind},
    storage::{
        self, AudioFileDescriptor, AudioFileKind, DummyStorage, Playable, Playlist, Storage, Tag,
        local::init_storage,
    },
};

use super::PlayableId;

#[derive(Debug, Error, PartialEq)]
pub enum StateError {
    #[error("StorageError: {0}")]
    StorageError(#[from] storage::StorageError),
}

pub type Result<T> = std::result::Result<T, StateError>;

pub struct PlaylistNode {
    pub value: Playlist,
    pub children: Vec<PlaylistNode>,
}

#[derive(Default)]
pub struct PlayerState {
    pub current_playable: Option<PlayableId>,
    pub current_index: Option<usize>,
    pub shuffle: bool,
    pub is_playing: bool,
}

pub struct State {
    playlist_names: Vec<PlaylistNode>,
    tag_names: Vec<Tag>,
    search_string: String,
    storage: Box<dyn Storage>,
    section: Section,
    playables: Vec<Playable>,
    recently_played: Vec<Playable>,
    pub player: PlayerState,
    random_generator: rand::rngs::ThreadRng,
}

impl Default for State {
    fn default() -> Self {
        let user_dirs = UserDirs::new().unwrap();
        let music_dir = user_dirs.audio_dir().unwrap();
        let res = init_storage(music_dir.join("music.db"));
        let storage: Box<dyn Storage> = if let Ok(storage) = res {
            info!("DB Storage initialization success");
            Box::new(storage)
        } else if let Ok(storage) = init_storage(":memory:") {
            error!("Persistent Storage initialization failed with error {res:?}");
            info!("Using Memory Storage");
            Box::new(storage)
        } else {
            error!("In memory storage failed");
            info!("Using Dummy Storage");
            Box::new(DummyStorage)
        };
        State::new(storage)
    }
}

impl State {
    pub fn new(storage: Box<dyn Storage>) -> Self {
        let section = Section::default();
        let playlist_names = Self::to_playlist_tree(storage.read_playlists().unwrap_or_default());
        let tag_names = storage.read_tags().unwrap_or_default();

        let mut instance = Self {
            search_string: String::new(),
            playlist_names,
            tag_names,
            section,
            playables: Vec::new(),
            storage,
            player: PlayerState::default(),
            random_generator: rand::rng(),
            recently_played: Vec::new(),
        };
        if let Err(err) = instance.load_playables() {
            error!("Error loading library: {err:?}");
        }
        instance
    }

    fn to_playlist_tree(playlists: Vec<Playlist>) -> Vec<PlaylistNode> {
        let mut tree = Vec::new();
        let mut children = Vec::new();
        playlists.into_iter().for_each(|p| {
            if p.parent_id.is_none() {
                tree.push(PlaylistNode {
                    value: p,
                    children: Vec::new(),
                });
            } else {
                children.push(p);
            }
        });

        children.into_iter().for_each(|c| {
            for p in &mut tree {
                if let Some(parent) = Self::find_parent(p, c.parent_id.unwrap()) {
                    parent.children.push(PlaylistNode {
                        value: c,
                        children: Vec::new(),
                    });
                    break;
                }
            }
        });

        tree
    }

    fn find_parent(tree: &mut PlaylistNode, node_id: i64) -> Option<&mut PlaylistNode> {
        if tree.value.id == node_id {
            Some(tree)
        } else {
            tree.children
                .iter_mut()
                .find_map(|child| Self::find_parent(child, node_id))
        }
    }

    pub fn load_playables(&mut self) -> Result<()> {
        match &self.section {
            Section::Library => self.playables = self.storage.read_library()?,
            Section::Favorites => self.playables = self.storage.read_likes()?,
            Section::Playlist(id) => self.playables = self.storage.read_playlist(*id)?,
            Section::Tag(id) => self.playables = self.storage.read_tag(*id)?,
            Section::RecentlyPlayed => self.playables = self.recently_played.clone(),
            _ => {}
        };
        Ok(())
    }

    pub fn playlists(&self) -> &[PlaylistNode] {
        &self.playlist_names
    }

    pub fn tags(&self) -> &[Tag] {
        &self.tag_names
    }

    pub fn set_section(&mut self, section: Section) -> Result<()> {
        self.section = section;
        self.search_string = String::new();
        self.load_playables()
    }

    pub fn section(&self) -> &Section {
        &self.section
    }

    fn apply_search_filter(&self, v: &Playable, search: &str) -> bool {
        let in_title = (*v).get_title().to_lowercase().contains(search);
        let in_artist = (*v).get_artist().to_lowercase().contains(search);
        let in_album = (*v).get_album().to_lowercase().contains(search);
        in_title || in_artist || in_album
    }

    pub fn playables(&self) -> impl Iterator<Item = &Playable> {
        self.playables
            .iter()
            .filter(|v| self.apply_search_filter(v, &self.search_string))
    }

    pub fn add_to_likes(&mut self, playable_id: &PlayableId) {
        if let Err(err) = self.storage.append_like(*playable_id) {
            error!("Error adding playable to likes: {err:?}");
        }
    }

    pub fn is_liked(&self, playable_id: &PlayableId) -> bool {
        let result = self.storage.is_liked(*playable_id);
        if let Err(err) = result {
            error!("Error checking if playable is liked: {err:?}");
            false
        } else {
            result.unwrap()
        }
    }

    pub fn remove_from_likes(&mut self, playable_id: &PlayableId) {
        if let Err(err) = self.storage.remove_from_likes(*playable_id) {
            error!("Error removing playable from likes: {err:?}");
        }
    }

    pub fn search(&mut self, val: String) {
        self.search_string = val;
    }

    pub fn append_bulk(&mut self, items: Vec<ScannedFile>) -> Result<()> {
        log::info!("Appending {} items to {}", items.len(), &self.section);
        let items: Vec<AudioFileDescriptor> =
            items.into_iter().map(AudioFileDescriptor::from).collect();

        if let Section::Playlist(id) = self.section {
            self.storage.bulk_append_to_playlist(id, &items)?;
            self.load_playables()?;
        } else {
            self.storage.bulk_append_to_library(&items)?;

            if let Section::Library = self.section {
                self.load_playables()?;
            }
        }
        Ok(())
    }

    pub fn bulk_remove(&mut self, indexes: &[usize], to_trash: bool) {
        let playables: Vec<&Playable> = indexes
            .iter()
            .map(|i| self.playables.get(*i).unwrap())
            .collect();
        let ids: Vec<PlayableId> = playables.iter().map(|p| p.get_id()).collect();

        if to_trash || self.section.eq(&Section::Library) {
            if let Err(err) = self.storage.bulk_remove_from_library(&ids) {
                error!("Error removing items from library\n{err:?}");
            }
            if to_trash {
                for playable in playables {
                    if playable.get_kind().eq(&PlayableKind::LocalFile)
                        && let Err(err) = trash::delete(playable.get_path())
                    {
                        error!("Error deleting {}.\n{err}", playable.get_path());
                    }
                }
            }
        } else {
            match &self.section {
                Section::Playlist(id) => {
                    if let Err(err) = self.storage.bulk_remove_from_playlist(*id, &ids) {
                        error!("Error removing items from playlist {id}\n{err:?}");
                    }
                }
                Section::Favorites => {
                    // if let Err(err) = self.storage.bulk_remove_from_favorites(&ids) {
                    //     error!("Error removing items from favorites\n{err:?}");
                    // }
                }
                Section::Tag(_id) => {
                    // if let Err(err) = self.storage.bulk_remove_from_tag(*id, &ids) {
                    //     error!("Error removing items from tag {id}\n{err:?}");
                    // }
                }
                _ => {}
            };
        }
    }

    pub fn create_playlist(
        &mut self,
        name: &str,
        kind: Option<crate::storage::PlaylistKind>,
    ) -> Result<i64> {
        let id = self.storage.create_playlist(name, kind, None)?;
        self.playlist_names = Self::to_playlist_tree(self.storage.read_playlists()?);
        Ok(id)
    }

    pub fn delete_playlist(&mut self, id: i64) -> Result<()> {
        let is_selected = self.section.eq(&Section::Playlist(id));
        self.storage.delete_playlist(id)?;
        self.playlist_names = Self::to_playlist_tree(self.storage.read_playlists()?);
        if is_selected {
            if self.playlist_names.is_empty() {
                self.section = Section::Library;
            } else {
                let id = self.playlist_names[0].value.id;
                self.section = Section::Playlist(id);
            }
            self.load_playables()?;
        }
        Ok(())
    }

    pub fn clear_playlist(&mut self, id: i64) -> Result<()> {
        self.storage.clear_playlist(id)?;
        if self.section.eq(&Section::Playlist(id)) {
            self.load_playables()?;
        }
        Ok(())
    }

    pub fn rename_playlist(&mut self, id: i64, name: &str) -> Result<()> {
        self.storage.rename_playlist(id, name)?;
        self.playlist_names = Self::to_playlist_tree(self.storage.read_playlists()?);
        Ok(())
    }

    pub fn append_to_tag(&mut self, tag_id: i64, playable_id: i64) -> Result<()> {
        self.storage.append_to_tag(tag_id, playable_id)?;
        if self.section.eq(&Section::Tag(tag_id)) {
            self.load_playables()?;
        }

        Ok(())
    }

    pub fn create_tag(&mut self, name: &str) -> Result<()> {
        self.storage.create_tag(name)?;
        self.tag_names = self.storage.read_tags()?;
        Ok(())
    }

    pub fn delete_tag(&mut self, id: i64) -> Result<()> {
        let is_selected = self.section.eq(&Section::Tag(id));
        self.storage.delete_tag(id)?;
        self.tag_names = self.storage.read_tags()?;
        if is_selected {
            if self.tag_names.is_empty() {
                self.section = Section::Library;
            } else {
                let id = self.tag_names[0].id;
                self.section = Section::Tag(id);
            }
            self.load_playables()?;
        }
        Ok(())
    }

    // pub fn add_to_recent_playables(&mut self, id: &PlayableId) {
    //     trace!("add_to_recent_playables: adding {id:?} to recent playables");
    //     if !self.recent_playables.contains(id) {
    //         self.recent_playables.push(*id);
    //     }
    // }
    pub fn next_playable(&mut self) {
        if self.player.current_index.is_none() {
            if !self.playables.is_empty() {
                self.player.current_index = Some(0);
                self.player.current_playable = Some(self.playables[0].id);
            }
            return;
        }

        let index = self.player.current_index.unwrap();
        let next_index = if self.player.shuffle {
            self.random_generator.random_range(0..self.playables.len())
        } else if index == self.playables.len() - 1 {
            0
        } else {
            index + 1
        };

        let next_id = self.playables[next_index].id;
        self.player.current_index = Some(next_index);
        self.player.current_playable = Some(next_id);
    }

    pub fn previous_playable(&mut self) {
        if self.player.current_index.is_none() {
            if !self.playables.is_empty() {
                self.player.current_index = Some(0);
                self.player.current_playable = Some(self.playables[0].id);
            }
            return;
        }

        let index = self.player.current_index.unwrap();
        let prev_index = if self.player.shuffle {
            self.random_generator.random_range(0..self.playables.len())
        } else if index == 0 {
            self.playables.len() - 1
        } else {
            index - 1
        };

        let prev_id = self.playables[prev_index].id;
        self.player.current_index = Some(prev_index);
        self.player.current_playable = Some(prev_id);
    }
}

impl From<ScannedFile> for AudioFileDescriptor {
    fn from(val: ScannedFile) -> Self {
        AudioFileDescriptor {
            title: val.title,
            artist: val.artist,
            album: val.album,
            year: val.year,
            genre: val.genre,
            duration: val.duration,
            path: val.path,
            artwork: val.artwork,
            kind: match val.kind {
                ScannedKind::LocalFile => AudioFileKind::LocalFile,
                ScannedKind::GoogleDrive => AudioFileKind::GoogleDrive,
                ScannedKind::Dropbox => AudioFileKind::Dropbox,
                ScannedKind::Youtube => AudioFileKind::Youtube,
                ScannedKind::Stream => AudioFileKind::Stream,
            },
        }
    }
}

mod tests {
    use super::*;

    #[allow(dead_code)]
    fn scanned_file(title: &str) -> ScannedFile {
        ScannedFile {
            title: title.to_string(),
            artist: format!("artist_{title}"),
            album: format!("album_{title}"),
            year: 2001,
            genre: format!("genre_{title}"),
            duration: 100,
            path: format!("path_{title}"),
            artwork: None,
            kind: ScannedKind::LocalFile,
        }
    }

    #[test]
    fn test_initialized_correctly() {
        let mut storage = Box::new(init_storage(":memory:").unwrap());
        let _ = storage.append_to_library(&AudioFileDescriptor::from(scanned_file("Test1")));
        let _ = storage.append_to_library(&AudioFileDescriptor::from(scanned_file("Test2")));
        let _ = storage.create_tag("tag1");
        let _ = storage.create_playlist("playlist1", None, None);

        let state = State::new(storage);
        assert_eq!(state.playables.len(), 2);
        assert_eq!(state.playlist_names.len(), 1);
        assert_eq!(state.tag_names.len(), 1);
    }

    #[test]
    fn test_append_bulk() {
        let mut storage = Box::new(init_storage(":memory:").unwrap());
        let _ = storage.create_playlist("test_playlist", None, None);
        let mut state = State::new(storage);
        let files = vec![scanned_file("Test1"), scanned_file("Test2")];
        assert!(state.append_bulk(files).is_ok());
        assert_eq!(state.playables.len(), 2);

        assert!(state.set_section(Section::Favorites).is_ok());
        let files = vec![scanned_file("Test3"), scanned_file("Test4")];
        assert!(state.append_bulk(files).is_ok());
        assert_eq!(state.playables.len(), 0);

        assert!(state.set_section(Section::Playlist(1)).is_ok());
        let files = vec![scanned_file("Test5"), scanned_file("Test6")];
        assert!(state.append_bulk(files).is_ok());
        assert_eq!(state.playables.len(), 2);
        assert_eq!(state.playables[0].get_title(), "Test5");
        assert_eq!(state.playables[1].get_title(), "Test6");

        assert!(state.set_section(Section::Library).is_ok());
        assert_eq!(state.playables.len(), 6);
    }

    #[test]
    fn test_delete_playlist() {
        let mut storage = Box::new(init_storage(":memory:").unwrap());
        let _ = storage.create_playlist("test_playlist", None, None);
        let _ = storage.create_playlist("test_playlist2", None, None);

        let mut state = State::new(storage);
        assert!(state.set_section(Section::Playlist(2)).is_ok()); //test_playlist2
        let files = vec![
            scanned_file("Test1"),
            scanned_file("Test2"),
            scanned_file("Test3"),
        ];
        assert!(state.append_bulk(files).is_ok());
        assert_eq!(state.playables.len(), 3);

        assert!(state.set_section(Section::Playlist(1)).is_ok()); //test_playlist1
        let files = vec![
            scanned_file("Test4"),
            scanned_file("Test5"),
            scanned_file("Test6"),
        ];
        assert!(state.append_bulk(files).is_ok());
        assert_eq!(state.playables.len(), 3);

        assert!(state.set_section(Section::Library).is_ok());
        assert_eq!(state.playables.len(), 6);

        assert!(state.set_section(Section::Playlist(1)).is_ok());
        assert!(state.delete_playlist(1).is_ok());

        assert_eq!(state.section(), &Section::Playlist(2));
        assert_eq!(state.playables.len(), 3);

        let _ = state.delete_playlist(2);
        assert_eq!(state.section(), &Section::Library);
        assert_eq!(state.playables().count(), 6);
    }

    #[test]
    fn test_clear_playlist() {
        let mut storage = Box::new(init_storage(":memory:").unwrap());
        let _ = storage.create_playlist("test_playlist", None, None);
        let _ = storage.create_playlist("test_playlist2", None, None);

        let mut state = State::new(storage);
        assert!(state.set_section(Section::Playlist(1)).is_ok()); //test_playlist1
        let files = vec![
            scanned_file("Test4"),
            scanned_file("Test5"),
            scanned_file("Test6"),
        ];
        assert!(state.append_bulk(files).is_ok());
        assert_eq!(state.playables.len(), 3);

        assert!(state.set_section(Section::Playlist(2)).is_ok()); //test_playlist2
        let files = vec![
            scanned_file("Test1"),
            scanned_file("Test2"),
            scanned_file("Test3"),
        ];
        assert!(state.append_bulk(files).is_ok());
        assert_eq!(state.playables.len(), 3);

        assert!(state.set_section(Section::Library).is_ok());
        assert_eq!(state.playables.len(), 6);

        assert!(state.set_section(Section::Playlist(2)).is_ok());
        assert_eq!(state.playables.len(), 3);
        assert!(state.clear_playlist(1).is_ok());
        assert_eq!(state.playables.len(), 3);
        assert!(state.set_section(Section::Playlist(1)).is_ok());
        assert_eq!(state.playables().count(), 0);

        assert!(state.set_section(Section::Playlist(2)).is_ok());
        assert_eq!(state.playables.len(), 3);
        assert!(state.clear_playlist(2).is_ok());
        assert_eq!(state.playables.len(), 0);
    }

    #[test]
    fn test_nested_playlists() {
        let mut storage = init_storage(":memory:").unwrap();
        let _ = storage.create_playlist("Folder", Some(crate::storage::PlaylistKind::Folder), None);
        let _ = storage.create_playlist("playlist", None, Some(1));
        let state = State::new(Box::new(storage));
        let playlists = state.playlists();
        assert_eq!(playlists.len(), 1);
        assert_eq!(playlists[0].children.len(), 1);
        assert_eq!(playlists[0].value.id, 1);
        assert_eq!(playlists[0].children[0].value.id, 2);
    }

    #[test]
    fn test_delete_tag() {
        let mut storage = Box::new(init_storage(":memory:").unwrap());
        let _ = storage.create_tag("tag1");
        let _ = storage.create_tag("tag2");

        let mut state = State::new(storage);
        let files = vec![
            scanned_file("Test1"),
            scanned_file("Test2"),
            scanned_file("Test3"),
            scanned_file("Test4"),
            scanned_file("Test5"),
            scanned_file("Test6"),
        ];
        let _ = state.append_bulk(files);
        let _ = state.append_to_tag(1, 1);
        let _ = state.append_to_tag(1, 2);
        let _ = state.append_to_tag(1, 3);
        let _ = state.append_to_tag(2, 4);
        let _ = state.append_to_tag(2, 5);
        let _ = state.append_to_tag(2, 6);

        assert!(state.set_section(Section::Tag(1)).is_ok());
        assert_eq!(state.playables().count(), 3);

        assert!(state.delete_tag(1).is_ok());
        assert_eq!(state.section(), &Section::Tag(2));
        assert_eq!(state.playables().count(), 3);

        assert!(state.delete_tag(2).is_ok());
        assert_eq!(state.section(), &Section::Library);
        assert_eq!(state.playables().count(), 6);
    }
}
