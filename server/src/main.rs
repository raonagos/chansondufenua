mod cache;
mod image;
mod sitemap;

use crate::image::*;
use crate::sitemap::*;

#[allow(unused_imports)]
use api::shared::*;
use app::App;
use crate_core::state::AppState;
use database::init_database;
use domain::cli::{AppCli, Parser};

use axum::{middleware, routing::get, Router};
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use leptos_meta::*;
use std::sync::Arc;
use tower_http::compression::{CompressionLayer, CompressionLevel};

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
    // load environment variables from .env file & check configuration environment
    _ = dotenvy::dotenv();
    _ = AppCli::parse();

    // init logger
    simple_logger::SimpleLogger::new()
        .with_module_level("html5ever", log::LevelFilter::Off)
        .env()
        .init()
        .expect("couldn't initialize the logger");

    let conf = get_configuration(None).expect("couldn't initialize the leptos configuration");
    let leptos_options = conf.leptos_options;

    // create db client & a shared state
    let pool = init_database().await.expect("couldn't initialize the database");
    let state = AppState {
        pool: Arc::new(pool),
        leptos_options: leptos_options.clone(),
    };

    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let compression = CompressionLayer::new().quality(CompressionLevel::Precise(7));

    let app = Router::new()
        .leptos_routes_with_context(
            &state,
            routes,
            {
                let state_ = state.clone();
                move || provide_context(state_.clone())
            },
            {
                let leptos_options_ = state.leptos_options.clone();
                move || shell(leptos_options_.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .route("/himene/sitemap.xml", get(generate_himene_sitemap))
        .route("/drive/genog/:timestamp/himene/:id", get(generate_og_img))
        .route("/drive/gentw/:timestamp/himene/:id", get(generate_tw_img))
        .layer(compression)
        .layer(middleware::from_fn(cache::handle))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html> 
        <html lang="fr" class="dark">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#0891b2" media="(prefers-color-scheme: light)"/>
                <meta name="theme-color" content="#155e75" media="(prefers-color-scheme: dark)"/>
                <link
                    rel="preload stylesheet"
                    r#type="text/css"
                    r#as="style"
                    href="https://fonts.googleapis.com/css2?family=Roboto+Serif:wght@400;700&display=swap"
                />
                <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Roboto+Serif:wght@400;700&display=swap"/>
                <link rel="shortcut icon" href="/logos/logo_b32.ico" r#type="image/x-icon" sizes="32x32" media="(prefers-color-scheme: light)"/>
                <link rel="shortcut icon" href="/logos/logo_w32.ico" r#type="image/x-icon" sizes="32x32" media="(prefers-color-scheme: dark)"/>

                <AutoReload options=options.clone()/>
                <HydrationScripts options=options.clone()/>
                <HashedStylesheet id="leptos" options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}
