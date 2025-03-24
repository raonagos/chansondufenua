use domain::entities::Song;

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::hooks::*;

#[component]
pub fn SongPage() -> impl IntoView {
    let params = use_params_map();
    let read_id_from_params = move || params.read().get("id");

    let resource_song = Resource::new(
        move || read_id_from_params(),
        |id| async move { api::song::get_song_fetch_artist(id.unwrap_or_default()).await },
    );

    view! {
        <Meta property="fb:app_id" content="383599779228826"/>
        <Meta property="fb:pages" content="109134754150923"/>
        <Meta property="og:type" content="website"/>
        <Meta property="og:image:width" content="1200"/>
        <Meta property="og:image:height" content="630"/>
        <Meta property="og:image:type" content="image/png"/>
        <Meta name="twitter:card" content="summary_large_image"/>
        <Meta name="twitter:image:width" content="1200"/>
        <Meta name="twitter:image:height" content="628"/>
        <Meta name="twitter:creator" content="@raonagos"/>
        <Meta name="twitter:site" content="@raonagos"/>

        <Meta property="og:locale" content="ty_PF"/>
        <Meta property="og:locale:alternate" content="fr_FR"/>

        <Suspense fallback=|| {
            "Bad request dud"
        }>{move || { resource_song.get().map(|s| { s.map(|song| RenderViewSong(RenderViewSongProps { song })) }) }}</Suspense>
    }
}

#[component]
fn RenderViewSong(song: Song) -> impl IntoView {
    let data = song.get_meta_data();

    view! {
        <Title text=data.page_title.clone()/>
        <Meta name="description" content=data.meta_description/>
        <Script type_="application/ld+json">{data.meta_jsonld}</Script>

        // meta social media
        <Meta property="og:description" content=data.meta_og_description.clone()/>
        <Meta property="og:title" content=data.page_title.clone()/>
        <Meta property="og:url" content=data.meta_og_url/>

        <Meta property="og:image" content=data.meta_img_url_og/>
        <Meta property="og:image:alt" content=data.meta_og_img_alt/>

        <Meta name="twitter:title" content=data.page_title/>
        <Meta name="twitter:description" content=data.meta_og_description/>
        <Meta name="twitter:image" content=data.meta_img_url_tw/>

        <div class="song">
            <section class="title-container">
                <h1 class="title-song" inner_html=data.song_title></h1>
                <a href="/himene/api" class="title-link">
                    "Ajouter des paroles"
                </a>
            </section>
            <div class="lyrics-display lyrics-song lyrics-view" inner_html=data.song_lyrics></div>
        </div>
    }
}
