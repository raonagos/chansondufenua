//! The chrome catalog: the words of [`Key`], read from `locales/*.toml`.
//!
//! # Why a file, and not a match
//!
//! The words used to be three exhaustive `match` arms inside [`Key`], which made
//! a missing translation a compile error and every finished sentence a rebuild.
//! That trade is wrong for this site: the Tahitian is a first pass waiting for a
//! native speaker and the French is the maintainer's, so the people who would
//! correct a sentence are not the people who run `cargo build`. The words now
//! live in files that are read once, at startup — correcting one is an edit and
//! a restart.
//!
//! What the compiler used to check, the boot checks instead: [`Catalog::load`]
//! refuses a file with a key no enum variant owns, and refuses `fr.toml` or
//! `en.toml` that does not carry every key. It does *not* require that of
//! `ty.toml`: an unfinished Tahitian translation is the normal state of this
//! catalog, and a missing line there is served from English.
//!
//! # Where the files are
//!
//! `LOCALES_DIR`, or the first of `./locales` and `<the binary's directory>/locales`
//! that holds `fr.toml`. The working directory first, so a clone runs as it
//! stands; beside the binary second, so a deployment is one directory. See
//! [`locate`].
//!
//! # The lookup order
//!
//! `locale → en → fr → the key's own name`. One order, applied to every key:
//! asked for a word in Tahitian, the catalog answers with the Tahitian if the
//! file has it, then the English, then the French, and only if all three are
//! silent with the key's name — which a reader should never see, and which the
//! boot's completeness check for `fr.toml` and `en.toml` makes unreachable in
//! practice.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::{Key, Lang};

/// The environment variable naming the directory the catalogs are read from.
///
/// Set, it is the *only* directory tried: a deployment that says where its
/// translations are should hear about a typo, not quietly serve the ones that
/// happen to sit next to the binary.
pub const ENV: &str = "LOCALES_DIR";

/// The directory name looked for when [`ENV`] is not set: here, and beside the
/// binary.
pub const DIR: &str = "locales";

/// One language's words: the key's name to the string it says.
///
/// Keys and words are borrowed from the file for the life of the process (see
/// [`read`]), so a page hands out `&'static str` and nothing is cloned per
/// request.
pub type Table = BTreeMap<&'static str, &'static str>;

/// The catalog this process serves its chrome from.
pub struct Catalog {
    /// One table per language, in [`Lang::ALL`] order — the same list every
    /// other "all the languages" question in this crate is answered from.
    tables: [Table; 3],
}

/// What a boot has to say about the catalog it read.
#[derive(Debug)]
pub struct Report {
    /// The directory the words came from.
    pub dir: PathBuf,
    /// How many keys there are to say.
    pub keys: usize,
    /// How many of them each language carries, in [`Lang::ALL`] order.
    pub words: [(Lang, usize); 3],
}

/// A catalog that cannot be read, or that says something it should not.
///
/// Every variant is a boot failure rather than a fallback: the whole reason the
/// words moved out of the compiler's reach is that only a check the maintainer
/// can see is left, and a silent one is not a check.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Neither the environment nor the two default places holds the French
    /// catalog — the one file the site cannot serve without.
    #[error("no {file} in any of {tried:?}")]
    NotFound {
        /// The directories that were tried, in order.
        tried: Vec<PathBuf>,
        /// The file that was looked for.
        file: &'static str,
    },
    /// The file is there and unreadable, or not a file at all.
    #[error("cannot read {path}: {source}")]
    Read {
        /// The file.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
    /// The file is not the flat `name = "words"` table this catalog is.
    #[error("{path} is not a table of name = \"words\" lines: {message}")]
    Parse {
        /// The file.
        path: PathBuf,
        /// The parser's own message, at the line it stopped on.
        message: String,
    },
    /// The file names a key no [`Key`] variant owns — a typo that would
    /// otherwise translate nothing, silently.
    #[error("{path} names {name:?}, which is not a key")]
    Unknown {
        /// The file.
        path: PathBuf,
        /// The name it used.
        name: String,
    },
    /// A file the lookup falls back to does not carry every key. `ty.toml` is
    /// deliberately not held to this.
    #[error("{path} does not say {key:?}")]
    Missing {
        /// The file.
        path: PathBuf,
        /// The key it is silent about.
        key: Key,
    },
}

/// The file a language's words live in.
pub fn file(lang: Lang) -> &'static str {
    match lang {
        Lang::Fr => "fr.toml",
        Lang::Ty => "ty.toml",
        Lang::En => "en.toml",
    }
}

/// The catalog, read once for the life of the process.
///
/// Installed by [`init`] at boot, or filled in by the first reader — which is
/// what lets a test that never runs a `main` still render a page's chrome.
static CATALOG: OnceLock<Catalog> = OnceLock::new();

fn loaded() -> &'static Catalog {
    CATALOG.get_or_init(|| {
        let dir = locate().unwrap_or_else(|error| panic!("{error}"));
        Catalog::load(&dir).unwrap_or_else(|error| panic!("{error}"))
    })
}

/// The language's words for `key`, by the module's one lookup order.
pub fn text(lang: Lang, key: Key) -> &'static str {
    loaded().words(lang, key)
}

/// Reads the catalog at boot and says what it found.
///
/// Panics if it cannot: a server that does not know what its own chrome says
/// has nothing to serve, and a process that refuses to start is far easier to
/// notice than one that boots and prints key names at strangers. Called first
/// thing in `main`, which is why it is also the line that reports the directory
/// the words came from.
pub fn init() -> Report {
    let dir = locate().unwrap_or_else(|error| panic!("{error}"));
    let catalog = Catalog::load(&dir).unwrap_or_else(|error| panic!("{error}"));
    let report = catalog.report(&dir);

    // A test may have read the catalog already, and there is only one process.
    let _ = CATALOG.set(catalog);

    report
}

/// The directory the catalogs are read from.
///
/// [`ENV`] alone when it is set, otherwise `./locales` and then the directory
/// the running binary sits in. The first directory that *holds* `fr.toml` wins,
/// so a clone that also has a stale `target/debug/locales` keeps reading its
/// own source tree — and a directory that holds the file is read strictly, so a
/// broken copy is a boot error rather than a quiet fall-through to another one.
pub fn locate() -> Result<PathBuf, Error> {
    let tried = roots();

    first_with_french(&tried).ok_or(Error::NotFound {
        tried,
        file: file(Lang::Fr),
    })
}

/// The directories tried when [`ENV`] is not set: here, then beside the binary.
fn roots() -> Vec<PathBuf> {
    if let Some(dir) = std::env::var_os(ENV).filter(|dir| !dir.is_empty()) {
        return vec![PathBuf::from(dir)];
    }

    let mut tried = vec![PathBuf::from(DIR)];
    if let Ok(exe) = std::env::current_exe()
        && let Some(beside) = exe.parent().map(|dir| dir.join(DIR))
        && !tried.contains(&beside)
    {
        tried.push(beside);
    }

    tried
}

/// The first of `tried` that holds the French catalog.
fn first_with_french(tried: &[PathBuf]) -> Option<PathBuf> {
    tried
        .iter()
        .find(|dir| dir.join(file(Lang::Fr)).is_file())
        .cloned()
}

impl Catalog {
    /// Reads, checks and keeps the three files of `dir`.
    ///
    /// `fr.toml` and `en.toml` must exist and carry every key, because they are
    /// what the lookup falls back to; `ty.toml` may be missing or short. A file
    /// that names a key no variant owns is refused in all three.
    pub fn load(dir: &Path) -> Result<Self, Error> {
        let mut tables = Vec::with_capacity(3);

        for lang in Lang::ALL {
            let strict = lang != Lang::Ty;
            let table = read(dir, lang, strict)?;

            if strict {
                for key in Key::ALL {
                    if !table.contains_key(key.name()) {
                        return Err(Error::Missing {
                            path: dir.join(file(lang)),
                            key,
                        });
                    }
                }
            }

            tables.push(table);
        }

        Ok(Self {
            tables: tables.try_into().expect("one table per language"),
        })
    }

    /// What the boot line says about this catalog, read out of `dir`.
    pub fn report(&self, dir: &Path) -> Report {
        Report {
            dir: dir.to_path_buf(),
            keys: Key::ALL.len(),
            words: Lang::ALL.map(|lang| (lang, self.raw(lang).len())),
        }
    }

    /// What `dir` says, as it says it — no fallback applied.
    ///
    /// The fallback is [`words`](Self::words)' business; this is for a test
    /// that has to know whether a line is *there*, and about how many words a
    /// language actually carries.
    pub fn raw(&self, lang: Lang) -> &Table {
        &self.tables[slot(lang)]
    }

    /// The words for `key` in `lang`, by the lookup order the module documents.
    pub fn words(&self, lang: Lang, key: Key) -> &'static str {
        for lang in [lang, Lang::En, Lang::Fr] {
            if let Some(words) = self.raw(lang).get(key.name()) {
                return words;
            }
        }

        key.name()
    }
}

/// Where `lang`'s table sits in [`Catalog::tables`].
///
/// Derived from [`Lang::ALL`] rather than written out again, so the tables and
/// the presentation order cannot drift apart.
fn slot(lang: Lang) -> usize {
    Lang::ALL
        .iter()
        .position(|candidate| *candidate == lang)
        .expect("every language is in Lang::ALL")
}

/// Reads one language's file.
///
/// The words are leaked into `&'static str` on purpose: the catalog is read
/// once and never written, a page renders a `&'static str`, and a leak of a few
/// kilobytes for the life of the process is the smallest way to say that. The
/// alternative — a `String` per key, cloned per render — would put the catalog
/// in every page's allocation path for nothing.
fn read(dir: &Path, lang: Lang, required: bool) -> Result<Table, Error> {
    let path = dir.join(file(lang));
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !required => {
            return Ok(Table::new());
        }
        Err(error) => {
            return Err(Error::Read {
                path,
                source: error,
            });
        }
    };

    let values: BTreeMap<String, String> =
        toml::from_str(&source).map_err(|error| Error::Parse {
            path: path.clone(),
            message: error.to_string(),
        })?;

    let mut table = Table::new();
    for (name, words) in values {
        if Key::from_name(&name).is_none() {
            return Err(Error::Unknown { path, name });
        }
        table.insert(leak(name), leak(words));
    }

    Ok(table)
}

/// A string that lives as long as the process does.
fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table built from the pairs a test cares about.
    fn table(pairs: &[(Key, &'static str)]) -> Table {
        pairs
            .iter()
            .map(|(key, words)| (key.name(), *words))
            .collect()
    }

    /// The tables are indexed by the position in [`Lang::ALL`], and the
    /// presentation order is that same list — so a language added in the middle
    /// of the list moves its own table with it.
    #[test]
    fn the_tables_are_in_the_order_the_languages_are_presented_in() {
        for (index, lang) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(slot(lang), index, "{} is not at {index}", lang.code());
        }
    }

    /// The lookup order, one key at a time: the locale, then English, then
    /// French, then the key's own name — and never another language's word when
    /// the asked-for one has one.
    #[test]
    fn a_missing_word_falls_back_the_way_the_module_documents() {
        let catalog = Catalog {
            tables: [
                table(&[(Key::NavHome, "Accueil")]),
                table(&[(Key::NavSongs, "Hīmene")]),
                table(&[(Key::NavHome, "Home"), (Key::NavSongs, "Songs")]),
            ],
        };

        // The asked-for language wins wherever it has a word.
        assert_eq!(catalog.words(Lang::Fr, Key::NavHome), "Accueil");
        assert_eq!(catalog.words(Lang::Ty, Key::NavSongs), "Hīmene");
        assert_eq!(catalog.words(Lang::En, Key::NavSongs), "Songs");

        // Then English, which is what an unfinished Tahitian translation gets.
        assert_eq!(catalog.words(Lang::Ty, Key::NavHome), "Home");

        // Then French, for a key English is silent about and Tahitian has no
        // file for at all.
        let catalog = Catalog {
            tables: [
                table(&[(Key::Save, "Enregistrer")]),
                Table::new(),
                Table::new(),
            ],
        };
        assert_eq!(catalog.words(Lang::En, Key::Save), "Enregistrer");
        assert_eq!(catalog.words(Lang::Ty, Key::Save), "Enregistrer");
    }

    /// A key no file carries is served as its own name: a reader sees what to
    /// report, and the boot's completeness check for `fr` and `en` means it
    /// takes a run with no catalog at all to get here.
    #[test]
    fn a_key_no_file_carries_is_served_as_its_own_name() {
        let empty = Catalog {
            tables: [Table::new(), Table::new(), Table::new()],
        };

        assert_eq!(empty.words(Lang::Fr, Key::NavHome), "nav_home");
        assert_eq!(empty.words(Lang::En, Key::NotFoundTitle), "not_found_title");

        // Every key's name is a real answer, not a panic.
        for key in Key::ALL {
            assert_eq!(empty.words(Lang::Ty, key), key.name());
        }
    }

    /// A scratch directory under `target/`, named for the test that uses it.
    fn scratch(name: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("locales-load-test")
            .join(name);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    /// A file carrying every key, which is what `fr.toml` and `en.toml` have to
    /// be — written rather than shipped, so this test does not move when a
    /// translation does.
    fn whole(lang: Lang) -> String {
        Key::ALL
            .iter()
            .map(|key| format!("{} = \"{}-{}\"", key.name(), lang.code(), key.name()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The same file with one key's line taken out — what an unfinished
    /// translation looks like, as opposed to a typo.
    fn whole_without(lang: Lang, missing: Key) -> String {
        whole(lang)
            .lines()
            .filter(|line| !line.starts_with(&format!("{} =", missing.name())))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn a_file_missing_a_key_is_refused_except_in_tahitian() {
        let dir = scratch("missing");
        std::fs::write(dir.join("fr.toml"), whole(Lang::Fr)).unwrap();
        std::fs::write(dir.join("en.toml"), whole_without(Lang::En, Key::NavHome)).unwrap();
        std::fs::write(dir.join("ty.toml"), "nav_home = \"Fa'aea\"\n").unwrap();

        match Catalog::load(&dir) {
            Err(Error::Missing { key, path }) => {
                assert_eq!(key, Key::NavHome);
                assert!(path.ends_with("en.toml"), "{path:?}");
            }
            Ok(_) => panic!("english was allowed to drop a key"),
            Err(other) => panic!("expected a missing key, got: {other}"),
        }
    }

    #[test]
    fn a_line_no_key_owns_is_refused_rather_than_translating_nothing() {
        let dir = scratch("unknown");
        std::fs::write(dir.join("fr.toml"), whole(Lang::Fr)).unwrap();
        std::fs::write(dir.join("en.toml"), whole(Lang::En)).unwrap();
        std::fs::write(
            dir.join("ty.toml"),
            "nav_home = \"Fa'aea\"\nnot_a_key = \"tāpiri\"\n",
        )
        .unwrap();

        match Catalog::load(&dir) {
            Err(Error::Unknown { name, .. }) => assert_eq!(name, "not_a_key"),
            Ok(_) => panic!("the typo did not stop the boot"),
            Err(other) => panic!("expected an unknown key, got: {other}"),
        }
    }

    #[test]
    fn a_short_tahitian_file_loads_and_the_english_answers_for_it() {
        let dir = scratch("short");
        std::fs::write(dir.join("fr.toml"), whole(Lang::Fr)).unwrap();
        std::fs::write(dir.join("en.toml"), whole(Lang::En)).unwrap();
        std::fs::write(dir.join("ty.toml"), "nav_home = \"Fa'aea\"\n").unwrap();

        let catalog = Catalog::load(&dir).expect("a short Tahitian file is legal");

        assert_eq!(catalog.words(Lang::Ty, Key::NavHome), "Fa'aea");
        assert_eq!(catalog.words(Lang::Ty, Key::NavSongs), "en-nav_songs");
        assert_eq!(catalog.raw(Lang::Ty).len(), 1);
        assert_eq!(catalog.raw(Lang::Fr).len(), Key::ALL.len());
    }

    /// The files this repository ships are the real ones: every key is in
    /// `fr.toml` and `en.toml`, and neither file carries a line the enum does
    /// not own.
    #[test]
    fn the_shipped_french_and_english_files_carry_every_key() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(DIR);
        let catalog = Catalog::load(&dir).expect("locales/ loads");

        assert_eq!(catalog.raw(Lang::Fr).len(), Key::ALL.len());
        assert_eq!(catalog.raw(Lang::En).len(), Key::ALL.len());
        assert!(
            catalog.raw(Lang::Ty).len() <= Key::ALL.len(),
            "a Tahitian line no key owns survived the load"
        );

        for key in Key::ALL {
            assert!(!catalog.words(Lang::Fr, key).trim().is_empty());
            assert!(!catalog.words(Lang::En, key).trim().is_empty());
        }
    }

    /// The walk over the candidate directories: the first one holding `fr.toml`
    /// wins, in the order the roots were given, and none holding it is an error
    /// rather than a silent empty catalog.
    #[test]
    fn the_first_directory_holding_the_french_file_wins() {
        let empty = scratch("locate-empty");
        let full = scratch("locate-full");
        std::fs::write(full.join("fr.toml"), whole(Lang::Fr)).unwrap();
        std::fs::write(full.join("en.toml"), whole(Lang::En)).unwrap();

        assert_eq!(
            first_with_french(&[empty.clone(), full.clone()]),
            Some(full.clone())
        );
        assert_eq!(first_with_french(&[full.clone(), empty]), Some(full));
        assert_eq!(first_with_french(&[scratch("locate-none")]), None);
    }
}
