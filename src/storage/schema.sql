-- 1) WAL + cache tuning
PRAGMA journal_mode = WAL;

PRAGMA synchronous = NORMAL;

PRAGMA temp_store = MEMORY;

PRAGMA cache_size = -2000;

-- ~4 MB page cache
PRAGMA mmap_size = 268435456;

-- allow up to 256 MB mmap
PRAGMA auto_vacuum = INCREMENTAL;

PRAGMA foreign_keys = ON;

-- 2) Normalized base tables
CREATE TABLE IF NOT EXISTS Artist (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE INDEX IF NOT EXISTS idx_artist_name ON Artist (name);

CREATE TABLE IF NOT EXISTS Album (
    id INTEGER PRIMARY KEY,
    artist_id INTEGER REFERENCES Artist (id),
    name TEXT NOT NULL COLLATE NOCASE
);

CREATE INDEX IF NOT EXISTS idx_album_name ON Album (name);

CREATE TABLE IF NOT EXISTS Genre (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE INDEX IF NOT EXISTS idx_genre_name ON Genre (name);

CREATE TABLE IF NOT EXISTS Playable (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    artist_id INTEGER REFERENCES Artist (id),
    album_id INTEGER REFERENCES Album (id),
    genre_id INTEGER REFERENCES Genre (id),
    duration INTEGER,
    source_url TEXT,
    type_id INTEGER NOT NULL,
    date_added INTEGER NOT NULL DEFAULT (strftime ('%s', 'now')),
    artwork BLOB
);

-- composite indexes for JOIN+ORDER
CREATE INDEX IF NOT EXISTS idx_playable_artist_title ON Playable (artist_id, title);

CREATE INDEX IF NOT EXISTS idx_playable_album_title ON Playable (album_id, title);

CREATE INDEX IF NOT EXISTS idx_playable_genre_title ON Playable (genre_id, title);

-- single-column indexes for ORDER BY duration/date_added
CREATE INDEX IF NOT EXISTS idx_playable_duration ON Playable (duration);

CREATE INDEX IF NOT EXISTS idx_playable_date_added ON Playable (date_added);

-- 3) Likes table (one LIKE per playable)
CREATE TABLE IF NOT EXISTS Like (
    playable_id INTEGER PRIMARY KEY REFERENCES Playable (id) ON DELETE CASCADE
);

-- 4) Playlists + junction
CREATE TABLE IF NOT EXISTS Playlist (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    kind TEXT CHECK (kind IN ('static', 'dynamic', 'folder')),
    position INTEGER
);

CREATE INDEX IF NOT EXISTS idx_playlist_name ON Playlist (name);

CREATE TABLE IF NOT EXISTS SmartPlaylistTags (
    playlist_id INTEGER NOT NULL REFERENCES Playlist (id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES Tag (id) ON DELETE CASCADE,
    PRIMARY KEY (playlist_id, tag_id)
);

CREATE TABLE IF NOT EXISTS PlaylistPlayable (
    playlist_id INTEGER NOT NULL REFERENCES Playlist (id) ON DELETE CASCADE,
    playable_id INTEGER NOT NULL REFERENCES Playable (id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, playable_id)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_pp_by_playable ON PlaylistPlayable (playable_id);

-- 5) Tags + junction
CREATE TABLE IF NOT EXISTS Tag (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE INDEX IF NOT EXISTS idx_tag_name ON Tag (name);

CREATE TABLE IF NOT EXISTS PlayableTag (
    tag_id INTEGER NOT NULL REFERENCES Tag (id) ON DELETE CASCADE,
    playable_id INTEGER NOT NULL REFERENCES Playable (id) ON DELETE CASCADE,
    PRIMARY KEY (tag_id, playable_id)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS idx_pt_by_playable ON PlayableTag (playable_id);

-- 6) FTS5 table for text-search on title/artist/album
CREATE VIRTUAL TABLE IF NOT EXISTS PlayableFTS USING fts5 (
    title,
    artist_name,
    album_name,
    content = 'Playable', -- external content mode
    content_rowid = 'id'
);

-- 7) Triggers to keep the FTS index in sync
CREATE TRIGGER IF NOT EXISTS trg_fts_insert AFTER INSERT ON Playable BEGIN
INSERT INTO
    PlayableFTS (rowid, title, artist_name, album_name)
VALUES
    (
        NEW.id,
        NEW.title,
        (
            SELECT
                name
            FROM
                Artist
            WHERE
                id = NEW.artist_id
        ),
        (
            SELECT
                name
            FROM
                Album
            WHERE
                id = NEW.album_id
        )
    );

END;

CREATE TRIGGER IF NOT EXISTS trg_fts_update AFTER
UPDATE OF title,
artist_id,
album_id ON Playable BEGIN
-- delete the old entry
INSERT INTO
    PlayableFTS (PlayableFTS, rowid)
VALUES
    ('delete', OLD.id);

-- insert the new
INSERT INTO
    PlayableFTS (rowid, title, artist_name, album_name)
VALUES
    (
        NEW.id,
        NEW.title,
        (
            SELECT
                name
            FROM
                Artist
            WHERE
                id = NEW.artist_id
        ),
        (
            SELECT
                name
            FROM
                Album
            WHERE
                id = NEW.album_id
        )
    );

END;

CREATE TRIGGER IF NOT EXISTS trg_fts_delete AFTER DELETE ON Playable BEGIN
INSERT INTO
    PlayableFTS (PlayableFTS, rowid)
VALUES
    ('delete', OLD.id);

END;
