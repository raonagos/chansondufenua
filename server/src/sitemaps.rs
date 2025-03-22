use axum::extract::State;
use crate_core::state::AppState;

use axum::http as AxumHttp;
use axum::response::{IntoResponse, Response};

pub async fn index_sitemap() -> Response {
    use AxumHttp::header::CONTENT_TYPE;
    ([(CONTENT_TYPE, "text/xml")], generate_index_sitemap()).into_response()
}

pub async fn himene_sitemap(state: State<AppState>) -> Response {
    use AxumHttp::header::CONTENT_TYPE;
    (
        [(CONTENT_TYPE, "text/xml")],
        generate_himene_sitemap(&state).await,
    )
        .into_response()
}

fn generate_index_sitemap() -> String {
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

async fn generate_himene_sitemap(state: &AppState) -> String {
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
