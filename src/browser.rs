use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=window,js_name=gameAudioError)]
    pub fn game_audio_error(message: &str);
    #[wasm_bindgen(js_namespace=window,js_name=gameSurvival)]
    pub fn game_survival(
        hp: f32,
        food: f32,
        water: f32,
        bowel: f32,
        bladder: f32,
        opacity: f32,
        dead: bool,
        look_scale: f32,
        menu_due: bool,
    );
    #[wasm_bindgen(js_namespace=window,js_name=gameAudio)]
    pub fn game_audio(ids: &[u32], values: &[f32]);
    #[wasm_bindgen(js_namespace=window,js_name=gameAudioListener)]
    pub fn game_audio_listener(
        session: u32,
        x: f32,
        y: f32,
        z: f32,
        fx: f32,
        fy: f32,
        fz: f32,
        ux: f32,
        uy: f32,
        uz: f32,
        master_gain: f32,
        origin_x: f64,
        origin_y: f64,
        origin_z: f64,
        seconds: f64,
    );

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
    pub fn game_update(
        speed: f32,
        camera: u32,
        x: f32,
        y: f32,
        z: f32,
        contacts: u32,
        steps: u32,
        chunks: u32,
        colliders: u32,
        absolute_x: f64,
        absolute_y: f64,
        absolute_z: f64,
        origin_x: f64,
        origin_y: f64,
        origin_z: f64,
        rebases: u32,
        travel: f64,
        lateral_band: f64,
        fuel: f32,
        paused: bool,
        started: bool,
    );
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
        fluid_name: &str,
        condition: f32,
        held: &str,
        throw_charge: f32,
        throw_active: bool,
        rpm: f32,
        ignition: bool,
        running: bool,
        starter: bool,
        seated: bool,
        handbrake: bool,
        gas: f32,
        oil: f32,
        alcohol: f32,
        water: f32,
        blood: f32,
        diesel: f32,
        control: u32,
        lmb_use: bool,
    );
}
