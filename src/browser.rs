use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = gameReady)]
    pub fn game_ready();
    #[wasm_bindgen(js_namespace = window, js_name = gameFailure)]
    pub fn game_failure(message: &str);
    #[wasm_bindgen(js_namespace = window, js_name = gameKeys)]
    pub fn game_keys() -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = gamePressed)]
    pub fn game_pressed() -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = gameReleased)]
    pub fn game_released() -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = gameMouseX)]
    pub fn game_mouse_x() -> f32;
    #[wasm_bindgen(js_namespace = window, js_name = gameMouseY)]
    pub fn game_mouse_y() -> f32;
    #[wasm_bindgen(js_namespace = window, js_name = gameWheel)]
    pub fn game_wheel() -> f32;
    #[wasm_bindgen(js_namespace = window, js_name = gameActions)]
    pub fn game_actions() -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = gameUpdate)]
    pub fn game_update(first_person: bool, travel: f64, paused: bool, started: bool);
    #[wasm_bindgen(js_namespace = window, js_name = gameFeedback)]
    pub fn game_feedback(
        actions: u32,
        context: u32,
        name: &str,
        notice: &str,
        carrying: bool,
        amount: f32,
        capacity: f32,
        has_name: bool,
        no_text: bool,
        gas: f32,
        oil: f32,
        alcohol: f32,
        water: f32,
        blood: f32,
        diesel: f32,
    );
    /// One sound frame. The view is valid only during the call.
    #[wasm_bindgen(js_namespace = window, js_name = gameAudio)]
    pub fn game_audio(frame: &[f32]);
}
