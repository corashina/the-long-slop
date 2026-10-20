//! One identity-bound Tab gesture for carry and the right hand.
use super::{InteractionState, items::ItemStore};
use crate::{
    player::PlayerState,
    reference::ItemId,
    simulation::{FrameInput, keys},
    vehicle::PhysicsWorld,
};

pub(super) struct HandControls {
    gesture: TabGesture,
    hold_seconds: f32,
}
#[derive(Clone, Copy)]
enum TabGesture {
    Idle,
    Pressed {
        item: ItemId,
        elapsed: f32,
        equip_eligible: bool,
    },
    Suppressed,
}
impl HandControls {
    pub(super) fn new(hold_seconds: f32) -> Self {
        Self {
            gesture: TabGesture::Idle,
            hold_seconds,
        }
    }
    pub(super) fn owner_changed(&mut self) {
        if matches!(self.gesture, TabGesture::Pressed { .. }) {
            self.cancel();
        }
    }
    pub(super) fn cancel(&mut self) {
        self.gesture = TabGesture::Suppressed;
    }
}
impl InteractionState {
    pub fn step_hand_controls(
        &mut self,
        input: FrameInput,
        player: &PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) -> bool {
        if !dt.is_finite() || dt <= 0. {
            return false;
        }
        let pressed = input.pressed & keys::RECENTER != 0;
        let held = input.held & keys::RECENTER != 0;
        let released = input.released & keys::RECENTER != 0;
        if self.held().is_none()
            && let Some(item) = self.equipment.item()
            && (input.held & keys::ROTATE != 0 || (pressed && player.seat().is_none()))
            && self
                .carry
                .allows_pickup(item, player.seat().is_some(), false)
        {
            self.equipment.release(items, physics);
            let body = items.get(item).unwrap().body.unwrap();
            let point = physics.rapier().bodies[body].translation().to_array();
            self.pickup_item(item, point, player, items, physics)
                .expect("released equipment can be carried");
            self.hand_controls.gesture = if held || pressed {
                TabGesture::Pressed {
                    item,
                    elapsed: 0.,
                    equip_eligible: false,
                }
            } else {
                TabGesture::Suppressed
            };
            return false;
        }
        let item = self.held();
        if let TabGesture::Pressed { item: owner, .. } = self.hand_controls.gesture
            && item != Some(owner)
        {
            self.hand_controls.cancel();
        }
        if matches!(self.hand_controls.gesture, TabGesture::Suppressed) {
            if !held && !pressed {
                self.hand_controls.gesture = TabGesture::Idle;
            }
            return false;
        }
        let Some(item) = item else {
            if pressed {
                self.hand_controls.cancel();
            }
            return false;
        };
        let can_equip = self.carry.rule(item).unwrap().can_hold_right_hand;
        if pressed {
            self.hand_controls.gesture = TabGesture::Pressed {
                item,
                elapsed: 0.,
                equip_eligible: can_equip && self.equipment.item().is_none(),
            };
        }
        let TabGesture::Pressed {
            elapsed,
            equip_eligible,
            ..
        } = &mut self.hand_controls.gesture
        else {
            return held && !can_equip;
        };
        if !pressed {
            *elapsed += dt;
        }
        if released || !held {
            let equip = released
                && *elapsed < self.hand_controls.hold_seconds
                && *equip_eligible
                && self.equipment.item().is_none();
            self.hand_controls.gesture = TabGesture::Idle;
            if equip {
                self.carry.drop(items, physics);
                self.equipment.retrieve(item, items, physics);
            }
            return false;
        }
        !can_equip || *elapsed > self.hand_controls.hold_seconds
    }
}
