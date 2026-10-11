use super::*;
use crate::reference::FluidPresentation;

#[derive(Clone, Copy, Debug, Default)]
pub struct ActionMask(pub u32);
impl ActionMask {
    pub const USE: u32 = 1;
    pub const DOOR: u32 = 2;
    pub const PICKUP: u32 = 4;
    pub const DROP: u32 = 8;
    pub const MOUNT: u32 = 16;
    pub const DISMOUNT: u32 = 32;
    pub const FILL: u32 = 64;
    pub const TUNE: u32 = 128;
    pub const ENTER: u32 = 256;
    pub const EXIT: u32 = 512;
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(u32)]
pub enum FeedbackContext {
    #[default]
    None = 0,
    Door = 1,
    Cap = 2,
    Container = 3,
    Seat = 4,
    Wheel = 5,
    Pump = 6,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ActionAvailability {
    pub actions: ActionMask,
    pub context: FeedbackContext,
    pub item: Option<ItemId>,
    pub part: Option<ItemId>,
    pub rim: Option<ItemId>,
    pub tire: Option<ItemId>,
    pub opening: Option<OpeningId>,
    pub slot: Option<WheelSlotId>,
    pub consumes_mouse: bool,
}
pub struct FluidFeedback<'a> {
    pub amount: f32,
    pub capacity: f32,
    pub quantities: [f32; 6],
    pub presentation: &'a FluidPresentation,
}
pub struct InteractionFeedback<'a> {
    pub actions: ActionMask,
    pub context: FeedbackContext,
    pub name: &'a str,
    pub carrying: bool,
    pub fluid: Option<FluidFeedback<'a>>,
}
impl InteractionState {
    pub fn available_actions(
        &self,
        target: Option<AimTarget>,
        player: &PlayerState,
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) -> ActionAvailability {
        let target = target
            .filter(|t| t.distance.is_finite() && t.distance >= 0. && t.distance <= self.reach);
        let mut a = ActionAvailability::default();
        if self.held.is_some() {
            a.actions.0 |= ActionMask::DROP;
        }
        if let Some(t) = target {
            a.item = Some(t.item);
            a.part = Some(t.item);
            a.slot = t.slot;
            if !t.part_slot
                && fitted(&self.parts, items, t.item)
                && self.hinges.iter().any(|h| h.reference.item == t.item)
            {
                a.actions.0 |= ActionMask::DOOR;
                a.context = FeedbackContext::Door;
            }
            if let Some(o) = self.openings.iter().find(|o| o.reference.cap == t.item) {
                a.opening = Some(o.reference.id);
                a.context = if self.supplies.contains(&o.reference.tank) {
                    FeedbackContext::Pump
                } else {
                    FeedbackContext::Cap
                };
                if o.reference.can_toggle {
                    a.actions.0 |= ActionMask::USE;
                }
                if o.reference.can_tune {
                    a.actions.0 |= ActionMask::TUNE;
                }
                if o.valve > 0.
                    && let Some(held) = self.held.as_ref()
                {
                    let (source, dest) = if self.supplies.contains(&o.reference.tank) {
                        (o.reference.tank, held.item)
                    } else {
                        (held.item, o.reference.tank)
                    };
                    let dest_open = !self.supplies.contains(&o.reference.tank)
                        || self
                            .openings
                            .iter()
                            .any(|v| v.reference.tank == dest && v.valve > 0.);
                    if source != dest
                        && dest_open
                        && self
                            .tank_rules
                            .get(&source)
                            .is_some_and(|r| !r.no_fill_from)
                        && self.tank_rules.get(&dest).is_some_and(|r| !r.no_fill_this)
                        && items
                            .get(source)
                            .and_then(|i| i.fluid.as_ref())
                            .is_some_and(|f| f.mixture.total() > 0.)
                        && items
                            .get(dest)
                            .and_then(|i| i.fluid.as_ref())
                            .is_some_and(|f| f.mixture.total() < f.capacity_liters)
                    {
                        a.actions.0 |= ActionMask::FILL;
                    }
                }
            }
            if self.seats.iter().any(|s| s.item == t.item)
                && self.held.is_none()
                && player.seat().is_none()
            {
                a.actions.0 |= ActionMask::ENTER;
                a.context = FeedbackContext::Seat;
            }
            // A part's child, such as a door mirror, acts for its part.
            let part = match items.get(t.item).map(|i| i.location) {
                Some(ItemLocation::Attached(parent))
                    if !self.parts.contains(&t.item) && self.parts.contains(&parent) =>
                {
                    Some(parent)
                }
                _ => self.parts.contains(&t.item).then_some(t.item),
            };
            if let Some(part) = part {
                a.part = Some(part);
                match self.held.as_ref() {
                    Some(held) if t.part_slot && held.item == part => {
                        a.actions.0 |= ActionMask::MOUNT;
                    }
                    None if !t.part_slot && fitted(&self.parts, items, part) => {
                        a.actions.0 |= ActionMask::DISMOUNT;
                    }
                    _ => {}
                }
            }
            if let Some(item) = items.get(t.item)
                && !t.part_slot
            {
                if matches!(
                    self.kinds.get(&t.item),
                    Some(ItemKind::Rim | ItemKind::Wheel | ItemKind::Tire)
                ) {
                    a.context = FeedbackContext::Wheel;
                }
                if self.held.is_none() {
                    match item.location {
                        ItemLocation::Attached(parent)
                            if matches!(self.kinds.get(&t.item), Some(ItemKind::Tire)) =>
                        {
                            a.actions.0 |= ActionMask::DISMOUNT;
                            a.rim = Some(parent);
                            a.tire = Some(t.item);
                        }
                        ItemLocation::WheelSlot(_)
                            if matches!(
                                self.kinds.get(&t.item),
                                Some(ItemKind::Rim | ItemKind::Wheel)
                            ) =>
                        {
                            a.actions.0 |= ActionMask::DISMOUNT;
                        }
                        ItemLocation::World
                            if t.slot.is_none()
                                && item.body.is_some_and(|b| {
                                    b != physics.chassis_handle()
                                        && physics
                                            .rapier()
                                            .bodies
                                            .get(b)
                                            .is_some_and(|body| body.is_dynamic())
                                }) =>
                        {
                            a.actions.0 |= ActionMask::PICKUP;
                        }
                        _ => {}
                    }
                } else if let Some(held) = self.held.as_ref() {
                    if let Some(slot) = t.slot {
                        let occupied = items
                            .iter()
                            .any(|i| i.location == ItemLocation::WheelSlot(slot));
                        if !occupied
                            && self
                                .slot_fits
                                .get(&slot)
                                .is_some_and(|s| s.contains(&held.item))
                        {
                            a.actions.0 |= ActionMask::MOUNT;
                            a.rim = Some(held.item);
                        }
                    } else {
                        let pair = if matches!(self.kinds.get(&held.item), Some(ItemKind::Tire)) {
                            (t.item, held.item)
                        } else {
                            (held.item, t.item)
                        };
                        let empty = items
                            .iter()
                            .all(|i| i.location != ItemLocation::Attached(pair.0));
                        let target_free = matches!(
                            item.location,
                            ItemLocation::World | ItemLocation::WheelSlot(_)
                        );
                        if empty && target_free && self.tire_fits.contains(&pair) {
                            a.actions.0 |= ActionMask::MOUNT;
                            a.rim = Some(pair.0);
                            a.tire = Some(pair.1);
                        }
                    }
                }
            }
        }
        if player.seat().is_some()
            && a.actions.0 & (ActionMask::USE | ActionMask::DOOR | ActionMask::TUNE) == 0
            && let Some(s) = self.seats.iter().find(|s| Some(s.id) == player.seat())
            && player.can_exit(physics, &s.exit)
        {
            a.actions.0 |= ActionMask::EXIT;
        }
        if a.actions.0 & ActionMask::MOUNT != 0 {
            a.actions.0 &= !ActionMask::DROP;
        }
        a.consumes_mouse = a.actions.0 & (ActionMask::DOOR | ActionMask::TUNE) != 0;
        a
    }
    pub fn feedback<'a>(
        &self,
        a: &ActionAvailability,
        items: &ItemStore,
        reference: &'a ReferenceManifest,
    ) -> InteractionFeedback<'a> {
        let name = a
            .item
            .and_then(|id| reference.item(id))
            .map_or("", |i| i.name.as_str());
        let fluid = a
            .opening
            .and_then(|id| reference.opening(id))
            .and_then(|o| {
                items
                    .get(o.tank)
                    .and_then(|i| i.fluid.as_ref())
                    .map(|f| FluidFeedback {
                        amount: f.mixture.total(),
                        capacity: f.capacity_liters,
                        quantities: f.mixture.quantities_liters,
                        presentation: &o.presentation,
                    })
            });
        InteractionFeedback {
            actions: a.actions,
            context: a.context,
            name,
            carrying: self.held.is_some(),
            fluid,
        }
    }
}
