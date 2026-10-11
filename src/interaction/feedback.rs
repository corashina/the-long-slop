use super::*;

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
    LightSwitch = 7,
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
    pub light_control: Option<LightControlId>,
    pub consumes_mouse: bool,
    pub lmb_use: bool,
    pub fill_reservation: Option<OpeningId>,
}
impl InteractionState {
    fn can_reserve_fill(&self, opening: &OpeningState, items: &ItemStore) -> bool {
        let Some(held) = &self.held else {
            return false;
        };
        if opening.valve <= 0. {
            return false;
        }
        let (source, destination) = if self.supplies.contains(&opening.reference.tank) {
            (opening.reference.tank, held.item)
        } else {
            (held.item, opening.reference.tank)
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
                .is_some_and(|f| f.mixture.total() > 0.)
    }
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
        let latched = self.latched_hinge.filter(|(index, local)| {
            self.bodies
                .get(&self.hinges[*index].reference.body)
                .and_then(|handle| physics.rapier().bodies.get(*handle))
                .is_some_and(|body| {
                    (*body.position() * *local
                        - Vector::from_array(player.body_pose(physics).position))
                    .length_squared()
                        < self.hinge_release_distance_squared
                })
        });
        a.item = latched
            .map(|(index, _)| self.hinges[index].reference.item)
            .or_else(|| target.and_then(|t| t.item));
        a.part = a.item;
        a.slot = target.and_then(|t| t.slot);
        if let Some(id) = a.item {
            if self.hinges.iter().any(|h| h.reference.item == id) {
                a.actions.0 |= ActionMask::DOOR;
                a.context = FeedbackContext::Door;
            }
            if latched.is_none()
                && let Some(o) = self.openings.iter().find(|o| o.reference.cap == id)
            {
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
                if self.can_reserve_fill(o, items) {
                    a.fill_reservation = Some(o.reference.id);
                    let destination = if self.supplies.contains(&o.reference.tank) {
                        self.held.as_ref().unwrap().item
                    } else {
                        o.reference.tank
                    };
                    if items
                        .get(destination)
                        .and_then(|i| i.fluid.as_ref())
                        .is_some_and(|f| f.mixture.total() < f.capacity_liters)
                    {
                        a.actions.0 |= ActionMask::FILL;
                    }
                }
            }
            if self.seats.iter().any(|s| s.item == id)
                && self.held.is_none()
                && player.seat().is_none()
            {
                a.actions.0 |= ActionMask::ENTER;
                a.context = FeedbackContext::Seat;
            }
            if let Some(item) = items.get(id) {
                if matches!(
                    self.kinds.get(&id),
                    Some(ItemKind::Rim | ItemKind::Wheel | ItemKind::Tire)
                ) {
                    a.context = FeedbackContext::Wheel;
                }
                if self.held.is_none() {
                    match item.location {
                        ItemLocation::Attached(parent)
                            if matches!(self.kinds.get(&id), Some(ItemKind::Tire)) =>
                        {
                            a.actions.0 |= ActionMask::DISMOUNT;
                            a.rim = Some(parent);
                            a.tire = Some(id);
                        }
                        ItemLocation::WheelSlot(_) => {
                            a.actions.0 |= ActionMask::DISMOUNT;
                        }
                        ItemLocation::World
                            if a.slot.is_none()
                                && matches!(
                                    self.kinds.get(&id),
                                    Some(
                                        ItemKind::Container
                                            | ItemKind::Rim
                                            | ItemKind::Wheel
                                            | ItemKind::Tire
                                    )
                                )
                                && item.body.is_some_and(|b| {
                                    b != physics.chassis_handle()
                                        && physics
                                            .rapier()
                                            .bodies
                                            .get(b)
                                            .is_some_and(|b| b.is_dynamic())
                                }) =>
                        {
                            a.actions.0 |= ActionMask::PICKUP;
                        }
                        _ => {}
                    }
                } else if a.slot.is_some() {
                    a.actions.0 |= ActionMask::MOUNT;
                }
            }
        }
        let drink_guard = self.held.as_ref().is_some_and(|held| {
            self.tank_rules
                .get(&held.item)
                .is_some_and(|tank| !tank.no_drink)
        });
        let latched_fill = self
            .fill_opening
            .and_then(|id| self.openings.iter().find(|o| o.reference.id == id))
            .is_some_and(|opening| self.can_reserve_fill(opening, items));
        a.lmb_use = a.fill_reservation.is_none() && !latched_fill && !drink_guard;
        if latched.is_none()
            && let Some(t) = target
            && let Some(control) = self.light_controls.get(&t.collider)
            && self.collider_nodes.get(&t.collider) == Some(&t.node)
            && !(self.seat_action_latched && self.usable_light_controls.contains(control))
        {
            a.light_control = Some(*control);
            a.actions.0 |= ActionMask::USE;
            a.context = FeedbackContext::LightSwitch;
        }
        if player.seat().is_some()
            && a.actions.0 & (ActionMask::USE | ActionMask::DOOR | ActionMask::TUNE) == 0
            && let Some(s) = self.seats.iter().find(|s| Some(s.id) == player.seat())
            && player.can_exit(physics, &s.exit)
        {
            a.actions.0 |= ActionMask::EXIT;
        }
        a.consumes_mouse = a.actions.0 & (ActionMask::DOOR | ActionMask::TUNE) != 0;
        a
    }
}
