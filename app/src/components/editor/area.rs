use super::utils::*;
use super::*;

use leptos::prelude::*;
use leptos::*;

#[component]
pub fn Area(name: String) -> impl IntoView {
    let (ta_value, set_ta_value) = signal(String::default());

    let update_ta_value = move || {
        let ta_elm = document().get_element_by_id(ID_EDITABLE).unwrap();
        let value = ta_elm.inner_html();
        set_ta_value.set(value);
    };

    let on_kd = move |evt: ev::KeyboardEvent| {
        let selection = window().get_selection().unwrap().unwrap();
        let selection_node = selection.anchor_node().unwrap();
        let is_ancestor = contains_ancestor(&selection_node, "SUP");

        let key = evt.key();
        // <-- test french & others languages
        let keys_delete = ["Delete", "Backspace"];

        if keys_delete.contains(&key.as_str()) && is_ancestor {
            let sup_parent = selection_node.parent_node().unwrap();
            let parent = sup_parent.parent_node().unwrap();
            _ = parent.remove_child(&sup_parent);

            update_ta_value();
        }

        if false == key.is_empty() && is_ancestor {
            evt.prevent_default();
        }
    };

    view! {
        <div class="mb-4">
            <input type="hidden" name=name prop:value=move || ta_value.get()/>
            <div
                id=ID_EDITABLE
                on:keydown=on_kd
                on:input=move |_| update_ta_value()
                contenteditable="true"
                spellcheck="false"
                class="lyrics-editor form-lyrics"
            ></div>
        </div>
    }
}
