//! Source sleep deadlines use normal simulation time.
use crate::reference::{SeatId, SeatReference, SleepReference};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SleepCamera {
    pub absolute_position: [f64; 3],
    pub rotation: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum SleepPhase {
    Idle,
    Charging,
    Sleeping,
}

#[derive(Clone, Copy, Debug)]
pub struct SleepInput {
    pub held: bool,
    pub active: bool,
    pub camera_allowed: bool,
    pub dead: bool,
    pub show_text: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SleepOutput {
    pub phase: SleepPhase,
    pub eligible: bool,
    pub show_hint: bool,
    pub charge: f32,
    pub wake_time: f64,
    pub cooldown_remaining: f64,
    pub progress: f32,
}

/// seatscript.showSleepText of the occupied seat.
pub fn show_text(seats: &[SeatReference], seat: Option<SeatId>) -> bool {
    seat.is_some_and(|id| seats.iter().any(|s| s.id == id && s.show_sleep_text))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClockFactors {
    pub time_scale: f32,
    pub need_scale: f32,
    pub fluid_scale: f32,
    pub cycle_extra_per_sim_second: f32,
    pub cloud_scale: f32,
}

#[derive(Clone, Debug)]
pub struct SleepState {
    output: SleepOutput,
}

impl SleepState {
    pub fn new(reference: &SleepReference) -> Self {
        Self {
            output: SleepOutput {
                phase: SleepPhase::Idle,
                eligible: false,
                show_hint: false,
                charge: 0.,
                wake_time: -f64::from(reference.cooldown_seconds),
                cooldown_remaining: 0.,
                progress: 0.,
            },
        }
    }

    /// Call at the step endpoint, after consumers used the sampled factors.
    pub fn step(
        &mut self,
        r: &SleepReference,
        input: SleepInput,
        now: f64,
        dt: f32,
    ) -> SleepOutput {
        if !now.is_finite() || !dt.is_finite() || dt < 0. {
            return self.output;
        }
        if input.dead {
            self.interrupt();
        }
        if !input.active {
            self.cancel_charge();
        }
        if self.output.phase == SleepPhase::Sleeping && input.active {
            if now >= self.output.wake_time {
                self.interrupt();
            } else {
                self.output.progress = (1.
                    - (self.output.wake_time - now) / f64::from(r.sleep_real_seconds * 2.))
                .clamp(0., 1.) as f32;
            }
        } else if self.output.phase != SleepPhase::Sleeping {
            self.output.eligible = !input.dead
                && input.active
                && input.camera_allowed
                && now > self.output.wake_time + f64::from(r.cooldown_seconds);
            if self.output.eligible && input.held {
                if self.output.charge >= 1. {
                    self.output.phase = SleepPhase::Sleeping;
                    self.output.charge = 0.;
                    self.output.eligible = false;
                    self.output.progress = 0.;
                    self.output.wake_time = now + f64::from(r.sleep_real_seconds * 2.);
                } else {
                    self.output.phase = SleepPhase::Charging;
                    self.output.charge = (self.output.charge + r.hold_gain * dt).min(1.);
                }
            } else {
                let eligible = self.output.eligible;
                self.cancel_charge();
                self.output.eligible = eligible;
            }
        }
        self.output.show_hint = self.output.eligible && input.show_text;
        self.output.cooldown_remaining =
            (self.output.wake_time + f64::from(r.cooldown_seconds) - now).max(0.);
        self.output
    }

    pub fn clock_factors(&self, r: &SleepReference) -> ClockFactors {
        let sleeping = self.output.phase == SleepPhase::Sleeping;
        let scale = if sleeping {
            r.sleep_cycle_seconds / r.sleep_real_seconds * 0.5
        } else {
            1.
        };
        ClockFactors {
            time_scale: if sleeping { 2. } else { 1. },
            need_scale: if sleeping {
                scale * r.need_modifier
            } else {
                1.
            },
            fluid_scale: scale,
            cycle_extra_per_sim_second: if sleeping { scale } else { 0. },
            cloud_scale: scale,
        }
    }

    pub fn output(&self) -> SleepOutput {
        self.output
    }
    pub fn cancel_charge(&mut self) {
        if self.output.phase == SleepPhase::Charging {
            self.output.phase = SleepPhase::Idle;
        }
        self.output.charge = 0.;
        self.output.eligible = false;
    }
    pub fn interrupt(&mut self) {
        self.output.phase = SleepPhase::Idle;
        self.output.charge = 0.;
        self.output.eligible = false;
        self.output.progress = 0.;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{AnchorId, ItemId, ManagedEvidence, SourceId};

    #[test]
    fn hint_shows_only_in_flagged_seat() {
        let source = SourceId {
            file: String::new(),
            path_id: 0,
        };
        let seat = |id, show_sleep_text| SeatReference {
            default_pitch_degrees: 0.,
            pitch_limits_degrees: [0.; 2],
            id: SeatId(id),
            source: source.clone(),
            item: ItemId(0),
            anchor: AnchorId(0),
            vehicle: 0,
            driver: false,
            driving2: false,
            hit_nodes: Vec::new(),
            yaw_limits_degrees: [0.; 2],
            exit_distance: 0.,
            min_exit_angle_degrees: 0.,
            exit_height: 0.,
            show_sleep_text,
        };
        let seats = [seat(0, true), seat(1, false)];
        let r = SleepReference {
            source: source.clone(),
            need_modifier: 1.,
            hold_gain: 1.,
            sleep_real_seconds: 1.,
            sleep_cycle_seconds: 1.,
            cooldown_seconds: 1.,
            sleep_cam_ring_min: 0.,
            sleep_cam_ring_max: 0.,
            sleep_cam_height_min: 0.,
            sleep_cam_height_max: 0.,
            sleep_layer_mask: 0,
            cloud_low_min_speed: 0.,
            cloud_low_mid_speed: 0.,
            cloud_low_max_speed: 0.,
            cloud_high_min_speed: 0.,
            cloud_high_mid_speed: 0.,
            cloud_high_max_speed: 0.,
            evidence: ManagedEvidence {
                assembly_sha256: String::new(),
                methods: Vec::new(),
            },
        };
        for (seat, hint) in [
            (Some(SeatId(0)), true),
            (Some(SeatId(1)), false),
            (None, false),
        ] {
            let input = SleepInput {
                held: false,
                active: true,
                camera_allowed: true,
                dead: false,
                show_text: show_text(&seats, seat),
            };
            let output = SleepState::new(&r).step(&r, input, 10., 0.02);
            assert!(output.eligible);
            assert_eq!(output.show_hint, hint);
        }
    }
}
