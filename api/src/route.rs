use super::Result;

use leptos::{prelude::*, server_fn::codec::GetUrl};

#[server(prefix = "/haha", endpoint = "sitemap.xml", input = GetUrl)]
pub async fn sitemap() -> Result<String> {
    let sitemap = r#"<?xml version="1.0" encoding="UTF-8"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
        <url>
        <loc>https://www.chansondufenua.pf/</loc>
        <lastmod>2024-08-21</lastmod>
        <changefreq>monthly</changefreq>
        <priority>1.0</priority>
        </url>
        </urlset>
    "#
    .to_string();

    use axum::http::{header, HeaderValue};

    let response = expect_context::<leptos_axum::ResponseOptions>();
    let xlm_value = HeaderValue::from_static("text/xml");
    response.insert_header(header::CONTENT_TYPE, xlm_value);

    Ok(sitemap)
}

#[server(prefix = "/haha/og/:timestamp/himene/:id", endpoint = "", input = GetUrl)]
pub async fn og_image_generator() -> Result<()> {
    use axum::extract::Path;

    let Path((timestamp, id)) = leptos_axum::extract::<Path<(i64, String)>>().await?;

    leptos::logging::debug_warn!("{id} {timestamp}");

    Ok(())
}
