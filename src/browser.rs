use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = gameSceneScale)]
    pub fn game_scene_scale() -> f32;
    #[wasm_bindgen(js_name = gameAudioError)]
    pub fn game_audio_error(message: &str);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioSurvival)]
    pub fn game_audio_survival(
        kind: u32,
        emitter: u32,
        action: u32,
        clip: u32,
        gain: f32,
        pitch: f32,
        x: f32,
        y: f32,
        z: f32,
    );
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioSurvivalEmitter)]
    pub fn game_audio_survival_emitter(kind: u32, x: f32, y: f32, z: f32);
    #[wasm_bindgen(js_namespace = window, js_name = gameAudioSurvivalMaster)]
    pub fn game_audio_survival_master(gain: f32, revision: u32, seconds: f64);

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

    #[wasm_bindgen(js_namespace = window, js_name = gameLoading)]
    pub fn game_loading();
    #[wasm_bindgen(js_namespace = window, js_name = gameLoadingProgress)]
    pub fn game_loading_progress(roads: u32, total: u32, rendering: bool, world: bool);
    #[wasm_bindgen(js_namespace = window, js_name = gameLoadingBudget)]
    pub fn game_loading_budget(name: &str, used: f64, limit: f64, unit: &str);
    #[wasm_bindgen(js_namespace = window, js_name = gameStartupStage)]
    pub fn game_startup_stage(stage: &str, started: bool);
    #[wasm_bindgen(js_namespace = window, js_name = gameStartupFailureDetails)]
    pub fn game_startup_failure_details(json: &str);
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
    #[wasm_bindgen(js_namespace=window,js_name=gameSleep)]
    pub fn game_sleep(
        phase: u32,
        eligible: bool,
        charge: f32,
        cooldown_remaining: f64,
        progress: f32,
    );
    #[wasm_bindgen(js_namespace = window, js_name = gameNow)]
    pub fn game_now() -> f64;
    #[wasm_bindgen(js_namespace = window, js_name = gamePerformanceFrame)]
    pub fn game_performance_frame(physics_ms: f64, world_ms: f64) -> bool;
    #[wasm_bindgen(js_namespace = window, js_name = gamePerformanceCounters)]
    pub fn game_performance_counters(
        steps: u32,
        wheel_contacts: u32,
        bodies: u32,
        colliders: u32,
        roads: u32,
        terrain: u32,
        objects: u32,
        jobs: u32,
        pending: u32,
        cpu_bytes: f64,
        gpu_bytes: f64,
    );
    #[wasm_bindgen(js_namespace = window, js_name = gameMeasureRequested)]
    pub fn game_measure_requested() -> bool;
    #[wasm_bindgen(js_namespace = window, js_name = gameMeasurements)]
    pub fn game_measurements(json: &str);

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
    #[wasm_bindgen(js_namespace = window, js_name = gameInventoryKeys)]
    pub fn game_inventory_keys() -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = gameMouseX)]
    pub fn game_mouse_x() -> f32;
    #[wasm_bindgen(js_namespace = window, js_name = gameMouseY)]
    pub fn game_mouse_y() -> f32;
    #[wasm_bindgen(js_namespace = window, js_name = gameWheel)]
    pub fn game_wheel() -> f32;
    #[wasm_bindgen(js_namespace = window, js_name = gameActions)]
    pub fn game_actions() -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = gameDrivingAssists)]
    pub fn game_driving_assists() -> u32;
    #[wasm_bindgen(js_namespace = window, js_name = gameDrivingState)]
    pub fn game_driving_state(gear: i32, assists: u32, seated: bool);
    #[wasm_bindgen(js_namespace = window, js_name = gameUpdate)]
    pub fn game_update(
        speed: f32,
        camera: u32,
        x: f32,
        y: f32,
        z: f32,
        contacts: u32,
        steps: u32,
        surfaces: u32,
        colliders: u32,
        absolute_x: f64,
        absolute_y: f64,
        absolute_z: f64,
        origin_x: f64,
        origin_y: f64,
        origin_z: f64,
        rebases: u32,
        travel: f64,
        prepared_roads: f64,
        fuel: f32,
        paused: bool,
        started: bool,
    );
    #[wasm_bindgen(js_namespace=window,js_name=gameInventory)]
    pub fn game_inventory(selected: u32, a: u32, b: u32, c: u32, d: u32, progress: f32);
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
