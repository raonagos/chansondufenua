-- 0003_search.sql — searching the catalogue by title.
--
-- `artist_fts` (0001) already answers "who" with FTS5 and `remove_diacritics 2`,
-- which is SQLite's spelling of v3's `punct_lower_ascii` SEARCH ANALYZER: a
-- needle typed without an ʻokina or a macron still finds the name that carries
-- one. This migration gives the *titles* the same treatment, so one search box
-- finds both halves of the catalogue — `ahani` reaches `'Āhani e` and `mama`
-- reaches `Māmā Tahiti`.
--
-- External content again: the text lives in `song.title`, the index lives here,
-- and the three triggers keep them in step. The `unicode61` tokenizer splits on
-- punctuation exactly as it does for artists — `'Āhani e` is indexed as `ahani`
-- and `e` — so an ʻokina *inside* a word (`Mā'ohi` → `ma`, `ohi`) stays the
-- known limitation v3 shipped with, recorded in `search_artists`'s own tests.
--
-- `rebuild` runs once, here, so a database that already holds songs is
-- searchable on the first request after this migration rather than only after
-- the next import. On a fresh database the table is empty and the triggers do
-- all the work.
--
-- The lyrics are deliberately NOT indexed. A search box that matched a whole
-- 6000-character lyric would return every song in the catalogue for a common
-- French word; v3 searched titles and names only, and so does this.

CREATE VIRTUAL TABLE song_fts USING fts5 (
    title,
    content = 'song',
    content_rowid = 'rowid',
    tokenize = "unicode61 remove_diacritics 2"
);

CREATE TRIGGER song_fts_after_insert AFTER INSERT ON song BEGIN
    INSERT INTO song_fts (rowid, title) VALUES (new.rowid, new.title);
END;

CREATE TRIGGER song_fts_after_delete AFTER DELETE ON song BEGIN
    INSERT INTO song_fts (song_fts, rowid, title) VALUES ('delete', old.rowid, old.title);
END;

CREATE TRIGGER song_fts_after_update AFTER UPDATE ON song BEGIN
    INSERT INTO song_fts (song_fts, rowid, title) VALUES ('delete', old.rowid, old.title);
    INSERT INTO song_fts (rowid, title) VALUES (new.rowid, new.title);
END;

INSERT INTO song_fts (song_fts) VALUES ('rebuild');
