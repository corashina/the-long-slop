use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioBegin)]
    pub fn game_audio_begin(session: u32, started: bool, paused: bool, time: f64);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioOriginShift)]
    pub fn game_audio_origin_shift(x: f32, y: f32, z: f32);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioListener)]
    pub fn game_audio_listener(x: f32, y: f32, z: f32, qx: f32, qy: f32, qz: f32, qw: f32);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioEmitter)]
    pub fn game_audio_emitter(
        id: u32,
        x: f32,
        y: f32,
        z: f32,
        qx: f32,
        qy: f32,
        qz: f32,
        qw: f32,
        alive: bool,
    );
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioVoice)]
    pub fn game_audio_voice(id: u32, playing: bool, gain: f32, pitch: f32, looping: bool);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioStarter)]
    pub fn game_audio_starter(emitter: u32, enabled: bool, time: f64);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioEvent)]
    pub fn game_audio_event(
        session: u32,
        sequence: u32,
        binding: u32,
        emitter: u32,
        x: f32,
        y: f32,
        z: f32,
        gain: f32,
        pitch: f32,
        time: f64,
    );
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioMixer)]
    pub fn game_audio_mixer(id: u32, db: f32);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioRoute)]
    pub fn game_audio_route(id: u32, mixer: u32);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioRadio)]
    pub fn game_audio_radio(
        power: bool,
        am: bool,
        frequency: f32,
        volume: f32,
        static_gain: f32,
        primary_clip: u32,
        primary_offset: f64,
        primary_gain: f32,
        primary_next: u32,
        secondary_clip: u32,
        secondary_offset: f64,
        secondary_gain: f32,
        secondary_next: u32,
    );
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioCalendar)]
    pub fn game_audio_calendar() -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioEnd)]
    pub fn game_audio_end();
    #[wasm_bindgen(js_namespace = window, js_name = gameHasSaveRequest)]
    pub fn game_has_save_request() -> bool;
    #[wasm_bindgen(js_namespace = window, js_name = gameSaveRequest)]
    pub fn game_save_request() -> String;
    #[wasm_bindgen(js_namespace = window, js_name = gameSaveResult)]
    pub fn game_save_result(id: u32, kind: &str, payload: &str, error: &str);

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
