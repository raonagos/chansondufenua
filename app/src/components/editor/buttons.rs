use super::utils::*;
use super::*;

use leptos::prelude::*;
use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[component]
pub fn BtnAnchorChord(chord_name: String) -> impl IntoView {
    let chord_name_2 = chord_name.clone();

    let on_click = move |evt: ev::MouseEvent| {
        evt.prevent_default();

        let ta_elm = document().get_element_by_id(ID_EDITABLE).unwrap();
        let chbx_minor_elm = document().get_element_by_id(ID_CHBX_MINOR).unwrap();
        let chbx_bemol_elm = document().get_element_by_id(ID_CHBX_BEMOL).unwrap();
        let chbx_diez_elm = document().get_element_by_id(ID_CHBX_DIEZ).unwrap();

        let chbx_minor_elm: HtmlInputElement =
            chbx_minor_elm.dyn_into::<HtmlInputElement>().unwrap();
        let chbx_bemol_elm = chbx_bemol_elm.dyn_into::<HtmlInputElement>().unwrap();
        let chbx_diez_elm = chbx_diez_elm.dyn_into::<HtmlInputElement>().unwrap();

        let selection = window().get_selection().unwrap().unwrap();
        let selection_node = selection.anchor_node().unwrap();

        let is_child = ta_elm.contains(Some(&selection_node));

        if false == is_child || contains_ancestor(&selection_node, "SUP") {
            return;
        }

        let mut chord_name = chord_name.clone();

        if chbx_bemol_elm.checked() {
            chord_name = format!("{}b", &chord_name);
        } else if chbx_diez_elm.checked() {
            chord_name = format!("{}#", &chord_name);
        }
        if chbx_minor_elm.checked() {
            chord_name = format!("{}m", &chord_name);
        }

        let anchor_chord = document().create_element("SUP").unwrap();
        _ = anchor_chord.set_attribute("data-nosnippet", "true");
        let text_chord = document().create_text_node(&chord_name);
        _ = anchor_chord.append_child(&text_chord);

        let range = selection.get_range_at(0).unwrap();
        _ = range.delete_contents();
        _ = range.insert_node(&anchor_chord);

        fired_editable_input();
    };

    view! {
        <button type="button" on:click=on_click class="form-chord-btn">
            {chord_name_2}
        </button>
    }
}

#[component]
pub fn BtnCustomChord() -> impl IntoView {
    let input_ref = NodeRef::<html::Input>::new();

    let on_click = move |evt: ev::MouseEvent| {
        evt.prevent_default();
        let ta_elm = document().get_element_by_id(ID_EDITABLE).unwrap();

        let selection = window().get_selection().unwrap().unwrap();
        let selection_node = selection.anchor_node().unwrap();

        let is_child = ta_elm.contains(Some(&selection_node));

        let input_elm = input_ref.get().unwrap();
        let input_value = input_elm.value();

        if false == is_child || contains_ancestor(&selection_node, "SUP") || input_value.is_empty()
        {
            return;
        }

        let anchor_chord = document().create_element("SUP").unwrap();
        _ = anchor_chord.set_attribute("data-nosnippet", "true");
        _ = anchor_chord.set_attribute("data-god", "");
        let text_chord = document().create_text_node(&input_value);
        _ = anchor_chord.append_child(&text_chord);

        let range = selection.get_range_at(0).unwrap();
        _ = range.delete_contents();
        _ = range.insert_node(&anchor_chord);

        fired_editable_input();
    };

    view! {
        <div class="form-custom-chord">
            <input node_ref=input_ref type="text" class="form-custom-chord-lyrics" placeholder="Sol7b"/>
            <button type="button" on:click=on_click class="form-custom-chord-btn">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15"></path>
                </svg>
            </button>
        </div>
    }
}

#[component]
pub fn CheckboxMinor() -> impl IntoView {
    view! {
        <label for=ID_CHBX_MINOR class="form-chbx-btn">
            <span>"m"</span>
            <input type="checkbox" name=ID_CHBX_MINOR id=ID_CHBX_MINOR/>
        </label>
    }
}

#[component]
pub fn CheckboxBemol() -> impl IntoView {
    view! {
        <label for=ID_CHBX_BEMOL class="form-chbx-btn">
            <span>"b"</span>
            <input type="checkbox" name=ID_CHBX_BEMOL id=ID_CHBX_BEMOL/>
        </label>
    }
}

#[component]
pub fn CheckboxDiez() -> impl IntoView {
    view! {
        <label for=ID_CHBX_DIEZ class="form-chbx-btn">
            <span>"#"</span>
            <input type="checkbox" name=ID_CHBX_DIEZ id=ID_CHBX_DIEZ/>
        </label>
    }
}
