use domain::entities::Song;

use leptos::prelude::*;
use leptos_meta::*;

#[component]
pub fn HomePage(#[prop(optional)] canonical: bool) -> impl IntoView {
    view! {
        <Show when=move || canonical>
            <Link rel="canonical" href="https://www.chansondufenua.pf"/>
        </Show>
        <Meta
            name="description"
            content="Découvrez un monde musical unique où l'art des accords et la maîtrise des mélodies élèvent votre musique vers de nouveaux sommets. Laissez-vous séduire par la poésie des paroles, véritable fenêtre sur l'âme tahitienne, qui vous transporte dans un voyage lyrique et émouvant. Plongez dans une collection raffinée de chansons tahitiennes, savamment sélectionnées pour allier tradition et modernité, offrant une expérience musicale enrichissante et inoubliable. Découvrez des mélodies envoûtantes et des rythmes captivants, célébrant la richesse culturelle de Tahiti et invitant les auditeurs à explorer et apprécier la beauté et la profondeur de cette culture unique. Plongez dans une collection raffinée de chansons tahitiennes, alliant tradition et modernité. Chanson du fenua, retrouvez vos paroles de chanson tahitiennes et polynésiennes."
        />
        <div class="home">
            <Title/>
            <Card/>
            <section class="mb-20">
                <p class="text-syno">
                    "Découvrez un monde musical unique où l'art des accords et la maîtrise des mélodies élèvent votre musique vers de nouveaux sommets. Laissez-vous séduire par la poésie des paroles, véritable fenêtre sur l'âme tahitienne, qui vous transporte dans un voyage lyrique et émouvant. Plongez dans une collection raffinée de chansons tahitiennes, savamment sélectionnées pour allier tradition et modernité, offrant une expérience musicale enrichissante et inoubliable. Découvrez des mélodies envoûtantes et des rythmes captivants, célébrant la richesse culturelle de Tahiti et invitant les auditeurs à explorer et apprécier la beauté et la profondeur de cette culture unique."
                </p>
            </section>
            <Table1/>
            <Table2/>
            <Footer/>
        </div>
    }
}

#[component]
fn Title() -> impl IntoView {
    view! {
        <section class="title-container">
            <h1 class="title">"Chanson du fenua"</h1>
            <p class="subtitle">"L'élégance de la musique polynésienne"</p>
            <a href="/himene" class="title-link">
                "Découvrir les chansons"
            </a>
        </section>
    }
}

#[component]
fn Card() -> impl IntoView {
    view! {
        <section class="card-container">
            <div class="card">
                <h2 class="card-title">"Chansons exquises"</h2>
                <p>"Plongez dans une collection raffinée de chansons tahitiennes, alliant tradition et modernité."</p>
            </div>
            <div class="card">
                <h2 class="card-title">"Paroles envoûtantes"</h2>
                <p>"Laissez-vous séduire par la poésie des paroles, une fenêtre sur l'âme tahitienne."</p>
            </div>
            <div class="card">
                <h2 class="card-title">"Harmonies divines"</h2>
                <p>"Maîtrisez l'art des accords et élevez votre musique vers de nouveaux sommets."</p>
            </div>
        </section>
    }
}

#[component]
fn Table1() -> impl IntoView {
    let resource = Resource::new(
        || (),
        |_| async move {
            use crate::api::song::get_songs_fetch_artist;
            get_songs_fetch_artist(0, Some(5)).await.unwrap_or_default()
        },
    );

    let songs = move || resource.get().unwrap_or_default();

    view! {
        <section class="tab-container">
            <div class="tab-subcontainer text-right md:ml-auto">
                <h2 class="tab-title">"Les dernières ajouts"</h2>
                <table class="max-md:mr-auto ml-auto">
                    <Suspense>
                        <tbody>
                            <For each=songs key=|s| s.get_id() let:song>
                                <SongRow inverse=true song/>
                            </For>
                        </tbody>
                    </Suspense>
                </table>
            </div>
        </section>
    }
}

#[component]
fn Table2() -> impl IntoView {
    let resource_by_view = Resource::new(
        || (),
        |_| async move {
            use crate::api::song::get_songs_fetch_artist_by_view;
            get_songs_fetch_artist_by_view(0, Some(5))
                .await
                .unwrap_or_default()
        },
    );

    let songs_by_view = move || resource_by_view.get().unwrap_or_default();

    view! {
        <section class="tab-container">
            <div class="tab-subcontainer">
                <h2 class="tab-title">"Les plus vues"</h2>
                <table class="max-md:mx-auto">
                    <Suspense>
                        <tbody>
                            <For each=songs_by_view key=|s| s.get_id() let:song>
                                <SongRow song/>
                            </For>
                        </tbody>
                    </Suspense>
                </table>
            </div>
        </section>
    }
}

#[component]
fn SongRow(song: Song, #[prop(optional)] inverse: bool) -> impl IntoView {
    let href = format!("/himene/{}", song.get_id());
    let href_r = href.clone();

    use leptos::either::Either;

    let left = view! {
        <td class="p-4 row-lyrics">
            <a class="tab-lyrics" href=href.clone()>
                {song.get_clean_lyrics()}
            </a>
        </td>
        <td class="p-4">
            <a href=href.clone()>{song.get_title()}</a>
        </td>
    };

    let right = view! {
        <td class="p-4">
            <a href=href_r.clone()>{song.get_title()}</a>
        </td>
        <td class="p-4 row-lyrics">
            <a class="tab-lyrics" href=href_r>
                {song.get_clean_lyrics()}
            </a>
        </td>
    };

    view! {
        <tr
            class="tab-row"
            role="button"
            tabindex="0"
            aria-label=format!("Go to the song {}", song.get_title())
            onclick=format!("window.location='{}'", href)
        >

            {if inverse { Either::Left(left) } else { Either::Right(right) }}

        </tr>
    }
}

#[component]
fn Footer() -> impl IntoView {
    view! {
        <section class="foot-container">
            <h2 class="foot-title">"Votre odyssée musicale commence ici"</h2>
            <p class="foot-text">"Rejoignez la communauté des passionnés de la musique polynésienne"</p>
            <div class="foot-link-container">
                <a href="https://facebook.com/chansondufenua" class="foot-link-media">
                    "Facebook"
                </a>
                <a href="/himene/api" attr:class="foot-link">
                    "C'est parti !"
                </a>
            </div>
        </section>
    }
}
