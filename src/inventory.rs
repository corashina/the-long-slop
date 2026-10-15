//! Slot membership is independent of carry and right-hand ownership.
use crate::{
    reference::{InventoryReference, ItemId},
    simulation::{FrameInput, keys},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryCommand {
    Select(u8),
    Swap(u8),
}
#[derive(Clone, Copy, Default)]
pub struct InventoryContext {
    pub carried: Option<ItemId>,
    pub equipped: Option<ItemId>,
    pub ancestor_carry: bool,
    pub seated_clutch: bool,
}
pub struct InventoryState {
    pub slots: [Option<ItemId>; 4],
    pub selected: u8,
    pub swap_progress: f32,
    gain: f32,
    swapped: bool,
}
impl InventoryState {
    pub fn new(r: &InventoryReference) -> Self {
        Self {
            slots: [None; 4],
            selected: r.selected_slot,
            swap_progress: 0.,
            gain: r.swap_gain,
            swapped: false,
        }
    }
    pub fn cancel_input(&mut self) {
        self.swap_progress = 0.;
        self.swapped = false;
    }
    pub fn forget(&mut self, id: ItemId) {
        for slot in &mut self.slots {
            if *slot == Some(id) {
                *slot = None;
            }
        }
    }
    pub fn step(
        &mut self,
        input: FrameInput,
        dt: f32,
        context: InventoryContext,
    ) -> Option<InventoryCommand> {
        if context.seated_clutch || !dt.is_finite() || dt <= 0. {
            return None;
        }
        let bits = [
            keys::INVENTORY1,
            keys::INVENTORY2,
            keys::INVENTORY3,
            keys::INVENTORY4,
        ];
        if !self.swapped
            && let Some(i) = bits.iter().position(|b| input.released & b != 0)
        {
            return Some(InventoryCommand::Select(i as u8));
        }
        if let Some(i) = bits.iter().position(|b| input.held & b != 0) {
            if !self.swapped
                && !context.ancestor_carry
                && !(context.carried.is_some()
                    && context.equipped.is_some()
                    && self.slots[i].is_some())
            {
                if self.swap_progress < 1. {
                    self.swap_progress += self.gain * dt;
                } else {
                    self.swapped = true;
                    self.swap_progress = 0.;
                    return Some(InventoryCommand::Swap(i as u8));
                }
            }
        } else {
            self.swapped = false;
        }
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_priority_hold_order_and_cancellation() {
        let mut s = InventoryState {
            slots: [None; 4],
            selected: 0,
            swap_progress: 0.,
            gain: 5.,
            swapped: false,
        };
        let context = InventoryContext::default();
        let key2 = 1 << 22;
        let key3 = 1 << 23;
        assert_eq!(
            s.step(
                FrameInput {
                    released: key2 | key3,
                    ..Default::default()
                },
                0.1,
                context
            ),
            Some(InventoryCommand::Select(1))
        );
        for _ in 0..2 {
            assert_eq!(
                s.step(
                    FrameInput {
                        held: key3,
                        ..Default::default()
                    },
                    0.1,
                    context
                ),
                None
            );
        }
        assert_eq!(
            s.step(
                FrameInput {
                    held: key3,
                    ..Default::default()
                },
                0.1,
                context
            ),
            Some(InventoryCommand::Swap(2))
        );
        assert_eq!(
            s.step(
                FrameInput {
                    released: key3,
                    ..Default::default()
                },
                0.1,
                context
            ),
            None
        );
        s.step(
            FrameInput {
                held: key2,
                ..Default::default()
            },
            0.1,
            context,
        );
        s.cancel_input();
        assert_eq!(s.swap_progress, 0.);
        assert_eq!(s.step(FrameInput::default(), 0.1, context), None);
        s.slots[1] = Some(ItemId(1));
        let blocked = InventoryContext {
            carried: Some(ItemId(2)),
            equipped: Some(ItemId(3)),
            ..context
        };
        for _ in 0..4 {
            assert_eq!(
                s.step(
                    FrameInput {
                        held: key2,
                        ..Default::default()
                    },
                    0.1,
                    blocked
                ),
                None
            );
        }
        assert_eq!(s.swap_progress, 0.);
    }
}
