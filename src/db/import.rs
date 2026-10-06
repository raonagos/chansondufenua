//! Step 3b: read a `surreal export` dump and load it into SQLite.
//!
//! A `surreal export` is not SQL and not JSON. It is SurrealQL *text*:
//!
//! ```text
//! -- TABLE DATA: artist
//! INSERT [ { created_at: '2024-09-13T20:14:38.229323832Z', fullname: 'Jonas',
//!            id: artist:1kvm9y2tcplm43wgeuni, updated_at: d'2024-09-29T…Z' }, … ]
//! ```
//!
//! One `INSERT` per table, one enormous line, values that are single-quoted
//! strings, `d'…'` datetimes, `table:key` record links, arrays and nested
//! objects. So this module has a small recursive-descent parser for that
//! subset, rather than a regex — the lyrics alone contain `<`, `>`, `"`, `'`,
//! `&` and `\`, and a regex over those is how you lose a chord.
//!
//! Three deliberate choices:
//!
//! * **`user` rows are counted, never imported.** They hold argon2id password
//!   hashes for 135 records. The v4 schema has no user table and no auth, so
//!   there is nothing to import them into — and no reason to move hashes off
//!   the host they came from.
//! * **Ids are preserved.** A song's id is its stable key — the `id` field of
//!   the JSON API and of the MCP tools — and `Song::get_path` is its address:
//!   a slug built from the title, which is what keeps every v3 URL alive as a
//!   301. See `migrations/0002_slugs.sql`.
//! * **Credit order is preserved**, because it is visible on the page.
//!
//! The import is idempotent: re-running it over the same dump updates rows
//! rather than duplicating them, so a corrected dump can simply be re-imported.

use sqlx::SqlitePool;
use thiserror::Error;

use super::DbResult;
use super::queries::assign_slug;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImportError {
    #[error("unexpected end of input")]
    UnexpectedEof,

    #[error("unexpected {found:?} at byte {at}, expected {expected}")]
    Unexpected {
        found: String,
        expected: String,
        at: usize,
    },

    #[error("line {line}: {message}")]
    Record { line: usize, message: String },
}

// ---------------------------------------------------------------------------
// The subset of SurrealQL values we actually meet
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `'text'` or `"text"`, with backslash escapes already resolved.
    Str(String),
    /// `d'2025-03-09T20:09:12.123456789Z'`.
    Datetime(String),
    /// `artist:1kvm9y2tcplm43wgeuni` — kept whole, as `table:key`.
    RecordId(String),
    Number(f64),
    Bool(bool),
    Null,
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Self::Str(_) => "a string",
            Self::Datetime(_) => "a datetime",
            Self::RecordId(_) => "a record id",
            Self::Number(_) => "a number",
            Self::Bool(_) => "a boolean",
            Self::Null => "null",
            Self::Array(_) => "an array",
            Self::Object(_) => "an object",
        }
    }

    /// The text of a string or datetime, whichever spelling the exporter used.
    ///
    /// Both spellings really occur in the same dump: some `created_at` values
    /// are plain quoted strings, others are `d'…'`. Treating either as an error
    /// would reject half the file.
    fn text(&self) -> Option<&str> {
        match self {
            Self::Str(s) | Self::Datetime(s) => Some(s),
            _ => None,
        }
    }

    fn record_key(&self) -> Option<&str> {
        match self {
            Self::RecordId(rid) => Some(rid.rsplit(':').next().unwrap_or(rid)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn skip_whitespace(&mut self) {
        let trimmed = self.rest().trim_start();
        self.pos = self.src.len() - trimmed.len();
    }

    fn eat(&mut self, literal: &str) -> bool {
        self.skip_whitespace();
        if self.rest().starts_with(literal) {
            self.pos += literal.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, literal: &str) -> Result<(), ImportError> {
        if self.eat(literal) {
            Ok(())
        } else {
            Err(self.unexpected(literal))
        }
    }

    fn unexpected(&self, expected: &str) -> ImportError {
        ImportError::Unexpected {
            found: self.rest().chars().take(24).collect(),
            expected: expected.to_owned(),
            at: self.pos,
        }
    }

    /// `INSERT [ … ]` — the whole line.
    fn parse_insert(&mut self) -> Result<Vec<Value>, ImportError> {
        self.expect("INSERT")?;
        match self.parse_value()? {
            Value::Array(rows) => Ok(rows),
            other => Err(ImportError::Unexpected {
                found: other.type_name().to_owned(),
                expected: "an array of records".to_owned(),
                at: self.pos,
            }),
        }
    }

    fn parse_value(&mut self) -> Result<Value, ImportError> {
        self.skip_whitespace();
        let rest = self.rest();
        let Some(first) = rest.chars().next() else {
            return Err(ImportError::UnexpectedEof);
        };

        match first {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '\'' => Ok(Value::Str(self.parse_string('\'')?)),
            '"' => Ok(Value::Str(self.parse_string('"')?)),
            // `d'…'` / `r'…'` casts. Checked before the bare-identifier arm.
            'd' if rest.starts_with("d'") => {
                self.pos += 1;
                Ok(Value::Datetime(self.parse_string('\'')?))
            }
            'r' if rest.starts_with("r'") => {
                self.pos += 1;
                Ok(Value::Str(self.parse_string('\'')?))
            }
            c if c.is_ascii_digit() || c == '-' || c == '+' => {
                Ok(Value::Number(self.parse_number()?))
            }
            c if c.is_ascii_alphabetic() || c == '_' => self.parse_word(),
            _ => Err(self.unexpected("a value")),
        }
    }

    fn parse_string(&mut self, quote: char) -> Result<String, ImportError> {
        self.expect(&quote.to_string())?;
        let mut out = String::new();

        loop {
            let rest = self.rest();
            let mut chars = rest.chars();
            let Some(c) = chars.next() else {
                return Err(ImportError::UnexpectedEof);
            };

            if c == quote {
                self.pos += quote.len_utf8();
                return Ok(out);
            }

            if c == '\\' {
                let escaped = chars.next().ok_or(ImportError::UnexpectedEof)?;
                out.push(match escaped {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    other => other,
                });
                self.pos += 1 + escaped.len_utf8();
                continue;
            }

            out.push(c);
            self.pos += c.len_utf8();
        }
    }

    fn parse_number(&mut self) -> Result<f64, ImportError> {
        let rest = self.rest();
        let end = rest
            .find(|c: char| !matches!(c, '0'..='9' | '+' | '-' | '.' | 'e' | 'E'))
            .unwrap_or(rest.len());
        let text = &rest[..end];
        let value = text.parse::<f64>().map_err(|_| ImportError::Unexpected {
            found: text.to_owned(),
            expected: "a number".to_owned(),
            at: self.pos,
        })?;
        self.pos += end;
        Ok(value)
    }

    /// A bare word: `true`, `false`, `NONE`, or the `table:key` part of a
    /// record link.
    fn parse_word(&mut self) -> Result<Value, ImportError> {
        let word = self.take_while(|c| c.is_ascii_alphanumeric() || c == '_');
        if word.is_empty() {
            return Err(self.unexpected("a bare word"));
        }

        match word {
            "true" => return Ok(Value::Bool(true)),
            "false" => return Ok(Value::Bool(false)),
            "NONE" | "null" | "NULL" => return Ok(Value::Null),
            _ => {}
        }

        // `table:key`. The key may start with a digit (`song:0laf5klk…`), which
        // is why this is not parsed as a plain identifier.
        if self.eat(":") {
            let key = self.take_while(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if key.is_empty() {
                return Err(self.unexpected("a record key"));
            }
            return Ok(Value::RecordId(format!("{word}:{key}")));
        }

        // Anything else bare is something this parser does not understand.
        // Refusing beats guessing: a one-shot importer that quietly drops a
        // field is worse than one that stops.
        Err(ImportError::Unexpected {
            found: word.to_owned(),
            expected: "a value (quote bare words)".to_owned(),
            at: self.pos - word.len(),
        })
    }

    fn take_while(&mut self, accept: impl Fn(char) -> bool) -> &'a str {
        let rest = self.rest();
        let end = rest.find(|c: char| !accept(c)).unwrap_or(rest.len());
        let taken = &rest[..end];
        self.pos += end;
        taken
    }

    fn parse_object(&mut self) -> Result<Value, ImportError> {
        self.expect("{")?;
        let mut fields = Vec::new();

        loop {
            if self.eat("}") {
                return Ok(Value::Object(fields));
            }
            let key = self.parse_key()?;
            self.expect(":")?;
            fields.push((key, self.parse_value()?));
            if self.eat(",") {
                continue;
            }
            self.expect("}")?;
            return Ok(Value::Object(fields));
        }
    }

    fn parse_array(&mut self) -> Result<Value, ImportError> {
        self.expect("[")?;
        let mut items = Vec::new();

        loop {
            if self.eat("]") {
                return Ok(Value::Array(items));
            }
            items.push(self.parse_value()?);
            if self.eat(",") {
                continue;
            }
            self.expect("]")?;
            return Ok(Value::Array(items));
        }
    }

    fn parse_key(&mut self) -> Result<String, ImportError> {
        self.skip_whitespace();
        match self.rest().chars().next() {
            Some('\'') => self.parse_string('\''),
            Some('"') => self.parse_string('"'),
            Some(c) if c.is_ascii_alphabetic() || c == '_' => Ok(self
                .take_while(|c| c.is_ascii_alphanumeric() || c == '_')
                .to_owned()),
            _ => Err(self.unexpected("a field name")),
        }
    }
}

// ---------------------------------------------------------------------------
// Typed records
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedArtist {
    pub id: String,
    pub fullname: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedSong {
    pub id: String,
    pub title: String,
    pub lyrics: String,
    pub view_count: i64,
    pub published: bool,
    /// Artist ids, in credit order.
    pub artist_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A parsed dump. `user_rows` is a count and nothing else — see the module doc.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Dump {
    pub artists: Vec<ImportedArtist>,
    pub songs: Vec<ImportedSong>,
    pub user_rows: usize,
}

impl Dump {
    /// Parse every `INSERT` statement in a `surreal export`.
    ///
    /// Non-`INSERT` lines (the `--` banners, `DEFINE …`, `OPTION IMPORT;`) are
    /// skipped — the schema is owned by `migrations/`, not by the dump.
    pub fn parse(source: &str) -> Result<Self, ImportError> {
        let mut dump = Self::default();

        for (index, line) in source.lines().enumerate() {
            let line_number = index + 1;
            if !line.trim_start().starts_with("INSERT ") {
                continue;
            }

            let rows = Parser::new(line)
                .parse_insert()
                .map_err(|error| match error {
                    // The parser works in byte offsets; a human wants a line number.
                    ImportError::Unexpected {
                        found,
                        expected,
                        at,
                    } => ImportError::Record {
                        line: line_number,
                        message: format!(
                            "cannot parse near byte {at}: expected {expected}, found {found:?}"
                        ),
                    },
                    other => ImportError::Record {
                        line: line_number,
                        message: other.to_string(),
                    },
                })?;

            for row in rows {
                let Value::Object(fields) = row else {
                    return Err(ImportError::Record {
                        line: line_number,
                        message: format!("expected a record object, found {}", row.type_name()),
                    });
                };
                match table_of(&fields, line_number)? {
                    "artist" => dump.artists.push(artist_from(&fields, line_number)?),
                    "song" => dump.songs.push(song_from(&fields, line_number)?),
                    // Present in the dump, deliberately not imported.
                    "user" => dump.user_rows += 1,
                    _ => {}
                }
            }
        }

        Ok(dump)
    }

    /// Check every artist a song credits actually exists in the dump.
    ///
    /// The foreign key would catch this too, but only after a partial write and
    /// with a message about `song_artist` that does not say which id is missing.
    fn check_credits_resolve(&self) -> Result<(), ImportError> {
        let known: std::collections::HashSet<&str> =
            self.artists.iter().map(|a| a.id.as_str()).collect();

        for song in &self.songs {
            for artist_id in &song.artist_ids {
                if !known.contains(artist_id.as_str()) {
                    return Err(ImportError::Record {
                        line: 0,
                        message: format!(
                            "song {} credits artist {artist_id}, which is not in this dump — \
                             the export is incomplete",
                            song.id
                        ),
                    });
                }
            }
        }
        Ok(())
    }
}

// -- field access -----------------------------------------------------------

/// The table a record belongs to: the part before the `:` of its `id`.
fn table_of(fields: &[(String, Value)], line: usize) -> Result<&str, ImportError> {
    match field(fields, "id", line)? {
        Value::RecordId(full) => Ok(full.split(':').next().unwrap_or("")),
        other => Err(ImportError::Record {
            line,
            message: format!("`id` should be a record id, found {}", other.type_name()),
        }),
    }
}

fn field<'a>(
    fields: &'a [(String, Value)],
    name: &str,
    line: usize,
) -> Result<&'a Value, ImportError> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .ok_or_else(|| ImportError::Record {
            line,
            message: format!("a record is missing its `{name}` field"),
        })
}

fn text_field(fields: &[(String, Value)], name: &str, line: usize) -> Result<String, ImportError> {
    let value = field(fields, name, line)?;
    value
        .text()
        .map(str::to_owned)
        .ok_or_else(|| ImportError::Record {
            line,
            message: format!("`{name}` should be a string, found {}", value.type_name()),
        })
}

fn key_field(fields: &[(String, Value)], name: &str, line: usize) -> Result<String, ImportError> {
    field(fields, name, line)?
        .record_key()
        .map(str::to_owned)
        .ok_or_else(|| ImportError::Record {
            line,
            message: format!("`{name}` should be a record id"),
        })
}

/// The id of the record itself.
fn id_field(fields: &[(String, Value)], line: usize) -> Result<String, ImportError> {
    key_field(fields, "id", line)
}

fn artist_from(fields: &[(String, Value)], line: usize) -> Result<ImportedArtist, ImportError> {
    Ok(ImportedArtist {
        id: id_field(fields, line)?,
        fullname: text_field(fields, "fullname", line)?,
        created_at: text_field(fields, "created_at", line)?,
        updated_at: text_field(fields, "updated_at", line)?,
    })
}

fn song_from(fields: &[(String, Value)], line: usize) -> Result<ImportedSong, ImportError> {
    let credits = field(fields, "artists", line)?;
    let Value::Array(items) = credits else {
        return Err(ImportError::Record {
            line,
            message: format!(
                "`artists` should be an array, found {}",
                credits.type_name()
            ),
        });
    };

    let mut artist_ids = Vec::with_capacity(items.len());
    for item in items {
        let id = item.record_key().ok_or_else(|| ImportError::Record {
            line,
            message: format!(
                "`artists` contains {}, expected a record id",
                item.type_name()
            ),
        })?;
        artist_ids.push(id.to_owned());
    }

    let published = match field(fields, "published", line)? {
        Value::Bool(value) => *value,
        Value::Number(value) => *value != 0.0,
        other => {
            return Err(ImportError::Record {
                line,
                message: format!(
                    "`published` should be a boolean, found {}",
                    other.type_name()
                ),
            });
        }
    };

    let view_count = match field(fields, "view_count", line)? {
        Value::Number(value) => *value as i64,
        other => {
            return Err(ImportError::Record {
                line,
                message: format!(
                    "`view_count` should be a number, found {}",
                    other.type_name()
                ),
            });
        }
    };

    Ok(ImportedSong {
        id: id_field(fields, line)?,
        title: text_field(fields, "title", line)?,
        lyrics: text_field(fields, "lyrics", line)?,
        view_count,
        published,
        artist_ids,
        created_at: text_field(fields, "created_at", line)?,
        updated_at: text_field(fields, "updated_at", line)?,
    })
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub artists: usize,
    pub songs: usize,
    pub credits: usize,
    /// Counted, not imported. See the module doc.
    pub user_rows_skipped: usize,
}

/// Parse `source` and load it, in one transaction.
pub async fn import_dump(pool: &SqlitePool, source: &str) -> DbResult<ImportReport> {
    let dump = Dump::parse(source)?;
    load(pool, &dump).await
}

/// Load an already-parsed dump.
///
/// Everything happens inside one transaction: a dump that fails half way
/// through leaves the previous data exactly as it was.
pub async fn load(pool: &SqlitePool, dump: &Dump) -> DbResult<ImportReport> {
    dump.check_credits_resolve()?;

    let mut tx = pool.begin().await?;

    for artist in &dump.artists {
        // Upsert rather than INSERT OR REPLACE: REPLACE is a DELETE followed by
        // an INSERT, and `song_artist.artist_id` is ON DELETE RESTRICT, so
        // replacing a credited artist would be refused. It would also churn the
        // FTS index through a needless delete.
        sqlx::query(
            "INSERT INTO artist (id, fullname, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(id) DO UPDATE SET \
                 fullname = excluded.fullname, \
                 created_at = excluded.created_at, \
                 updated_at = excluded.updated_at",
        )
        .bind(&artist.id)
        .bind(&artist.fullname)
        .bind(&artist.created_at)
        .bind(&artist.updated_at)
        .execute(&mut *tx)
        .await?;
    }

    let mut credits = 0usize;
    for song in &dump.songs {
        sqlx::query(
            "INSERT INTO song \
             (id, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(id) DO UPDATE SET \
                 title = excluded.title, \
                 lyrics = excluded.lyrics, \
                 view_count = excluded.view_count, \
                 published = excluded.published, \
                 created_at = excluded.created_at, \
                 updated_at = excluded.updated_at",
        )
        .bind(&song.id)
        .bind(&song.title)
        .bind(&song.lyrics)
        .bind(song.view_count)
        .bind(song.published)
        .bind(&song.created_at)
        .bind(&song.updated_at)
        .execute(&mut *tx)
        .await?;

        // The address, minted from the title by the same function the form and
        // the backfill use. Idempotent for a title that has not changed — the
        // slug comes back identical — and for one that has, the song moves and
        // the slug it used to live at stays in `song_slug`, which is what makes
        // the old URL a 301 rather than a 404.
        assign_slug(&mut tx, &song.id, &song.title).await?;

        // Clear then re-insert, so a re-import cannot accumulate stale credits
        // and a changed credit order is honoured.
        sqlx::query("DELETE FROM song_artist WHERE song_id = ?1")
            .bind(&song.id)
            .execute(&mut *tx)
            .await?;

        for (position, artist_id) in song.artist_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO song_artist (song_id, artist_id, position) VALUES (?1, ?2, ?3)",
            )
            .bind(&song.id)
            .bind(artist_id)
            .bind(position as i64)
            .execute(&mut *tx)
            .await?;
            credits += 1;
        }
    }

    tx.commit().await?;

    Ok(ImportReport {
        artists: dump.artists.len(),
        songs: dump.songs.len(),
        credits,
        user_rows_skipped: dump.user_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::db::queries::{SongOrder, artists, songs};
    use crate::domain::Song;

    /// A miniature export, exercising every syntax the real one contains:
    /// single- and double-quoted strings, backslash escapes, `d'…'` datetimes,
    /// plain-string timestamps, `table:key` record links, nested arrays,
    /// booleans, numbers, and the `--` banners we must ignore.
    const SAMPLE: &str = r#"-- ------------------------------
-- TABLE: artist
-- ------------------------------

DEFINE TABLE artist SCHEMAFULL;

-- ------------------------------
-- TABLE DATA: artist
-- ------------------------------

INSERT [ { created_at: '2024-09-13T20:14:38.229323832Z', fullname: 'Jonas', id: artist:1kvm9y2tcplm43wgeuni, updated_at: d'2024-09-29T16:13:50.161997956Z' }, { created_at: d'2024-08-28T16:57:28.361141126Z', fullname: 'Barthélémy', id: artist:uynyy7ei47uolm5ns3h2, updated_at: d'2024-09-29T16:13:50.162647406Z' } ]

-- ------------------------------
-- TABLE DATA: song
-- ------------------------------

INSERT [ { artists: [artist:1kvm9y2tcplm43wgeuni, artist:uynyy7ei47uolm5ns3h2], created_at: d'2025-01-02T15:39:15.188295761Z', id: song:7bvi97gjea010jgxejv5, lyrics: "E pa<sup data-nosnippet=\"\">D</sup>pe meha'i<div>T<sup data-nosnippet></sup>ext</div><div><br></div>A & B \\ done", published: true, title: "Pape meha'i", updated_at: d'2025-01-02T15:39:15.188295761Z', view_count: 285 } ]

-- ------------------------------
-- TABLE DATA: user
-- ------------------------------

INSERT [ { ROLE: 'GUESS', id: user:0cbdjtwxu46c2p6msxts, password: '$argon2id$v=19$m=19456,t=2,p=1$abc$def', username: 'U4jBhVHBKVbHgL9' } ]
"#;

    #[test]
    fn parses_the_sample_export() {
        let dump = Dump::parse(SAMPLE).expect("parse");

        assert_eq!(dump.artists.len(), 2);
        assert_eq!(dump.songs.len(), 1);
        assert_eq!(dump.user_rows, 1, "user rows are counted");

        let jonas = &dump.artists[0];
        assert_eq!(
            jonas.id, "1kvm9y2tcplm43wgeuni",
            "the `artist:` prefix is stripped"
        );
        assert_eq!(jonas.fullname, "Jonas");
        assert_eq!(jonas.created_at, "2024-09-13T20:14:38.229323832Z");
        assert_eq!(jonas.updated_at, "2024-09-29T16:13:50.161997956Z");
        // `d'…'` and a plain quoted string are both accepted, because the real
        // dump mixes them.
        assert_eq!(dump.artists[1].fullname, "Barthélémy");
        assert_eq!(dump.artists[1].created_at, "2024-08-28T16:57:28.361141126Z");

        let song = &dump.songs[0];
        assert_eq!(song.id, "7bvi97gjea010jgxejv5");
        assert_eq!(song.title, "Pape meha'i");
        assert_eq!(song.view_count, 285);
        assert!(song.published);
        assert_eq!(
            song.artist_ids,
            ["1kvm9y2tcplm43wgeuni", "uynyy7ei47uolm5ns3h2"]
        );

        // Escapes are resolved, and the chord markup — including the bare
        // `data-nosnippet` spelling — survives untouched.
        assert_eq!(
            song.lyrics,
            "E pa<sup data-nosnippet=\"\">D</sup>pe meha'i<div>T<sup data-nosnippet></sup>ext</div>\
             <div><br></div>A & B \\ done"
        );
    }

    #[test]
    fn a_malformed_dump_names_the_line() {
        let broken = "INSERT [ { id: song:abc }\n";
        let error = Dump::parse(broken).unwrap_err();
        assert!(
            matches!(&error, ImportError::Record { line: 1, .. }),
            "expected a line-1 error, got {error}"
        );
    }

    #[test]
    fn a_missing_field_is_reported_not_defaulted() {
        // No `title`: importing this would silently produce an empty title.
        // `artists` is present-but-empty on purpose: every record in a real
        // export carries the key (43 songs, one of them with no credit), so a
        // *missing* `artists` is a malformed dump and is reported as such.
        // Leaving it out here would make that error fire before this one.
        let broken = "INSERT [ { artists: [], created_at: d'2025-01-01T00:00:00Z', id: song:abc, \
                      lyrics: 'x', published: true, updated_at: d'2025-01-01T00:00:00Z', \
                      view_count: 1 } ]";
        let error = Dump::parse(broken).unwrap_err();
        assert!(error.to_string().contains("title"), "{error}");
    }

    #[test]
    fn an_unquoted_bare_word_is_refused() {
        // Guessing here is how a field quietly becomes the string "FULL".
        let broken = "INSERT [ { id: song:abc, title: FULL } ]";
        assert!(Dump::parse(broken).is_err());
    }

    #[test]
    fn a_dangling_credit_is_refused_before_any_write() {
        let dump = Dump::parse(
            "INSERT [ { created_at: d'2025-01-01T00:00:00Z', fullname: 'Jonas', \
             id: artist:1kvm9y2tcplm43wgeuni, updated_at: d'2025-01-01T00:00:00Z' } ]\n\
             INSERT [ { artists: [artist:nosuchartist000000], \
             created_at: d'2025-01-01T00:00:00Z', id: song:aaaaaaaaaaaaaaaaaaaa, \
             lyrics: 'x', published: true, title: 'A title', \
             updated_at: d'2025-01-01T00:00:00Z', view_count: 1 } ]",
        )
        .expect("parse");

        let error = dump.check_credits_resolve().unwrap_err();
        assert!(
            error.to_string().contains("nosuchartist000000"),
            "the message should name the missing artist: {error}"
        );
    }

    #[tokio::test]
    async fn loading_the_sample_twice_is_idempotent() {
        let db = Db::open_in_memory().await.unwrap();
        let dump = Dump::parse(SAMPLE).unwrap();

        let first = load(db.pool(), &dump).await.unwrap();
        assert_eq!(first.artists, 2);
        assert_eq!(first.songs, 1);
        assert_eq!(first.credits, 2);

        let second = load(db.pool(), &dump).await.unwrap();
        assert_eq!(second, first, "a second run must not duplicate anything");

        assert_eq!(
            songs(db.pool(), SongOrder::Newest, None)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(artists(db.pool()).await.unwrap().len(), 2);
        let credits: i64 = sqlx::query_scalar("SELECT count(*) FROM song_artist")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(credits, 2);
    }

    /// The address survives a re-import unchanged, and a *renamed* song in a
    /// corrected dump moves — with the slug it used to live at left in the
    /// history, so the old URL redirects rather than 404s.
    #[tokio::test]
    async fn loading_assigns_the_slug_and_a_rename_moves_it_once() {
        let db = Db::open_in_memory().await.unwrap();
        let dump = Dump::parse(SAMPLE).unwrap();
        load(db.pool(), &dump).await.unwrap();

        let id = "7bvi97gjea010jgxejv5";
        let first = crate::db::song(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(first.get_slug().as_deref(), Some("pape-mehai"));

        // Same dump again: the address must not move, and must not grow a `-2`.
        load(db.pool(), &Dump::parse(SAMPLE).unwrap())
            .await
            .unwrap();
        let again = crate::db::song(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(again.get_slug().as_deref(), Some("pape-mehai"));

        // A corrected dump with a new title: the song moves, the old slug stays
        // resolvable and points at the new address.
        let renamed = SAMPLE.replace(r#"title: "Pape meha'i""#, r#"title: "Pape meha'i nui""#);
        load(db.pool(), &Dump::parse(&renamed).unwrap())
            .await
            .unwrap();

        let moved = crate::db::song(db.pool(), id).await.unwrap().unwrap();
        assert_eq!(moved.get_slug().as_deref(), Some("pape-mehai-nui"));

        let retired = crate::db::song_at(db.pool(), "pape-mehai")
            .await
            .unwrap()
            .unwrap();
        assert!(!retired.is_canonical());
        assert_eq!(retired.song().get_id(), id);
        assert_eq!(retired.song().get_path(), "/himene/pape-mehai-nui");
    }

    #[tokio::test]
    async fn loading_preserves_credit_order_and_lyrics() {
        let db = Db::open_in_memory().await.unwrap();
        load(db.pool(), &Dump::parse(SAMPLE).unwrap())
            .await
            .unwrap();

        let song = crate::db::song(db.pool(), "7bvi97gjea010jgxejv5")
            .await
            .unwrap()
            .expect("present");
        let names: Vec<String> = song
            .get_artists()
            .iter()
            .map(|a| a.get_fullname())
            .collect();
        assert_eq!(names, ["Jonas", "Barthélémy"]);
        assert!(song.get_lyrics().contains(r#"<sup data-nosnippet></sup>"#));
        assert!(song.get_lyrics().contains("A & B \\ done"));
    }

    #[tokio::test]
    async fn re_importing_after_a_credit_change_replaces_the_credits() {
        let db = Db::open_in_memory().await.unwrap();
        load(db.pool(), &Dump::parse(SAMPLE).unwrap())
            .await
            .unwrap();

        let fewer_source = SAMPLE.replace(
            "artists: [artist:1kvm9y2tcplm43wgeuni, artist:uynyy7ei47uolm5ns3h2]",
            "artists: [artist:uynyy7ei47uolm5ns3h2]",
        );
        let fewer = Dump::parse(&fewer_source).unwrap();
        load(db.pool(), &fewer).await.unwrap();

        let credits: i64 = sqlx::query_scalar("SELECT count(*) FROM song_artist")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(credits, 1, "the removed credit must not linger");
    }

    /// The real thing: 43 songs, 34 artists, 135 user rows, straight out of the
    /// maintainer's export.
    ///
    /// `.run/` is gitignored, so a fresh clone will not have the dump. The test
    /// says so out loud rather than passing silently.
    #[tokio::test]
    async fn the_real_dump_round_trips_into_sqlite() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".run/inbox/chansondufenua_backup_22032025.db");
        let Ok(source) = std::fs::read_to_string(&path) else {
            eprintln!("SKIPPED: {} is not present", path.display());
            return;
        };

        let dump = Dump::parse(&source).expect("the real dump must parse");
        assert_eq!(dump.artists.len(), 34);
        assert_eq!(dump.songs.len(), 43);
        assert_eq!(dump.user_rows, 135);

        let db = Db::open_in_memory().await.unwrap();
        let report = load(db.pool(), &dump).await.unwrap();
        assert_eq!(report.artists, 34);
        assert_eq!(report.songs, 43);
        assert_eq!(report.credits, 42);
        assert_eq!(report.user_rows_skipped, 135);

        // Nothing created a `user` table on the way through.
        let users: i64 =
            sqlx::query_scalar("SELECT count(*) FROM sqlite_master WHERE name = 'user'")
                .fetch_one(db.pool())
                .await
                .unwrap();
        assert_eq!(users, 0, "no auth tables in v4");

        // Ordering matches the live site exactly: created_at DESC, id asc.
        let stored = songs(db.pool(), SongOrder::Newest, None).await.unwrap();
        assert_eq!(stored.len(), 43);
        let mut expected = dump.songs.iter().collect::<Vec<_>>();
        expected.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        assert_eq!(
            stored.iter().map(Song::get_id).collect::<Vec<_>>(),
            expected.iter().map(|s| s.id.clone()).collect::<Vec<_>>()
        );

        // Every song comes back byte-for-byte, credits and all.
        for song in &stored {
            let original = dump
                .songs
                .iter()
                .find(|s| s.id == song.get_id())
                .expect("every stored song came from the dump");
            assert_eq!(song.get_title(), original.title, "{}", song.get_id());
            assert_eq!(song.get_lyrics(), original.lyrics, "{}", song.get_id());
            assert_eq!(song.get_view_count() as i64, original.view_count);
            assert_eq!(song.is_published(), original.published);
            assert_eq!(
                song.get_artists()
                    .iter()
                    .map(|a| a.get_id())
                    .collect::<Vec<_>>(),
                original.artist_ids,
                "credit order for {}",
                song.get_id()
            );
        }
    }
}
