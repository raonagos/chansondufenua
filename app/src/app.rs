use crate::components::BodyWrapper;
use crate::pages::*;

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::*;
use leptos_router::*;

//todo: add an error page handler to the routes

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html> 
        <html lang="fr" class="dark">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#0891b2" media="(prefers-color-scheme: light)"/>
                <meta name="theme-color" content="#155e75" media="(prefers-color-scheme: dark)"/>
                <link
                    rel="preload stylesheet"
                    r#type="text/css"
                    r#as="style"
                    href="https://fonts.googleapis.com/css2?family=Roboto+Serif:wght@400;700&display=swap"
                />
                <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Roboto+Serif:wght@400;700&display=swap"/>
                <link rel="shortcut icon" href="/logos/logo_b32.ico" r#type="image/x-icon" sizes="32x32" media="(prefers-color-scheme: light)"/>
                <link rel="shortcut icon" href="/logos/logo_w32.ico" r#type="image/x-icon" sizes="32x32" media="(prefers-color-scheme: dark)"/>

                // todo: move css style files here
                // <script src="https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4"></script>

                <AutoReload options=options.clone()/>
                <HydrationScripts options=options.clone()/>
                <HashedStylesheet id="leptos" options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

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
                    <Route path=StaticSegment("") view=|| view! { <HomePage/> }/>
                    <Route path=StaticSegment("aepa") view=|| view! { <HomePage canonical=true/> }/>
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
