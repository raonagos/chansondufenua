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
2. Read [env.example](./env.example) for the two variables the binary knows (`DATABASE_URL`, `HOST`, `PORT`). Nothing loads a `.env` file for you; export them, or put them on the command line.
3. Bundle the assets and start the application:

```bash
cargo build && topcoat asset bundle && ./target/debug/chansondufenua
```

`topcoat asset bundle` is not optional: it writes `target/debug/assets/`, where the binary finds the stylesheet, the fonts and the logos. `cargo run` alone panics looking for it. Install the CLI once with `cargo install topcoat-cli`.

The database is a SQLite file. It does not have to exist — the first boot creates `data/chansondufenua.db`, applies [migrations/](./migrations), and serves an empty songbook until you add something.

4. Open your browser and go to the URL: `http://localhost:3000`.

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
- `i18n` : the French and Tahitian catalogs, checked at compile time.

[ARCHITECTURE.xml](./ARCHITECTURE.xml) draws how these fit together.

There is **no stylesheet in the tree**. The palette and the type faces live in `src/ui/palette.rs`, and `build.rs` renders them into Tailwind's `@theme` block at build time, so a token is changed in Rust and nowhere else.
