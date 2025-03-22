use leptos::prelude::*;
use leptos::*;

#[component]
pub(super) fn Header() -> impl IntoView {
    view! {
        <header>
            <div>
                <div>
                    <a href="/">
                        <img class="w-[64px] h-[64px]" src="/logos/logo_w144.webp" width="64" height="64" alt="logo chanson du fenua"/>
                    </a>
                </div>
                <span class="flex-1"></span>
                <BtnHamburger/>
                <nav id="navigation" class="max-md:h-0">
                    <span class="max-md:mt-6"></span>
                    <a href="/aepa" class="nav-link">
                        "Accueil"
                    </a>
                    <a href="/himene" class="nav-link">
                        "Chanson"
                    </a>
                </nav>
            </div>
        </header>
    }
}

#[component]
fn BtnHamburger() -> impl IntoView {
    let sp1_node = NodeRef::<html::Span>::new();
    let sp2_node = NodeRef::<html::Span>::new();
    let (h0, hmax) = ("max-md:h-0", "max-md:h-[120px]");
    let (w35, w28, w21) = ("w-[35px]", "w-[28px]", "w-[21px]");
    let (is_expanded, set_aria_expanded) = signal(false);

    let toggle_hamburger = move |_| {
        let nav_node = document().get_element_by_id("navigation");

        set_aria_expanded.update(|a| *a = !*a);

        if let (Some(nav_elm), Some(sp1_elm), Some(sp2_elm)) =
            (nav_node, sp1_node.get(), sp2_node.get())
        {
            if nav_elm.class_list().contains(h0)
                && sp1_elm.class_list().contains(w35)
                && sp2_elm.class_list().contains(w35)
            {
                _ = nav_elm.class_list().replace(h0, hmax);
                _ = sp1_elm.class_list().replace(w35, w28);
                _ = sp2_elm.class_list().replace(w35, w21);
            } else {
                _ = nav_elm.class_list().replace(hmax, h0);
                _ = sp1_elm.class_list().replace(w28, w35);
                _ = sp2_elm.class_list().replace(w21, w35);
            }
        }
    };

    let aria_expanded = move || {
        if is_expanded.get() {
            return "true";
        }
        "false"
    };

    view! {
        <div role="button" class="hamburger-btn" tabindex="0" aria-label="Toggle menu" aria-expanded=aria_expanded on:click=toggle_hamburger>
            <span class="hamburger w-[35px]"></span>
            <span node_ref=sp1_node class="hamburger w-[35px] transition-all"></span>
            <span node_ref=sp2_node class="hamburger w-[35px] transition-all"></span>
        </div>
    }
}
