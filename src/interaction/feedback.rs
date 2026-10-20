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
    pub const SWITCH: u32 = 1024;
    pub const PUSH: u32 = 2048;
    pub const GRIP: u32 = 4096;
    pub const THROW: u32 = 8192;
    pub const ROTATE: u32 = 32768;
    pub const RELEASE_GRIP: u32 = 65536;
    pub const PRIMARY_SWITCH: u32 = 131072;
    pub const SCROLL: u32 = 262144;
    pub const CONSUME: u32 = 524288;
    pub const DRINK: u32 = 1048576;
    pub const HELD_SWITCH: u32 = 2097152;
    pub const WASH: u32 = 4194304;
    pub const SCRUB: u32 = 8388608;
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
    Mechanism = 7,
    Part = 8,
    Push = 9,
    LightSwitch = 10,
    RadioVolume = 11,
    RadioTuning = 12,
    Food = 13,
    Paint = 14,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ActionAvailability {
    pub actions: ActionMask,
    pub context: FeedbackContext,
    pub item: Option<ItemId>,
    pub part: Option<ItemId>,
    pub rim: Option<ItemId>,
    pub opening: Option<OpeningId>,
    pub slot: Option<SlotTarget>,
    pub consumes_mouse: bool,
    pub light_control: Option<LightControlId>,
    pub lmb_use: bool,
    pub fill_reservation: Option<OpeningId>,
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
    fn can_reserve_fill(&self, opening: &OpeningState, items: &ItemStore) -> bool {
        let Some(held) = self.held() else {
            return false;
        };
        if opening.valve <= 0. {
            return false;
        }
        let (source, destination) = if self.supplies.contains(&opening.reference.tank) {
            (opening.reference.tank, held)
        } else {
            (held, opening.reference.tank)
        };
        let destination_open = !self.supplies.contains(&opening.reference.tank)
            || self
                .openings
                .iter()
                .any(|o| o.reference.tank == destination && o.valve > 0.);
        source != destination
            && destination_open
            && self
                .tank_rules
                .get(&source)
                .is_some_and(|r| !r.no_fill_from)
            && self
                .tank_rules
                .get(&destination)
                .is_some_and(|r| !r.no_fill_this)
            && items
                .get(source)
                .and_then(|i| i.fluid.as_ref())
                .is_some_and(|f| f.mixture().total() > 0.)
    }
    pub fn available_actions(
        &self,
        aim: &ResolvedAim,
        player: &PlayerState,
        items: &ItemStore,
        physics: &PhysicsWorld,
        needs: crate::survival::NeedsOutput,
    ) -> ActionAvailability {
        if needs.dead {
            return ActionAvailability::default();
        }
        let valid = |target: Option<AimTarget>| {
            target.filter(|target| {
                target.distance.is_finite()
                    && target.distance >= 0.
                    && target.distance <= self.reach
            })
        };
        let use_target = valid(aim.use_target);
        let grab_target = valid(aim.grab_target);
        let installation_target = valid(aim.installation_target);
        let filling_target = valid(aim.filling_target);
        let focus = valid(aim.focus_target);
        let mut a = ActionAvailability::default();
        a.item = use_target
            .or(grab_target)
            .or(filling_target)
            .or(installation_target)
            .or(valid(aim.push_target))
            .or(focus)
            .map(|t| t.item);
        if self.held().is_none()
            && let Some(id) = self.equipment.item()
            && self.held_controls.contains_key(&id)
        {
            a.actions.0 |= ActionMask::HELD_SWITCH;
            a.context = FeedbackContext::LightSwitch;
            a.item = Some(id);
        }
        if let Some(held) = self.held() {
            a.actions.0 |= ActionMask::DROP | ActionMask::THROW;
            if items
                .get(held)
                .is_some_and(|item| item.food.is_some() && !item.removed)
                && needs.can_eat
            {
                a.actions.0 |= ActionMask::CONSUME;
                a.context = FeedbackContext::Food;
                a.item = Some(held);
            } else if self
                .tank_rules
                .get(&held)
                .is_some_and(|tank| !tank.no_drink)
                && items
                    .get(held)
                    .and_then(|item| item.fluid.as_ref())
                    .is_some_and(|fluid| fluid.mixture().total() > 0.)
                && a.fill_reservation.is_none()
            {
                a.actions.0 |= ActionMask::DRINK;
                a.item = Some(held);
            }
            if self.held_controls.contains_key(&held) {
                a.actions.0 |= ActionMask::HELD_SWITCH;
                a.context = FeedbackContext::LightSwitch;
            }
            if self.carry.allows_held_rotation() {
                a.actions.0 |= ActionMask::ROTATE;
            }
        }
        if let Some(t) = use_target {
            if self.mechanisms.contains(t.item) {
                a.context = FeedbackContext::Mechanism;
                if self.mechanisms.radio_tunable(t.item) {
                    a.context = if self.mechanisms.radio_volume(t.item) {
                        FeedbackContext::RadioVolume
                    } else {
                        FeedbackContext::RadioTuning
                    };
                    a.actions.0 |= ActionMask::TUNE | ActionMask::SCROLL;
                } else if self.mechanisms.turn_state(t.item).is_some() {
                    a.actions.0 |= ActionMask::SWITCH;
                    if self.mechanisms.primary_switch(t.item) {
                        a.actions.0 |= ActionMask::PRIMARY_SWITCH;
                    }
                } else {
                    a.actions.0 |= ActionMask::DOOR;
                    if self.mechanisms.scroll_eligible(t.item) {
                        a.actions.0 |= ActionMask::SCROLL;
                    }
                }
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
            }
            if self.seats.iter().any(|s| s.item == t.item)
                && self.held().is_none()
                && player.seat().is_none()
            {
                a.actions.0 |= ActionMask::ENTER;
                a.context = FeedbackContext::Seat;
            }
        }
        if let Some(t) = filling_target
            && let Some(o) = self
                .openings
                .iter()
                .find(|o| o.reference.cap == t.item && o.valve > 0.)
            && let Some(held) = self.held()
        {
            a.opening = Some(o.reference.id);
            if self.can_reserve_fill(o, items) {
                a.fill_reservation = Some(o.reference.id);
            }
            let (source, dest) = if self.supplies.contains(&o.reference.tank) {
                (o.reference.tank, held)
            } else {
                (held, o.reference.tank)
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
                    .is_some_and(|f| f.mixture().total() > 0.)
                && items
                    .get(dest)
                    .and_then(|i| i.fluid.as_ref())
                    .is_some_and(|f| f.mixture().total() < f.capacity_liters)
            {
                a.actions.0 |= ActionMask::FILL;
            }
        }
        if a.fill_reservation.is_some() {
            a.actions.0 &= !ActionMask::DRINK;
        }
        if self.held().is_none() {
            if let Some(t) = grab_target
                && matches!(self.kinds.get(&t.item), Some(ItemKind::Tire))
                && let Some(ItemLocation::Attached(rim)) = items.get(t.item).map(|i| i.location)
                && self.is_wheel(rim)
                && items
                    .get(rim)
                    .is_some_and(|item| item.location == ItemLocation::World)
                && self
                    .carry
                    .allows_pickup(t.item, player.seat().is_some(), true)
                && items
                    .get(rim)
                    .and_then(|item| item.body)
                    .is_some_and(|body| {
                        self.carry
                            .can_acquire(t.item, t.point, player, body, physics)
                            .is_ok()
                    })
            {
                a.actions.0 |= ActionMask::DISMOUNT;
                a.rim = Some(rim);
                a.part = Some(t.item);
                a.context = FeedbackContext::Wheel;
            } else if let Some(t) = grab_target
                && aim.grab_block.is_none()
                && let Some(item) = items.get(t.item)
            {
                a.part = Some(t.item);
                if self.is_wheel(t.item) || matches!(self.kinds.get(&t.item), Some(ItemKind::Tire))
                {
                    a.context = FeedbackContext::Wheel;
                }
                match item.location {
                    ItemLocation::Attached(_)
                        if self.storage.stored(t.item)
                            && self
                                .carry
                                .allows_pickup(t.item, player.seat().is_some(), true)
                            && self.storage.loose_body(t.item).is_some_and(|body| {
                                self.carry
                                    .can_acquire(t.item, t.point, player, body, physics)
                                    .is_ok()
                            }) =>
                    {
                        a.actions.0 |= ActionMask::PICKUP;
                    }
                    ItemLocation::Attached(_)
                        if self.attachments.target(t.item).is_some()
                            && self
                                .carry
                                .allows_pickup(t.item, player.seat().is_some(), true)
                            && self.attachments.loose_body(t.item).is_some_and(|body| {
                                self.carry
                                    .can_acquire(t.item, t.point, player, body, physics)
                                    .is_ok()
                            }) =>
                    {
                        a.actions.0 |= ActionMask::DISMOUNT;
                        a.context = FeedbackContext::Part;
                    }
                    ItemLocation::PartSlot(_)
                        if self.parts.can_remove(t.item, player, items).is_ok()
                            && self.parts.loose_body(t.item).is_some_and(|body| {
                                self.carry
                                    .can_acquire(t.item, t.point, player, body, physics)
                                    .is_ok()
                            }) =>
                    {
                        a.actions.0 |= ActionMask::DISMOUNT;
                        if use_target.is_none() {
                            a.context = FeedbackContext::Part;
                        }
                    }
                    ItemLocation::WheelSlot(_)
                        if self.is_wheel(t.item)
                            && self
                                .carry
                                .allows_pickup(t.item, player.seat().is_some(), true)
                            && !self.parts.covered(t.item) =>
                    {
                        a.actions.0 |= ActionMask::DISMOUNT;
                    }
                    ItemLocation::World
                        if t.slot.is_none()
                            && item.body.is_some_and(|body| {
                                self.carry
                                    .can_acquire(t.item, t.point, player, body, physics)
                                    .is_ok()
                            }) =>
                    {
                        a.actions.0 |= ActionMask::PICKUP;
                    }
                    _ => {}
                }
            }
        } else if let Some(held) = self.held() {
            if let Some(t) = installation_target
                && aim.installation_block.is_none()
                && let Some(slot) = t.slot
            {
                let fits = match slot {
                    SlotTarget::Part(slot) => self.parts.can_install(held, slot, items).is_ok(),
                    SlotTarget::Attachment(target) => {
                        self.attachments.can_mount(held, target, items).is_ok()
                    }
                    SlotTarget::Wheel(_) | SlotTarget::Tire(_) => {
                        self.wheel_installation_block(aim, items).is_none()
                    }
                };
                if fits {
                    a.actions.0 |= ActionMask::MOUNT;
                    a.slot = Some(slot);
                    a.part = Some(held);
                    a.rim = self.is_wheel(held).then_some(held);
                }
            }
        }
        if self.push.gripping() {
            a.actions.0 |= ActionMask::RELEASE_GRIP;
            a.context = FeedbackContext::Push;
        } else if self.held().is_none()
            && let Some(t) = valid(aim.push_target)
            && self.push.supports(t.item)
        {
            a.actions.0 |= if self
                .collider_bindings
                .get(&t.collider)
                .is_some_and(|b| self.push.is_grip(t.item, b.collider))
            {
                ActionMask::GRIP
            } else {
                ActionMask::PUSH
            };
            a.context = FeedbackContext::Push;
        }
        let drink_guard = self.held().is_some_and(|held| {
            self.tank_rules
                .get(&held)
                .is_some_and(|tank| !tank.no_drink)
        });
        let latched_fill = self
            .fill_opening
            .and_then(|id| self.openings.iter().find(|o| o.reference.id == id))
            .is_some_and(|opening| self.can_reserve_fill(opening, items));
        let food_guard = self.held().is_some_and(|held| {
            items
                .get(held)
                .is_some_and(|item| item.food.is_some() && !item.removed)
        });
        a.lmb_use = a.fill_reservation.is_none()
            && !latched_fill
            && !drink_guard
            && !food_guard
            && !self.cleaning.reserves_lmb(self.held());
        if self.latched_mechanism.is_none()
            && let Some(t) = aim.light_target
            && t.distance.is_finite()
            && t.distance >= 0.
            && t.distance <= self.reach
            && self.light_controls.get(&t.collider) == Some(&t.control)
            && self.light_nodes.get(&t.collider) == Some(&t.node)
            && !(self.seat_action_latched && self.usable_light_controls.contains(&t.control))
        {
            a.light_control = Some(t.control);
            a.actions.0 |= ActionMask::USE;
            a.actions.0 &= !(ActionMask::DOOR
                | ActionMask::SWITCH
                | ActionMask::PRIMARY_SWITCH
                | ActionMask::SCROLL);
            a.context = FeedbackContext::LightSwitch;
        }
        if player.seat().is_some()
            && a.actions.0
                & (ActionMask::USE | ActionMask::DOOR | ActionMask::TUNE | ActionMask::SWITCH)
                == 0
            && let Some(s) = self.seats.iter().find(|s| Some(s.id) == player.seat())
            && player.can_exit(physics, &s.exit)
        {
            a.actions.0 |= ActionMask::EXIT;
        }
        if self.carry.throw_charge() >= self.carry.reference_throw_minimum() {
            a.actions.0 &= !ActionMask::MOUNT;
        }
        let blocked_install = aim.installation_block.is_some()
            || (self.held().is_some() && installation_target.is_some() && a.slot.is_none());
        if a.actions.0 & ActionMask::MOUNT != 0 || blocked_install {
            a.actions.0 &= !(ActionMask::DROP | ActionMask::THROW);
        }
        a.consumes_mouse = a.actions.0 & (ActionMask::DOOR | ActionMask::TUNE) != 0;
        if let Some(held) = self
            .held()
            .filter(|_| self.cleaning.reserves_lmb(self.held()))
        {
            a.item = Some(held);
            a.actions.0 &= !(ActionMask::DOOR
                | ActionMask::FILL
                | ActionMask::PRIMARY_SWITCH
                | ActionMask::HELD_SWITCH);
            match self.cleaning.prompt(held) {
                Some(185) => a.actions.0 |= ActionMask::WASH,
                Some(186) => a.actions.0 |= ActionMask::SCRUB,
                _ => {}
            }
        }
        a
    }
    pub fn paint_feedback(
        &self,
        aim: &ResolvedAim,
        reference: &ReferenceManifest,
    ) -> Option<(f32, f32)> {
        let item = aim.focus_target?.item;
        let tool = reference
            .conditions
            .sprays
            .iter()
            .find(|t| t.item == item)?;
        if tool.hide_look_at_count || (self.held() == Some(item) && tool.hide_carried_count) {
            return None;
        }
        Some((self.cleaning.get(item)?.quantity?, tool.maximum_quantity))
    }
    pub fn feedback<'a>(
        &self,
        a: &ActionAvailability,
        items: &'a ItemStore,
        reference: &'a ReferenceManifest,
    ) -> InteractionFeedback<'a> {
        let name = a
            .item
            .map_or("", |id| labels::display_name(reference, items, id));
        let fluid = a
            .opening
            .and_then(|id| reference.openings.iter().find(|o| o.id == id))
            .and_then(|o| {
                items
                    .get(o.tank)
                    .and_then(|i| i.fluid.as_ref())
                    .map(|f| FluidFeedback {
                        amount: f.mixture().total(),
                        capacity: f.capacity_liters,
                        quantities: f.mixture().quantities_liters,
                        presentation: &o.presentation,
                    })
            });
        InteractionFeedback {
            actions: a.actions,
            context: if a
                .item
                .is_some_and(|id| reference.conditions.sprays.iter().any(|t| t.item == id))
            {
                FeedbackContext::Paint
            } else {
                a.context
            },
            name,
            carrying: self.held().is_some(),
            fluid,
        }
    }
}
