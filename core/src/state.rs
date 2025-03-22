use database::database::Database;

use axum::extract::FromRef;
use leptos::config::LeptosOptions;
use std::{ops::Deref, sync::Arc};

/// Shared state like database through the request. Use [`leptos_axum::extract`] to get it.
///
/// ## Example
/// ```
/// #[server]
/// pub async fn get_item() -> Result<Item, ServerFnError> {
///     ...
///     let AppState { pool: db, .. } = leptos_axum::extract::<AppState>().await?;
///     ...
/// }
/// ```
///
/// [`leptos_axum::extract`] : https://docs.rs/leptos_axum/latest/leptos_axum/fn.extract.html
#[derive(Debug, Clone, FromRef)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: Arc<dyn Database>,
}

impl Deref for AppState {
    type Target = dyn Database;

    fn deref(&self) -> &Self::Target {
        &*self.pool
    }
}
