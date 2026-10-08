# Contributing to Chanson du *fenua*

Bug reports, feature requests, translations and pull requests are all welcome.

## Build and run

Rust is the whole toolchain: no second target, no Node, no database server. Install
the asset CLI once (`cargo install topcoat-cli`), then:

```bash
cargo build && topcoat asset bundle && ./target/debug/chansondufenua
```

`cargo build` builds the library and the binary. `topcoat asset bundle` writes
`target/debug/assets/` — the stylesheet, the fonts and the logos — and is not
optional, because `cargo run` alone panics looking for it. The binary then serves
the site on <http://localhost:3000>.

The database is a SQLite file and does not have to exist: the first boot creates
`data/chansondufenua.db` and applies [migrations/](./migrations).
[env.example](./env.example) lists the four variables the binary reads; nothing
loads a `.env` file for you, so export them or put them on the command line.

## Tests

`cargo test` runs the library's unit tests plus the integration tests in
[tests/](./tests), which drive a real *file-backed* database under
`target/test-dbs/`. No test needs the network, a running server, or the corpus.

A change is ready when the three gates are clean:

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test
```

## Translations

The chrome's words — the navigation, the buttons, the messages, the 404 — and the
site's own prose are not in the code but in three flat `name = "words"` files in
[locales/](./locales): `fr.toml` (French, the default), `ty.toml` (Tahitian) and
`en.toml` (English). They are read **at startup, not compiled**, so correcting a
sentence is an edit and a restart, with no `cargo build` and no toolchain on the
machine that serves the site — the people who can check the Tahitian are not the
people who run the compiler.

`fr.toml` and `en.toml` must carry **every** key, because they are what a missing
translation falls back to; a missing line, or a name no key owns, stops the server
and names the file and the key. The names are `i18n::Key`'s, snake_case (see
`src/i18n.rs`): adding a key is a code change, translating one is not. `ty.toml` is
the exception — a key it does not carry is served from English, and then from
French.

The files are found in `LOCALES_DIR` if set, otherwise `./locales`, otherwise
`locales/` beside the binary. The lyrics are the content and are never translated.

## Notes

One crate, split by responsibility: `domain` (the rules, knowing nothing of SQLite
or HTTP), `db` (the pool, the migrations, the SQL), `pages` (one module per route),
`ui` (the shell, the chrome, the design tokens), `routes` (what is not a page —
`robots.txt`, the sitemaps, `llms.txt`, the JSON API, the cards).

There is **no stylesheet in the tree**: the palette and the type faces live in
`src/ui/palette.rs`, and `build.rs` renders them into Tailwind's `@theme` block.

## Sending a change

1. **Create a branch**: `git checkout -b feature/your-feature-name`
2. **Make your changes** and commit them.
3. **Push to your fork**: `git push origin feature/your-feature-name`
4. **Open a pull request** on the
   [upstream repository](https://github.com/raonagos/chansondufenua) with a clear
   title and description.

## Reporting a problem

Open an issue with a clear title, what you expected, what happened instead, and
the steps to reproduce it; add your operating system and any detail that helps.
