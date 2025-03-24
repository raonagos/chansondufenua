use domain::entities::Song;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn AllSongPage() -> impl IntoView {
    let params = use_params_map();
    let resource = Resource::new(
        move || {
            let page = params.with(|pm| {
                if let Some(page) = pm.get("page") {
                    return page;
                }
                0_u8.to_string()
            });
            page.parse::<u8>().unwrap_or_default()
        },
        |page| async move {
            use api::song::get_songs;
            get_songs(page, Some(255)).await.unwrap_or_default()
        },
    );

    let songs = move || resource.get().unwrap_or_default();

    let show_fk = || {
        view! {
            <tr>
                <td colspan="2" class="text-center py-4">
                    "Pas de chanson"
                </td>
            </tr>
        }
    };

    let suspense_fk = || {
        view! {
            <tr>
                <td colspan="2" class="text-center py-4">
                    "Chargement..."
                </td>
            </tr>
        }
    };

    view! {
        <div class="all-song">
            <h1 class="title">"Toutes les chansons"</h1>
            <div class="tab-container">
                <table class="w-full">
                    <thead>
                        <tr>
                            <th class="tab-th">"Titre"</th>
                            <th class="tab-th hidden md:table-cell">"Artiste"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <Suspense fallback=suspense_fk>
                            <Show when=move || !songs().is_empty() fallback=show_fk>
                                <For each=songs key=|s| s.get_id() let:song>
                                    <Row song/>
                                </For>
                            </Show>
                        </Suspense>
                    </tbody>
                </table>
            </div>
        </div>
    }
}

#[component]
fn Row(song: Song) -> impl IntoView {
    let href = format!("/himene/{}", song.get_id());
    let artists = song
        .get_artists()
        .iter()
        .map(|a| a.get_fullname())
        .collect::<Vec<String>>()
        .join(", ");

    view! {
        <tr
            class="tab-row"
            role="button"
            tabindex="0"
            aria-label=format!("Go to the song {}", song.get_title())
            onclick=format!("window.location='{}'", href)
        >
            <td class="tab-cell">
                <a href class="tab-link">
                    {song.get_title()}
                </a>
            </td>
            <td class="tab-cell hidden md:table-cell">{artists}</td>
        </tr>
    }
}
