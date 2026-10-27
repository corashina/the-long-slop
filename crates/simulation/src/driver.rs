//! Runs the simulation from renderer commands. It owns the runtime, the input queue and the step clock.
use crate::boundary::{SimCommand, SimFrame, StateBlock};
use crate::reference::ReferenceManifest;
use crate::simulation::{FIXED_DT, FrameInput, StepClock, keys};
use crate::starting_area::GarageRuntime;
use crate::vehicle::VehiclePose;
use glam::{Quat, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraMode {
    #[default]
    Cockpit,
    Chase,
}

pub struct SimDriver {
    pub runtime: GarageRuntime,
    /// Input of the next fixed step.
    input: FrameInput,
    /// Accepted frame input that no frame consumed yet.
    frame_input: FrameInput,
    clock: StepClock,
    previous: VehiclePose,
    current: VehiclePose,
    /// Player camera at the interpolation of the last frame.
    pub rendered: VehiclePose,
    pub camera: CameraMode,
    /// Fixed steps of the last frame.
    pub frame_steps: u32,
    /// Simulated seconds of the last frame.
    pub render_delta_seconds: f32,
    /// Milliseconds of the last frame steps.
    pub physics_ms: f64,
    skip_elapsed: bool,
    /// The renderer shows the garage.
    ready: bool,
    delta_seconds: f64,
    calendar: [u8; 2],
}

impl SimDriver {
    pub fn new(runtime: GarageRuntime) -> Self {
        let current = runtime.area.player.view(&runtime.physics).camera_pose;
        Self {
            runtime,
            input: FrameInput::default(),
            frame_input: FrameInput::default(),
            clock: StepClock::default(),
            previous: current,
            current,
            rendered: current,
            camera: CameraMode::default(),
            frame_steps: 0,
            render_delta_seconds: 0.,
            physics_ms: 0.,
            skip_elapsed: false,
            ready: false,
            delta_seconds: 0.,
            calendar: [1, 1],
        }
    }

    /// Applies the menu actions and settings of `command` and queues its input.
    pub fn accept(&mut self, reference: &ReferenceManifest, command: SimCommand) {
        self.delta_seconds = command.delta_seconds;
        self.calendar = command.calendar;
        self.ready = command.ready;
        if command.hold {
            clear_profile_input();
            self.frame_input.clear();
            return;
        }
        let actions = command.actions;
        if actions & SimCommand::PAUSE != 0 {
            self.runtime.paused = true;
            self.clear_input();
        }
        if actions & SimCommand::START != 0 && self.ready && !self.runtime.needs_output().menu_due {
            self.runtime.started = true;
            self.runtime.paused = false;
            self.clear_input();
        }
        if actions & SimCommand::RECOVER != 0 {
            self.runtime.recover(reference);
            self.clear_input();
        }
        if let Some(assists) = command.assists {
            let _ = self.runtime.set_driving_assists(assists);
        }
        // The browser validates stored settings. A refused value keeps the current settings.
        if let Some(settings) = command.settings
            && settings.valid(&reference.driving)
        {
            self.runtime.driving_input.settings = settings;
        }
        let frame = command.input;
        if !self.ready || !self.runtime.started || self.runtime.paused {
            self.input.clear();
            clear_profile_input();
            self.frame_input.clear();
            self.runtime.clear_input();
            // The Controls screen shows the pad axes while the game is paused.
            self.runtime.driving_input.preview(&frame.gamepad);
            return;
        }
        if actions != 0 {
            return;
        }
        if !self.runtime.needs_output().dead
            && !self.runtime.sleeping()
            && frame.pressed & keys::CAMERA != 0
            && self.runtime.area.player.seat_vehicle().is_some()
        {
            self.runtime.audio_motion_revision = self.runtime.audio_motion_revision.wrapping_add(1);
            self.camera = match self.camera {
                CameraMode::Cockpit => CameraMode::Chase,
                CameraMode::Chase => CameraMode::Cockpit,
            };
        }
        self.runtime.sleep_camera_allowed = self.camera == CameraMode::Cockpit;
        self.frame_input.merge(frame);
    }

    /// Runs the fixed steps of the accepted frame time.
    pub fn advance(&mut self, reference: &ReferenceManifest) {
        self.physics_ms = 0.;
        self.frame_steps = 0;
        self.render_delta_seconds = 0.;
        if !self.ready || !self.runtime.started || self.runtime.paused {
            return;
        }
        let blocked = self.skip_elapsed
            && (self.runtime.recovering
                || !self.runtime.world_supported()
                || self.runtime.sleeping()
                || self.runtime.needs_output().dead);
        let Some(raw) = self.frame_input.take_frame(&mut self.skip_elapsed, blocked) else {
            if blocked {
                self.input.clear();
                self.runtime.clear_input();
            }
            return;
        };
        let started = now();
        #[cfg(target_arch = "wasm32")]
        let _profile = crate::cpu_profile::Timer::new(crate::cpu_profile::Stage::Physics);
        if let Some(audio) = &mut self.runtime.audio {
            let [month, day] = self.calendar;
            audio.broadcasts.calendar(month, day);
        }
        let real = self.delta_seconds as f32;
        let delta = real * self.runtime.clock_factors(reference).time_scale;
        self.runtime.area.conditions.update(delta, true);
        let frame = self
            .runtime
            .advance_inventory_frame(reference, raw, delta, self.rendered);
        frame.merge_fixed_input(&mut self.input);
        self.clock.accept_frame(self.delta_seconds);
        loop {
            let scale = self.runtime.clock_factors(reference).time_scale;
            if !self.clock.take_step(scale) {
                break;
            }
            self.frame_steps += 1;
            self.render_delta_seconds += FIXED_DT;
            self.previous = self.current;
            let old_origin = self.runtime.stream.origin.offset;
            let input = self.input.take_step();
            self.runtime.step(reference, input);
            let new_origin = self.runtime.stream.origin.offset;
            for i in 0..3 {
                self.previous.position[i] -= (new_origin[i] - old_origin[i]) as f32;
            }
            self.current = self.view();
            if self.runtime.paused {
                self.clock.reset();
                break;
            }
        }
        let scale = self.runtime.clock_factors(reference).time_scale;
        self.rendered = interpolate(self.previous, self.current, self.clock.alpha(scale));
        self.physics_ms = now() - started;
    }

    /// Writes the runtime state into `frame`.
    pub fn fill(&mut self, reference: &ReferenceManifest, frame: &mut SimFrame) {
        let alpha = self
            .clock
            .alpha(self.runtime.clock_factors(reference).time_scale);
        self.runtime.fill_frame(reference, alpha, frame);
        if self.camera == CameraMode::Chase {
            frame.state.flags |= StateBlock::CHASE;
        }
    }

    /// Drops queued input and restarts the step clock at the player view.
    pub fn clear_input(&mut self) {
        clear_profile_input();
        self.input.clear();
        self.frame_input.clear();
        self.runtime.clear_input();
        self.current = self.view();
        self.clock.reset();
        self.previous = self.current;
        self.rendered = self.current;
        self.runtime.area.begin_step();
        self.skip_elapsed = true;
    }

    /// Starts a loaded runtime: paused, with no queued input.
    pub fn restart(&mut self, camera: CameraMode) {
        self.camera = camera;
        self.current = self.view();
        self.previous = self.current;
        self.rendered = self.current;
        self.input.clear();
        self.frame_input.clear();
        self.clock = StepClock::default();
        self.frame_steps = 0;
        self.render_delta_seconds = 0.;
        self.ready = false;
        self.runtime.paused = true;
        self.skip_elapsed = true;
    }

    /// Keys held in the next fixed step.
    pub fn held(&self) -> u32 {
        self.input.held
    }

    /// Player body `pose` at the rendered camera. It moves with the camera interpolation.
    pub fn rendered_body_pose(&self, mut pose: VehiclePose) -> VehiclePose {
        for i in 0..3 {
            pose.position[i] += self.rendered.position[i] - self.current.position[i];
        }
        pose
    }

    fn view(&self) -> VehiclePose {
        self.runtime
            .area
            .player
            .view(&self.runtime.physics)
            .camera_pose
    }
}

pub fn interpolate(a: VehiclePose, b: VehiclePose, alpha: f32) -> VehiclePose {
    VehiclePose {
        position: Vec3::from_array(a.position)
            .lerp(Vec3::from_array(b.position), alpha)
            .to_array(),
        rotation: Quat::from_array(a.rotation)
            .slerp(Quat::from_array(b.rotation), alpha)
            .to_array(),
    }
}

#[cfg(target_arch = "wasm32")]
fn now() -> f64 {
    crate::host::game_now()
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> f64 {
    0.
}

fn clear_profile_input() {
    #[cfg(target_arch = "wasm32")]
    if crate::cpu_profile::enabled() {
        crate::host::game_cpu_clear_input();
    }
}
