use database::database::Database;

use axum::extract::FromRef;
use leptos::config::LeptosOptions;
use std::{ops::Deref, sync::Arc};

/// Shared state like database through the request. Use [`expect_context`](https://docs.rs/leptos/latest/leptos/prelude/fn.expect_context.html) to get it or [`State`](https://docs.rs/axum/latest/axum/extract/struct.State.html) with middleware.
///
/// ## With `#[server]` fn
/// ```
/// #[server]
/// pub async fn get_item() -> Result<Item, ServerFnError> {
///     ...
///     let AppState { pool: db, .. } = expect_context::<AppState>();
///     ...
/// }
/// ```
///
/// ## With middleware
/// ```
/// pub async fn get_item(State(state): State<AppState>) -> _ {
///     ...
///     let AppState { pool: db, .. } = state;
///     ...
/// }
/// ```
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
