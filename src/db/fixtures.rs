//! Test fixtures: real rows from the v3 database.
//!
//! Not invented — these are four songs copied verbatim out of the 22 March
//! 2025 SurrealDB export, chosen to cover the awkward cases the schema has
//! to survive: a song with no credited artist, a song with two (where the
//! credit *order* is visible on the page), chords placed inside words, and
//! macrons in a title (which a byte-based length check would miscount).
//!
//! `id`, `created_at` and `view_count` are preserved from the dump so the
//! fixtures exercise the same shapes as production data.

use sqlx::SqlitePool;

/// An artist referenced by [`SONGS`], with its v3 id and timestamps.
pub struct FixtureArtist {
    pub id: &'static str,
    pub fullname: &'static str,
    pub created_at: &'static str,
    pub updated_at: &'static str,
}

/// A song, exactly as the v3 database held it.
pub struct Fixture {
    pub id: &'static str,
    pub title: &'static str,
    pub lyrics: &'static str,
    /// Credit list, in order. Empty for songs with no known artist.
    pub artists: &'static [&'static str],
    pub created_at: &'static str,
    pub updated_at: &'static str,
    pub view_count: u32,
}

/// The artists credited by [`SONGS`], keyed by `fullname`.
pub const ARTISTS: &[FixtureArtist] = &[
    FixtureArtist {
        id: r#"1kvm9y2tcplm43wgeuni"#,
        fullname: r#"Jonas"#,
        created_at: r#"2024-09-13T20:14:38.229323832Z"#,
        updated_at: r#"2024-09-29T16:13:50.161997956Z"#,
    },
    FixtureArtist {
        id: r#"uynyy7ei47uolm5ns3h2"#,
        fullname: r#"Apatea Flores"#,
        created_at: r#"2024-08-28T16:57:28.361141126Z"#,
        updated_at: r#"2024-09-29T16:13:50.162647406Z"#,
    },
    FixtureArtist {
        id: r#"508no60p9ir9unq2kz58"#,
        fullname: r#"Teiho Tetoofa"#,
        created_at: r#"2024-08-28T16:57:28.361868055Z"#,
        updated_at: r#"2024-09-29T16:13:50.162214058Z"#,
    },
];

/// Four real songs.
pub const SONGS: &[Fixture] = &[
    Fixture {
        id: r#"273mwnta2a7v3l8ewn38"#,
        title: r#"Māmā Tahiti"#,
        lyrics: r#"Le reine des rei<sup data-nosnippet="">F</sup>nes<div>La plus belle des tahitiennes</div><div>C'est Māmā Tahi<sup data-nosnippet="">Bb</sup>ti</div><div>C'est écri<sup data-nosnippet="">Gm</sup>t dans le coe<sup data-nosnippet="">C</sup>ur de tous ses enfants<sup data-nosnippet="">F</sup>&nbsp; &nbsp; &nbsp;&nbsp;<sup data-nosnippet="">C</sup></div><div><br></div><div>C'est celle que j'ai<sup data-nosnippet="">F</sup>me</div><div>Perle polynésienne</div><div>C'est Māmā Tahi<sup data-nosnippet="">Bb</sup>ti</div><div>Tout le monde l'envi<sup data-nosnippet="">C</sup>e, la tahitienne du pacifique<sup data-nosnippet="">F</sup>&nbsp; &nbsp; &nbsp;&nbsp;<sup data-nosnippet="">C</sup></div><div><br></div><div>Māmā Tahi<sup data-nosnippet="">F</sup>ti, Tahiti Nui<sup data-nosnippet="">F7</sup></div><div>E fa'ano'ano<sup data-nosnippet="">Bb</sup>'a 'oe</div><div>Nō te 'apera<sup data-nosnippet="">Gm</sup>, tā 'oe mau ta<sup data-nosnippet="">C</sup>ma</div><div>I roto<sup data-nosnippet="">F</sup> i tō 'ā'a<sup data-nosnippet="">C</sup>u</div><div>Māmā Tahi<sup data-nosnippet="">F</sup>ti, Tahiti Nui<sup data-nosnippet="">F7</sup></div><div>Ma belle reine du pa<sup data-nosnippet="">Bb</sup>cifique</div><div>Belle vahine<sup data-nosnippet="">Gm</sup>, qui m'a tant aimé<sup data-nosnippet="">C</sup></div><div>Comme une pun<sup data-nosnippet="">Bb</sup>a h<sup data-nosnippet="">C</sup>ere<sup data-nosnippet="">F</sup></div>"#,
        artists: &[r#"Jonas"#],
        created_at: r#"2024-09-13T20:14:38.232843098Z"#,
        updated_at: r#"2024-09-19T17:50:46.366892652Z"#,
        view_count: 125,
    },
    Fixture {
        id: r#"luvw0mxon9hov8w6y2bk"#,
        title: r#"Ratatum et ratamtam"#,
        lyrics: r#"Les tahiti<sup data-nosnippet="true">D</sup>ennes se balancent au ryt<sup data-nosnippet="true">G</sup>hme du ta<sup data-nosnippet="true">D</sup>mtam<div>Ratatum<sup data-nosnippet="true">A</sup>, ratatum et ratamta<sup data-nosnippet="true">D</sup>m</div><div>Mais de te voi<sup data-nosnippet="true">D</sup>r dans les bras d'une a<sup data-nosnippet="true">G</sup>utre<sup data-nosnippet="true">D</sup></div><div>Mon coeur bou<sup data-nosnippet="true">G</sup>t le tamta<sup data-nosnippet="true">D</sup>m</div><div>Ratatum<sup data-nosnippet="true">A</sup>, ratatum et ratamta<sup data-nosnippet="true">D</sup>m&nbsp; &nbsp; &nbsp; &nbsp;<sup data-nosnippet="true">D7</sup></div><div><br></div><div>Mais pour fui<sup data-nosnippet="true">G</sup>r mes ennuis</div><div>Je préfère danse<sup data-nosnippet="true">D</sup>r</div><div>Au ryt<sup data-nosnippet="true">A</sup>hme du tamtam</div><div>Ratatum, ratatum et ratamta<sup data-nosnippet="true">D</sup>m</div><div><br></div><div>Oh oui alo<sup data-nosnippet="true">A</sup>rs mon coeur en joie</div><div>Mon coeur c'est le tamta<sup data-nosnippet="true">D</sup>m</div>"#,
        artists: &[],
        created_at: r#"2025-02-13T21:26:38.549752320Z"#,
        updated_at: r#"2025-02-13T21:26:38.549777547Z"#,
        view_count: 38,
    },
    Fixture {
        id: r#"4cfl27ia9hndgetgr1o7"#,
        title: r#"Te here fenua"#,
        lyrics: r#"O<sup data-nosnippet>F</sup> vau nei, e tu<sup data-nosnippet>C</sup>puraa ia<div>No te he<sup data-nosnippet>Bb</sup>re o te Atua<sup data-nosnippet>Dm     C</sup></div><div>Ua ti<sup data-nosnippet>F</sup>'a iana</div><div>I te tu<sup data-nosnippet>C</sup>'u mai ia'u i te ao ma<sup data-nosnippet>Bb</sup>ohi</div><div>Maohi nei<sup data-nosnippet>C</sup></div><div><br></div><div>Opua<sup data-nosnippet>F</sup>ra'a tei tupu</div><div>Fa'a<sup data-nosnippet>C</sup>ri'i poupou te fenua ia'u</div><div>Ua<sup data-nosnippet>Bb</sup> mahora te rima o te mau<sup data-nosnippet>Dm</sup> tupuna<sup data-nosnippet>C</sup></div><div>I roto<sup data-nosnippet>Bb</sup> i te here<sup data-nosnippet>C</sup></div><div><br></div><div>E ma'ama<sup data-nosnippet>F</sup> teie</div><div>No roto mai<sup data-nosnippet>C</sup> te here fenua</div><div>Ua utu<sup data-nosnippet>Bb</sup>utu mai ia'u</div><div>Na roto<sup data-nosnippet>Dm</sup> i te ha'api<sup data-nosnippet>C</sup>'ira'a</div><div>O<sup data-nosnippet>Bb</sup> ta'u i mau mai<sup data-nosnippet>C</sup>, i mau mai<sup data-nosnippet>F</sup></div><div><br></div><div>'A 'amu<sup data-nosnippet>Dm</sup>, 'a 'amu i te here fenua<sup data-nosnippet>Am</sup></div><div>Fa'ateniteni i te Atua<sup data-nosnippet>Bb</sup></div><div>I te tama maohi e<sup data-nosnippet>F</sup></div><div>A haere ra<sup data-nosnippet>C</sup></div><div>Na te ta'i ara<sup data-nosnippet>Gm</sup> o te fenua nei<sup data-nosnippet>C</sup></div><div>Ha'amana'o<sup data-nosnippet>Gm</sup> 'oe i tera ma'a parau<sup data-nosnippet>C</sup> iti</div><div><br></div><div>Tahiri mai<sup data-nosnippet>F</sup> i ni'a i te Atua</div><div>Tahiri mai<sup data-nosnippet>C</sup> i ni'a i te fenua</div><div>E nāna<sup data-nosnippet>Bb</sup> e ra'atira ia 'oe<sup data-nosnippet>C</sup></div><div>Na to 'oe fa<sup data-nosnippet>Bb</sup>'aro'o e tau<sup data-nosnippet>C</sup> o i to tere<sup data-nosnippet>F</sup></div>"#,
        artists: &[r#"Apatea Flores"#, r#"Teiho Tetoofa"#],
        created_at: r#"2024-08-28T16:57:28.363425698Z"#,
        updated_at: r#"2024-09-19T17:50:46.367112190Z"#,
        view_count: 167,
    },
    Fixture {
        id: r#"ieu8uhm9007h6l7tsue9"#,
        title: r#"E hīmene"#,
        lyrics: r#"(o) Kare kare iā<sup data-nosnippet="true">C</sup>na<div>'Ia vai roto ru<sup data-nosnippet="true">F</sup>a</div><div>E atu tō 'oe i<sup data-nosnippet="true">G</sup>ne</div><div>'A tu'u iāna e<sup data-nosnippet="true">C</sup></div><div><br></div><div>E hīmene<sup data-nosnippet="true">F</sup> ho'i mai ra<sup data-nosnippet="true">C</sup></div><div>'A mate 'ava<sup data-nosnippet="true">G</sup></div><div>Tē arofa e<sup data-nosnippet="true">C</sup></div>"#,
        artists: &[],
        created_at: r#"2024-12-17T16:18:39.134244061Z"#,
        updated_at: r#"2024-12-17T16:18:39.134276924Z"#,
        view_count: 77,
    },
];

/// Insert [`ARTISTS`] and [`SONGS`] into `pool`.
///
/// Idempotent per row (`INSERT OR IGNORE`), so a test may seed a pool that
/// already holds some of them.
pub async fn seed(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for artist in ARTISTS {
        sqlx::query(
            "INSERT OR IGNORE INTO artist (id, fullname, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(artist.id)
        .bind(artist.fullname)
        .bind(artist.created_at)
        .bind(artist.updated_at)
        .execute(pool)
        .await?;
    }

    for song in SONGS {
        sqlx::query(
            "INSERT OR IGNORE INTO song \
             (id, title, lyrics, view_count, published, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6)",
        )
        .bind(song.id)
        .bind(song.title)
        .bind(song.lyrics)
        .bind(song.view_count)
        .bind(song.created_at)
        .bind(song.updated_at)
        .execute(pool)
        .await?;

        for (position, fullname) in song.artists.iter().enumerate() {
            let artist = ARTISTS
                .iter()
                .find(|a| a.fullname == *fullname)
                .expect("fixture credits an artist missing from ARTISTS");
            sqlx::query(
                "INSERT OR IGNORE INTO song_artist (song_id, artist_id, position) \
                 VALUES (?1, ?2, ?3)",
            )
            .bind(song.id)
            .bind(artist.id)
            .bind(position as i64)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// The fixture with this id, if any.
pub fn by_id(id: &str) -> Option<&'static Fixture> {
    SONGS.iter().find(|s| s.id == id)
}
