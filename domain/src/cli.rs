// re-export Parser
pub use clap::Parser;

#[derive(Parser)]
#[command(name = "Chanson du Fenua")]
#[command(version, about = "Sing tahitian song with lyrics & chords.")]
// Add/Mark `Option` if you add more database provider or make a `feature`
pub struct AppCli {
    /// Namespace database provider
    #[arg(long, env = "DB_NAMESPACE")]
    pub db_namespace: String,

    /// Database database provider
    #[arg(long, env = "DB_DATABASE")]
    pub db_database: String,

    /// Endpoint database provider
    #[arg(long, env = "DB_ENDPOINT")]
    pub db_endpoint: String,

    /// User database provider
    #[arg(long, env = "DB_USER")]
    pub db_user: String,

    /// Password database provider
    #[arg(long, env = "DB_PASSWORD")]
    pub db_password: String,
}
