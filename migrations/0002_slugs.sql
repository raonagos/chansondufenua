-- 0002_slugs.sql — the URL a song is published under.
--
-- Until this migration a song had exactly one address, `/himene/{id}`, the v3
-- record key. From here the canonical address is a slug built from the title
-- (`/himene/ahani-e`), the id URL 301s to it, and no slug the site has ever
-- published is ever handed to a different song.
--
-- Two columns and a table:
--
--   * `song.slug` is the *current* address. Empty means "no slug" — the song is
--     published under its id, which is what a title with no Latin letters in it
--     gets (see `src/domain/slug.rs`) and what a database that was written
--     before this migration gets until the backfill in `Db::open` runs.
--   * `song_slug` is the *history*: one row per slug ever assigned, never
--     deleted. It is what makes a retired slug redirect instead of 404, and its
--     primary key is what makes re-issuing one to another song impossible rather
--     than merely discouraged.
--
-- The backfill is NOT here. What a title slugifies to is a product rule with a
-- transliteration table (`src/domain/slug.rs`), and SQLite has no honest way to
-- spell it — a nested `replace()` chain would be a second implementation of the
-- rule, free to drift from the Rust one. So this file is DDL only and the
-- backfill runs in Rust, once, when the database is opened.
--
-- `DELETE` on a song still cascades to its credits; it now also cascades to its
-- slugs, because a slug for a song that is gone has nothing to redirect to.
-- There is no delete path in this application (v4.1's MCP is read-only), so this
-- is about the schema not lying rather than about a code path.

ALTER TABLE song ADD COLUMN slug TEXT NOT NULL DEFAULT '';

CREATE TABLE song_slug (
    slug    TEXT PRIMARY KEY NOT NULL,
    song_id TEXT NOT NULL REFERENCES song (id) ON DELETE CASCADE
);

-- A song's slugs are read back when it is renamed; the index is for that read,
-- and for the resolver's `WHERE slug = ?` primary key lookup (which uses the
-- table's own key, not this one).
CREATE INDEX song_slug_song_idx ON song_slug (song_id);

-- One song per slug. Partial, because every song that has no slug yet shares the
-- empty string and an unconditional unique index would refuse the second one.
CREATE UNIQUE INDEX song_slug_current_idx ON song (slug) WHERE slug <> '';
