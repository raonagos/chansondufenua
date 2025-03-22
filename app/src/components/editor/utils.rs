/// Is child has `tag_name` ancestors ?
pub fn contains_ancestor(child: &web_sys::Node, tag_name: &str) -> bool {
    if let Some(parent) = child.parent_element() {
        if parent.tag_name().eq(&tag_name) {
            return true;
        }
        if contains_ancestor(&parent, tag_name) {
            return true;
        }
    }

    false
}

pub fn fired_editable_input() {
    let ta_elm = leptos::prelude::document()
        .get_element_by_id(super::ID_EDITABLE)
        .unwrap();
    let input_event = leptos::ev::Event::new("input").unwrap();
    input_event.init_event_with_bubbles_and_cancelable("input", true, true);
    _ = ta_elm.dispatch_event(&input_event);
}
