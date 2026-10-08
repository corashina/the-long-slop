use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = gameReady)]
    pub fn game_ready();
    #[wasm_bindgen(js_namespace = window, js_name = gameKeys)]
    pub fn game_keys() -> u32;
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
        station_x: f64,
        station_y: f64,
        station_z: f64,
        station_distance: f64,
        refuel_eligible: bool,
    );
}
