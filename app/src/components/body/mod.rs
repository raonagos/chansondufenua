mod header;

use header::*;
use leptos::prelude::*;

#[component]
pub fn BodyWrapper(children: Children) -> impl IntoView {
    view! {
        <Header/>
        <main>{children()}</main>
        <footer>
            <p>
                "2024 Chanson du fenua. Tous droits réservés "
                <a class="hover:underline" href="https://www.rao-nagos.pf" target="_blank" rel="noopener noreferrer">
                    "❤️"
                </a> ". Contributing to this "
                <a class="hover:underline" href="https://github.com/raonagos/chansondufenua" target="_blank" rel="noopener noreferrer">
                    "project"
                </a> "."
            </p>
        </footer>
        <Analytics/>
    }
}

#[component]
fn Analytics() -> impl IntoView {
    let resource = Resource::new(
        || (),
        |_| async move { api::core::is_the_project_in_production().await },
    );

    let is_prod = move || {
        resource
            .get()
            .map(|r| r.unwrap_or_default())
            .unwrap_or_default()
    };

    view! {
        <Suspense>
            <Show when=is_prod>
                <noscript>
                    <img width="1" height="1" src="https://analytics.rao-nagos.pf/ingress/d076744f-de2d-4dab-ac1c-6d1098354bfd/pixel.gif"/>
                </noscript>
                <script defer="true" src="https://analytics.rao-nagos.pf/ingress/d076744f-de2d-4dab-ac1c-6d1098354bfd/script.js"></script>
            </Show>
        </Suspense>
    }
}
