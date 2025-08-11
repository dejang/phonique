use super::{
    AudioFileDescriptor, AudioFileKind, Playable, Playlist, Result, Storage, StorageError,
};
use log::trace;
use rusqlite::{Connection, OpenFlags, params};
use std::{collections::HashSet, path::Path};

const SCHEMA: &str = include_str!("schema.sql");

#[derive(Debug)]
pub struct LocalStorage {
    conn: Connection,
}

impl LocalStorage {
    pub(crate) fn maybe_insert_artist(&mut self, name: &str) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO Artist (name) VALUES (?1) ON CONFLICT(name) DO NOTHING RETURNING id",
        )?;
        let mut rows = stmt.query([name])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            let mut stmt = self.conn.prepare("SELECT id FROM Artist WHERE name = ?1")?;
            let mut rows = stmt.query([name])?;
            if let Some(row) = rows.next()? {
                Ok(row.get(0)?)
            } else {
                Err(StorageError::QueryError)
            }
        }
    }
    pub(crate) fn maybe_insert_genre(&mut self, name: &str) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO Genre (name) VALUES (?1) ON CONFLICT(name) DO NOTHING RETURNING id",
        )?;
        let mut rows = stmt.query([name])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            let mut stmt = self.conn.prepare("SELECT id FROM Genre WHERE name = ?1")?;
            let mut rows = stmt.query([name])?;
            if let Some(row) = rows.next()? {
                Ok(row.get(0)?)
            } else {
                Err(StorageError::QueryError)
            }
        }
    }
    pub(crate) fn maybe_insert_album(&mut self, name: &str, artist_id: i64) -> Result<i64> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM Album WHERE name = ?1 AND artist_id = ?2")?;
        let mut rows = stmt.query(params![name, artist_id])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            let mut stmt = self
                .conn
                .prepare("INSERT INTO Album (name, artist_id) VALUES (?1, ?2) RETURNING id")?;
            let mut rows = stmt.query(params![name, artist_id])?;
            if let Some(row) = rows.next()? {
                Ok(row.get(0)?)
            } else {
                Err(StorageError::QueryError)
            }
        }
    }
}

fn to_playable(row: &rusqlite::Row<'_>) -> std::result::Result<Playable, rusqlite::Error> {
    let id = row.get(0)?;
    let title = row.get(1)?;
    let artist_name = row.get(2)?;
    let album_name = row.get(3)?;
    let genre_name = row.get(4)?;
    let duration = row.get(5)?;
    let source_url = row.get(6)?;
    let type_id = AudioFileKind::try_from(row.get::<usize, i64>(7)?).unwrap();
    let date_added = row.get(8)?;
    let artwork = row.get(9)?;

    Ok(Playable {
        id,
        title,
        artist_name,
        album_name,
        genre_name,
        duration,
        source_url,
        type_id,
        date_added,
        artwork,
    })
}

impl Storage for LocalStorage {
    // Library
    fn read_library(&self) -> Result<Vec<Playable>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.title,
                    a.name  AS artist_name,
                    al.name AS album_name,
                    g.name  AS genre_name,
                    p.duration,
                    p.source_url,
                    p.type_id,
                    p.date_added,
                    p.artwork
             FROM Playable p
             LEFT JOIN Artist a  ON p.artist_id = a.id
             LEFT JOIN Album al  ON p.album_id   = al.id
             LEFT JOIN Genre g   ON p.genre_id   = g.id",
        )?;
        trace!("read_library: Query");
        let rows: Vec<Playable> = stmt
            .query_map([], to_playable)?
            .map(|result| result.map_err(StorageError::from))
            .collect::<Result<Vec<_>>>()?;

        trace!("read_library: Done {} entries", rows.len());
        Ok(rows)
    }

    fn read_library_from_ids(&self, ids: &[i64]) -> Result<Vec<Playable>> {
        let query = format!(
            "SELECT p.id, p.title,
                    a.name  AS artist_name,
                    al.name AS album_name,
                    g.name  AS genre_name,
                    p.duration,
                    p.source_url,
                    p.type_id,
                    p.date_added,
                    p.artwork
             FROM Playable p
             LEFT JOIN Artist a  ON p.artist_id = a.id
             LEFT JOIN Album al  ON p.album_id   = al.id
             LEFT JOIN Genre g   ON p.genre_id   = g.id
             WHERE p.id IN ({})",
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",")
        );

        let mut stmt = self.conn.prepare(&query)?;
        trace!("read_library_from_ids: Query");
        let out = stmt
            .query_map(params![], to_playable)?
            .map(|r| r.map_err(StorageError::from))
            .collect::<Result<Vec<_>>>()?;
        trace!("read_library_from_ids: Done {} entries", out.len());
        Ok(out)
    }
    fn append_to_library(&mut self, arg: &AudioFileDescriptor) -> Result<i64> {
        let existing = self.filter_library_by_paths(std::slice::from_ref(&arg.path))?;
        if !existing.is_empty() {
            return Err(StorageError::DuplicateEntry);
        }
        let artist_name = &arg.artist;
        let artist_id: Option<i64> = if !artist_name.is_empty() {
            Some(self.maybe_insert_artist(artist_name)?)
        } else {
            None
        };
        trace!("append_to_library: artist_id: {artist_id:?}");

        let genre = &arg.genre;
        let genre_id: Option<i64> = if !genre.is_empty() {
            Some(self.maybe_insert_genre(genre)?)
        } else {
            None
        };
        trace!("append_to_library: genre_id: {genre_id:?}");
        let album_name = &arg.album;
        let album_id: Option<i64> = if !album_name.is_empty() && artist_id.is_some() {
            Some(self.maybe_insert_album(album_name, artist_id.unwrap())?)
        } else {
            None
        };
        trace!("append_to_library: album_id: {album_id:?}");
        let title = &arg.title;
        let source_url = &arg.path;
        let duration = arg.duration;
        let kind = arg.kind as i64;

        let mut stmt = self.conn.prepare(
            "INSERT INTO Playable(title,artist_id,album_id,genre_id,duration,source_url,type_id,artwork) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)"
        )?;
        trace!("append_to_library: execute");
        let _ = stmt.execute(params![
            title,
            artist_id,
            album_id,
            genre_id,
            duration,
            source_url,
            kind,
            arg.artwork,
        ])?;
        trace!("append_to_library: done");
        Ok(self.conn.last_insert_rowid())
    }
    fn remove_from_library(&mut self, id: i64) -> Result<()> {
        trace!("remove_from_library: execute");
        self.conn
            .execute("DELETE FROM Playable WHERE id = ?", params![id])?;
        trace!("remove_from_library: removed {id}");
        Ok(())
    }

    // Likes
    fn read_likes(&self) -> Result<Vec<Playable>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.title,
                    a.name  AS artist_name,
                    al.name AS album_name,
                    g.name  AS genre_name,
                    p.duration,
                    p.source_url,
                    p.type_id,
                    p.date_added,
                    p.artwork
             FROM Playable p
             LEFT JOIN Artist a  ON p.artist_id = a.id
             LEFT JOIN Album al  ON p.album_id   = al.id
             LEFT JOIN Genre g   ON p.genre_id   = g.id
             WHERE p.id IN (SELECT playable_id FROM Like)",
        )?;
        trace!("read_likes: query");
        let out = stmt
            .query_map([], to_playable)?
            .map(|r| r.map_err(StorageError::from))
            .collect::<Result<Vec<Playable>>>()?;
        trace!("read_likes: done {} entries", out.len());
        Ok(out)
    }
    fn append_like(&mut self, playable_id: i64) -> Result<()> {
        trace!("append_like: execute");
        self.conn.execute(
            "INSERT OR IGNORE INTO Like(playable_id) VALUES (?)",
            params![playable_id],
        )?;
        trace!("append_like: added {playable_id}");
        Ok(())
    }
    fn remove_from_likes(&mut self, playable_id: i64) -> Result<()> {
        trace!("remove_from_likes: execute");
        self.conn.execute(
            "DELETE FROM Like WHERE playable_id = ?",
            params![playable_id],
        )?;
        trace!("remove_from_likes: removed {playable_id}");
        Ok(())
    }

    // Playlists
    fn create_playlist(
        &mut self,
        name: &str,
        kind: Option<super::PlaylistKind>,
        parent_id: Option<i64>,
    ) -> Result<i64> {
        let playlist_kind = kind.unwrap_or_default();
        let mut stmt = self.conn.prepare(
            "INSERT INTO Playlist (name, kind, parent_id) VALUES (?, ?, ?) ON CONFLICT(name) DO NOTHING RETURNING id",
        )?;
        trace!("create_playlist: query");
        let mut rows = stmt.query(params![name, playlist_kind.to_string(), parent_id])?;
        if let Some(row) = rows.next()? {
            trace!(
                "create_playlist: added playlist{} with kind {} with id {:?}",
                name,
                playlist_kind,
                row.get::<usize, i64>(0)?
            );
            Ok(row.get(0)?)
        } else {
            Err(StorageError::PlaylistExists)
        }
    }
    fn read_playlists(&self) -> Result<Vec<Playlist>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, parent_id, name, kind, position FROM Playlist")?;
        trace!("read_playlists: query");
        let mut rows = stmt.query(())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            trace!("read_playlists: adding row {row:?}");
            let kind_str: Option<String> = row.get(3)?;
            out.push(Playlist {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                kind: super::PlaylistKind::from(kind_str),
                position: row.get(4)?,
            });
        }
        trace!("read_playlists: done {} entries", out.len());
        Ok(out)
    }
    fn delete_playlist(&mut self, playlist_id: i64) -> Result<()> {
        trace!("delete_playlist: execute");
        self.conn
            .execute("DELETE FROM Playlist WHERE id = ?", params![playlist_id])?;
        trace!("delete_playlist: removed {playlist_id}");
        Ok(())
    }
    fn read_playlist(&self, playlist_id: i64) -> Result<Vec<Playable>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.title,
                    a.name  AS artist_name,
                    al.name AS album_name,
                    g.name  AS genre_name,
                    p.duration,
                    p.source_url,
                    p.type_id,
                    p.date_added,
                    p.artwork
             FROM Playable p
             LEFT JOIN Artist a  ON p.artist_id = a.id
             LEFT JOIN Album al  ON p.album_id   = al.id
             LEFT JOIN Genre g   ON p.genre_id   = g.id
             WHERE p.id IN (SELECT playable_id FROM PlaylistPlayable WHERE playlist_id = ?)",
        )?;
        trace!("read_playlist: query");
        let out = stmt
            .query_map(params![playlist_id], to_playable)?
            .map(|r| r.map_err(StorageError::from))
            .collect::<Result<Vec<_>>>()?;
        trace!("read_playlist: done {} entries", out.len());
        Ok(out)
    }

    // Does not allow duplicates in the playlist
    fn append_to_playlist(&mut self, playlist_id: i64, playable_id: i64) -> Result<()> {
        if self
            .read_playlist(playlist_id)?
            .iter()
            .any(|p| p.id == playable_id)
        {
            return Err(StorageError::DuplicateEntry);
        }

        // Get the next position by finding the max position + 1
        let next_position: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM PlaylistPlayable WHERE playlist_id = ?",
            params![playlist_id],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "INSERT OR IGNORE INTO PlaylistPlayable(playlist_id, playable_id, position) VALUES (?, ?, ?)",
            params![playlist_id, playable_id, next_position],
        )?;
        trace!(
            "append_to_playlist: added {playable_id} to {playlist_id} at position {next_position}"
        );
        Ok(())
    }
    fn remove_from_playlist(&mut self, playlist_id: i64, playable_id: i64) -> Result<()> {
        trace!("remove_from_playlist: execute");
        self.conn.execute(
            "DELETE FROM PlaylistPlayable WHERE playlist_id = ? AND playable_id = ?",
            params![playlist_id, playable_id],
        )?;
        trace!("remove_from_playlist: removed {playable_id} from {playlist_id}");
        Ok(())
    }

    fn clear_playlist(&mut self, id: i64) -> Result<()> {
        trace!("clear_playlist: execute");
        self.conn.execute(
            "DELETE FROM PlaylistPlayable WHERE playlist_id = ?",
            params![id],
        )?;
        trace!("clear_playlist: cleared playlist {id}");
        Ok(())
    }

    fn rename_playlist(&mut self, playlist_id: i64, name: &str) -> Result<()> {
        trace!("rename_playlist: execute");
        self.conn.execute(
            "UPDATE Playlist SET name = ? WHERE id = ?",
            params![name, playlist_id],
        )?;
        trace!("rename_playlist: renamed {playlist_id} to {name}");
        Ok(())
    }

    // Tags
    fn create_tag(&mut self, name: &str) -> Result<i64> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO Tag (name) VALUES (?) ON CONFLICT(name) DO NOTHING RETURNING id",
        )?;
        trace!("create_tag: query");
        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            trace!(
                "create_tag: added tag {} with id {:?}",
                name,
                row.get::<usize, i64>(0)?
            );
            Ok(row.get(0)?)
        } else {
            Err(StorageError::QueryError)
        }
    }
    fn append_to_tag(&mut self, tag_id: i64, playable_id: i64) -> Result<()> {
        trace!("append_to_tag: execute");
        self.conn.execute(
            "INSERT OR IGNORE INTO PlayableTag(tag_id, playable_id) VALUES (?, ?)",
            params![tag_id, playable_id],
        )?;
        trace!("append_to_tag: added playable_id {playable_id} to tag_id {tag_id}");
        Ok(())
    }
    fn read_tag(&self, tag_id: i64) -> Result<Vec<Playable>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.title,
                    a.name  AS artist_name,
                    al.name AS album_name,
                    g.name  AS genre_name,
                    p.duration,
                    p.source_url,
                    p.type_id,
                    p.date_added,
                    p.artwork
             FROM Playable p
             LEFT JOIN Artist a  ON p.artist_id = a.id
             LEFT JOIN Album al  ON p.album_id   = al.id
             LEFT JOIN Genre g   ON p.genre_id   = g.id
             WHERE p.id IN (SELECT playable_id FROM PlayableTag WHERE tag_id = ?)",
        )?;
        trace!("read_tag: query");
        let out = stmt
            .query_map(params![tag_id], to_playable)?
            .map(|r| r.map_err(StorageError::from))
            .collect::<Result<Vec<_>>>()?;
        trace!("read_tag: done {} entries", out.len());
        Ok(out)
    }
    fn remove_from_tag(&mut self, tag_id: i64, playable_id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM PlayableTag WHERE tag_id = ? AND playable_id = ?",
            params![tag_id, playable_id],
        )?;
        Ok(())
    }
    fn read_tags(&self) -> Result<Vec<super::Tag>> {
        let mut stmt = self.conn.prepare("SELECT id, name FROM Tag")?;
        trace!("read_tags: query");
        let mut rows = stmt.query(())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            trace!("read_tags: adding row {row:?}");
            out.push(super::Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            });
        }
        trace!("read_tags: done {} entries", out.len());
        Ok(out)
    }
    fn delete_tag(&mut self, tag_id: i64) -> Result<()> {
        trace!("delete_tag: execute");
        self.conn
            .execute("DELETE FROM Tag WHERE id = ?", params![tag_id])?;
        trace!("delete_tag: removed id {tag_id}");
        Ok(())
    }
    fn rename_tag(&mut self, tag_id: i64, name: &str) -> Result<()> {
        trace!("rename_tag: execute");
        self.conn.execute(
            "UPDATE Tag SET name = ? WHERE id = ?",
            params![name, tag_id],
        )?;
        trace!("rename_tag: renamed id {tag_id} to {name}");
        Ok(())
    }

    fn bulk_append_to_library(&mut self, playables: &[AudioFileDescriptor]) -> Result<Vec<i64>> {
        let mut row_ids = Vec::with_capacity(playables.len());
        self.conn.execute("BEGIN IMMEDIATE", ())?;
        trace!("bulk_append_to_library: execute");
        for playable in playables {
            let res = self.append_to_library(playable);
            if let Err(StorageError::DuplicateEntry) = res {
                continue;
            }
            let row_id = res?;
            row_ids.push(row_id);
        }
        // flush to disk once
        self.conn.execute("COMMIT", ())?;
        trace!("bulk_append_to_library: done");
        Ok(row_ids)
    }

    fn bulk_remove_from_library(&mut self, playable_ids: &[i64]) -> Result<()> {
        trace!("bulk_remove_from_library: execute");
        self.conn.execute("BEGIN IMMEDIATE", ())?;
        for id in playable_ids {
            self.remove_from_library(*id)?;
        }
        self.conn.execute("COMMIT", ())?;
        trace!("bulk_remove_from_library: done");
        Ok(())
    }

    fn bulk_remove_from_playlist(&mut self, playlist_id: i64, indexes: &[i64]) -> Result<()> {
        self.conn.execute("BEGIN IMMEDIATE", ())?;
        for index in indexes {
            self.remove_from_playlist(playlist_id, *index)?;
        }
        self.conn.execute("COMMIT", ())?;
        trace!("bulk_remove_from_playlist: done");
        Ok(())
    }

    fn is_liked(&self, playable_id: i64) -> Result<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM Like where playable_id = ?")?;
        let mut rows = stmt.query([playable_id])?;
        if (rows.next()?).is_some() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Bulk append a list of playable items to a playlist.
    /// Will check if items already exist in the library and append them if they don't.
    fn bulk_append_to_playlist(
        &mut self,
        playlist_id: i64,
        playables: &[AudioFileDescriptor],
    ) -> Result<()> {
        let paths = &playables
            .iter()
            .map(|p| p.path.clone())
            .collect::<Vec<String>>();
        let existing = self.filter_library_by_paths(paths)?;
        // none of the items already exist in the library
        // so we append them and return their ids
        let ids = if existing.is_empty() {
            self.bulk_append_to_library(playables)?
        } else {
            // some items already exist in the library
            // so we append only the new ones
            let existing_paths = existing
                .iter()
                .map(|p| p.source_url.clone())
                .collect::<HashSet<String>>();
            self.bulk_append_to_library(
                &playables
                    .iter()
                    .filter(|p| !existing_paths.contains(&p.path))
                    .cloned()
                    .collect::<Vec<AudioFileDescriptor>>(),
            )?;
            // now we are guaranteed that all items are in the library
            let existing = self.filter_library_by_paths(paths)?;
            existing.iter().map(|p| p.id).collect()
        };
        self.conn.execute("BEGIN IMMEDIATE", ())?;
        for id in ids {
            let res = self.append_to_playlist(playlist_id, id);
            if let Err(StorageError::DuplicateEntry) = res {
                continue;
            } else {
                res?;
            }
        }
        self.conn.execute("COMMIT", ())?;
        Ok(())
    }

    fn filter_library_by_paths(&self, paths: &[String]) -> Result<Vec<Playable>> {
        let paths: HashSet<String> = HashSet::from_iter(paths.iter().cloned());
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.title,
                        a.name  AS artist_name,
                        al.name AS album_name,
                        g.name  AS genre_name,
                        p.duration,
                        p.source_url,
                        p.type_id,
                        p.date_added,
                        p.artwork
                 FROM Playable p
                 LEFT JOIN Artist a  ON p.artist_id = a.id
                 LEFT JOIN Album al  ON p.album_id   = al.id
                 LEFT JOIN Genre g   ON p.genre_id   = g.id",
        )?;
        trace!("read_library_stream: Query");
        let rows = stmt
            .query_map([], to_playable)?
            .map(|result| result.map_err(StorageError::from))
            .filter(|r| paths.contains(&r.as_ref().unwrap().source_url))
            .collect::<Result<Vec<_>>>()?;

        Ok(rows)
    }
}

pub fn init_storage<T: AsRef<Path>>(path: T) -> Result<LocalStorage> {
    let conn = if path.as_ref().to_str().unwrap().eq(":memory:") {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn
    } else {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.execute_batch(SCHEMA)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "locking_mode", "EXCLUSIVE")?;
        // conn.pragma_update(None, "read_uncommitted", &1)?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "cache_size", -20000)?; // ~80 MiB
        conn.pragma_update(None, "mmap_size", 536870912)?; // 512 MiB
        conn.pragma_update(None, "automatic_index", 1)?;
        conn.pragma_update(None, "journal_size_limit", 10485760)?; // 10 MiB
        conn.pragma_update(None, "wal_autocheckpoint", 1000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        // bump statement cache & busy timeout
        conn.set_prepared_statement_cache_capacity(100);
        conn.busy_timeout(std::time::Duration::from_secs(1))?;
        conn
    };

    Ok(LocalStorage { conn })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::PlaylistKind;

    fn setup() -> LocalStorage {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(SCHEMA).unwrap();
        LocalStorage { conn: db }
    }

    fn local_file(title: &str) -> AudioFileDescriptor {
        AudioFileDescriptor {
            title: title.to_string(),
            artist: format!("artist_{title}"),
            album: format!("album_{title}"),
            year: 2021,
            genre: format!("genre_{title}"),
            duration: 100,
            artwork: None,
            path: format!("/tmp/test_{title}.mp3"),
            kind: AudioFileKind::LocalFile,
        }
    }

    #[test]
    fn test_init_storage() {
        let db_path = "/tmp/test-init.db";
        let storage = init_storage(db_path);
        assert!(storage.is_ok());
        std::fs::remove_file(db_path).unwrap();
        std::fs::remove_file(format!("{db_path}-shm")).unwrap();
        std::fs::remove_file(format!("{db_path}-wal")).unwrap();
    }

    #[test]
    fn test_maybe_insert_artist() {
        let mut storage = setup();
        let id = storage.maybe_insert_artist("test");
        if let Err(e) = &id {
            panic!("Error: {e}");
        }
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
        let id = storage.maybe_insert_artist("test");
        if let Err(e) = &id {
            panic!("Error: {e}");
        }
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
    }

    #[test]
    fn test_maybe_insert_genre() {
        let mut storage = setup();
        let id = storage.maybe_insert_genre("test");
        if let Err(e) = &id {
            panic!("Error: {e}");
        }
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
        let id = storage.maybe_insert_genre("test");
        if let Err(e) = &id {
            panic!("Error: {e}");
        }
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
    }

    #[test]
    fn test_maybe_insert_album() {
        let mut storage = setup();
        let artist_id = storage.maybe_insert_artist("test");
        if let Err(e) = &artist_id {
            panic!("Error: {e}");
        }
        let artist_id = artist_id.unwrap();
        let id = storage.maybe_insert_album("test", artist_id);
        if let Err(e) = &id {
            panic!("Error: {e}");
        }
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
        let id = storage.maybe_insert_album("test", artist_id);
        if let Err(e) = &id {
            panic!("Error: {e}");
        }
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
    }

    #[test]
    fn test_read_library() {
        let storage = setup();
        let library = storage.read_library();
        assert!(library.is_ok());
        assert!(library.unwrap().is_empty());
    }

    #[test]
    fn test_read_library_from_ids() {
        let mut storage = setup();
        let files = vec![
            local_file("test1"),
            local_file("test2"),
            local_file("test3"),
        ];

        let ids = storage.bulk_append_to_library(&files).unwrap();

        let library = storage.read_library_from_ids(&ids);
        assert!(library.is_ok());
        assert_eq!(library.as_ref().unwrap().len(), 3);
        assert_eq!(library.as_ref().unwrap()[0].id, 1);
        assert_eq!(library.as_ref().unwrap()[1].id, 2);
        assert_eq!(library.as_ref().unwrap()[2].id, 3);
    }

    #[test]
    fn test_append_to_library() {
        let mut storage = setup();
        let localfile = local_file("test");
        let id = storage.append_to_library(&localfile);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
        let library = storage.read_library();
        assert!(library.is_ok());
        assert_eq!(library.as_ref().unwrap().len(), 1);
        assert_eq!(library.as_ref().unwrap()[0].id, 1);
        assert_eq!(library.as_ref().unwrap()[0].title, "test");
        assert_eq!(
            library.as_ref().unwrap()[0].artist_name,
            Some("artist_test".to_string())
        );
        assert_eq!(
            library.as_ref().unwrap()[0].album_name,
            Some("album_test".to_string())
        );
        assert_eq!(
            library.as_ref().unwrap()[0].genre_name,
            Some("genre_test".to_string())
        );
        assert_eq!(library.as_ref().unwrap()[0].duration, 100);
        assert_eq!(
            library.as_ref().unwrap()[0].source_url,
            "/tmp/test_test.mp3"
        );
        assert_eq!(
            library.as_ref().unwrap()[0].type_id,
            AudioFileKind::LocalFile
        );

        let res = storage.append_to_library(&localfile);
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), StorageError::DuplicateEntry);
    }

    #[test]
    fn test_remove_from_library() {
        let mut storage = setup();
        let localfile = local_file("test");
        let id = storage.append_to_library(&localfile);
        assert!(id.is_ok());
        let library = storage.read_library();
        assert!(library.is_ok());
        assert_eq!(library.as_ref().unwrap().len(), 1);
        assert_eq!(library.as_ref().unwrap()[0].id, 1);
        let res = storage.remove_from_library(1);
        assert!(res.is_ok());
        let library = storage.read_library();
        assert!(library.is_ok());
        assert_eq!(library.as_ref().unwrap().len(), 0);
    }
    #[test]
    fn test_read_likes() {
        let storage = setup();
        let likes = storage.read_likes();
        assert!(likes.is_ok());
        assert!(likes.unwrap().is_empty());
    }

    #[test]
    fn test_append_like() {
        let mut storage = setup();
        let localfile = local_file("Test");
        let id = storage.append_to_library(&localfile);
        assert!(id.is_ok());
        let res = storage.append_like(1);
        assert!(res.is_ok());
        let likes = storage.read_likes();
        assert!(likes.is_ok());
        assert_eq!(likes.as_ref().unwrap().len(), 1);
        assert_eq!(likes.as_ref().unwrap()[0].title, localfile.title);
    }

    #[test]
    fn test_remove_from_likes() {
        let mut storage = setup();
        let localfile = local_file("Test");
        let id = storage.append_to_library(&localfile);
        assert!(id.is_ok());
        let res = storage.append_like(1);
        assert!(res.is_ok());
        let likes = storage.read_likes();
        assert!(likes.is_ok());
        assert_eq!(likes.as_ref().unwrap().len(), 1);
        assert_eq!(likes.as_ref().unwrap()[0].id, 1);
        let res = storage.remove_from_likes(1);
        assert!(res.is_ok());
        let likes = storage.read_likes();
        assert!(likes.is_ok());
        assert!(likes.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_create_playlist() {
        let mut storage = setup();
        let id = storage.create_playlist("test", None, None);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
        let id = storage.create_playlist("test", None, None);
        assert!(id.is_err());
        assert_eq!(id.err().unwrap().to_string(), "Playlist already exists");

        let id = storage.create_playlist("test2", None, None);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 2);
    }

    #[test]
    fn test_read_all_playlists() {
        let mut storage = setup();
        let playlists = storage.read_playlists();
        assert!(playlists.is_ok());
        assert!(playlists.as_ref().unwrap().is_empty());

        let id = storage.create_playlist("test", None, None);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
        let id = storage.create_playlist("test2", None, None);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 2);

        let playlists = storage.read_playlists();
        assert!(playlists.is_ok());
        assert_eq!(playlists.as_ref().unwrap().len(), 2);
        assert_eq!(playlists.as_ref().unwrap()[0].id, 1);
        assert_eq!(playlists.as_ref().unwrap()[0].name, "test");
        assert_eq!(playlists.as_ref().unwrap()[1].id, 2);
        assert_eq!(playlists.as_ref().unwrap()[1].name, "test2");
    }

    #[test]
    fn test_delete_playlist() {
        let mut storage = setup();
        let id = storage.create_playlist("test", None, None);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
        let playlists = storage.read_playlists();
        assert!(playlists.is_ok());
        assert_eq!(playlists.as_ref().unwrap().len(), 1);

        let res = storage.delete_playlist(1);
        assert!(res.is_ok());
        let playlists = storage.read_playlists();
        assert!(playlists.is_ok());
        assert_eq!(playlists.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn test_append_to_playlist() {
        let mut storage = setup();
        let id = storage.create_playlist("test", None, None);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let localfile = local_file("test");
        let id = storage.append_to_library(&localfile);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let res = storage.append_to_playlist(1, 1);
        assert!(res.is_ok());

        let playlist = storage.read_playlist(1);
        assert!(playlist.is_ok());
        assert_eq!(playlist.as_ref().unwrap().len(), 1);
        assert_eq!(playlist.as_ref().unwrap()[0].id, 1);

        let res = storage.append_to_playlist(1, 1);
        assert_eq!(res.err().unwrap(), StorageError::DuplicateEntry);
    }

    #[test]
    fn test_remove_from_playlist() {
        let mut storage = setup();
        let id = storage.create_playlist("test", None, None);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let localfile = local_file("test");
        let id = storage.append_to_library(&localfile);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let res = storage.append_to_playlist(1, 1);
        assert!(res.is_ok());

        let playlist = storage.read_playlist(1);
        assert!(playlist.is_ok());
        assert_eq!(playlist.as_ref().unwrap().len(), 1);
        assert_eq!(playlist.as_ref().unwrap()[0].id, 1);

        let res = storage.remove_from_playlist(1, 1);
        assert!(res.is_ok());

        let playlist = storage.read_playlist(1);
        assert!(playlist.is_ok());
        assert!(playlist.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_rename_playlist() {
        let mut storage = setup();
        let id = storage.create_playlist("test", None, None);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let playlists = storage.read_playlists();
        assert!(playlists.is_ok());
        assert_eq!(playlists.as_ref().unwrap().len(), 1);
        assert_eq!(playlists.as_ref().unwrap()[0].name, "test");

        let res = storage.rename_playlist(1, "test2");
        assert!(res.is_ok());
        let playlists = storage.read_playlists();
        assert!(playlists.is_ok());
        assert_eq!(playlists.as_ref().unwrap().len(), 1);
        assert_eq!(playlists.as_ref().unwrap()[0].name, "test2");
    }

    #[test]
    fn test_create_tag() {
        let mut storage = setup();
        let id = storage.create_tag("test");
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
    }

    #[test]
    fn test_read_tag() {
        let mut storage = setup();
        let id = storage.create_tag("test");
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let tag = storage.read_tag(1);
        assert!(tag.is_ok());
        assert_eq!(tag.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn test_append_to_tag() {
        let mut storage = setup();
        let id = storage.create_tag("test");
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let localfile = local_file("test");
        let id = storage.append_to_library(&localfile);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let res = storage.append_to_tag(1, 1);
        assert!(res.is_ok());

        let tag = storage.read_tag(1);
        assert!(tag.is_ok());
        assert_eq!(tag.as_ref().unwrap().len(), 1);
        assert_eq!(tag.as_ref().unwrap()[0].id, 1);
    }

    #[test]
    fn test_remove_from_tag() {
        let mut storage = setup();
        let id = storage.create_tag("test");
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let localfile = local_file("test");
        let id = storage.append_to_library(&localfile);
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let res = storage.append_to_tag(1, 1);
        assert!(res.is_ok());

        let tag = storage.read_tag(1);
        assert!(tag.is_ok());
        assert_eq!(tag.as_ref().unwrap().len(), 1);
        assert_eq!(tag.as_ref().unwrap()[0].id, 1);

        let res = storage.remove_from_tag(1, 1);
        assert!(res.is_ok());

        let tag = storage.read_tag(1);
        assert!(tag.is_ok());
        assert_eq!(tag.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn test_read_tags() {
        let mut storage = setup();
        let id = storage.create_tag("test");
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let id = storage.create_tag("test2");
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 2);

        let tags = storage.read_tags();
        assert!(tags.is_ok());
        assert_eq!(tags.as_ref().unwrap().len(), 2);
        assert_eq!(tags.as_ref().unwrap()[0].id, 1);
        assert_eq!(tags.as_ref().unwrap()[0].name, "test");
        assert_eq!(tags.as_ref().unwrap()[1].id, 2);
        assert_eq!(tags.as_ref().unwrap()[1].name, "test2");
    }

    #[test]
    fn test_delete_tag() {
        let mut storage = setup();
        let id = storage.create_tag("test");
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);
        let tags = storage.read_tags();
        assert!(tags.is_ok());
        assert_eq!(tags.as_ref().unwrap().len(), 1);
        assert_eq!(tags.as_ref().unwrap()[0].id, 1);
        assert_eq!(tags.as_ref().unwrap()[0].name, "test");

        let res = storage.delete_tag(1);
        assert!(res.is_ok());

        let tags = storage.read_tags();
        assert!(tags.is_ok());
        assert_eq!(tags.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn test_rename_tag() {
        let mut storage = setup();
        let id = storage.create_tag("test");
        assert!(id.is_ok());
        assert_eq!(id.unwrap(), 1);

        let tags = storage.read_tags();
        assert!(tags.is_ok());
        assert_eq!(tags.as_ref().unwrap().len(), 1);
        assert_eq!(tags.as_ref().unwrap()[0].id, 1);
        assert_eq!(tags.as_ref().unwrap()[0].name, "test");

        let res = storage.rename_tag(1, "test2");
        assert!(res.is_ok());

        let tags = storage.read_tags();
        assert!(tags.is_ok());
        assert_eq!(tags.as_ref().unwrap().len(), 1);
        assert_eq!(tags.as_ref().unwrap()[0].id, 1);
        assert_eq!(tags.as_ref().unwrap()[0].name, "test2");
    }

    #[test]
    fn test_bulk_append_to_library() {
        let mut storage = setup();
        let localfiles: Vec<AudioFileDescriptor> = vec![
            local_file("test1"),
            local_file("test2"),
            local_file("test3"),
            local_file("test1"),
        ];
        let ids = storage.bulk_append_to_library(&localfiles);
        assert!(ids.is_ok());
        assert_eq!(ids.as_ref().unwrap().len(), 3);
        assert_eq!(ids.as_ref().unwrap()[0], 1);
        assert_eq!(ids.as_ref().unwrap()[1], 2);
        assert_eq!(ids.as_ref().unwrap()[2], 3);
    }

    #[test]
    fn test_bulk_remove_from_library() {
        let mut storage = setup();
        let localfiles: Vec<AudioFileDescriptor> = vec![
            local_file("test1"),
            local_file("test2"),
            local_file("test3"),
        ];
        let ids = storage.bulk_append_to_library(&localfiles);
        assert!(ids.is_ok());
        assert_eq!(ids.as_ref().unwrap().len(), 3);
        assert_eq!(ids.as_ref().unwrap()[0], 1);
        assert_eq!(ids.as_ref().unwrap()[1], 2);
        assert_eq!(ids.as_ref().unwrap()[2], 3);
        assert!(storage.read_library().unwrap().len() == 3);

        let playlist_id = storage.create_playlist("test", None, None).unwrap();
        let id = *ids.as_ref().unwrap().first().unwrap();
        let res = storage.append_to_playlist(playlist_id, id);
        assert!(res.is_ok());

        let playlist = storage.read_playlist(playlist_id);
        assert!(playlist.is_ok());
        assert_eq!(playlist.as_ref().unwrap().len(), 1);
        assert_eq!(playlist.as_ref().unwrap()[0].id, id);

        let _ = storage.bulk_remove_from_library(ids.as_ref().unwrap());
        let library = storage.read_library();
        assert!(library.is_ok());
        assert_eq!(library.as_ref().unwrap().len(), 0);

        let playlist = storage.read_playlist(playlist_id);
        assert!(playlist.is_ok());
        assert_eq!(playlist.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn test_is_liked() {
        let mut storage = setup();
        let localfiles: Vec<AudioFileDescriptor> = vec![
            local_file("test1"),
            local_file("test2"),
            local_file("test3"),
        ];
        let result = storage.bulk_append_to_library(&localfiles);
        assert!(result.is_ok());

        let result = storage.append_like(1);
        assert!(result.is_ok());

        let likes = storage.read_likes().unwrap();
        assert_eq!(likes.len(), 1);
        assert_eq!(likes[0].id, 1);

        assert!(storage.is_liked(1).unwrap());
        assert!(!storage.is_liked(2).unwrap());
        assert!(!storage.is_liked(3).unwrap());
    }

    #[test]
    fn test_playlist_kind_enum() {
        let mut storage = setup();

        // Test creating playlists with different kinds
        let static_id = storage
            .create_playlist("Static Playlist", Some(PlaylistKind::Static), None)
            .unwrap();
        let dynamic_id = storage
            .create_playlist("Dynamic Playlist", Some(PlaylistKind::Dynamic), None)
            .unwrap();
        let folder_id = storage
            .create_playlist("Folder", Some(PlaylistKind::Folder), None)
            .unwrap();

        // Read all playlists and verify their kinds
        let playlists = storage.read_playlists().unwrap();
        assert_eq!(playlists.len(), 3);

        let static_playlist = playlists.iter().find(|p| p.id == static_id).unwrap();
        let dynamic_playlist = playlists.iter().find(|p| p.id == dynamic_id).unwrap();
        let folder_playlist = playlists.iter().find(|p| p.id == folder_id).unwrap();

        assert_eq!(static_playlist.kind, PlaylistKind::Static);
        assert_eq!(dynamic_playlist.kind, PlaylistKind::Dynamic);
        assert_eq!(folder_playlist.kind, PlaylistKind::Folder);

        // Test Display trait implementation
        assert_eq!(static_playlist.kind.to_string(), "static");
        assert_eq!(dynamic_playlist.kind.to_string(), "dynamic");
        assert_eq!(folder_playlist.kind.to_string(), "folder");

        // Test From trait implementation
        assert_eq!(PlaylistKind::from("static"), PlaylistKind::Static);
        assert_eq!(PlaylistKind::from("dynamic"), PlaylistKind::Dynamic);
        assert_eq!(PlaylistKind::from("folder"), PlaylistKind::Folder);
        assert_eq!(PlaylistKind::from("unknown"), PlaylistKind::Static); // fallback

        // Test default behavior (None should create Static playlist)
        let default_id = storage
            .create_playlist("Default Playlist", None, None)
            .unwrap();
        let playlists = storage.read_playlists().unwrap();
        let default_playlist = playlists.iter().find(|p| p.id == default_id).unwrap();
        assert_eq!(default_playlist.kind, PlaylistKind::Static);

        // Test Default trait
        assert_eq!(PlaylistKind::default(), PlaylistKind::Static);
    }

    #[test]
    fn test_bulk_append_to_playlist() {
        let mut storage = setup();
        let playlist_id = storage
            .create_playlist("Bulk Append Playlist", None, None)
            .unwrap();
        let song1 = local_file("test1");
        let song2 = local_file("test2");

        let mut songs = vec![song1, song2];
        storage
            .bulk_append_to_playlist(playlist_id, &songs)
            .unwrap();
        let playlist = storage.read_playlist(playlist_id).unwrap();
        assert_eq!(playlist.len(), 2);
        let library = storage.read_library().unwrap();
        assert_eq!(library.len(), 2);

        let song3 = local_file("test3");
        songs.push(song3);
        storage
            .bulk_append_to_playlist(playlist_id, &songs)
            .unwrap();
        let playlist = storage.read_playlist(playlist_id).unwrap();
        assert_eq!(playlist.len(), 3);
        let library = storage.read_library().unwrap();
        assert_eq!(library.len(), 3);
    }

    #[test]
    fn test_clear_playlist() {
        let mut storage = setup();
        let playlist_id = storage
            .create_playlist("Clear Playlist", None, None)
            .unwrap();
        let song1 = local_file("test1");
        let song2 = local_file("test2");

        let songs = vec![song1, song2];
        storage
            .bulk_append_to_playlist(playlist_id, &songs)
            .unwrap();
        let playlist = storage.read_playlist(playlist_id).unwrap();
        assert_eq!(playlist.len(), 2);
        let library = storage.read_library().unwrap();
        assert_eq!(library.len(), 2);

        storage.clear_playlist(playlist_id).unwrap();
        let playlist = storage.read_playlist(playlist_id).unwrap();
        assert_eq!(playlist.len(), 0);
        let library = storage.read_library().unwrap();
        assert_eq!(library.len(), 2);
    }
}
