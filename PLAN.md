# PLAN.md — *Chanson du fenua* v4: Topcoat + SQLite rewrite

> **Status:** proposed · **Branch:** `rewrite/topcoat` · **Base:** `main` @ v3.1.3
> **Author of this plan:** alice (working for tetuaoro)
> This plan replaces the Leptos 0.7 + Axum + SurrealDB stack. `main` is left untouched.

---

## 1. Why we're doing this

Three reasons, all from the maintainer:

1. **Maintainability.** Leptos 0.7.7 means SSR *plus* WASM hydration *plus*
   `cargo-leptos` *plus* a second (wasm) build profile. That is a lot of moving
   parts for a songbook, and it is the reason the project is paused.
2. **Simplicity.** Topcoat (`tokio-rs/topcoat`) is server-rendered only: no WASM,
   no hydration, no client build step. Page components are `async` and can call
   the database directly, so a whole layer (the server functions in `api/`)
   stops existing. One small binary is the goal.
3. **Agent readiness.** The web is being read by AI agents now. We want
   `chansondufenua.pf` to score well on Cloudflare's
   *["Is Your Site Agent-Ready?"](https://isitagentready.com/)* checks:
   `robots.txt`, sitemaps, `Link` response headers, **Markdown content
   negotiation**, content signals, and protocol discovery.

Plus two explicit asks:

- **SQLite, embedded** — no SurrealDB server to run; the engine ships inside the binary.
- **Styling by `class!` composition** — no hand-written `.css`/`.scss` files.

---

## 2. Decisions (with rationale)

### 2.1 Architecture — collapse the hexagon ✅ *(agrees with the maintainer)*

**Current:** a 7-crate workspace (`api app core database domain server web`)
drawn as a hexagon in `ARCHITECTURE.xml`.

**The hexagon is not earning its keep here.** Ports-and-adapters pays off when
there are *several interchangeable adapters* to swap behind a stable core. This
app has exactly one of each:

| Layer | What it really was | Verdict |
|---|---|---|
| `domain` | `Song`, `Artist`, HTML cleaning, JSON-LD, meta tags | **keep** (as a module) |
| `database` | one `Database` trait + one SurrealDB impl | **keep the boundary, drop the crate** |
| `api` | Leptos `#[server]` functions | **delete** — Topcoat components call the DB directly |
| `core` | held `AppState { leptos_options, pool }` | **delete** — Topcoat's `Cx` app-context replaces it |
| `app` / `web` | Leptos pages + wasm hydrate entry | **delete** — becomes Topcoat pages |
| `server` | Axum router + middleware + binaries | **delete** — Topcoat's `serve` + router |
| `domain::cli` | clap args for SurrealDB creds | **delete** — SQLite needs no credentials |

**Recommendation:** one crate, five modules. Keep the *discipline* that made the
hexagon worth drawing — `domain` must not know about SQLite or HTTP — but drop
the crate ceremony. If a second adapter ever appears, extracting a crate is a
mechanical `git mv`, not a rewrite.

Two crates (a `lib` + a thin `bin`) is the only split worth considering if we
want integration tests against the lib. Default: **single crate**.

### 2.2 Database — SQLite, embedded

- **`sqlx`** with `features = ["sqlite", "runtime-tokio", "migrate"]`.
  It bundles SQLite (via `libsqlite3-sys`) so **no `libsqlite3` is needed on the
  host** and there is no server process, no namespace/database/user/password —
  exactly the "embed into the binary" goal.
- Async-native, which matters because Topcoat handlers are `async` (a blocking
  driver would need `spawn_blocking` shims everywhere).
- Migrations via `sqlx::migrate!("./migrations")`, replacing the SurrealDB
  `DEFINE TABLE/FUNCTION` script.
- Enable **WAL** + `busy_timeout`; the current app writes on read
  (`view_count += 1`), so concurrent reads/writes do happen.

> Alternative if we want the absolute minimum: `rusqlite` + `bundled` behind a
> tiny pool. Slightly less code, but sync. **Recommendation stands: `sqlx`.**

The one-off conversion of the existing SurrealDB data into SQLite is **step 3b**
(below). The schema itself does *not* need the dump — it is transcribed from
`database/migrations/surrealdb` in step 3a.

### 2.3 Styling — `class!` composition, zero hand-written CSS

- Author all styling as **`class!` composition over Tailwind utilities**,
  compiled by Topcoat's built-in **`tailwind` feature** ("Tailwind CSS without
  Node", wired into the asset pipeline). No `.scss`/`.css` is written by hand;
  `style/main.scss`, `style/tailwind.scss` and `style/editor.scss` are deleted.
- Keep a **design-token module** (`ui/theme.rs`) so colors/spacing are named
  once in Rust and composed everywhere — this is the "Rust design system" the
  current `tailwind.scss` `@theme` block gestures at.
- **Print stays.** A chord songbook gets printed. That is done with Tailwind's
  `print:` variants inside classes, not a separate print stylesheet — the point
  of the rule is *no hand-written CSS*, not *no CSS emitted*.
- Fallback if the `tailwind` feature can't build in this environment (it may
  fetch a standalone binary): vendor a minimal utility set via a build script,
  or use `topcoat-css` local modules. To be proven in step 1.

### 2.4 i18n — homegrown, because Topcoat has none

Topcoat's roadmap lists **"Localization support" as not-yet-done**. The site is
**French + Tahitian** (`og:locale ty_PF`, `fr_FR`; metadata already mixes
`Lyrics of | Paroles de | Parau hīmene nō`).

Plan: a ~100-line `i18n` module, no dependencies:

- `enum Lang { Fr, Ty }` (+ `En` later, trivially).
- A compile-time-checked message catalog (`enum Key` → `match lang`), so a
  missing translation is a **compile error**, not a runtime blank.
- Resolution order: explicit `?lang=` → `lang` cookie → `Accept-Language` →
  default `Fr`.
- Emit `<html lang>`, `<link rel="alternate" hreflang>`, and `og:locale`.
- Content (the song lyrics themselves) is *not* translated — only chrome/labels.

### 2.5 Serving & deployment

- Topcoat serves plain HTTP on a local port; **`uping` terminates TLS in front**
  (same pattern already used for ZeroClaw). This finally lets
  `chansondufenua.pf` drop nginx.
- Request/Cache handling: replace the `cached` + "206 on cache hit" middleware
  (which returns `206 Partial Content` for a *full* body — not correct) with
  proper `Cache-Control` on hashed assets plus Topcoat's `compression`.

### 2.6 What we drop / freeze

- **Dropped:** SurrealDB, Leptos, Axum, `cargo-leptos`, wasm target,
  `leptos_meta/router/axum`, `eserde` (only needed for Surreal's `RecordId`),
  `ammonia`'s inline use, the `[patch.crates-io] ring` git pin (was a
  rustls/leptos-era security patch), `headless_chrome`, `tikv-jemallocator`,
  `clap`, the `.env`/`env.example` DB credentials.
- **No new product features.** Per the maintainer: this is a rewrite, so we do
  **not** pull in previously-planned/checked roadmap items (ukulele-chords
  extras, transposition tool, etc.). Goal is **parity**.
- **Decided (2026-10-04):** OG/Twitter cards render in **pure Rust**
  (`image` + text shaping). `headless_chrome` and its server-side Chromium
  requirement are dropped — see §7 item 6.

---

## 3. Target layout

```
chansondufenua/
├─ Cargo.toml                 # single binary crate, v4.0.0
├─ build.rs                   # renders the Tailwind stylesheet at build time
├─ PLAN.md                    # this file
├─ README.md  CONTRIBUTING.md # updated (no more leptos/cargo-leptos)
├─ migrations/
│  └─ 0001_init.sql           # songs, artists, song_artists (SQLite)
├─ legacy/
│  └─ surrealdb.surql         # v3 schema kept as the source of truth for 3a
├─ assets/                    # logos (webp/ico), fonts
└─ src/
   ├─ main.rs                 # Topcoat serve + router + DB pool in app context
   ├─ domain/
   │  ├─ mod.rs
   │  ├─ song.rs              # Song + rules: clean_lyrics, jsonld, meta, markdown
   │  └─ artist.rs
   ├─ db/
   │  ├─ mod.rs               # pool, migrate, queries
   │  └─ seed.rs              # optional: import dump -> SQLite
   ├─ i18n.rs                 # Lang + catalogs
   ├─ ui/                     # reusable components + design tokens
   │  ├─ mod.rs  layout.rs  theme.rs
   ├─ pages/
   │  ├─ home.rs              # / and /aepa
   │  ├─ songs.rs             # /himene
   │  ├─ song.rs              # /himene/{id}      (SSR + markdown negotiation)
   │  └─ editor.rs            # /himene/api       (create song + chord tools)
   └─ routes/
      ├─ sitemap.rs           # /sitemap.xml, /himene/sitemap.xml
      └─ og.rs                # /drive/genog|gentw/... (see §7)
```

---

## 4. Steps — one commit each

Ordered so the app runs (minimally) as early as possible, then grows to parity.
Every step must build and (where applicable) pass tests **before** its commit.

| # | Commit (`add:`/`update:` style, matches repo) | Deliverable | Done when |
|---|---|---|---|
| 0 | `add: rewrite plan (topcoat + sqlite)` | this file, on `rewrite/topcoat` | committed |
| 1 | `add: bootstrap topcoat single-crate app` | new `Cargo.toml` + `src/main.rs`; leptos crates removed from the branch | `cargo run` serves a plain "hello" page; **tailwind feature proven or fallback chosen** |
| 2 | `add: domain entities and rules` | `src/domain/{song,artist}.rs` ported (`clean_lyrics`, `to_jsonld`, `get_meta_data`, + markdown render) with unit tests | `cargo test` green |
| 3a | `add: sqlite layer + schema` | `migrations/0001_init.sql`, `src/db/*` (pool, WAL, queries, create_song ported), `src/db/fixtures.rs` with a few representative songs | songs/artists round-trip in tests; **no dump required** |
| 3b | `add: importer for surreal dump` → then run it | `src/db/import.rs` reading the `surreal export` output | deferred until the dump arrives; **not on the critical path** (§5) |
| 4 | `add: app shell, layout and design tokens` | `src/ui/*` (layout, header/nav, footer, `class!` tokens) | pages render inside the shell; print variants present |
| 5 | `add: home page` | `/` and `/aepa` (hero, cards, latest + most-viewed tables) | parity with `HomePage` |
| 6 | `add: songs index page` | `/himene` table | parity with `AllSongPage` |
| 7 | `add: song page + metadata` | `/himene/{id}` + `<title>`, description, JSON-LD, OG/Twitter meta, `view_count` increment | parity with `SongPage` |
| 8 | `add: i18n module and fr/ty catalogs` | `src/i18n.rs` + `hreflang`/`lang`/`og:locale`, labels wired | switching locale changes chrome text |
| 9 | `add: create-song page with chord tools` | `/himene/api` form + chord editor + artist picker | parity with `CreateSongPage` |
| 10 | `add: agent-readiness (robots, sitemap, md negotiation, link headers)` | §6 checklist: `robots.txt`, sitemaps, `Link` headers, `Accept: text/markdown`, `llms.txt`, read-only JSON API | `isitagentready.com` scan improves |
| 11 | `add: og/twitter image route` | `og.rs` per §7 decision | images render |
| 12 | `update: docs, version v4.0.0, drop leptos leftovers` | README/CONTRIBUTING, `ARCHITECTURE.xml` refreshed, dead files removed | no `leptos`/`surrealdb` anywhere; tag `v4.0.0` |

> Small commits, reviewable diffs, `main` untouched. Work is pushed to
> `alice-agent-zc/chansondufenua` (the fork); nothing goes to `raonagos/*` without
> the maintainer's go-ahead (see §7 item 1 for credentials and authorship).

---

## 5. Data migration (step 3b — deferred, not blocking)

> **The dump is not on the critical path.** The schema is fully recoverable from
> `database/migrations/surrealdb` (the `DEFINE TABLE`/`DEFINE FIELD` statements
> and the seven `fn::` functions), so step 3a builds the SQLite layer *and its
> tests* against hand-written fixtures. Only the real 46 rows need the export.
> Steps 1–12 all proceed without it.

1. `surreal export --ns <ns> --db <db> --user ... --pass ... > dump.surrealql`
   (maintainer runs this on the live host — I have no credentials/access).
2. Write a small importer that reads the dump (or a JSON export) and inserts
   into SQLite: `artist`, `song`, `song_artists`, preserving ids, `created_at`,
   `updated_at`, `view_count`, `published`.
3. Verify counts (~46 songs) and spot-check a few `/himene/{id}` pages.

Schema sketch:

```sql
CREATE TABLE artist (
  id         TEXT PRIMARY KEY,
  fullname   TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE song (
  id         TEXT PRIMARY KEY,
  title      TEXT NOT NULL,
  lyrics     TEXT NOT NULL,
  view_count INTEGER NOT NULL DEFAULT 1,
  published  INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE song_artist (
  song_id   TEXT NOT NULL REFERENCES song(id)   ON DELETE CASCADE,
  artist_id TEXT NOT NULL REFERENCES artist(id) ON DELETE CASCADE,
  position  INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (song_id, artist_id)
);
```

(The current SurrealDB schema asserts `title` 4–100 chars, `lyrics` 100–6000
chars, `fullname` 4–50 chars. Replicate as `CHECK` constraints so the editor
cannot store garbage either way.)

---

## 6. Agent-readiness checklist (step 10)

Mapped to the isitagentready.com categories:

- **Discoverability**
  - `robots.txt`: keep `User-agent: *`, drop the dead `sitemap.xml.br/.gz`
    entries, keep the sitemap directives, keep `Disallow: /himene/api`.
  - `sitemap.xml` **index** → `/sitemap.xml` (home) + `/himene/sitemap.xml`
    (all songs, `lastmod` from `updated_at`).
  - `Link` response headers (RFC 8288) on HTML pages: `rel="sitemap"`,
    `rel="alternate"; type="text/markdown"`, `rel="describedby"` → JSON API.
- **Content Accessibility**
  - **Markdown content negotiation**: when `Accept: text/markdown`, serve a
    clean Markdown version of a song (title, artists, lyrics with chords).
    This is the single highest-value item for agents and is cheap — the domain
    already renders `clean_lyrics`, and Topcoat has no markdown, so we emit it
    ourselves.
  - `llms.txt` at the root: what the site is + canonical entry points.
- **Bot Access Control**
  - Explicit AI-bot rules + a **Content-Signal** policy in `robots.txt`
    (search vs. ai-input vs. ai-train) — the maintainer chooses the policy.
- **Protocol Discovery**
  - Read-only **JSON API catalog**: `GET /api/songs`, `GET /api/songs/{id}`,
    `GET /api/health` (a small, documented surface). Enables the "API Catalog"
    / MCP / Agent-Skills checks later without commitment now.
- **Commerce** — not applicable (free, non-transactional).

---

## 7. Risks & open questions

**Blocking / needs the maintainer**

1. **Push access & git identity.** ✅ **Resolved (2026-10-04).** The maintainer
   created the GitHub account **`alice-agent-zc`** and registered this agent's
   SSH key on it, as **both** an authentication key and a signing key. alice
   pushes to the maintainer-created **fork** `alice-agent-zc/chansondufenua`;
   `origin` stays `raonagos/chansondufenua`, and `main` is never pushed to
   directly. Commits on this branch are **authored and committed as
   `alice-agent-zc <337614769+alice-agent-zc@users.noreply.github.com>`** and
   **signed** (`gpg.format=ssh`, no GPG keyring needed), so GitHub reports
   `verified: true` on each one — checked against the forge API, not assumed.

   *Decided (2026-10-04, maintainer):* **alice stays the author.** The work
   should be attributable to the agent that did it — now and for future
   collaboration. This supersedes the earlier "the maintainer pushes / commit as
   `tetuaoro`" call.

   Pitfall for whoever clones next: a clone can carry **repo-local**
   `user.name` / `user.email`, which silently override the global identity.
   Check `git config --local --list` before committing, or the commits go out
   under someone else's name and unsigned.
2. **SurrealDB dump** — needed for **step 3b only** (loading the real 46 rows).
   I have no access to the live database, so the maintainer runs
   `surreal export` when convenient. Steps 1–12 use committed fixtures instead
   and are **not blocked** by this.
3. **"Roadmap that are checked"** — I found no checkbox roadmap in the repo, so
   I've assumed it means *do not add previously-planned features; rebuild the
   current app only*. Correct me if you meant a specific list.

**Technical risks & mitigations**

4. **Topcoat is 0.10.0, published 2026-04-17, self-described "early-stage and
   experimental — expect breaking changes."** Mitigation: pin the exact version,
   keep page code thin, isolate Topcoat-specific bits in `ui/`.
5. **Tailwind feature build** (may fetch a standalone binary). Proven in step 1
   (§10.4) — it downloads the pinned standalone CLI. `cmake` is now installed
   system-wide (`/usr/bin/cmake`, 3.28.3) if a transitive dep needs it; the old
   `workspace/.tools/cmake/bin` workaround is obsolete.
6. **OG image generation** currently needs Chromium on the server. Options:
   (a) pure-Rust image rendering, (b) pre-generated static cards, (c) keep
   headless Chrome (contradicts "simple"). **Recommendation: (a)**. Decide at
   step 11.
7. **`view_count` writes on every page view** — fine on SQLite/WAL, but it makes
   the page non-idempotent and uncacheable. Considered acceptable (parity);
   revisit later.
8. **i18n is homegrown** (Topcoat has none). Small and typed, but if Topcoat
   ships official localization we should migrate to it.

---

## 8. Definition of done

- `chansondufenua.pf` is served by a **single Rust binary** (Topcoat + embedded SQLite).
- Feature parity with v3.1.3: home, song list, song page (meta/JSON-LD/OG),
  create-song form with chord tools, sitemaps.
- **No hand-written CSS**, no WASM, no separate API layer, no external DB server.
- `isitagentready.com` score materially improved (robots, sitemap, Link headers,
  Markdown negotiation, `llms.txt`).
- `cargo test` green; `main` untouched; work on `rewrite/topcoat`, committed step by step.

---

## 9. Appendix — the `alice-agent-zc` account

**Created by the maintainer on 2026-10-04.** That is the correct order of
events, and the reasoning from the earlier check is worth keeping on record:

- Network reachability was fine (`github.com` → 200; `api.github.com` ok), and
  `git`/`ssh` exist — but there is **no `gh` CLI**, **no mail tool**, and no
  mailbox this agent can read.
- Account creation is blocked in practice by GitHub's **CAPTCHA** and by
  **email verification**, and it is against GitHub's Acceptable Use for an agent
  to self-register: accounts must be registered by a human. Bot accounts are
  permitted, but a human must create and own them.

**Outcome:** the maintainer created `alice-agent-zc` (name *Alice*), set the
commit identity to that account's noreply address, added this agent's SSH key as
**both an authentication key and a signing key**, and forked the repository for
the work. So: the maintainer minted the identity and the credentials; alice uses
them. Neither side could have done the other's half — which is exactly why the
split is right.

---

## 10. Appendix — verified in step 1 (2026-10-04)

Everything below was **proven by building and running**, not read off docs:

1. **Topcoat 0.10.0 builds and serves here.** `topcoat::start(module_router!().build())`,
   `#[page]`, `#[component]`, `#[layout]`, `#[page]`-derived routes — all work.
   Binary binds `127.0.0.1:3000`; override with `HOST` / `PORT`.
2. **`edition = "2024"` is required.** Topcoat's attribute macros emit code
   referencing `Future` unqualified, which only resolves via the Rust 2024
   prelude. On edition 2021 every `#[page]`/`#[component]` fails with
   `cannot find trait Future in this scope`. The crate's own tests are 2024.
3. **`class!` (and `view!`) must be imported.** They are proc macros re-exported
   from `topcoat::view`, so `use topcoat::view::{class, view}` is needed —
   `class!` is not in the prelude.
4. **The `tailwind` feature works, including the download.** `build.rs` calling
   `topcoat::tailwind::BuildConfig::new().render()` downloads the pinned
   standalone **Tailwind CLI v4.3.2** from GitHub and writes minified CSS to
   `$OUT_DIR/tailwind.css`. It scans Rust sources for *literal* classes — so
   classes must never be assembled at runtime. Network egress to GitHub
   releases is allowed from this host.
5. **`tailwind::stylesheet!()` is an asset**, so it panics at render time with
   *"no asset config registered in this router context"* unless the router
   installs `.assets(AssetBundle::load().unwrap())`. That in turn reads
   `assets/` **next to the binary**, which is produced by
   `topcoat asset bundle` — i.e. **`topcoat-cli` is a build/deploy dependency**,
   not just a dev convenience. Installed here as
   `workspace/.tools/cargo-install/bin/topcoat` (v0.10.0).
   Consequence: `cargo run` alone is not enough; the flow is
   `cargo build && topcoat asset bundle && ./target/debug/chansondufenua`.
6. **Response headers already include `vary: Accept`.** Useful — RFC 8288
   `Link` headers and the `Accept: text/markdown` negotiation in step 10 are
   the natural next move on that axis.
7. **`#[memoize]` (per-request) and the `sitemap` feature exist** and are the
   replacements for the old `cached` middleware (`server/src/cache.rs`) and the
   hand-rolled `sitemap` module respectively.
