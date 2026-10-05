//! Integration tests against a real, *file-backed* database.
//!
//! The unit tests in `src/db/queries.rs` run in memory, which cannot tell you
//! whether WAL mode actually took, whether `PRAGMA foreign_keys` is really on,
//! or whether a second process would see the rows. These tests can.
//!
//! Build artefacts only: everything lands in `target/test-dbs/`. That directory
//! is not `gitignore`d by coincidence — `/target/` already is, and the sandbox
//! this was written in blocks `/tmp`.

use std::path::{Path, PathBuf};

use chansondufenua::db::{self, Db, DbError, SongOrder};

fn scratch(tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-dbs");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join(format!("{tag}.db"));
    // Start from a clean slate: the point is to prove creation from nothing.
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(dir.join(format!("{tag}.db{suffix}")));
    }
    path
}

fn url_of(path: &Path) -> String {
    // Three slashes: `sqlite://` + an absolute path.
    format!("sqlite://{}", path.display())
}

#[tokio::test]
async fn creating_and_reopening_a_file_database_works() {
    let path = scratch("persist");
    let url = url_of(&path);

    {
        let db = Db::open(&url).await.expect("create");
        db::create_song(
            db.pool(),
            "Une chanson persistée",
            &"y".repeat(120),
            "Jonas",
        )
        .await
        .expect("insert");
    }

    // Reopening must not re-run migrations destructively, and the row must be
    // there. This is the test that would catch a `CREATE TABLE` that lost its
    // `IF NOT EXISTS` guard in a future migration.
    let db = Db::open(&url).await.expect("reopen");
    let all = db::songs(db.pool(), SongOrder::Newest, None)
        .await
        .expect("read");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].get_title(), "Une chanson persistée");
    assert_eq!(all[0].get_artists().len(), 1);

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn wal_and_foreign_keys_are_on() {
    let path = scratch("pragmas");
    let db = Db::open(&url_of(&path)).await.expect("open");

    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(db.pool())
        .await
        .expect("journal_mode");
    assert_eq!(
        journal.to_lowercase(),
        "wal",
        "WAL keeps readers from blocking the per-view view_count UPDATE"
    );

    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(db.pool())
        .await
        .expect("foreign_keys");
    assert_eq!(foreign_keys, 1, "song_artist FKs are not being enforced");

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn a_song_cannot_be_credited_to_an_artist_that_does_not_exist() {
    let path = scratch("fk");
    let db = Db::open(&url_of(&path)).await.expect("open");

    let error = sqlx::query(
        "INSERT INTO song_artist (song_id, artist_id, position) VALUES ('nope', 'nope', 0)",
    )
    .execute(db.pool())
    .await
    .unwrap_err();
    assert!(
        error.to_string().to_lowercase().contains("foreign key"),
        "expected a foreign-key violation, got {error}"
    );

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn deleting_a_song_removes_its_credits() {
    let path = scratch("cascade");
    let db = Db::open(&url_of(&path)).await.expect("open");

    db::fixtures::seed(db.pool()).await.expect("seed");
    let credits_before: i64 = sqlx::query_scalar("SELECT count(*) FROM song_artist")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert!(credits_before > 0, "fixtures should have credits");

    sqlx::query("DELETE FROM song WHERE id = '4cfl27ia9hndgetgr1o7'")
        .execute(db.pool())
        .await
        .unwrap();

    let credits_after: i64 = sqlx::query_scalar("SELECT count(*) FROM song_artist")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(credits_after, credits_before - 2, "ON DELETE CASCADE");

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn the_schema_rejects_what_the_domain_rejects() {
    let path = scratch("checks");
    let db = Db::open(&url_of(&path)).await.expect("open");

    // The domain is the authority; these CHECKs are the backstop. If they ever
    // disagree, one of the two has drifted. Each case feeds otherwise-valid
    // values so the *intended* constraint is the one that fires.
    let valid_lyrics = "y".repeat(120);

    let too_short_title = sqlx::query(
        "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
         VALUES ('aaaa00000000000000a', 'abc', ?1, 1, 1, ?2, ?2)",
    )
    .bind(&valid_lyrics)
    .bind("2026-01-01T00:00:00.000000000Z")
    .execute(db.pool())
    .await
    .unwrap_err();
    assert!(mentions_check(&too_short_title), "title: {too_short_title}");

    let zero_views = sqlx::query(
        "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
         VALUES ('bbbb00000000000000b', 'A valid title', ?1, 0, 1, ?2, ?2)",
    )
    .bind(&valid_lyrics)
    .bind("2026-01-01T00:00:00.000000000Z")
    .execute(db.pool())
    .await
    .unwrap_err();
    assert!(mentions_check(&zero_views), "view_count: {zero_views}");

    let blank_artist = sqlx::query(
        "INSERT INTO artist (id, fullname, created_at, updated_at) VALUES ('cccc00000000000000c', '', ?1, ?1)",
    )
    .bind("2026-01-01T00:00:00.000000000Z")
    .execute(db.pool())
    .await
    .unwrap_err();
    assert!(mentions_check(&blank_artist), "artist: {blank_artist}");

    // The ceilings are backstops too. `title` and `fullname` are 255 characters
    // now, so an over-long value must be refused here even though the domain
    // gets to it first.
    let long_title = sqlx::query(
        "INSERT INTO song (id, title, lyrics, view_count, published, created_at, updated_at) \
VALUES ('dddd00000000000000d', ?1, ?2, 1, 1, ?3, ?3)",
    )
    .bind("T".repeat(256))
    .bind(&valid_lyrics)
    .bind("2026-01-01T00:00:00.000000000Z")
    .execute(db.pool())
    .await
    .unwrap_err();
    assert!(mentions_check(&long_title), "title ceiling: {long_title}");

    let long_artist = sqlx::query(
        "INSERT INTO artist (id, fullname, created_at, updated_at) VALUES ('eeee00000000000000e', ?1, ?2, ?2)",
    )
    .bind("A".repeat(256))
    .bind("2026-01-01T00:00:00.000000000Z")
    .execute(db.pool())
    .await
    .unwrap_err();
    assert!(
        mentions_check(&long_artist),
        "artist ceiling: {long_artist}"
    );

    let _ = std::fs::remove_file(&path);
}

fn mentions_check(error: &sqlx::Error) -> bool {
    error.to_string().to_lowercase().contains("check")
}

#[tokio::test]
async fn a_domain_error_is_never_mistaken_for_a_database_error() {
    let path = scratch("errors");
    let db = Db::open(&url_of(&path)).await.expect("open");

    let error = db::create_song(db.pool(), "abc", &"y".repeat(120), "")
        .await
        .unwrap_err();
    assert!(
        matches!(error, DbError::Domain(_)),
        "handlers need to distinguish 422 from 500 without string matching: {error:?}"
    );

    let _ = std::fs::remove_file(&path);
}
