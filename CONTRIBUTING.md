# Contributing to Chanson du *fenua*

Thank you for considering contributing to **Chanson du fenua** ! We welcome contributions from everyone. Here are some guidelines to help you get started :

## Ways to Contribute

- **Bug Reports** : If you find a bug, please report it using the issue tracker. Provide as much detail as possible to help us reproduce and fix the issue.
- **Feature Requests** : Have an idea for a new feature? Submit a feature request and describe the problem you're trying to solve.
- **Code Contributions** : We welcome pull requests! Please follow the guidelines below to ensure a smooth process.
- **Documentation** : Improvements to documentation are always appreciated. If you find any gaps or unclear sections, feel free to submit updates.

## Getting Started

1. Clone this repository : `gh repo clone raonagos/chansondufenua`.
2. Navigate to the project directory: `cd chansondufenua`.
3. Build the application: `cargo build`.

## Usage

1. Make sure you have [rust](https://www.rust-lang.org/learn/get-started) installed. That is the whole toolchain — there is no second target to install, no Node, and no database server to run.
2. Read [env.example](./env.example) for the four variables the binary knows (`DATABASE_URL`, `HOST`, `PORT`, `LOCALES_DIR`). Nothing loads a `.env` file for you; export them, or put them on the command line.
3. Bundle the assets and start the application:

```bash
cargo build && topcoat asset bundle && ./target/debug/chansondufenua
```

`topcoat asset bundle` is not optional: it writes `target/debug/assets/`, where the binary finds the stylesheet, the fonts and the logos. `cargo run` alone panics looking for it. Install the CLI once with `cargo install topcoat-cli`.

The database is a SQLite file. It does not have to exist — the first boot creates `data/chansondufenua.db`, applies [migrations/](./migrations), and serves an empty songbook until you add something.

Those three commands produce and run the whole site. Pages, stylesheet, fonts, logos and the database engine are compiled into the one executable; there is no client-side runtime, no separate database server and no asset pipeline beyond staging the bundled files next to the binary.

4. Open your browser and go to the URL: `http://localhost:3000`.

## Translations

The words in the chrome — the navigation, the buttons, the messages, the 404 — are not in the code. They are three files, one per language, in [locales/](./locales):

```
locales/fr.toml   French, the site's default
locales/ty.toml   Tahitian
locales/en.toml   English
```

Each file is a flat list of `name = "words"` lines, and a `#` starts a comment:

```toml
nav_home = "Accueil"
not_found_title = "La page n'existe pas."
```

They are read **at startup, not compiled**: correcting a sentence is an edit and a restart, with no `cargo build` and no toolchain on the machine that serves the site. That is deliberate — the people who can check the Tahitian are not the people who run the compiler.

Two rules the boot enforces:

- `fr.toml` and `en.toml` must carry **every** key, because they are what a missing translation falls back to. A line missing from either one stops the server, with the file and the key named.
- A name no key owns stops the server too. The names are `i18n::Key`'s, snake_case (see `src/i18n.rs`); listing one here does not put it on a page, the enum has to name it as well. Adding a key is a code change, translating one is not.

`ty.toml` is the exception: an unfinished Tahitian translation is its normal state, so a key it does not carry is served from English, and then from French. Delete a line, restart, and that one string changes.

Where the files are found: `LOCALES_DIR` if it is set, otherwise `./locales`, otherwise `locales/` beside the binary. A deployment can therefore ship the directory next to the executable and be edited in place.

The song lyrics are the content and are never translated — the catalog is only the chrome around them.

## Agent readiness

The web is read by more than browsers now, and the site is built to be legible to the ones that do not run JavaScript. The clearest example is the song list. Version 3 rendered each row as a click handler — the link had no destination:

```html
<tr class="tab-row" role="button" tabindex="0" onclick="window.location='/himene/7114wvk91gffr2bj6wza'">
  <td class="tab-cell"><a href class="tab-link">Āhani e</a></td>
```

The 3.x table was also filled in by the browser, so a client without a JavaScript engine read *"Chargement…"* and no songs at all. Now the same page arrives complete, with real destinations:

```html
<tr>
  <td><a href="/himene/7114wvk91gffr2bj6wza">Āhani e</a></td>
```

Every song is followable, keyboard-reachable, middle-clickable and copyable. Alongside that, the site serves `robots.txt`, two XML sitemaps, an `llms.txt`, RFC 8288 `Link` headers, a Markdown version of any song when asked (`Accept: text/markdown`), and a small read-only JSON API — so a song can be read as a page, as Markdown, or as JSON, and every form is discoverable from the others.

## Contribute

1. **Create a new branch** :
```bash
git checkout -b feature/your-feature-name
```
2. **Make Your Changes** : Implement your feature or bug fix. Make sure to follow the coding standards and write tests if applicable.
3. **Commit your changes** :
```bash
git add .
git commit -m "Describe your changes"
```
4. **Push your fork** :
```bash
git push origin feature/your-feature-name
```
5. **Open a PR** : Go to the [upstream repository](https://github.com/raonagos/chansondufenua) and open a pull request with a clear title and description or via cli :
```bash
gh pr create
```

## Code Style and Standards

- Follow the existing code style and conventions.
- Write clear and concise commit messages.
- Include tests for new features and bug fixes.
- Ensure your code passes all existing tests.

## Reporting Issues

- Provide a clear and descriptive title.
- Describe the expected behavior and the actual behavior.
- Include steps to reproduce the issue.
- Mention your operating system and any other relevant details.

## Notes

The project is **one crate** — a library target (so the integration tests in [tests/](./tests) can drive the real database code) and a binary target, not a workspace. Inside it, the code is split by responsibility:

- `domain` : `Song`, `Artist` and the rules around them. It knows nothing about SQLite or HTTP, which is what keeps the rest of the modules swappable.
- `db` : the connection pool, the migrations, and every SQL statement the application runs.
- `pages` : one module per route — the home page, the song list, a song sheet, the create-song form.
- `ui` : the shell, the chrome, and the design tokens the pages compose.
- `routes` : the things that are not pages — `robots.txt`, the sitemaps, `llms.txt`, the read-only JSON API, the social cards.
- `i18n` : the languages, the addresses, and the lookup order for the chrome — the words themselves are the files in [locales/](./locales), one per language, read at startup (see [Translations](#translations)).

[ARCHITECTURE.xml](./ARCHITECTURE.xml) draws how these fit together.

There is **no stylesheet in the tree**. The palette and the type faces live in `src/ui/palette.rs`, and `build.rs` renders them into Tailwind's `@theme` block at build time, so a token is changed in Rust and nowhere else.
