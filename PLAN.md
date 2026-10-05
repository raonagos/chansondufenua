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
- **No `.css` file in the tree — not even a token file.** Tailwind v4 reads
  `@theme` only from CSS, and `BuildConfig` exposes exactly one knob for
  supplying it, `input(path)` (verified against `topcoat-tailwind` 0.10.0's
  `src/build/config.rs`, not guessed). So `build.rs` **generates** the input
  stylesheet into `$OUT_DIR/tailwind-input.css` from `src/ui/palette.rs`, which
  it pulls in with `include!`. One palette, two readers: the crate compiles it
  as a module, the build script as its own opening items.
- That generated file lands in `$OUT_DIR` — inside `target/`, gitignored — which
  is precisely where `topcoat-tailwind` writes its own default input when
  `input` is unset. Nothing about the build differs from the default path except
  which CSS text is fed in, and that text is derived from Rust.
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
  the `[patch.crates-io] ring` git pin (was a
  rustls/leptos-era security patch), `headless_chrome`, `tikv-jemallocator`,
  `clap`, the `.env`/`env.example` DB credentials.
- **Kept, contrary to the first draft of this plan: `ammonia`.** The draft listed
  "ammonia's inline use" as dropped, on the theory that `clean_lyrics` only
  needed a text extractor. Step 2 showed why that was wrong: v3 rendered the
  **raw** `lyrics` column into the page (`MetaSongData.song_lyrics =
  self.get_lyrics()`), and the "add the lyrics" form is open to anyone. The
  sanitiser is load-bearing, not decoration — `Song::lyrics_html()` is now the
  only route from the column to a template.
- **No new product features.** Per the maintainer: this is a rewrite, so we do
  **not** pull in previously-planned/checked roadmap items (ukulele-chords
  extras, transposition tool, etc.). Goal is **parity**.
- **Decided (2026-10-04):** OG/Twitter cards render in **pure Rust**
  (`image` + text shaping). `headless_chrome` and its server-side Chromium
  requirement are dropped — see §7 item 6.
- **No authentication, and no auth tables** (maintainer, 2026-10-04). v3 carried a
  `user` table (135 rows of argon2id hashes) and a `DEFINE ACCESS account` JWT
  signing key. v4 has **no** `user`, `session`, `token`, `access`, `role` or
  `credential` table, and none is planned: the only write surface is the
  anonymous create-song form, exactly as v3's public pages already behave, and
  the admin path stays "the maintainer edits the database". The importer therefore
  *counts and skips* those rows, and a test asserts `sqlite_master` holds no
  `user` table after a full import of the real dump. The JWT key in the dump is
  consequently dead weight — see §12.4.

---

## 3. Target layout

```
chansondufenua/
├─ Cargo.toml                 # single crate, v4.0.0 — lib + bin targets
├─ build.rs                   # renders the Tailwind stylesheet at build time
├─ PLAN.md                    # this file
├─ README.md  CONTRIBUTING.md # updated (no more leptos/cargo-leptos)
├─ migrations/
│  └─ 0001_init.sql           # artist, song, song_artist, artist_fts (SQLite)
├─ legacy/
│  └─ surrealdb.surql         # v3 schema kept as the source of truth for 3a
├─ tests/
│  └─ sqlite_file.rs          # integration tests on a real file-backed database
├─ data/                      # the SQLite file lives here (gitignored)
├─ assets/                    # logos (webp/ico), fonts
└─ src/
   ├─ main.rs                 # Topcoat serve + router + DB pool in app context
   ├─ lib.rs                  # `pub mod db; pub mod domain;` — so tests/ and
   │                          #   src/bin/import.rs can reuse the real code paths
   ├─ bin/
   │  └─ import.rs            # step 3b: one-shot SurrealQL → SQLite
   ├─ domain/
   │  ├─ mod.rs
   │  ├─ song.rs              # Song + rules: clean_lyrics, jsonld, meta, markdown
   │  └─ artist.rs
   ├─ db/
   │  ├─ mod.rs               # pool, WAL, migrations, DbError
   │  ├─ queries.rs           # every SQL statement in the app
   │  ├─ fixtures.rs          # four real songs from the v3 dump
   │  └─ import.rs            # step 3b
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

> **Added in 3a: `src/lib.rs`.** Two reasons, both concrete: `tests/` can only
> reach the database layer through a library target, and the 3b importer needs
> the same `db` code the server uses. Still **one crate** — the lib/bin split is
> a target, not a workspace. §2.1's "two crates is the only split worth
> considering" anticipated exactly this; this is the binary half of it.

---

## 4. Steps — one commit each

Ordered so the app runs (minimally) as early as possible, then grows to parity.
Every step must build and (where applicable) pass tests **before** its commit.

| # | Commit (`add:`/`update:` style, matches repo) | Deliverable | Done when |
|---|---|---|---|
| 0 ✅ | `add: rewrite plan (topcoat + sqlite)` | this file, on `rewrite/topcoat` | committed |
| 1 ✅ | `add: bootstrap topcoat single-crate app` | new `Cargo.toml` + `src/main.rs`; leptos crates removed from the branch | `cargo run` serves a plain "hello" page; **tailwind feature proven or fallback chosen** |
| 2 ✅ | `add: domain entities and rules` | `src/domain/{song,artist}.rs` ported (`clean_lyrics`, `to_jsonld`, `get_meta_data`, + markdown render) with unit tests | `cargo test` green |
| 3a ✅ | `add: sqlite layer + schema` | `migrations/0001_init.sql`, `src/db/*` (pool, WAL, queries, create_song ported), `src/db/fixtures.rs` — **four real songs**, not invented ones; `src/lib.rs` added so `tests/` can drive a file-backed database | 52 tests green (46 lib + 6 file-backed), clippy clean, binary boots and creates the schema |
| 3b ✅ | `add: surreal export importer (step 3b)` | `src/db/import.rs` (SurrealQL parser + transactional load) + `examples/import.rs` CLI | importer written and **proven against the real dump** (34 artists / 43 songs / 42 credits, byte-for-byte round-trip, order matches live); *running* it on real data is held for the v4 release — `11226ca` |
| 4 ✅ | `add: app shell, layout and design tokens` | `src/ui/*` (layout, header/nav, footer, `class!` tokens) | pages render inside the shell; print variants present |
| 5 ✅ | `add: home page` | `/` and `/aepa` (hero, cards, latest + most-viewed tables) | parity with `HomePage` |
| 6 ✅ | `add: songs index page` | `/himene` table | parity with `AllSongPage` |
| 7 ✅ | `add: song page + metadata` | `/himene/{id}` + `<title>`, description, JSON-LD, OG/Twitter meta, `view_count` increment | parity with `SongPage` |
| 8 ✅ | `add: i18n module and fr/ty catalogs` | `src/i18n.rs` + `hreflang`/`lang`/`og:locale`, labels wired | switching locale changes chrome text |
| 9 | `add: create-song page with chord tools` | `/himene/api` form + chord editor + artist picker | parity with `CreateSongPage` |
| 10 | `add: agent-readiness (robots, sitemap, md negotiation, link headers)` | §6 checklist: `robots.txt`, sitemaps, `Link` headers, `Accept: text/markdown`, `llms.txt`, read-only JSON API | `isitagentready.com` scan improves |
| 11 | `add: og/twitter image route` | `og.rs` per §7 decision | images render |
| 12 | `update: docs, version v4.0.0, drop leptos leftovers` | README/CONTRIBUTING, `ARCHITECTURE.xml` refreshed, dead files removed | no `leptos`/`surrealdb` anywhere; tag `v4.0.0` |

> Small commits, reviewable diffs, `main` untouched. Work is pushed to
> `alice-agent-zc/chansondufenua` (the fork); nothing goes to `raonagos/*` without
> the maintainer's go-ahead (see §7 item 1 for credentials and authorship).

---

## 5. Data migration (step 3b)

> **Update 2026-10-04 — the dump arrived, and it changes the picture favourably.**
> `chansondufenua_backup_22032025.db` (105 KB, sha256 `1a8ad5b7…0fc2b`) is *not* a
> SQLite file despite the extension. It is a **SurrealQL export from v3**, and it
> is a direct ancestor of the live database:
>
> * **43 of the 46 live songs are in it, with identical record ids.** All 43 ids
>   in the dump exist live; none were deleted. The 3 live-only songs were added
>   after 22 March 2025.
> * **All 43 songs are byte-for-byte identical to what the site serves today.**
>   Each live page's `lyrics-display` block was extracted and compared raw against
>   the dump's `lyrics` column: 43/43 identical, 0 mismatches. So importing the
>   dump cannot regress a single character of content.
> * It also carries the parts of v3 that are **not** web content: the `DEFINE
>   ACCESS account` JWT signing key, and 135 `user` rows holding argon2id hashes.
>   Neither is imported, and the file must never be committed (§12.4).

> **Update 2026-10-04 (maintainer): the import runs at the v4 release, not now.**
> The code is finished and proven (`11226ca`); what is deferred is *executing* it.
> Reasons: (a) the rewrite must not be gated on data — every step from 4 onward
> works from committed fixtures; (b) a release-time import can take a **fresh**
> export, which closes the 3-song gap exactly rather than approximately; and (c)
> the dump is more useful kept intact as the specification of the data shape than
> consumed now. The dump stays in `.run/inbox/` (gitignored) as that reference.

So the import is 4 % incomplete (3 songs) whenever it does run. Two ways to close
the gap, in preference order:

1. **A fresh export at release time** — now the default, since it costs one
   command and closes the gap exactly rather than approximately.
2. **Scrape the 3 missing songs** from the live pages (which is where the other
   43 were just validated against anyway).

Either way it is a top-up, not a blocker: the schema, the id scheme and the
credit model are all confirmed against real data.

The mechanism, end to end:

1. The exporter is `surreal export`, which emits exactly the format now sitting
   in `.run/inbox/`: `DEFINE …` statements, then one `INSERT [ {…}, {…} ]` line
   per table.
2. `src/db/import.rs` reads that and inserts into SQLite, preserving `id`,
   `created_at`, `updated_at`, `view_count`, `published` and the **order** of
   each song's credit list. Timestamps are kept at nanosecond precision.
   `examples/import.rs` is the CLI: `cargo run --example import -- <dump>`.
3. Verify: counts (43 songs / 34 artists), id-for-id equality with the live
   sitemap, and the byte-for-byte lyrics comparison above.

Schema as built (`migrations/0001_init.sql`):

```sql
CREATE TABLE artist (
  id         TEXT PRIMARY KEY,
  fullname   TEXT NOT NULL CHECK (length(fullname) BETWEEN 4 AND 50),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE song (
  id         TEXT PRIMARY KEY,
  title      TEXT NOT NULL CHECK (length(title) BETWEEN 4 AND 100),
  lyrics     TEXT NOT NULL CHECK (length(lyrics) BETWEEN 100 AND 6000),
  view_count INTEGER NOT NULL DEFAULT 1 CHECK (view_count > 0),
  published  INTEGER NOT NULL DEFAULT 1 CHECK (published IN (0,1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE song_artist (
  song_id   TEXT NOT NULL REFERENCES song(id)   ON DELETE CASCADE,
  artist_id TEXT NOT NULL REFERENCES artist(id) ON DELETE RESTRICT,
  position  INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (song_id, artist_id)
);
```

(The v3 schema asserts `title` 4–100 chars, `lyrics` 100–6000, `fullname` 4–50.
Replicated as `CHECK` constraints so the editor cannot store garbage either way.
SQLite's `length()` counts characters, matching SurrealDB's `string::len()` — a
byte-based check would miscount every title containing ā, ', ē or ō.)

> **Deviation from the first draft of this plan:** `song_artist.artist_id` is
> `ON DELETE RESTRICT`, not `CASCADE`. Cascading would let a single
> `DELETE FROM artist` silently strip a credit off every song that named them —
> changing song pages without anyone editing a song. Restricting forces the
> deletion to be explicit about the credits first. The `song_id` side *is*
> `CASCADE`: deleting a song should take its credit rows with it.

## 5b. What the dump revealed that we did not know

Three things worth recording, because each one is an argument for the rewrite:

1. **Chords are marked up inconsistently in production.** Some songs use
   `<sup data-nosnippet="">F</sup>`; others use `<sup data-nosnippet>F</sup>`
   with a bare attribute. Both are live, in the same database. Any sanitiser
   that whitelists only one spelling silently mangles the other, which is
   exactly why `lyrics_html()` names `data-nosnippet` as a *generic* attribute
   rather than matching a literal string.
2. **`/himene` is client-side rendered.** The server sends
   `<!--s-1-o--><tr><td>Chargement...</td></tr><!--s-1-c-->` and the real rows
   arrive in a `<template>` that hydration clones in. An agent — or anything
   without a JS engine — fetching that page sees *"Chargement…"* and no songs
   at all.
3. **The song links are not links.** Rows are
   `<a href class="tab-link">Title</a>` — an `href` with no value, plus an
   `onclick="window.location=…"`. Crawlers cannot follow them. This is the
   single clearest example of the problem step 10 exists to fix, and it is worth
   demonstrating with a before/after in the README.

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
- **Real links, not JS navigation.** Every song row renders
  `<a href="/himene/{id}">Title</a>` **server-side**. v3 emitted
  `<a href class="tab-link">` — an `href` with no value — plus
  `onclick="window.location=…"`. That element is not a link: nothing without a
  JavaScript engine can follow it, and it is also unreachable by keyboard,
  un-middle-clickable and un-copyable. The `href` is what makes the page a
  document rather than an app. `data-nosnippet` on the chord `<sup>`s is kept for
  the reason the maintainer gave: Google prints page text in snippets, and a chord
  sitting over a syllable reads as a typo there. Chords belong in the song, not in
  the search result — the metas are already chord-free via `clean_lyrics`.
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
2. **SurrealDB dump** — ✅ **Received (2026-10-04)** and analysed (§5, §5b). The
   importer is written and proven against it (`11226ca`). Loading it into the
   shipped database is **held for the v4 release** by maintainer decision (§5);
   steps 4–12 work from committed fixtures and are **not blocked**.
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

---

## 11. Appendix — verified in step 2 (2026-10-04)

The domain layer is ported: **23 unit tests green**, `cargo clippy --all-targets`
clean, and the app still boots (`GET /` → 200; Tailwind asset → 200 `text/css`).
What the step established, and what it changed about the plan:

1. **Parity is now tested against the live site, not eyeballed.** Real markup was
   pulled from `https://www.chansondufenua.pf/himene/7114wvk91gffr2bj6wza`
   ("'Āhani e", 2B Brothers Tahiti) and its real `og:description` used as an
   oracle. `clean_lyrics_matches_live_output` asserts byte-for-byte equality with
   what the v3 site serves today. `clean_lyrics` is therefore frozen as a *parity*
   function, quirks included.
2. **The real lyrics markup** — worth writing down, it drove three decisions:
   lines are `<div>`, blank lines are `<div><br></div>`, and chords are
   `<sup data-nosnippet="true">C</sup>` placed **inline inside words**:
   `Hina'a<sup>Eb</sup>ro` is one word whose chord falls on the "ro". Chord
   removal must therefore *rejoin*, not separate — which is exactly why v3's
   regex order (`<sup.*?</sup>` before `<.*?>`) matters.
3. **ammonia is kept** (see §2.6), with two adjustments:
   - its defaults allow `div`, `br`, `sup`, `p`, but strip every `data-*`
     attribute, so `data-nosnippet` was added to `generic_attributes`. Losing it
     would have silently changed how chords appear in search snippets.
   - because that attribute now survives sanitising, v3's literal `<sup>` pattern
     became `<sup.*?</sup>`. For attribute-free input the two are identical.
4. **A v3 bug found and deliberately *not* fixed.** The `&.*?;` → `" "` rule runs
   after `<.*?>` → `", "`, so named entities are deleted rather than decoded:
   `Line &amp; more` becomes `Line more` in `og:description` and in the JSON-LD
   `text`. Pinned by `clean_lyrics_drops_named_entities_v3_parity` so it is a
   recorded behaviour rather than a later surprise. Fixing it is a behaviour
   change and belongs in its own step (8 or 10), for the maintainer to decide.
5. **`lyrics_markdown()` is new, and lossless.** Chords stay where the author put
   them, rendered `[Eb]` at the same offset; `<div><br></div>` becomes a blank
   line. This is the payload for the `Accept: text/markdown` negotiation in
   step 10 — the cheapest high-value agent-readiness item on the list.
6. **Validation moved into the domain.** v3 expressed the bounds only as SurrealDB
   `ASSERT` clauses. They are now `Song::validate()` / `Artist::validate_fullname()`
   too, so bad input is rejected *before* a write; the SQL `CHECK` constraints in
   step 3a remain the backstop. Same numbers: title 4..=100, lyrics 100..=6000,
   fullname 4..=50, `view_count > 0`, artists ≤ 75.
7. **Owned-`String` getters kept, `eserde` dropped.** `Song`/`Artist` keep v3's
   getter names and signatures, but `eserde` (needed only to read Surreal's
   `RecordId`) is replaced by plain `serde`. Two `//todo` assertions in v3's
   artist tests — impossible then — are real assertions now.
8. **Build/run flow, for the record:** `cargo build && topcoat asset bundle &&
   ./target/debug/chansondufenua`. `cargo run` alone still panics (§10.5). A
   local `.run/smoke.sh` does the three steps and curls the result; `.run/` is
   gitignored.

---

## 12. Appendix — verified in step 3a (2026-10-04)

The SQLite layer, built against the real dump rather than guesses.

1. **`id` is the v3 record key, verbatim.** Every song's URL is
   `/himene/{id}`, so re-minting ids would have quietly broken every inbound
   link and reset the sitemap history. The dump's ids are 20 chars of `[0-9a-z]`;
   new ids are made with SQLite's own `lower(hex(randomblob(10)))`, which is the
   same shape and costs zero dependencies (no RNG crate).

2. **Timestamps are RFC 3339 in UTC, at nanosecond precision, stored as TEXT.**
   v3 wrote values like `2025-03-09T20:09:12.123456789Z`. Truncating to
   milliseconds on import would have made every migrated row differ from live.
   TEXT also sorts lexicographically in chronological order, so
   `ORDER BY created_at DESC` needs no conversion.

3. **Ordering was measured, not assumed.** `/himene` lists songs by
   `created_at DESC` — confirmed by extracting all 46 rows from the live page's
   hydration template and checking the sequence against the dump's timestamps
   (43/43 in exactly that order). The `ORDER BY` clauses carry an `s.id`
   tiebreaker: without it, two songs sharing a `created_at` interleave their
   credit rows and the row-folding reader emits half-songs.

4. **The dump is now git-ignored, and it has to be.** It contains a
   `DEFINE ACCESS account … WITH JWT ALGORITHM HS512 KEY '<secret>'` — the
   production JWT signing key — plus 135 `user` rows with argon2id hashes.
   `.run/` was already ignored; the importer reads from there and never stages
   the file. **If that access definition is still live, the key in this file
   should be rotated**: it has now been copied off the host, and the export
   format gives no way to tell whether the definition is still in force.

5. **Artist search uses FTS5, not `LIKE`.** v3 indexed `artist.fullname` with a
   `punct_lower_ascii` SEARCH ANALYZER (PUNCT tokenizer; LOWERCASE + ASCII
   filters). The SQLite analogue is `unicode61 remove_diacritics 2`, so
   `barthelemy` finds `Barthélémy` and `theo` finds `Théo Sulpice` — both real
   names in the data, both pinned by tests. Two honest limits are recorded as
   tests rather than hidden: (a) punctuation splits tokens, so `T'Angelo` is
   indexed as `t` + `angelo` and `tangelo` does **not** match — identical to
   v3's PUNCT tokenizer, so not a regression; (b) the search needle is quoted
   before it reaches FTS5, so `*`, `NEAR(` and `"` are treated as text.

6. **`create_song` is atomic.** v3's `fn::create_song` created artists and then
   the song as separate statements; a failure in between left orphan artists.
   Here it is one transaction, and validation happens *before* the transaction
   opens. `a_rejected_song_leaves_nothing_behind` asserts exactly that: a
   two-character title plus a brand-new artist name leaves the artist count
   unchanged. Credit order is preserved (`position`), and duplicate names in the
   form field are collapsed in order rather than tripping the primary key.

7. **`src/lib.rs` was added.** `tests/` cannot reach a binary-only crate, and
   the 3b importer needs the same `db` code as the server. This is a target
   split inside one crate, not a return to the seven-crate workspace (§3).

8. **Fixtures are real songs, not invented ones.** Four rows copied verbatim
   from the dump, chosen to cover the awkward cases: one with no credited artist
   (5 of the 43 are like this), one with two artists where credit order shows on
   the page, one with chords *inside* words, and the shortest lyric set. A
   synthetic fixture would have been shorter to write and would have missed all
   four.

9. **Numbers:** 52 tests (46 lib + 6 file-backed), `cargo clippy --all-targets`
   clean, `cargo fmt --check` clean. The binary boots, creates
   `data/chansondufenua.db`, reports WAL mode, and applies migration 1 —
   including the FTS5 shadow tables.

---

## 13. Appendix — verified in step 3b (2026-10-04)

1. **A second binary breaks `topcoat asset bundle`, and `default-run` does not
   fix it.** It scans `target/debug/` for the binary to read assets out of; with
   `src/bin/import.rs` present it refuses to guess:

   ```text
   cargo produced multiple targets; pass --bin or --package to choose one,
   or set `[package] default-run` in Cargo.toml
   ```

   Setting `default-run = "chansondufenua"` — the fix it suggests — was tried and
   **ignored** by topcoat-cli 0.10.0. The importer therefore lives in
   **`examples/import.rs`**, which builds into `target/debug/examples/`, so the
   deploy path stays `cargo build && topcoat asset bundle && ./target/debug/chansondufenua`.
   Invoke it with `cargo run --example import -- <dump>`. Anything that adds a
   `src/bin/*.rs` to this crate will break the deploy step until it declares its
   binary explicitly — keep dev-only binaries in `examples/`.

2. **Tailwind scans every source file, including `examples/` and doc comments.**
   Adding the importer changed the emitted stylesheet's hash and size
   (`tailwind-3e1ec238c97a489a.css`, 8949 B, from 8887 B). Class-like words in
   prose or in a doc comment can end up in the bundle. Nothing broke, but it means
   the CSS hash is not a stable function of the pages alone — do not treat a hash
   change as evidence that a page changed.

3. **A clippy suggestion was wrong about lifetimes, and one was right.** The lint
   flagged `table_of<'a>(fields: &'a [..]) -> Result<&'a str, _>` (one input
   lifetime — genuinely elidable, fixed) and I misread it as pointing at
   `field<'a>` (two input lifetimes — *not* elidable; "fixing" it fails with
   `E0106`). Read the span before applying a lint suggestion across several
   similar functions.

4. **The one test failure in this step was the test, not the code.**
   `a_missing_field_is_reported_not_defaulted` omitted both `title` and `artists`
   from its fixture; because `song_from` reads `artists` first, the error named
   `artists`. A real export always carries the key (one of the 43 songs has
   `artists: []`), so the strictness is correct and the fixture was simply
   unrealistic. Worth recording because "the failing assertion is the wrong one"
   is the common case, and the temptation is to loosen the code instead.

5. **Numbers:** 61 tests (55 lib + 6 file-backed), clippy and fmt clean, binary
   boots and serves `GET /` → 200 with the Tailwind asset, schema created with
   exactly `artist`, `artist_fts`, `song`, `song_artist` (+ FTS shadow tables) —
   **no `user`, `session`, `access` or `token` table anywhere**.

## 14. Appendix — review round 1 (2026-10-04)

Seven inline comments on draft PR #11 (`tetuaoro`), all addressed.

### Bounds moved

Four constants. Each was checked against the 2025-03-22 export *before* being
changed — the real data sits well inside every new range:

| bound | before | after | real range in the export | source |
|---|---|---|---|---|
| `artist.fullname` min | 4 | **1** | shortest 5 ("Jonas") | review |
| `artist.fullname` max | 50 | **255** | longest 18 ("2B Brothers Tahiti") | review |
| `song.title` max | 100 | **255** | longest 28 | review |
| `song.artists` max | 75 | **10** | busiest song has 2 | review |

`TITLE_MIN` (4), `LYRICS_MIN` (100) and `LYRICS_MAX` (6000) are unchanged and
still v3's. The "transcribed verbatim from v3" comments on the constants and in
`migrations/0001_init.sql` were rewritten: after this, two of the bounds are
ours and the comments had to stop claiming otherwise.

**Consequence worth naming:** with `FULLNAME_MIN = 1` *and*
`Artist::split_fullnames` dropping blanks, no short name can be invalid any
more. The only reachable way for the create-song form to breach the artist rule
is to exceed the ceiling. A small review note moved the sole failure mode from
"too short" to "too long", and the tests were rewritten to say so.

### Tests that had to move with them

Five assertions broke — and **four had been passing for the wrong reason**,
each hardcoding a literal that merely happened to sit outside the old bound:

| test | literal | now |
|---|---|---|
| `domain::artist::validate_fullname_bounds` | `"Joe"` | `""`, `"   "`, `"a"` |
| `domain::song::validate_propagates_the_artist_rule` | `"Joe"` | `""` |
| `db::queries::an_invalid_artist_name_is_a_domain_error` | `"abc"` | `FULLNAME_MAX + 1` |
| `db::queries::title_length_is_counted_in_characters_not_bytes` | `101` | `TITLE_MAX + 1` |
| `sqlite_file::the_schema_rejects_what_the_domain_rejects` | `'abc'` | `''` |

All five now derive from the constant, so the next bound change cannot quietly
turn a test into a tautology. Two ceiling cases were added to the SQL-level
test, whose upper ends were previously uncovered.

### Traps hit

- **Three edits to one file in a single batch raced, and two were silently
  lost.** Each edit re-reads the whole file and writes it back, so concurrent
  writes clobber each other. The verification greps caught it; the "replaced 1
  occurrence" success messages did not. One file at a time, or one scripted
  edit.
- **Editing `0001_init.sql` in place invalidates the checksum sqlx stores in
  `_sqlx_migrations`**, so any database created before the edit then refuses to
  open. v4 has never been deployed, so editing the initial migration is still
  the right call — but the local `data/chansondufenua.db` had to be moved aside,
  and this stops being acceptable the moment v4 ships. The migration header now
  says so.

### Verified

`cargo fmt --check` and `cargo clippy --all-targets` clean. **61 tests green**
(55 lib + 6 file-backed). Build → `topcoat asset bundle` → boot: `GET /` 200 and
the hashed stylesheet 200 `text/css`, with the database recreated from the
amended migration.

### `env.example`

Restored at the root — it was deleted in step 1. Documents the two variables
that exist, `DATABASE_URL` and `HOST`/`PORT`, with the honest note that nothing
loads the file automatically (there is no dotenv loader). `HOST`/`PORT` support
was confirmed empirically rather than assumed: with `PORT=3999`, `:3999` answers
200 and `:3000` refuses.

---

## 15. Appendix — verified in step 4 (2026-10-04)

The app shell: layout, chrome, and the design tokens they compose. `src/ui/`
replaces v3's Leptos `components/` plus two SCSS files.

### The palette is Rust, not CSS

`src/ui/palette.rs` is the single definition of every colour. It is read twice:
`mod palette` for the crate, `include!("src/ui/palette.rs")` for `build.rs`.
`build.rs` renders the `@theme` block into `$OUT_DIR/tailwind-input.css` and
hands that path to `BuildConfig::input`.

The `include!` is why the file carries no `//!` inner docs and no `use`
statements — it has to compile as the opening items of a build-script crate root
exactly as happily as it does as a module.

### Proven live, not just present

A colour was edited in `palette.rs` and the build re-run:

| | `--color-tahiti-1000` | asset hash |
|---|---|---|
| after edit | `#0a222d` | `tailwind-79c2950cd4d1aa86.css` |
| after revert | `#0a222c` | `tailwind-69dd4bce0d05c7dc.css` |

The hash returns to its original value on revert, so the pipeline is
content-addressed: a token change busts caches by itself, with nothing to
remember.

### Two guards the palette made possible

- `every_colour_used_by_a_token_exists` — scans every class in every token for
  `tahiti*` names and asserts each one has a palette entry. A colour with no
  `--color-*` behind it is the quietest failure this module can have: the class
  is still valid to write, Tailwind emits no rule for it, and the element renders
  unstyled with nothing logged.
- `the_palette_defines_each_name_once` — `build.rs` writes the block, so a
  duplicate name would let one definition silently shadow another.

### Judgement calls

1. **The themed `@theme` route is generated, so §2.3 now holds literally.**
   The previous shape kept `src/styles/theme.css` in the tree. `BuildConfig` has
   no Rust-side theme API, so *something* has to reach Tailwind as CSS — the
   choice was where that CSS lives. It now lives in `src/ui/palette.rs` as data
   and is materialised at build time.
2. **The hamburger is a checkbox.** v3 needed Leptos signals. v4 has no client
   runtime, so the toggle is a `peer`-driven checkbox — same look, works with JS
   off. Slight a11y trade-off: it announces as a checkbox, not a button.
3. **The Google Fonts `<link>` is dropped.** v3 requested *Roboto Serif* while
   its own stack named *Roboto/Arial/serif*, so the request could never apply. It
   was an external render-blocking request buying nothing. If Serif is wanted,
   the honest fixes are a `<link>` matching the stack, or Topcoat's `font` module
   to self-host — `--font-sans` in `palette.rs` is the single place to change it.
4. **`class="dark"` on `<html>` is kept.** v3's stylesheet contains zero `.dark`
   rules — dark mode is `prefers-color-scheme`, which is what v4 compiles to.
   `palette.rs` inherits that deliberately.

### Known gap, carried to step 5

`GET /himene/nope` returns **404 with an empty body**. Topcoat's default 404 is
bare markup; v3 at least said *"La page n'existe pas."* Step 5 should add a
branded `error_boundary` in the layout rather than widening step 4.

### Verified

`cargo fmt --check` and `cargo clippy --all-targets` clean. **70 tests green**
(64 lib + 6 file-backed). `GET /`, `/aepa`, `/himene` → 200; `/himene/nope` →
404; the `<link>`ed stylesheet → 200 `text/css` and carries
`--color-tahiti-1000:#0a222c` and `--font-sans:Roboto, Arial, serif`. Nav
`aria-current="page"` on the current section. `<html lang="fr">`.

---

## 16. Appendix — Rust/UI assessment (2026-10-04)

Requested by the maintainer for the design: <https://rust-ui.com/docs/components>.

**Verdict: it cannot be used with Topcoat.** It is a **Dioxus** component library
(`rsx!`, `dioxus::prelude` — 11 occurrences on the Button page; zero `view!`).
The installation page says Leptos is the supported framework and Dioxus is
planned, while the component pages themselves are Dioxus — so the docs lag the
library. Across the three pages fetched, **`topcoat` appears zero times**, as do
`htmx`, `datastar` and `alpine`.

What *is* portable is the styling: the components are Tailwind-class-based
(`/assets/tailwind-dxh583b0ef4220fbc4.css`), and they are advertised as
copy-and-paste with no third-party dependency, so the class recipes could be
transcribed into `class!` tokens. What is not portable is behaviour: the
components are built on framework reactivity (`onclick` handlers over signals),
and v4 ships no client runtime. Interactive pieces would have to be rebuilt on
`htmx` / `datastar`, which Topcoat does provide.

Options, pending a decision:

1. **Adopt Rust/UI's visual language only** — transcribe its Tailwind recipes
   into `ui/theme.rs` and its palette into `ui/palette.rs`. Keeps the
   zero-dependency, zero-CSS posture; costs a manual pass per component.
2. **Keep v3's look** (what step 4 does today) and revisit later.
3. **Switch framework to Dioxus** to get Rust/UI directly. This reintroduces the
   client runtime and the hydration the rewrite exists to remove, so it trades
   away the agent-readiness goal.

---

## 17. Appendix — verified in step 5 (2026-10-05)

The home page: `/` and `/aepa`, transcribed from v3's `HomePage`
(`app/src/pages/index.rs`). Hero, three cards, synopsis, the two song tables, the
closing call to action — v3's copy, byte for byte.

### One bug found, and it was in step 3a

`songs(pool, order, Some(n))` applied `LIMIT n` to the **song ⟕ artist join**, not
to the songs. A song with two credits spends two rows, so `LIMIT 5` returned
**four** songs. It was visible the moment the page rendered: "Les plus vues" asked
for five and listed four — the busiest song in the export carries two credits.

Fixed in `db/queries.rs`: `page_select` picks the page of *ids* first and the join
only decorates it. `SongOrder::clause` became `SongOrder::song_keys` — a
*song-only* ordering, because the keys have to be valid before the join exists.
`a_limited_read_counts_songs_not_credit_rows` pins it: `Some(2)` used to return one
song, and the fixtures' two-credit song is now asserted to arrive whole, in order.

Nothing in steps 0–4 consumed a limit, which is why 82 green tests did not notice.

### The `<head>` problem

Topcoat 0.10 has **no per-page `<head>` API**. A view can declare a status code and
response headers and nothing else document-level; a page cannot set the `<title>`,
and a layout cannot ask a page for one.

What v3 actually needed was small: the home page's `<meta name="description">`, and
a `<link rel="canonical">` on `/aepa`. So the head is the layout's, and
`document_head(cx)` decides the per-route half from `uri(cx).path()`.

Three things worth knowing:

* **The title is the site title.** v3's `<Title text="Chanson du Fenua"/>` sits on
  the app root and is overridden only by the song page, so the capital F is v3's,
  not a typo — `Song::get_meta_data` writes the lowercase one. Step 7 needs a real
  per-page mechanism; this constant is what every page shows until then.
* **`/` carries no canonical and `/aepa` does**, with no trailing slash — exactly
  as the live site emits it.
* **The description is one `const`** (`pages::home::copy::DESCRIPTION`), shared by
  the page and the head, and `the_description_is_the_copy_it_claims_to_be` asserts
  it equals synopsis + first card + tagline. The page and the meta tag cannot drift.

### The 404

§15 left this gap, with the note "step 5 should add a branded `error_boundary` in
the layout". That is half the story, and the other half is the interesting one:

* An `error_boundary` in the layout catches errors raised **by a page**. It does
  **not** catch a URL that matches no route — nothing else runs for a URL nothing
  was registered for: no layout, no boundary, no page. The response is Topcoat's
  bare nine bytes, `not found`.
* A **pathless layer** would wrap every request, but it returns a `Response`, so it
  would have to re-render the whole document by hand.

The intended mechanism is `not_found!("/")` (`src/pages/mod.rs`), which registers a
catch-all page that does nothing but fail with a `NotFoundError` — and an error
raised *by a handler* does travel back through the layout, where the boundary
catches it and the status stays 404. Both halves are needed; neither is enough on
its own.

`GET /himene/nope` now answers 404 with the header, the footer, "La page n'existe
pas." and a link home.

### Deliberate differences from v3

1. **Rows served, not streamed.** v3 fetched each table from the browser inside
   `<Suspense>`, so the first byte held two empty `<table>` elements. The live page
   still shows this: its two `<tbody>`s sit **after `</body>`** in the wire format.
   Here both queries run before the page renders.
2. **Rows are links, not scripts.** v3 put `onclick="window.location=…"` plus
   `role="button"` and `tabindex="0"` on every `<tr>`, *and* correct anchors inside
   it. The scripts are gone: `onclick`, `role="button"` and `tabindex` appear zero
   times in the rendered page.
3. **The surface is the v4 panel.** The one visible change. v3's cards and table
   panels were `bg-neutral-200/10` under a `backdrop-blur-md`; v4 uses `theme::CARD`
   — the same dark wash, hairline and shadow as the header and footer. This follows
   the depth direction chosen in 4b, and it is reversible in one token if it is the
   wrong call.
4. **`CARD` lost its padding.** Home panels want `p-8`, the song page's panel wants
   `p-6`, and two padding utilities in one class list resolve by stylesheet order
   rather than by intent. `CARD_ROOMY` is the `p-8` variant, and
   `the_panel_variants_keep_the_card_surface` checks the repeated surface classes —
   a `StaticClass` **cannot** be composed from constants, because `class!` composes
   only in attribute position, where the type is inferred.

### Judgement calls

1. **"C'est parti !" still points at `/himene/api`** — v3's target; the create-song
   page arrives in step 9. Until then it is a branded 404 rather than a blank one.
   Repointing it at `/himene` would be a behaviour change v3 did not make.
2. **A row keeps its hover tint** but is no longer clickable. It still reads as one
   row responding; only the links inside it are targets.
3. **The synopsis keeps `max-w-[800px]`** rather than `max-w-prose`: it is a display
   paragraph, not body copy.
4. **The tables keep v3's column order** — lyrics first in "Les dernières ajouts",
   title first in "Les plus vues". Asymmetric, and v3's.

### Verified

`cargo fmt --check` and `cargo clippy --all-targets` clean. **83 tests green**
(77 lib + 6 file-backed), up from 70.

Live, via `.run/step5.sh` — the real export imported (43 songs / 34 artists), the
binary started, the HTML read back: **29/29 checks passed**, covering the hero,
cards, synopsis, both tables, both `<head>` variants, the 404, and the absence of
`onclick` / `role="button"` / `tabindex` / streaming placeholders.

**Parity against the live site** (`https://www.chansondufenua.pf`, fetched
2026-10-05), comparing row ids:

| table | ours | live | agreement |
|---|---|---|---|
| Les dernières ajouts | `7114wvk9, gosqh0y5, …` | `8nntgjk4, a9v3692, 0zgqae96, 7114wvk9, gosqh0y5` | our first two are live's last two, in order |
| Les plus vues | `fb9yubmi, r2a95dk6, b35dvri7, cghcaesq, 6qq0qmv` | `fb9yubmi, r2a95dk6, b35dvri7, 6qq0qmv, cghcaesq` | 3 of 5 in the same position, all 5 shared |

Both differences are the two already-known ones: the dump predates **three songs**
the live site has since added (so live's newest table starts three rows earlier),
and **view counts have moved** since 22 March 2025 (which is why two songs trade
places 4 and 5 in the most-viewed table). Both close at release, when the export is
refreshed.
---

## 18. Appendix — verified in step 6 (2026-10-05)

The song index: `/himene`, transcribed from v3's `AllSongPage`
(`app/src/pages/himene/allsong.rs`). One heading, one table, the artist column
dropped below `md`. v3's copy, byte for byte.

### A real defect, found by the verification and not by the tests

`build.rs` printed two `rerun-if-changed` lines — `build.rs` itself and
`src/ui/palette.rs` — and those two lines were the whole problem. **Cargo reruns
a build script when any file in the package changes only if the script prints no
`rerun-if-changed` at all; printing one replaces that default with the list.**
So `build.rs` (which runs Tailwind) was rerun only when `palette.rs` changed,
and Tailwind's whole job is to scan the *rest* of the source.

Consequence: the stylesheet served with step 5 was the stylesheet of step 4b.
`truncate`, `max-md:hidden`, `pt-[9px]`, the `w-[500px]`/`w-[800px]` measures —
all of them were in the markup and **none of them were in the CSS**. On a phone
the lyric column showed in full instead of hiding, and on every width it pushed
the table wide instead of truncating. Nothing failed: `cargo test` never reads
the stylesheet, and step 5's smoke test asserted on the class name in the HTML,
which was there.

The step-6 verification was written to fetch the stylesheet and look for the
utilities it needs — that check is what surfaced it. Fixed in `build.rs` by
adding `cargo:rerun-if-changed=src`, and `the stylesheet carries the new
utilities` now asserts both the new step-6 classes *and* the two step-5 ones the
bug had swallowed, plus the absence of one utility named only in a doc comment
(Tailwind v4 scans comments; a class name written in prose is dead CSS).

### Two differences from v3, both inherited from step 5

1. **The rows are served, not streamed.** v3 built the list in the browser — a
   `Resource` inside `<Suspense>` — so the first byte carried
   `<td>Chargement...</td>` and the rows arrived afterwards, in a second request.
   The live page still does it: its `<tbody>` is assembled *after* `</body>` in
   the wire format, so the HTML the server sends contains an empty table.
2. **The rows are links, not scripts.** v3's `<tr>` carried
   `onclick="window.location=…"`, with `role="button"` and `tabindex="0"` to
   match, *and* correct `<a href>` elements inside. The anchors stay; the rest is
   gone — including `aria-label="Go to the song …"`, an English sentence on a
   French page describing a role that no longer exists.

### Unbounded, on purpose

v3 asked for `page 0, limit 255`. There was never a page 2 to ask for:
`app/src/app.rs` registers `himene/""` and `himene/:id`, so the page's
`use_params_map().get("page")` could only ever read `0`. The 255th song would
have been invisible with nothing to page to, and the call site said
"pagination" while doing something else. v4 reads every published song; when the
catalogue outgrows one page, the segment arrives *with* the pagination it
implies.

### One addition to v3's row rule

`last:border-b-0`. v3's panel was borderless (`bg-transparent` under a
`shadow-md`), so a rule under every row collided with nothing. The v4 panel is
bordered, and on the last row the row's rule and the panel's edge land a pixel
apart and read as a double line. v3's rule is otherwise kept utility for
utility.

### Judgement calls

1. **The page title is `theme::H1`**, not v3's smaller heading — step 4b's
   typography direction applied to the second page that has a title. It makes
   the index heading *larger* than the home hero, which is the one thing here
   worth a second look; it is one token away from changing.
2. **The surface is the v4 panel**, following §17 rather than re-deciding it.
   `INDEX_PANEL` is `CARD`'s surface plus `overflow-hidden` — the clipping is
   the only reason the table's square corners do not poke out of the radius —
   and **no padding**, because the cells carry it (`px-6`); a `p-8` on the panel
   would inset the table's own edge and leave the header row floating.
3. **`INDEX_COLUMN_ARTIST` is one token**, not the two spellings v3 needed. It
   is a property of the *column*, so it composes with `INDEX_HEAD` and
   `INDEX_CELL` at the two places that column is declared.
4. **An uncredited song keeps its artist cell, empty.** v3 joined an empty
   `Vec` and rendered the empty string; dropping the `<td>` would shift nothing
   today and would break the table the moment a column follows it.

### The trap this step found

`#[page("/himene")]` emits a **unit struct named after its handler**, in the
same module, in the *type* namespace. `let songs = …` in that module is then
read as a pattern matching the unit struct rather than as a new binding, and the
compiler reports a type mismatch on the *expression* — the error lands several
lines from the cause. The index's rows are bound to `listed`; every future page
in `src/pages/` has the same constraint.

### Verified

`cargo fmt --check` and `cargo clippy --all-targets` clean. **88 tests green**
(82 lib + 6 file-backed). Against the real 22 March dump, imported into a
scratch database: `/himene` → 200 with **43 rows for 43 published songs**, 43
links, none duplicated, no row without one; credited songs carry their artists
comma-joined in v3's order, uncredited ones carry an empty cell; zero `onclick`,
`role="button"`, `tabindex`, `aria-label` or `Chargement` in the page. The
`<link>`ed stylesheet → 200 `text/css` carrying `md:table-cell`,
`last:border-b-0`, `px-6`, `rounded-card` — and, now, `truncate` and
`max-md:hidden`. `/` unchanged (ten rows), `/himene/nope` still 404 with the
chrome.

---

## 19. Appendix — verified in step 7 (2026-10-05)

The song sheet: `/himene/{id}`, transcribed from v3's `SongPage`
(`app/src/pages/himene/song.rs`) — the title, the lyric with its chords, and, for
the first time here, a `<head>` that belongs to the page rather than to the site.

### The head is the layout's job, so a song costs one extra read

Topcoat 0.10 has no per-page `<head>` API: in node position a view may declare a
`StatusCode` and response headers and nothing else. v3 went the other way —
`leptos_meta` hoisted the page's `<Title>`/`<Meta>`/`<Script>` up into the
document. So `document_head` in `src/ui/layout.rs` reads `uri(cx).path()` and
loads the same row the page loads, by primary key.

That second read is a deliberate trade, not an oversight: it is an indexed
lookup in a database compiled into this binary, and the alternative is a head
that is wrong for every song — which is the whole reason this step has a metadata
section. A path the layout cannot resolve (a missing song, a draft, a deeper path
under `/himene/`) falls back to the site head; the page is what turns those into
a 404, so the fallback is never what a visitor sees.

### v3's tags, kept — and the one that had to change

The Open Graph and Twitter set is v3's, field for field, including the two
`twitter_` fields that differ from their Open Graph twins (the images genuinely
are different crops). They are emitted **only** on a song route, which is where
v3 declared them; the home page has never carried them and still does not.
`og:url` and the canonical link are the same URL, because v3 set both from
`meta_og_url`.

`theme-color` is the one place v3's markup had to go: it carried two values
scoped to `prefers-color-scheme`. v4 is dark-only, so one value is correct, and
the scheme itself is declared by the base rule `build.rs` writes next to the
`@theme` block. A dark document that does not declare its scheme gets light
native scrollbars and controls.

The JSON-LD is rendered through `Unescaped`. That is not a hole in the escaping:
`<script>` is a raw-text element, so entities inside it are **not** decoded, and
an escaped JSON-LD block is broken JSON that a crawler cannot parse. The escaping
that matters happened in the domain — `serde_json` serialises it and the only
visitor-writable string in it has already been through `ammonia`.

### The lyric is parsed, not injected

v3 dropped `data.song_lyrics` into the page with `inner_html`, so every visual
decision about a chord had to be a CSS rule matching a `<sup>`. v4 parses the
same sanitised HTML into lines and spans (`Song::lyrics_lines`) and renders them
with the site's own tokens: a line is a positioned `div`, a blank line is the
same `div` with the verse-break height, and a chord is an absolutely positioned
`<sup>` that takes no width so the words underneath still read as one line. v3's
`data-nosnippet` on every chord is kept — a search engine that quotes the page
prints the chords inline with the words, where they read as typos.

Three differences from v3, all deliberate:

1. **The credits are shown.** v3 carried the artists only inside the page's
   metadata, so a reader arriving from a search result never saw who wrote the
   song. The index has listed them since step 6; the sheet now does too.
2. **A missing or unpublished song is a 404.** v3 rendered nothing at all when
   the fetch failed. `db::song` returns drafts on purpose (the editor needs
   them), so the page — not the query — is what filters on `published`.
3. **The title is a heading in the display face.** v3 set it in the body face at
   a fixed size, in a slant no shipped face can draw.

### `view_count`: one line the draft was missing

The plan's step-7 row lists the `view_count` increment among the deliverables,
and v3 has it — `fn::get_song_fetch_artist` opened with
`UPDATE song SET view_count += 1`. The draft of this step had every visible
thing and not this one, because nothing renders the count on this page: it is
only read back by the home page's most-viewed table. Found by reading the Done
column rather than the code.

Order is the one difference. v3 incremented before it read, so an id that did not
exist incremented nothing and a *draft* still counted. v4 increments only for a
sheet it is about to serve, which is what the most-viewed table means by a view.

### Three defects the verification found, and the tests could not

1. **The stylesheet carried an invalid declaration.** Tailwind reads the *text*
   of every scanned file, and `src/db/import.rs` holds SurrealQL test data —
   `artists: [artist:uynyy…]` — whose bracketed id it read as an arbitrary-value
   utility. The served CSS contained `.\[artist\:uynyy…\]{artist:uynyy…}`: not a
   rule, a broken declaration, on every page. Fixed in `build.rs` with
   `@source not "<manifest>/src/db"`; nothing outside `src/ui` and `src/pages`
   carries markup.
2. **Prose that named the utilities it published.** `RADII`/`SHADOWS`' doc
   comments spelled the very class names they generate, so `rounded-panel` and
   `shadow-lift` were compiled into the stylesheet for no one to use — in a file
   whose own header claims none of its comments spells a class name. Reworded,
   and the claim is now true. The same trap caught this module's own guard table:
   listing Tailwind's nine weight utilities would put eight unusable rules in
   every stylesheet, so the names are assembled with `concat!` — a compile-time
   constant, so the table is unchanged, but the scanner no longer reads a class.
3. **The dead-CSS check itself was wrong.** It read the pseudo-class half of a
   selector as part of the class: `focus-visible:outline-2`, `hover:bg-ink-800`,
   `last:border-b-0` and `peer-focus-visible:ring-2` all appeared "unused" while
   sitting in the markup. The name now ends at the first unescaped colon.

What survives all that is unavoidable from Rust and is now **reported rather than
asserted**: Tailwind reads `src/` as text, so `&'static str`, a `.filter(` call
and the ordinary word "invisible" in a sentence all compile to rules nothing can
match — six of them. The assertion is scoped to the vocabulary `palette.rs`
publishes (24 rules), which is the part this repository owns, and a second check
now asserts the direction that actually breaks a page: **every class in the
markup has a rule behind it**. Stylesheet: 22 138 B → **20 776 B**.

### Verified

`cargo fmt --check` and `cargo clippy --all-targets` clean. **99 tests green**
(93 lib + 6 file-backed). Against the real 22 March dump, imported into a
scratch database: `/himene/7114wvk91gffr2bj6wza` → 200 with title
`'Āhani e - 2B Brothers Tahiti | Chanson du fenua`, 50 lyric lines, 50 chords,
6 verse gaps, the credits `2B Brothers Tahiti`; canonical == `og:url` == the
song's own URL; the JSON-LD parses as a `MusicComposition` naming the song and
carrying the clean lyric text; **43 of 43 published songs** render a sheet;
two fetches moved that song's `view_count` 29 → 31. `/` and `/himene` unchanged
— ten rows, 43 links, `<title>Chanson du Fenua</title>` — and `/himene/nope`
still 404 with the chrome. 41/41 HTML checks and 27/27 stylesheet checks in
`.run/step7.sh`.

---

## 20. Appendix — verified in step 8 (2026-10-05)

French and Tahitian. `src/i18n.rs` is a `Lang` enum, a `Key` enum, and one
exhaustive match per language; the layout and the pages ask it for the request's
language and for the words of the chrome. §2.4 asked for a homegrown module
because Topcoat's localization support is on its roadmap and not in the crate,
and this is that module — **no new dependency**, only the `serde` and `topcoat`
the crate already had.

### The compiler is the translator's checklist

A missing translation is a compile error, not a blank line. The two matches are
over the whole `Key` enum, so adding a key stops the build until both languages
answer for it — which is what §2.4 asked for and the reason the catalog is an
enum rather than a table of strings. A unit test closes the other half that the
compiler cannot: no key may hold the same string in both languages, so a key
given its French words and a Tahitian placeholder that equals them fails.

### What is translated, and what is deliberately not

Chrome and labels only. That means the navigation, the 404, and the fixed action
labels: the hero button and the closing button on the home page, the index's
heading and two column titles, and the song sheet's "add lyrics" button. Twelve
keys in all.

Left in French, on purpose, and each for the same reason — they are not labels:

- **the song lyrics**, which are the content;
- **the home page's prose** (the hero's standfirst, the three cards, the synopsis,
  the teaching paragraph). Translating prose is *writing*; a rewrite should not
  put new words in the maintainer's mouth, and that is a separate decision from
  giving the chrome a language;
- **the footer sentence**, which is a single sentence with two links spliced into
  the middle of it. Split into catalog fragments it would be ungrammatical in
  both languages. It stays exactly as v3 wrote it, English words and all.

`Facebook` stays a constant for the same kind of reason: a brand name is not a
translation unit.

### The resolution order, and the cookie that makes it stick

§2.4's order, implemented as a pure function over the three sources so that it
is testable without a request: `?lang=` → the `lang` cookie → `Accept-Language`
→ French. Quality values are honoured and an entry the site cannot serve is
skipped rather than scored, so `en;q=1.0, ty;q=0.5` is Tahitian — a first-match
implementation would have called it French.

An explicit `?lang=` is written back as a cookie, and that is not decoration: no
link in the chrome carries the parameter, so without it the choice would last
exactly one page. The jar is `CookieLayer`, registered in `lib.rs` with
`.cookies()`; `i18n::resolve` is called by the layout *and* by each page that has
a label, so the write is guarded on the stored value — a page whose cookie
already agrees does not restate it, which is why the several calls per request
produce one `Set-Cookie`. `Path=/`, because a choice made on `/himene/x` has to
reach `/`.

### `og:locale` follows the language — the one change to v3's metadata

v3 wrote `ty_PF` and `fr_FR` as constants with Tahitian first. They are now
`Lang::og_locale` and its alternate, so a page served in French says so. The
default page therefore carries **`fr_FR`** where v3 carried `ty_PF`;
`?lang=ty` reproduces v3's pair exactly. That is the point of the step rather
than a regression, and `.run/step7.sh` carries one updated assertion to say so.

Alongside it, a `hreflang` cluster on every page: `fr`, `ty`, and an `x-default`
that points at the URL with no parameter. Every page on this site exists in both
languages and the parameter is the only difference, so the alternates are the
page's own URL plus `?lang=` — built from `uri(cx).path()`, which is why the
cluster never stacks a second `?` when the reader arrives already carrying one.
The canonical deliberately keeps no parameter: the canonical is the page, and the
language is a variant of it.

### Reo Tahiti: a first pass, awaiting a native review

The Tahitian catalog was written without a native speaker to check it. The
vocabulary is small and conservative — `hīmene` (song, the site's own word),
`fa'aea` (welcome, used for the front page), `parau hīmene` (lyrics, the phrase
the song page's metadata already mixes into its description), `tāpiri` (to add) —
and every string is short enough to correct in `i18n.rs` alone, with no code
touching it. This is the step's judgement call and its one known gap: the
plumbing is done, the words want a fluent reader.

### A trap this step walked past

Tailwind scans `src/` as **text**, so a catalog is a source of class names as
much as any markup is: a Tahitian word that happens to be a utility would be
compiled into every stylesheet as a rule nothing can match. Checked, not assumed:
`.run/step8.sh` reads the served CSS and fails if the set of dead rules grows past
the six ambient ones step 7 documented. It did not grow — the stylesheet is
**20 776 B**, byte for byte the size step 7 left it.

One smaller thing, recorded because it cost a minute: `#[component]` puts the
request context in scope only for a function that declares `cx: &Cx`. A
component without one — `sheet_body` was without one — has no way to ask for the
language, so the parameter had to be added; the call site is unchanged.

### Verified

`cargo fmt --check` and `cargo clippy --all-targets` clean. **103 tests green**
(97 lib + 6 file-backed), five of them new: the catalog is complete and actually
differs per language, the tag reader accepts `fr`/`fr-FR`/`ty_PF` and rejects
`en`, and `Accept-Language` is ranked by quality.

Against the real 22 March dump, imported into a scratch database and served
(`.run/step8.sh`): **49/49 HTML checks and 13/13 stylesheet checks**. In
particular — `/` is `lang="fr"` with the nav `Accueil`/`Chanson` and `/` with
`?lang=ty` is `lang="ty"` with `Fa'aea`/`Hīmene`; the cookie is honoured, and an
explicit `?lang=ty` beats a `lang=fr` cookie; `Accept-Language: ty-PF,ty;q=0.9`
is Tahitian and `?lang=en` falls through to French without setting anything;
`?lang=ty` responds `Set-Cookie: lang=ty; SameSite=Lax; Path=/; Max-Age=31536000`
and a request whose cookie already says `ty` gets no `Set-Cookie` at all; the
404, the index heading and columns, and both buttons translate; the `hreflang`
cluster is complete and correctly formed on `/`, `/himene`, a song and the 404;
`og:locale` swaps with the language; and the lyric block is byte-identical in the
two languages, which is the check that says the step stopped where it said it
would.

## 21. Appendix — verified in step 9 (2026-10-05)

The create-song page is `src/pages/editor.rs`: one route, `/himene/api`, with two
methods. The URL is v3's because it is in the wild — the home page's closing
button and every song sheet link to it — and until this step those links landed
on the branded 404. They now land on the form.

### One route, two methods — no second protocol

v3 was a Leptos page plus a `#[server]` function: the form posted to a
serialization endpoint that returned a `Song` and set `Location` for the browser
to follow. Here the page answers its own `POST`. The shapes are the same
Post/Redirect/Get; what goes away is the extra endpoint.

- **303, not v3's 302.** Both are followed with a `GET` by every browser that
  matters; 303 is the code that says so, and the point of the redirect is that
  the form must not be re-posted.
- **A rejected submission is a 400 with the form filled back in.** v3 returned a
  bare error and lost the whole lyric. The bounds are the domain's — the page
  does not restate them — and the message is deliberately *not* shown: it is
  written for a log line (`expected 100..=6000 characters, got 42`), so the page
  names the two fields a person can fix instead.
- `#[page([GET, POST] "/himene/api")]` — **no comma** between the method list and
  the path. The documented `#[page([GET, POST], "/…")]` spelling does not compile
  in 0.10: the parser reads the methods and then peeks for the path literal, and a
  comma is neither production.

The body is read only for a `POST`, because `Form` on a `GET` reads the query
string — and `/himene/api?lang=ty`, which step 8 made every page able to serve, is
a query string that is not a song.

### The artist field, and why it is one field

v3 kept a hidden input for the value and a visible one for typing, so it could
require the script to move names across. A submission from a browser with the
script switched off therefore carried no artists at all, and the chips were the
only way to remove one. Here the field the browser posts is the field a person
types in (`name="artists"`, comma-separated — the same split `db::create_song`
already uses), and the chips are a *view* of its value: pressing one takes that
name back out of the string. Without the script there are no chips and the field
still submits.

The `<datalist>` is the whole artist table, not the top ten. An artist who is not
in the list is not in it for a reason, and a truncated one invites a near-duplicate
name. `db::search_artists` exists for a client that queries as the author types;
there is no such client, so it is not called.

### The chord tools, in the page

v3's were Rust compiled to WebAssembly and hydrated over the server's markup; the
rewrite has no client build step (§2.1), so the same behaviour is roughly fifty
lines of vanilla script, unescaped into the page like the JSON-LD block. Nothing
about the site depends on it running — it is the editor, and an editor needs a
browser.

The naming rule is v3's, character for character: flat *else* sharp, then minor —
so `b` + `#` both ticked is `Cbm`, exactly as v3 composed it. The three modifier
checkboxes are v3's order and letters (`m`, `#`, `b`).

Three deliberate differences in the editor itself, all in the module docs:

- **The chord is in flow, not absolutely positioned.** v3 drew editor chords above
  their syllable, the same way the sheet does, which meant the caret and the chord
  were never quite in the same place. A chord is still tinted and monospaced so it
  reads as markup; the geometry is left alone. The *sheet* still draws it above
  the word.
- **The caret may enter a chord.** v3 swallowed `keydown` while the caret sat
  inside a `<sup>`, and read Backspace there as "remove the chord". Here a chord is
  a node like any other: a click puts the caret in it, a keystroke extends its
  text, and backspacing past one removes it — which is the browser's own
  behaviour. v3 swallowed the whole event, not only the key it meant to.
- **`data-god` is gone.** v3 tagged a custom chord with it; no stylesheet reads it.

The field names are the page's own (`title`, `artists`, `lyrics`), not v3's
serialization names (`data[title]`, …), because there is no server function left
for them to address. The empty `disabled` submit button v3 carried exists only to
make `ActionForm` behave and is not reproduced.

### The title, and the layout's one branch

Topcoat 0.10 has no per-page `<head>` API — recorded in step 7 and unchanged
here — so the only place a non-song route can name itself is the `#[layout]`. The
create-song page is the first route that needs to: it says `Ajouter des paroles |
Chanson du Fenua` rather than the site's name. The layout recognises it by
`crate::pages::editor::PATH`, a constant that restates what the `#[page]`
attribute declares (the macro takes a literal path), and that is what the form's
`action` and the layout's rule both read, so there is one spelling of the URL.

### `sanitise_lyrics`, extracted

`Song::lyrics_html` was the ammonia builder inline; it is now a free function,
`sanitise_lyrics`, and the method calls it. The reason is the rejected submission:
a form handed back with the author's work in it seeds the hidden lyric field from
the *sanitised* submission, because a value on its way back to the browser is no
more trustworthy than it was on the way in. Its test asserts `<script>` and
`onclick` do not survive that round trip while `data-nosnippet` does.

### The page stores what was written; the renderer is the choke point

Worth stating plainly, because the step's verification turns on it: the `song`
column holds what the author submitted, markup and all, and `Song::lyrics_html`
is the only route from that column to a template (§2.6). So a submission carrying
a `<script>` is **accepted** — and the proof is not the request but the sheet it
produces. `.run/step9.sh` posts one, follows the 303, and asserts on the served
sheet that `<script>` and `onclick` are nowhere in it and the chord and its
surrounding text are.

This replaces the first draft of that check, which expected a 400 and read the
form back. It was a wrong expectation, not a found bug: the submission is
well-formed, is stored as written, and is inert where it matters.

### Two traps, both about Tailwind's output rather than its input

1. **The served selector escapes more than the colon.** `[&_sup]:font-mono`
   reaches the stylesheet as `.\[\&_sup\]\:font-mono sup`, and `min-h-[200px]` as
   `.min-h-\[200px\]`. The first draft of the stylesheet assertions escaped only
   the `:` — the form Tailwind uses for a plain utility — so four new rules read
   as missing when they were present. A needle that escapes less than Tailwind
   does finds nothing.
2. **A dead-rule detector must undo `&amp;` first.** The served markup carries
   `[&amp;_sup]:font-mono`, because an attribute value escapes `&`; the stylesheet
   carries `[&_sup]`. Compared raw, a class the page *does* use reads as a rule
   Tailwind emitted for nothing. And one word in a doc comment did emit a real
   dead rule: `entity-escaping its contents` compiled `.contents`. Reworded, and
   the check now guards it — dead rules stand at four (`fixed`, `invisible`,
   `lowercase`, `static`) rather than six. The two that left the list did not
   leave the stylesheet: `collapse` and `filter` are words in the editor's own
   JavaScript, which is served inside the page, so the detector now finds them
   among the markup as well. That is the heuristic being textual, not the rules
   being used.

### Verified

`cargo fmt --check` and `cargo clippy --all-targets` clean. **108 tests green**
(102 lib + 6 file-backed), five of them new in `editor.rs`: every field of the
form parses when absent, the credit field splits the way the database does, a
rejected submission keeps its lyric, a valid one becomes a song whose chords
survive the round trip, and a rejected one is the domain's error rather than a
database fault.

Against the real 22 March dump, imported into a scratch database and served
(`.run/step9.sh`): **54/54 HTML checks and 24/24 stylesheet checks**. In
particular — `/himene/api` is 200 and carries its own title; the seven natural
chord buttons, the three modifiers and the custom field are all there; the
artist list holds all 34 names; the page is French by default and `?lang=ty`
translates the heading, the lyric label and the save button; a valid `POST`
answers 303 to `/himene/{id}`, the created sheet carries the chord with
`data-nosnippet` intact and both credits, the new song appears on the index, and
the index still lists all 44; a rejection answers 400 with the title *and* the
lyric still in the form; and the sheet built from a submission carrying a
`<script>` contains neither the script nor its handler. The stylesheet still
carries every token the form introduced — `22442 B`.
