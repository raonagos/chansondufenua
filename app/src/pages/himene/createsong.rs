use crate::api::artist::get_artists;
use crate::api::song::CreateSong;

use crate::components::{
    Area, BtnAnchorChord, BtnCustomChord, CheckboxBemol, CheckboxDiez, CheckboxMinor, ID_EDITABLE,
};

use leptos::ev::{Event, KeyboardEvent};
use leptos::prelude::*;
use web_sys::HtmlInputElement;

#[component]
pub fn CreateSongPage() -> impl IntoView {
    let resource = Resource::new(
        || (),
        |_| async move { get_artists().await.unwrap_or_default() },
    );
    let for_each_r = move || resource.get().unwrap_or_default();
    let datalist_selector = "datalist-artists".to_string();
    let datalist_selector_ = datalist_selector.clone();

    let chords_name = ["C", "D", "E", "F", "G", "A", "B"];
    view! {
        <div class="create-song">
            <h1 class="title">"Ajouter des paroles"</h1>
            <ActionFormcomponent>
                <div class="mb-4">
                    <label for="title" class="form-l-title">
                        "Titre"
                    </label>
                    <input id="title" name="data[title]" class="form-title"/>
                </div>

                <div class="mb-4">
                    <Suspense>
                        <datalist id=datalist_selector.clone()>
                            <For each=for_each_r key=|a| a.get_id() let:artist>
                                <option value=artist.get_fullname()>{artist.get_fullname()}</option>
                            </For>
                        </datalist>
                    </Suspense>
                    <label for="custom-select-input" class="form-l-selected">
                        "Artistes"
                    </label>
                    <CustomSelect id="custom-select-input".to_string() name="data[artists]".to_string() datalist_selector=datalist_selector_/>
                </div>

                <div class="mb-4">
                    <label for=ID_EDITABLE class="form-l-lyrics">
                        "Paroles"
                    </label>
                    <div class="form-lyrics-w">
                        <div class="form-btn-lyrics">
                            <For each=move || chords_name key=|c| c.to_owned() let:chord_name>
                                <BtnAnchorChord chord_name=chord_name.to_string()/>
                            </For>
                            <CheckboxMinor/>
                            <CheckboxDiez/>
                            <CheckboxBemol/>
                            <BtnCustomChord/>
                        </div>
                        <Area name="data[lyrics]".to_string()/>
                    </div>
                </div>

                <div class="form-submit">
                    <button type="submit" disabled style="display:none;"></button>
                    <button type="submit" class="form-submitter">
                        "Enregistrer"
                    </button>
                </div>
            </ActionFormcomponent>
        </div>
    }
}

#[component]
fn ActionFormcomponent(children: Children) -> impl IntoView {
    let action = ServerAction::<CreateSong>::new();

    view! {
        <ActionForm action=action class:form-action=true>
            {children()}
        </ActionForm>
    }
}

#[component]
fn CustomSelect(id: String, name: String, datalist_selector: String) -> impl IntoView {
    let signal = RwSignal::new(vec![]);
    let getter = move |s: &Vec<String>| s.join(",");
    let setter = move |s: &mut Vec<String>, new_value: String| {
        let mut value = new_value;
        if value.ends_with(",") {
            value = value.replace(",", "");
        }
        if false == value.is_empty() {
            s.push(value);
        }
    };
    let (artists, set_a) = create_slice(signal, getter, setter);

    let on_change = move |evt: Event| {
        let value = event_target_value(&evt);
        if false == value.is_empty() {
            let elm = event_target::<HtmlInputElement>(&evt);
            set_a.set(value);
            elm.set_value("");
        }
    };

    let on_keyup = move |evt: KeyboardEvent| {
        let value = event_target_value(&evt);
        if false == value.is_empty() {
            let key = evt.key();
            if key.eq(",") {
                let elm = event_target::<HtmlInputElement>(&evt);
                set_a.set(value);
                elm.set_value("");
            }
        }
    };

    let delete_selected_artist = move |index: usize| {
        signal.update(|artists| {
            artists.remove(index);
        });
    };

    let for_each_a = move || signal.get().into_iter().enumerate().collect::<Vec<_>>();

    view! {
        <input type="hidden" name=name prop:value=move || artists.get()/>
        <ul class="form-ulist-selected">
            <For each=for_each_a key=|a| a.clone() let:artist>
                <li
                    class="form-list-selected"
                    role="button"
                    tabindex="0"
                    aria-label="remove artist"
                    on:click=move |_| delete_selected_artist(artist.0)
                >
                    <span>{artist.1}</span>
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                        stroke="currentColor"
                        class="size-6 form-selected-delete"
                    >
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12"></path>
                    </svg>
                </li>
            </For>
        </ul>
        <input id=id list=datalist_selector name="custom-select" on:keyup=on_keyup on:change=on_change autocomplete="off" class="form-select"/>
    }
}
