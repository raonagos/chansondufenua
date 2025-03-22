use crate::components::BodyWrapper;
use crate::pages::*;

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::*;

/// Entrypoint component for web app.
///
/// Provides the routes and the error pages.
#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Title text="Chanson du Fenua"/>
        // ! seriously ?? if remove, hydrate navigation bugs
        <Stylesheet id="leptos" href=""/>

        <Router>
            <BodyWrapper>
                <Routes fallback=|| "La page n'existe pas.".into_view()>
                    <Route path=StaticSegment("") view=|| HomePage(HomePageProps { canonical: false })/>
                    <Route path=StaticSegment("aepa") view=|| HomePage(HomePageProps { canonical: true })/>
                    <ParentRoute path=StaticSegment("himene") view=Outlet>
                        <Route path=StaticSegment("") view=AllSongPage/>
                        <Route path=StaticSegment("api") view=CreateSongPage/>
                        // run this route in Async + <Suspense/>
                        <Route path=ParamSegment("id") view=SongPage ssr=SsrMode::Async/>
                    </ParentRoute>
                </Routes>
            </BodyWrapper>
        </Router>
    }
}
