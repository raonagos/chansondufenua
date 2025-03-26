use axum::extract::State;
use crate_core::state::AppState;

use axum::response::{IntoResponse, Response};

//todo: return an error response instead of unwrap

/// Generates and returns the himene sitemap as an HTTP response.
///
/// This function creates a sitemap specific to the "Himene" section of the website.
/// It is used by search engines to crawl and index the content related to Himene.
pub async fn generate_himene_sitemap(state: State<AppState>) -> Response {
    use axum::http::header::CONTENT_TYPE;
    ([(CONTENT_TYPE, "text/xml")], himene_sitemap(&state).await).into_response()
}

async fn himene_sitemap(state: &AppState) -> String {
    let AppState { pool: db, .. } = state;

    let url_elements = db
        .get_songs(0, Some(255))
        .await
        .unwrap()
        .iter()
        .map(url_element)
        .collect::<String>();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
            <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
                {}
            </urlset>"#,
        url_elements
    )
}

fn url_element(song: &domain::entities::Song) -> String {
    let id = song.get_id();
    let lastmod = song.get_updated_at().format("%Y-%m-%d");
    format!(
        r#"<url>
            <loc>https://www.chansondufenua.pf/himene/{}</loc>
            <lastmod>{}</lastmod>
            <changefreq>weekly</changefreq>
            <priority>0.9</priority>
        </url>"#,
        id, lastmod
    )
}
