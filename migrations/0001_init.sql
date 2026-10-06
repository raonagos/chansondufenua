-- 0001_init.sql — Chanson du fenua v4 (embedded SQLite)
--
-- Transcribed from the v3 SurrealDB schema, which is the source of truth for
-- parity. Four conventions:
--
--   * `id` keeps the *original* v3 record key (20 chars of [0-9a-z]). Every song
--     lives at `/himene/{id}`, so preserving the key preserves every inbound
--     link, the sitemap history and whatever search engines have already
--     indexed. Re-minting ids would be a silent SEO regression.
--   * Timestamps are RFC 3339 UTC, stored as TEXT. That is exactly what the v3
--     dump contained, it round-trips through `chrono` without loss, and it sorts
--     lexicographically in the same order it sorts chronologically.
--   * `CHECK` constraints mirror v3's `ASSERT` clauses, except where review
--     deliberately moved a bound on 2026-10-04 (`artist.fullname` 4..=50 ->
--     1..=255, `song.title` 4..=100 -> 4..=255). `src/domain` enforces the same
--     bounds *before* SQL is reached, so a CHECK failure means a bug in the
--     layer above — not bad user input.
--
--     This file is frozen once v4 is deployed. Until then it is edited in place,
--     which invalidates the checksum sqlx records in `_sqlx_migrations` for any
--     database already created — delete and recreate the local file when that
--     happens.
--   * `length()` counts characters (not bytes) for TEXT in SQLite, which matches
--     SurrealDB's `string::len()` and Rust's `chars().count()`. Bytes would break
--     every Tahitian title containing ā, ', ē or ō.

-- ---------------------------------------------------------------------------
-- artist
-- ---------------------------------------------------------------------------

CREATE TABLE artist (
    id         TEXT PRIMARY KEY NOT NULL,
    fullname   TEXT NOT NULL CHECK (length(fullname) BETWEEN 1 AND 255),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX artist_fullname_idx ON artist (fullname);

-- v3 indexed `artist.fullname` with a `punct_lower_ascii` SEARCH ANALYZER
-- (PUNCT tokenizer; LOWERCASE + ASCII filters) and BM25 ranking, so that typing
-- "mahoi" finds "Mā'ohi". FTS5 with `remove_diacritics 2` is the SQLite
-- spelling of that, and `bm25()` gives the same ranking function. It is an
-- *external content* table: the text lives in `artist`, the index lives here,
-- and the three triggers below keep them in step.
CREATE VIRTUAL TABLE artist_fts USING fts5 (
    fullname,
    content = 'artist',
    content_rowid = 'rowid',
    tokenize = "unicode61 remove_diacritics 2"
);

CREATE TRIGGER artist_fts_after_insert AFTER INSERT ON artist BEGIN
    INSERT INTO artist_fts (rowid, fullname) VALUES (new.rowid, new.fullname);
END;

CREATE TRIGGER artist_fts_after_delete AFTER DELETE ON artist BEGIN
    INSERT INTO artist_fts (artist_fts, rowid, fullname)
    VALUES ('delete', old.rowid, old.fullname);
END;

CREATE TRIGGER artist_fts_after_update AFTER UPDATE ON artist BEGIN
    INSERT INTO artist_fts (artist_fts, rowid, fullname)
    VALUES ('delete', old.rowid, old.fullname);
    INSERT INTO artist_fts (rowid, fullname) VALUES (new.rowid, new.fullname);
END;

-- ---------------------------------------------------------------------------
-- song
-- ---------------------------------------------------------------------------

CREATE TABLE song (
    id         TEXT PRIMARY KEY NOT NULL,
    title      TEXT NOT NULL CHECK (length(title) BETWEEN 4 AND 255),
    -- Stored raw: this column holds the chord markup (`<sup data-nosnippet>`)
    -- exactly as the author typed it. Never normalise it on the way in, or the
    -- rendered song stops matching what the author wrote.
    lyrics     TEXT NOT NULL CHECK (length(lyrics) BETWEEN 100 AND 6000),
    published  INTEGER NOT NULL DEFAULT 1 CHECK (published IN (0, 1)),
    view_count INTEGER NOT NULL DEFAULT 1 CHECK (view_count > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- `/himene` lists every published song newest-first (verified against the live
-- site: the order is exactly `created_at DESC`), and the home page shows a
-- "most viewed" table. Both are staged from these two indexes.
CREATE INDEX song_created_at_idx ON song (published, created_at DESC);
CREATE INDEX song_view_count_idx ON song (published, view_count DESC);

-- ---------------------------------------------------------------------------
-- song_artist — the ordered credit list
-- ---------------------------------------------------------------------------

-- v3 stored `song.artists` as an ordered `array<record<artist>>`, so credit
-- order is visible on the page and must survive the migration. A join table
-- with an explicit `position` is the relational spelling of that array.
CREATE TABLE song_artist (
    song_id   TEXT NOT NULL REFERENCES song (id) ON DELETE CASCADE,
    artist_id TEXT NOT NULL REFERENCES artist (id) ON DELETE RESTRICT,
    position  INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (song_id, artist_id)
);

CREATE INDEX song_artist_artist_idx ON song_artist (artist_id);
