mod cache;
mod image;
mod sitemap;

use crate::image::*;
use crate::sitemap::*;

#[allow(unused_imports)]
use api::shared::*;
use app::{shell, App};
use crate_core::state::AppState;
use database::init_database;
use domain::cli::{AppCli, Parser};

use axum::{middleware, routing::get, Router};
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
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
    let pool = init_database()
        .await
        .expect("couldn't initialize the database");
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
