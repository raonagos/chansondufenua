use crate::database::Database;

use domain::entities::{Artist, Song};
use domain::error::AppError;
use domain::forms::SongBodyCreation;
use domain::result::AppResult;

use eserde::Deserialize;
use serde::Serialize;
use surrealdb::engine::remote::ws::Client;
use surrealdb::sql::Datetime;
use surrealdb::{Connection, RecordId, Surreal};

/// Define an asynchronous function to get the surreal database instance.
///
/// It reads the environment variables from the .env file, connects to the database using the WebSocket protocol, sets the namespace and database to use, sets a random user & password, and returns the database instance.
///
/// ## Get database client
///
/// ```rust
/// let db_instance = init().await.expect("couldn't init surrealdb");
/// ```
pub async fn init() -> AppResult<Surreal<Client>> {
    use domain::cli::{AppCli, Parser};

    use surrealdb::engine::remote::ws::Ws;
    use surrealdb::opt::auth::Database as AuthDatabase;

    // clap crates
    let AppCli {
        db_namespace: namespace,
        db_database: database,
        db_endpoint: endpoint,
        db_user: username,
        db_password: password,
    } = AppCli::parse();

    #[allow(non_snake_case)]
    let DB: Surreal<Client> = Surreal::init();

    DB.connect::<Ws>(endpoint).await?;
    DB.use_ns(&namespace).use_db(&database).await?;
    DB.signin(AuthDatabase {
        namespace: &namespace,
        database: &database,
        username: &username,
        password: &password,
    })
    .await?;

    log::debug!("DB INITIALIZE SUCCESSFUL");

    Ok(DB)
}

// impl

#[async_trait::async_trait]
impl<C: Connection> Database for Surreal<C> {
    async fn create_song(&self, data: SongBodyCreation) -> AppResult<Song> {
        use surrealdb::sql::statements::{BeginStatement, CommitStatement};

        // clean tag & attrs elements, also allow `data-nosnippet` to tell to google :
        // « hey man, you present my website ugly ! Please do not snippet that. Thx. Love u »
        let clean_html = |input: &str| -> String {
            let allowed_attributes = vec!["data-nosnippet"];
            let cleaner = ammonia::Builder::default()
                .add_generic_attributes(&allowed_attributes)
                .clean(input);
            cleaner.to_string()
        };

        let lyrics = clean_html(&data.lyrics);

        let mut resource = self
            .query(BeginStatement::default())
            // ! why between statements ? this function should to revert all the changes if it fails
            .query("fn::create_song($title, $lyrics, $fullnames);")
            .query(CommitStatement::default())
            .bind(("title", data.title))
            .bind(("lyrics", lyrics))
            .bind(("fullnames", data.artists))
            .await?;

        resource
            .take::<Option<DbSong>>(0)?
            .map(Song::from)
            .ok_or_else(|| AppError::Unknown)
    }

    async fn get_songs(&self, page: u8, limit: Option<u8>) -> AppResult<Vec<Song>> {
        let mut response = self
            .query("RETURN fn::get_songs_fetch_artist($page, $limit);")
            .bind(("page", page))
            .bind(("limit", limit))
            .await?;

        response
            .take::<Vec<DbSongFetchArtist>>(0)
            .map(|songs| songs.into_iter().map(Song::from).collect::<Vec<_>>())
            .map_err(AppError::from)
    }

    async fn get_songs_by_view(&self, page: u8, limit: Option<u8>) -> AppResult<Vec<Song>> {
        let mut response = self
            .query("RETURN fn::get_songs_fetch_artist_by_view($page, $limit);")
            .bind(("page", page))
            .bind(("limit", limit))
            .await?;

        response
            .take::<Vec<DbSongFetchArtist>>(0)
            .map(|songs| songs.into_iter().map(Song::from).collect::<Vec<_>>())
            .map_err(AppError::from)
    }

    async fn get_one_song(&self, id: String) -> AppResult<Option<Song>> {
        let r = surrealdb::RecordId::from_table_key(DbSongFetchArtist::TAG, id);

        let mut response = self
            .query("RETURN fn::get_song_fetch_artist(<record> $id)")
            .bind(("id", r))
            .await?;

        response
            .take::<Option<DbSongFetchArtist>>(0)
            .map(|o| o.map(Song::from))
            .map_err(AppError::from)
    }

    async fn get_artists(&self) -> AppResult<Vec<Artist>> {
        self.select::<Vec<DbArtist>>(DbArtist::TAG)
            .await
            .map(|artists| artists.into_iter().map(Artist::from).collect::<Vec<_>>())
            .map_err(AppError::from)
    }
}

// entities

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Structure representing an artist in the database
pub struct DbArtist {
    /// Unique identifier of the entity
    #[eserde(compat)]
    pub id: Option<RecordId>,
    pub fullname: String,
    #[eserde(compat)]
    pub created_at: Datetime,
    #[eserde(compat)]
    pub updated_at: Datetime,
}

impl DbArtist {
    pub const TAG: &'static str = "artist";
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Structure representing a song in the database.
pub struct DbSong {
    #[eserde(compat)]
    pub id: Option<RecordId>,
    pub title: String,
    pub lyrics: String,
    pub view_count: u32,
    #[eserde(compat)]
    pub artists: Vec<RecordId>,
    pub published: bool,
    #[eserde(compat)]
    pub created_at: Datetime,
    #[eserde(compat)]
    pub updated_at: Datetime,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
/// Structure representing a song in the database with artists.
pub struct DbSongFetchArtist {
    #[eserde(compat)]
    pub id: Option<RecordId>,
    pub title: String,
    pub lyrics: String,
    pub view_count: u32,
    pub artists: Vec<DbArtist>,
    pub published: bool,
    #[eserde(compat)]
    pub created_at: Datetime,
    #[eserde(compat)]
    pub updated_at: Datetime,
}

impl DbSongFetchArtist {
    pub const TAG: &'static str = "song";
}

// impl

impl From<DbSongFetchArtist> for Song {
    fn from(song_: DbSongFetchArtist) -> Self {
        let id = song_
            .id
            .map(|r| r.key().to_string())
            .unwrap_or("undefined_song_id".into());

        let artists = song_
            .artists
            .into_iter()
            .map(Artist::from)
            .collect::<Vec<_>>();

        Self::new(
            id,
            song_.title,
            song_.lyrics,
            song_.view_count,
            artists,
            song_.published,
            song_.created_at.into(),
            song_.updated_at.into(),
        )
    }
}

impl From<DbSong> for Song {
    fn from(song_: DbSong) -> Self {
        let id = song_
            .id
            .map(|r| r.key().to_string())
            .unwrap_or("undefined_song_id".into());

        let artists = song_
            .artists
            .iter()
            .map(|r| Artist::with_id(r.to_string()))
            .collect::<Vec<_>>();

        Self::new(
            id,
            song_.title,
            song_.lyrics,
            song_.view_count,
            artists,
            song_.published,
            song_.created_at.into(),
            song_.updated_at.into(),
        )
    }
}

impl From<DbArtist> for Artist {
    fn from(artist_: DbArtist) -> Self {
        let id = artist_
            .id
            .map(|r| r.to_string())
            .unwrap_or("undefined_artist_id".into());

        Self::new(
            id,
            artist_.fullname,
            artist_.created_at.into(),
            artist_.updated_at.into(),
        )
    }
}
