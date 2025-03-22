/// Hydrates the client side after the first load from server.
/// 
/// The function takes the entry [`App`] component that contains the router.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
