//! The song index — `/himene`.
//!
//! **Placeholder.** Step 6 of `PLAN.md` replaces this with v3's `AllSongPage`: a
//! server-rendered table of every song with real `<a href>` links.
//!
//! The page exists in this step only so the header's "Chanson" link resolves and
//! the nav can mark itself current. v3's index was client-side rendered — the
//! server sent `<td>Chargement...</td>` and the rows arrived later from a
//! hydration template, so an agent or a crawler saw no songs at all. Serving it
//! from the database, in the first byte, is the fix step 6 makes.

use topcoat::{
    Result,
    router::page,
    view::{View, view},
};

use crate::ui::theme;

/// `/himene` — every song, newest first.
#[page("/himene")]
pub async fn songs() -> Result<impl View> {
    Ok(view! {
        <section class=(theme::PAGE)>
            <h1 class="text-3xl font-bold mb-6">"Chanson"</h1>
            <p class="opacity-70">
                "La liste des chansons arrive à l'étape 6."
            </p>
        </section>
    })
}
