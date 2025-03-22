use super::Result;

use leptos::prelude::*;

/// Returns true if the server runs as in production, else false.
#[server(prefix = "/bundle", endpoint = "i")]
pub async fn is_the_project_in_production() -> Result<bool> {
    use crate_core::state::AppState;

    let AppState { leptos_options, .. } = expect_context::<AppState>();

    match leptos_options.env {
        Env::PROD => Ok(true),
        _ => Ok(false),
    }
}
