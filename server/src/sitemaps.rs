use axum::extract::State;
use crate_core::state::AppState;

use axum::http as AxumHttp;
use axum::response::{IntoResponse, Response};

/// Generates and returns the index sitemap as an HTTP response.
///
/// This function creates an index sitemap, which is a list of URLs for the website.
/// It is used by search engines to crawl and index the site's content efficiently.
///
/// **Note:**
/// - Ensure that the `generate_index_sitemap` function is implemented to generate the
///   sitemap content correctly.
pub async fn generate_index_sitemap() -> Response {
    use AxumHttp::header::CONTENT_TYPE;
    ([(CONTENT_TYPE, "text/xml")], index_sitemap()).into_response()
}

/// Generates and returns the himene sitemap as an HTTP response.
///
/// This function creates a sitemap specific to the "Himene" section of the website.
/// It is used by search engines to crawl and index the content related to Himene.
pub async fn generate_himene_sitemap(state: State<AppState>) -> Response {
    use AxumHttp::header::CONTENT_TYPE;
    ([(CONTENT_TYPE, "text/xml")], himene_sitemap(&state).await).into_response()
}

fn index_sitemap() -> String {
    let sitemap = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <url>
                <loc>https://www.chansondufenua.pf/</loc>
                <lastmod>2024-08-21</lastmod>
                <changefreq>monthly</changefreq>
                <priority>1.0</priority>
            </url>
        </urlset>
    "#;

    sitemap.into()
}

async fn himene_sitemap(state: &AppState) -> String {
    let AppState { pool: db, .. } = state;

    let songs = db
        .get_songs(0, Some(255))
        .await
        .unwrap()
        .iter()
        .map(|song| {
            format!(
                r#"
        <url>
        <loc>https://www.chansondufenua.pf/himene/{}</loc>
        <lastmod>{}</lastmod>
        <changefreq>weekly</changefreq>
        <priority>0.9</priority>
        </url>"#,
                song.get_id(),
                song.get_updated_at().format("%Y-%m-%d")
            )
        })
        .collect::<String>();

    let sitemap = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
        {}
        </urlset>
    "#,
        songs
    );

    sitemap.into()
}
