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

The one-off conversion of the existing SurrealDB data into SQLite is **step 3**
(below) and needs a `surreal export` dump from the live instance.

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
- **Reconsidered (needs a call):** OG/Twitter image generation via
  `headless_chrome` (needs Chromium **on the server** — heavy for "simple").
  Proposal: render OG cards in pure Rust (`image` + text shaping) or ship
  pre-generated ones. Flagged in §7.

---

## 3. Target layout

```
chansondufenua/
├─ Cargo.toml                 # single binary crate, v4.0.0
├─ PLAN.md                    # this file
├─ README.md  CONTRIBUTING.md # updated (no more leptos/cargo-leptos)
├─ migrations/
│  └─ 0001_init.sql           # songs, artists, song_artists (SQLite)
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
| 3 | `add: sqlite layer + schema + data import` | `migrations/0001_init.sql`, `src/db/*`, `seed` from SurrealDB dump | songs/artists round-trip; import verified on real dump |
| 4 | `add: app shell, layout and design tokens` | `src/ui/*` (layout, header/nav, footer, `class!` tokens) | pages render inside the shell; print variants present |
| 5 | `add: home page` | `/` and `/aepa` (hero, cards, latest + most-viewed tables) | parity with `HomePage` |
| 6 | `add: songs index page` | `/himene` table | parity with `AllSongPage` |
| 7 | `add: song page + metadata` | `/himene/{id}` + `<title>`, description, JSON-LD, OG/Twitter meta, `view_count` increment | parity with `SongPage` |
| 8 | `add: i18n module and fr/ty catalogs` | `src/i18n.rs` + `hreflang`/`lang`/`og:locale`, labels wired | switching locale changes chrome text |
| 9 | `add: create-song page with chord tools` | `/himene/api` form + chord editor + artist picker | parity with `CreateSongPage` |
| 10 | `add: agent-readiness (robots, sitemap, md negotiation, link headers)` | §6 checklist: `robots.txt`, sitemaps, `Link` headers, `Accept: text/markdown`, `llms.txt`, read-only JSON API | `isitagentready.com` scan improves |
| 11 | `add: og/twitter image route` | `og.rs` per §7 decision | images render |
| 12 | `update: docs, version v4.0.0, drop leptos leftovers` | README/CONTRIBUTING, `ARCHITECTURE.xml` refreshed, dead files removed | no `leptos`/`surrealdb` anywhere; tag `v4.0.0` |

> Small commits, reviewable diffs, `main` untouched. Nothing is pushed without
> the maintainer's go-ahead (see §7, credentials).

---

## 5. Data migration (step 3)

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

1. **Push access & git identity.** I cloned over HTTPS and can commit locally.
   I have no push credentials for `raonagos/chansondufenua`. Commits on this
   branch are authored as `tetuaoro <65575727+tetuaoro@users.noreply.github.com>`
   (the repo's existing author) so they match history — say the word if you'd
   rather see a distinct agent identity.
2. **SurrealDB dump** for step 3 — I can't reach the live database.
3. **"Roadmap that are checked"** — I found no checkbox roadmap in the repo, so
   I've assumed it means *do not add previously-planned features; rebuild the
   current app only*. Correct me if you meant a specific list.

**Technical risks & mitigations**

4. **Topcoat is 0.10.0, published 2026-04-17, self-described "early-stage and
   experimental — expect breaking changes."** Mitigation: pin the exact version,
   keep page code thin, isolate Topcoat-specific bits in `ui/`.
5. **Tailwind feature build** (may fetch a standalone binary). Proven or
   replaced in step 1. `cmake` is available at `workspace/.tools/cmake/bin` if a
   transitive dep needs it.
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
