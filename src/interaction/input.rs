use super::{
    AimTarget, InteractionAction, InteractionState,
    feedback::{ActionAvailability, ActionMask},
};
use crate::simulation::{FrameInput, keys};
use crate::{player::PlayerState, vehicle::PhysicsWorld};
use rapier3d::math::Vector;

#[derive(Clone, Copy, Debug, Default)]
pub struct InteractionOutcome {
    pub action: Option<InteractionAction>,
    pub look_mouse: [f32; 2],
}

pub(super) struct ResolvedInput {
    pub use_input: FrameInput,
    pub grab_input: FrameInput,
    pub carry_input: FrameInput,
    pub push_input: FrameInput,
    pub look_mouse: [f32; 2],
}
fn remove(input: &mut FrameInput, keys: u32) {
    input.held &= !keys;
    input.pressed &= !keys;
    input.released &= !keys;
}
impl InteractionState {
    pub(super) fn active_mechanism_target(
        &self,
        player: &PlayerState,
        physics: &PhysicsWorld,
    ) -> Option<AimTarget> {
        let (item, body_id, local, collider) = self.latched_mechanism?;
        if !self.mechanisms.contains(item) {
            return None;
        }
        let body = physics.rapier().bodies.get(*self.bodies.get(&body_id)?)?;
        let point = *body.position() * local;
        let distance =
            point.distance_squared(Vector::from_array(player.body_pose(physics).position));
        (distance < self.mechanism_release_distance_squared).then_some(AimTarget {
            item,
            collider,
            point: point.to_array(),
            distance: distance.sqrt(),
            slot: None,
        })
    }
    pub(super) fn filter_cancelled_input(&mut self, mut input: FrameInput) -> FrameInput {
        let blocked = self.cancelled_input;
        self.cancelled_input &= input.held;
        remove(&mut input, blocked);
        if blocked & (keys::RECENTER | keys::TRANSLATE | keys::ROTATE) != 0 {
            input.wheel = 0.;
        }
        input
    }
    pub(super) fn drag_input(&self, input: FrameInput, available: &ActionAvailability) -> bool {
        available.use_item.is_some_and(|item| {
            self.mechanisms.drag_eligible(item)
                && (input.held & keys::USE != 0
                    || (available.lmb_use
                        && self.held_light_control().is_none()
                        && input.held & keys::PRIMARY != 0)
                    || (input.wheel != 0. && self.mechanisms.scroll_eligible(item)))
        })
    }
    pub(super) fn resolve_input(
        &self,
        input: FrameInput,
        available: &ActionAvailability,
    ) -> ResolvedInput {
        let mut resolved = ResolvedInput {
            use_input: input,
            grab_input: input,
            carry_input: input,
            push_input: input,
            look_mouse: input.mouse,
        };
        remove(&mut resolved.carry_input, keys::RECENTER);
        let primary_use = available.lmb_use && self.held_light_control().is_none();
        let use_held =
            input.held & keys::USE != 0 || (primary_use && input.held & keys::PRIMARY != 0);
        let drag = self.drag_input(input, available);
        let active_drag = available.active_drag && drag;
        let mounting = !self.carry.acquisition_held()
            && ((available.actions.0 & ActionMask::MOUNT != 0
                && input.released & keys::GRAB != 0
                && !self.f_gesture_consumed
                && self.carry.throw_charge() < self.carry.reference_throw_minimum())
                || (available.actions.0 & ActionMask::DISMOUNT != 0
                    && input.pressed & keys::GRAB != 0
                    && !self.push.gripping()));
        if active_drag {
            // RayStuff skips new F targets. Existing carry and grip release run separately.
            if self.held().is_none() && !self.push.gripping() {
                remove(&mut resolved.grab_input, keys::GRAB);
            }
            remove(&mut resolved.push_input, keys::GRAB);
        } else if mounting {
            remove(&mut resolved.use_input, keys::USE | keys::PRIMARY);
            resolved.use_input.mouse = [0.; 2];
            resolved.use_input.wheel = 0.;
        }
        let tuning = available.actions.0 & ActionMask::TUNE != 0;
        let tuning_mouse = tuning && use_held;
        if (drag || tuning_mouse) && !mounting || active_drag {
            resolved.look_mouse = [0.; 2];
            resolved.carry_input.mouse = [0.; 2];
        } else if self.held().is_some() && input.held & (keys::TRANSLATE | keys::ROTATE) != 0 {
            resolved.look_mouse = [0.; 2];
        }
        // Source Carry permits RMB to share wheel input with tuning. MMB always blocks it.
        if input.held & keys::ROTATE != 0 || (tuning && input.held & keys::TRANSLATE == 0) {
            resolved.carry_input.wheel = 0.;
        }
        if input.pressed & keys::PRIMARY != 0 && self.held_light_control().is_some() {
            remove(&mut resolved.carry_input, keys::PRIMARY);
        }
        resolved
    }
}
