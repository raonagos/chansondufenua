//! The home page — `/`, and the same page again at `/aepa`.
//!
//! **Placeholder.** Step 5 of `PLAN.md` replaces [`body`] with v3's `HomePage`:
//! hero, the three "why" cards, the latest and most-viewed tables, and the
//! closing links. What is here now exists to prove the shell wraps a real page,
//! renders the nav's active state, and prints.

use topcoat::{
    Result,
    router::page,
    view::{View, class, component, view},
};

use crate::ui::theme;

/// `/` — the front page.
#[page("/")]
pub async fn home() -> Result<impl View> {
    Ok(view! { home_body(canonical: false) })
}

/// `/aepa` — the same page under a Tahitian path, kept because it is published
/// and linked from the header.
#[page("/aepa")]
pub async fn aepa() -> Result<impl View> {
    Ok(view! { home_body(canonical: true) })
}

/// Shared body.
///
/// Named `home_body` rather than `body` on purpose: `#[page]` expands to code
/// that names Topcoat's `Body` type, and a component called `body` collides with
/// it.
///
/// A `#[component]` rather than a plain helper: `view!` needs the request
/// context, which `#[page]` and `#[component]` bind and a bare `async fn` does
/// not.
///
/// `canonical` is the only difference between the two routes. v3 rendered the
/// same component with `canonical=true` at `/aepa`; step 5 turns that into the
/// `<link rel="canonical">` that stops two URLs for one page competing in search
/// results.
#[component]
pub async fn home_body(canonical: bool) -> Result<impl View> {
    Ok(view! {
        <section class=(class!(theme::PAGE, "text-center"))>
            <h1 class="text-5xl font-bold mb-6 dark:text-tahiti">
                "Chanson du fenua"
            </h1>
            <p class="text-2xl italic mb-10 dark:text-gray-400">
                "Les plus belles chansons du fenua, avec paroles et accords."
            </p>
            <p class="text-sm opacity-70">
                if canonical {
                    "(servie aussi à /aepa)"
                } else {
                    ""
                }
            </p>
        </section>
    })
}
