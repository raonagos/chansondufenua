#[derive(Debug, Clone, serde::Serialize, eserde::Deserialize)]
pub struct SongBodyCreation {
    pub title: String,
    /// A string of html element
    pub lyrics: String,
    pub artists: String,
}
