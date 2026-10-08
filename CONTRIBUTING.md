# Contributing to Chanson du *fenua*

Thank you for considering contributing to **Chanson du fenua** ! Bug reports,
feature requests, translations and pull requests are all welcome.

## Build and run

[Rust](https://www.rust-lang.org/learn/get-started) is the whole toolchain: there
is no second target to install, no Node, and no database server to run. Install
the asset CLI once — `cargo install topcoat-cli` — then:

```bash
cargo build && topcoat asset bundle && ./target/debug/chansondufenua
```

Three commands, and the whole site runs:

- `cargo build` builds the library and the binary.
- `topcoat asset bundle` writes `target/debug/assets/`, where the binary finds the
  stylesheet, the fonts and the logos. It is not optional: `cargo run` alone
  panics looking for it.
- `./target/debug/chansondufenua` serves the site on <http://localhost:3000>.

The database is a SQLite file and does not have to exist: the first boot creates
`data/chansondufenua.db`, applies [migrations/](./migrations) and serves an empty
songbook until you add something. [env.example](./env.example) lists the four
variables the binary knows (`DATABASE_URL`, `HOST`, `PORT`, `LOCALES_DIR`).
Nothing loads a `.env` file for you: export them, or put them on the command line.

## Tests

```bash
cargo test
```

The suite is the library's unit tests plus the integration tests in
[tests/](./tests), which drive a real *file-backed* database under
`target/test-dbs/` — an in-memory database cannot tell you whether WAL mode took
or whether foreign keys are really enforced. No test needs the network, a running
server, or the imported corpus.

A change is ready when the three gates are clean:

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test
```

## Translations

The words in the chrome — the navigation, the buttons, the messages, the 404 — are
not in the code. They are three files, one per language, in [locales/](./locales):

```
locales/fr.toml   French, the site's default
locales/ty.toml   Tahitian
locales/en.toml   English
```

Each file is a flat list of `name = "words"` lines, and a `#` starts a comment:
`nav_home = "Accueil"`, `not_found_title = "La page n'existe pas."`

They are read **at startup, not compiled**: correcting a sentence is an edit and a
restart, with no `cargo build` and no toolchain on the machine that serves the
site. That is deliberate — the people who can check the Tahitian are not the
people who run the compiler.

Two rules the boot enforces:

- `fr.toml` and `en.toml` must carry **every** key, because they are what a missing
  translation falls back to. A line missing from either one stops the server, with
  the file and the key named.
- A name no key owns stops the server too. The names are `i18n::Key`'s, snake_case
  (see `src/i18n.rs`); listing one here does not put it on a page, the enum has to
  name it as well. Adding a key is a code change, translating one is not.

`ty.toml` is the exception: an unfinished Tahitian translation is its normal state,
so a key it does not carry is served from English, and then from French. Delete a
line, restart, and that one string changes.

Where the files are found: `LOCALES_DIR` if it is set, otherwise `./locales`,
otherwise `locales/` beside the binary. A deployment can therefore ship the
directory next to the executable and be edited in place.

The song lyrics are the content and are never translated. The catalog holds the
chrome — the labels around the content — and the site's own prose: the front
page's sentences, the footer's, and the description each page hands a search
engine. That prose is written once in French and once in English; a key the
Tahitian file does not carry is served from English.

## Code layout

The project is **one crate** — a library target (so the integration tests in
[tests/](./tests) can drive the real database code) and a binary target, not a
workspace. Inside it, the code is split by responsibility:

- `domain` : `Song`, `Artist` and the rules around them. It knows nothing about
  SQLite or HTTP, which is what keeps the rest of the modules swappable.
- `db` : the connection pool, the migrations, and every SQL statement the
  application runs.
- `pages` : one module per route — the home page, the song list, a song sheet, the
  create-song form.
- `ui` : the shell, the chrome, and the design tokens the pages compose.
- `routes` : the things that are not pages — `robots.txt`, the sitemaps,
  `llms.txt`, the read-only JSON API under `/api`, the social cards, the Markdown
  twin of a page (`Accept: text/markdown`) and the RFC 8288 `Link` headers that
  join the three forms of one song.
- `i18n` : the languages, the addresses, and the lookup order for the chrome — the
  words themselves are the files in [locales/](./locales), one per language, read
  at startup (see [Translations](#translations)).

[ARCHITECTURE.xml](./ARCHITECTURE.xml) draws how these fit together.

There is **no stylesheet in the tree**. The palette and the type faces live in
`src/ui/palette.rs`, and `build.rs` renders them into Tailwind's `@theme` block at
build time, so a token is changed in Rust and nowhere else.

## Sending a change

1. **Create a branch**: `git checkout -b feature/your-feature-name`
2. **Make your changes** and commit them: `git commit -m "Describe your changes"`
3. **Push to your fork**: `git push origin feature/your-feature-name`
4. **Open a pull request** on the
   [upstream repository](https://github.com/raonagos/chansondufenua) — or with
   `gh pr create` — with a clear title and description.

## Reporting a problem

Open an issue with a clear title, what you expected, what happened instead, and
the steps to reproduce it; add your operating system and any detail that helps.
