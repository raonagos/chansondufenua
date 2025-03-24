use crate_core::state::AppState;

use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    http as AxumHttp,
    response::{IntoResponse, Response},
};
use headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption as ImageFormat;
use headless_chrome::{Browser, LaunchOptions};
use leptos::logging;
use std::sync::Arc;

type Error = Arc<anyhow::Error>;

#[derive(Debug, PartialEq, Eq)]
enum Platform {
    OG,
    TW,
}

/// Generates an Open Graph, [`Platform::OG`], image in PNG format using the `headless_chrome` crate.
///
/// This function creates an image that can be used as an Open Graph image for social media sharing.
/// It utilizes a headless Chrome browser to render the image, so a compatible Chromium-based browser
/// must be installed on the server.
///
/// **Note :**
/// - Proper error handling should be implemented to manage scenarios where image generation fails.
/// - Consider the performance implications of running a headless browser on the server, especially under high load.
pub async fn generate_og_img(
    State(state): State<AppState>,
    Path((timestamp, id)): Path<(i64, String)>,
) -> Result<Response, AxumHttp::StatusCode> {
    let image_data = generate_image_from_id(&id, timestamp, Platform::OG, &state)
        .await
        .map_err(|e| {
            logging::error!("{e}");
            AxumHttp::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(([(AxumHttp::header::CONTENT_TYPE, "image/png")], image_data).into_response())
}

/// Generates a X.com (twitter), [`Platform::TW`], image in PNG format using the `headless_chrome` crate.
pub async fn generate_tw_img(
    State(state): State<AppState>,
    Path((timestamp, id)): Path<(i64, String)>,
) -> Result<Response, AxumHttp::StatusCode> {
    let image_data = generate_image_from_id(&id, timestamp, Platform::TW, &state)
        .await
        .map_err(|e| {
            logging::error!("{e}");
            AxumHttp::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(([(AxumHttp::header::CONTENT_TYPE, "image/png")], image_data).into_response())
}

async fn generate_image_from_id(
    id: &str,
    timestamp: i64,
    platform: Platform,
    state: &AppState,
) -> Result<Vec<u8>, Error> {
    let AppState { pool: db, .. } = state;

    let song = db
        .get_one_song(id.into())
        .await
        .map_err(|_| anyhow::anyhow!("Query Error"))?
        .ok_or_else(|| anyhow::anyhow!("Song not exist"))?;

    #[allow(unused_assignments)]
    let mut html = "".to_owned();
    let updated_at = song.get_uat_timestamp();
    if timestamp != updated_at {
        return Err(anyhow::anyhow!("Timestamp mismatch").into());
    }

    let lyrics = song.get_lyrics();
    html = format!(
        r#"<html>
    <head>
        <style>
        body, html {{
            overflow: hidden;
            height: 100%;
            margin: 0;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        sup {{
            display: none;
        }}
        .lyrics-display {{
            color: black;
            font-size: 2rem;
            text-align: center;
        }}
        .lyrics-display * {{
            padding: 4px 0px;
        }}
        </style>
    </head>
    <body>
        <div class="lyrics-display lyrics-song lyrics-view">{}</div>
    </body>
</html>"#,
        lyrics
    );

    // Set viewport based on platform
    let (width, height) = match platform {
        Platform::OG => (1200, 630),
        Platform::TW => (1200, 628),
    };

    use std::ffi::OsStr;

    let dis_gpu = "--disable-gpu";
    let dis_rasterizer = "--disable-software-rasterizer";

    // Initialize headless Chrome
    let options = LaunchOptions::default_builder()
        .headless(true)
        .args(vec![OsStr::new(dis_gpu), OsStr::new(dis_rasterizer)])
        .window_size(Some((width, height)))
        .build()
        .map_err(|_| anyhow!("LaunchOptions failed"))?;

    let browser = Browser::new(options)?;
    let tab = browser.new_tab()?;

    let html = html.escape_default().to_string();

    // Set HTML content
    tab.evaluate(
        &format!(
            r#"(function() {{
        let html = "{}";
        
        document.open();
        document.write(html);
        document.close();
        
    }})()"#,
            html
        ),
        false,
    )?;

    tab.find_element("body")?;

    // Take screenshot
    let screenshot = tab.capture_screenshot(ImageFormat::Png, Some(90), None, false)?;

    Ok(screenshot)
}
