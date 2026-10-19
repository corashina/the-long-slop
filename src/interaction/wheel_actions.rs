use super::{
    InteractionAction, InteractionState, ResolvedAim, items::ItemStore, wheels::WheelAssemblies,
};
use crate::{
    player::PlayerState,
    reference::{ItemId, WheelSlotId},
    simulation::{FrameInput, keys},
    vehicle::PhysicsWorld,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelAction {
    Remove(ItemId),
    Install { item: ItemId, target: WheelSlotId },
    Split(ItemId),
    Join { rim: ItemId, tire: ItemId },
}

impl InteractionState {
    pub(crate) fn complete_wheel_action(
        &mut self,
        action: WheelAction,
        input: FrameInput,
        aim: ResolvedAim,
        wheels: &mut WheelAssemblies,
        player: &PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Option<&'static str> {
        Some(match action {
            WheelAction::Remove(item) => {
                let slot = wheels.slot_for_item(item)?;
                match wheels.remove(slot, items, physics) {
                    Ok(rim) => {
                        self.completed_action(InteractionAction::Wheel(WheelAction::Remove(rim)));
                        let point = aim
                            .grab_target
                            .map_or(items.get(rim).unwrap().pose.position, |target| target.point);
                        match self.pickup_item(rim, point, player, items, physics) {
                            Ok(()) => "",
                            Err(_) => "Cannot carry this wheel.",
                        }
                    }
                    Err(_) => "Cannot remove this wheel.",
                }
            }
            WheelAction::Install { item, target } => {
                if wheels.can_install(item, target, items).is_err() {
                    return Some("This wheel does not fit the slot.");
                }
                self.drop_held(items, physics);
                if wheels.install(item, target, items, physics).is_ok() {
                    self.completed_action(InteractionAction::Wheel(WheelAction::Install {
                        item,
                        target,
                    }));
                    if let Some(sensor) = wheels.sensor(target) {
                        self.bind_wheel_sensor(sensor, item, physics);
                    }
                    ""
                } else {
                    "This wheel does not fit the slot."
                }
            }
            WheelAction::Split(rim) => {
                self.drop_held(items, physics);
                if let Ok((_, tire)) = wheels.split(rim, items, physics) {
                    self.completed_action(InteractionAction::PartRemove(tire));
                    if input.pressed & keys::GRAB != 0 {
                        let point = aim
                            .focus_target
                            .or(aim.grab_target)
                            .map_or(items.get(tire).unwrap().pose.position, |target| {
                                target.point
                            });
                        if self
                            .pickup_item(tire, point, player, items, physics)
                            .is_err()
                        {
                            return Some("Cannot carry this tire.");
                        }
                    }
                    ""
                } else {
                    "Cannot dismount this tire."
                }
            }
            WheelAction::Join { rim, tire } => {
                self.drop_held(items, physics);
                if wheels.assemble(rim, tire, items, physics).is_ok() {
                    self.completed_action(InteractionAction::Wheel(WheelAction::Join {
                        rim,
                        tire,
                    }));
                    ""
                } else {
                    "This tire does not fit the rim."
                }
            }
        })
    }
}
