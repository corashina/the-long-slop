//! Worn items. Source `fpscontroller.Wear`, `TakeOff`, and the `puton` Tab path.
use super::{GarageRuntime, StartingArea};
use crate::interaction::InteractionState;
use crate::interaction::feedback::ActionMask;
use crate::interaction::items::{ItemLocation, node_pose};
use crate::interaction::wear::{WEAR_SLOTS, WornItems};
use crate::reference::{ItemId, NodeId, ReferenceError, ReferenceManifest, SourceId};
use crate::simulation::{FIXED_DT, FrameInput, keys, player_keys};
use crate::vehicle::{PhysicsWorld, VehiclePose, to_rapier};
use rapier3d::math::Pose;
use rapier3d::prelude::RigidBodyType;

/// One pickup of a `clothing.wearables` prototype.
struct WearRule {
    pickup: SourceId,
    slot: usize,
    /// Source `edible != null`: it comes off first.
    consumable: bool,
    /// Body pose in the anchor frame.
    worn_offset: Pose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WornItem {
    pub id: ItemId,
    rule: usize,
}

pub struct WearRuntime {
    pub worn: WornItems<WornItem>,
    rules: Vec<WearRule>,
    anchors: [NodeId; WEAR_SLOTS],
    hold_seconds: f32,
    tab_elapsed: Option<f32>,
}

impl WearRuntime {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let anchors =
            <[NodeId; WEAR_SLOTS]>::try_from(reference.clothing.pose.wearable_anchors.as_slice())
                .map_err(|_| ReferenceError("Missing wearable anchors".into()))?;
        let mut rules = Vec::new();
        for w in &reference.clothing.wearables {
            let slot = w.type_index as usize;
            if slot >= WEAR_SLOTS {
                return Err(ReferenceError("Wearable type out of range".into()));
            }
            let record = reference
                .item_prototypes
                .iter()
                .find(|p| p.id == w.prototype)
                .ok_or_else(|| ReferenceError("Missing wearable prototype".into()))?;
            // Source Wear: the root, or the wearable CenterObj, takes the anchor pose.
            let root = to_rapier(node_pose(reference, record.root)?);
            let root_in_anchor = if w.use_center_object {
                let center = w
                    .center
                    .ok_or_else(|| ReferenceError("Missing wearable center".into()))?;
                (root.inverse() * to_rapier(node_pose(reference, center)?)).inverse()
            } else {
                Pose::IDENTITY
            };
            for pickup in &record.pickups {
                let body = record
                    .bodies
                    .iter()
                    .find(|b| b.id == pickup.body)
                    .ok_or_else(|| ReferenceError("Missing wearable body".into()))?;
                let body_local = to_rapier(VehiclePose {
                    position: body.pose.translation,
                    rotation: body.pose.rotation,
                });
                rules.push(WearRule {
                    pickup: pickup.pickup.source.clone(),
                    slot,
                    consumable: record.consumable.is_some(),
                    worn_offset: root_in_anchor * body_local,
                });
            }
        }
        Ok(Self {
            worn: WornItems::default(),
            rules,
            anchors,
            hold_seconds: reference.inventory.equip_hold_seconds,
            tab_elapsed: None,
        })
    }
    /// The rule of a wearable item, by its pickup component.
    fn rule(&self, interaction: &InteractionState, id: ItemId) -> Option<usize> {
        let source = &interaction.pickup_rule(id)?.source;
        self.rules.iter().position(|r| r.pickup == *source)
    }
    /// The slot of a wearable item.
    pub fn slot(&self, interaction: &InteractionState, id: ItemId) -> Option<usize> {
        self.rule(interaction, id).map(|r| self.rules[r].slot)
    }
    pub fn item(&self, slot: usize) -> Option<ItemId> {
        self.worn.slots[slot].map(|w| w.id)
    }
    /// The held item can go on: it is wearable and its slot is empty.
    fn can_put_on(&self, interaction: &InteractionState, id: ItemId) -> bool {
        self.slot(interaction, id)
            .is_some_and(|slot| self.worn.slots[slot].is_none())
    }
    /// HUD action for the held item.
    pub fn actions(&self, interaction: &InteractionState) -> u32 {
        match interaction.held() {
            Some(id) if self.can_put_on(interaction, id) => ActionMask::PUT_ON,
            _ => 0,
        }
    }
}

impl StartingArea {
    /// Source `Wear`: drop the item, remove it from physics, and put it in its slot.
    pub(super) fn put_on(&mut self, physics: &mut PhysicsWorld, id: ItemId) -> bool {
        let Some(rule) = self.wear.rule(&self.interaction, id) else {
            return false;
        };
        let slot = self.wear.rules[rule].slot;
        if !self.wear.worn.wear_if_empty(slot, WornItem { id, rule }) {
            return false;
        }
        if self.interaction.held() == Some(id) {
            self.interaction.drop_held(&mut self.items, physics);
        }
        let item = self.items.get_mut(id).expect("wearable item");
        item.location = ItemLocation::Worn(slot as u8);
        if let Some(body) = item.body {
            physics.edit_body(body, |b| b.set_enabled(false));
        }
        physics.refresh_queries();
        self.sync_worn(physics);
        true
    }

    /// Keep each worn item on its anchor. Run after the pose frames update.
    pub(super) fn sync_worn(&self, physics: &mut PhysicsWorld) {
        for (slot, item) in self.wear.worn.slots.iter().enumerate() {
            let Some(item) = item else { continue };
            let Some(anchor) = self.pose.anchor(self.wear.anchors[slot]) else {
                continue;
            };
            let pose = to_rapier(anchor) * self.wear.rules[item.rule].worn_offset;
            if let Some(body) = self.items.get(item.id).and_then(|i| i.body) {
                physics.edit_body(body, |b| {
                    b.set_position(pose, false);
                    b.set_enabled(false);
                });
            }
        }
    }

    /// Wear a restored item. The save names its slot.
    pub(super) fn restore_worn(
        &mut self,
        physics: &mut PhysicsWorld,
        slot: usize,
        id: ItemId,
    ) -> Result<(), crate::save::SaveError> {
        if self.wear.slot(&self.interaction, id) != Some(slot) || !self.put_on(physics, id) {
            return Err(crate::save::SaveError::InvalidLinks(
                "Worn item slot".into(),
            ));
        }
        Ok(())
    }
}

impl GarageRuntime {
    /// Source `TakeOff`: the item gets a dynamic CCD body. `pickup` sends it to the hand.
    fn take_off(&mut self, id: ItemId, pickup: bool) {
        let item = self.area.items.get_mut(id).expect("worn item");
        item.location = ItemLocation::World;
        let Some(body) = item.body else {
            return;
        };
        self.physics.edit_body(body, |b| {
            b.set_body_type(RigidBodyType::Dynamic, true);
            b.enable_ccd(true);
            b.set_enabled(true);
        });
        self.physics.refresh_queries();
        if !pickup {
            return;
        }
        let point = self.physics.rapier().bodies[body].translation().to_array();
        if self
            .area
            .interaction
            .pickup_item(
                id,
                point,
                &self.area.player,
                &mut self.area.items,
                &mut self.physics,
            )
            .is_err()
        {
            self.notice = "Cannot take this item.";
        }
    }

    /// Death2: every worn item comes off without pickup and keeps its last pose.
    pub(super) fn take_all_off(&mut self) {
        for item in self.area.wear.worn.take_all().into_iter().flatten() {
            self.take_off(item.id, false);
        }
    }

    /// Short Tab release puts the held item on. `removeSunglasses` release takes one off.
    pub(super) fn step_wear(&mut self, input: &mut FrameInput) {
        let wear = &mut self.area.wear;
        if input.pressed & keys::RECENTER != 0 {
            wear.tab_elapsed = Some(0.);
        } else if let Some(t) = &mut wear.tab_elapsed {
            *t += FIXED_DT;
        }
        if input.released & keys::RECENTER != 0 {
            let short = wear
                .tab_elapsed
                .take()
                .is_some_and(|t| t <= wear.hold_seconds);
            if let Some(id) = self.area.interaction.held()
                && short
                && wear.can_put_on(&self.area.interaction, id)
            {
                self.area.put_on(&mut self.physics, id);
                input.held &= !keys::RECENTER;
                input.pressed &= !keys::RECENTER;
                input.released &= !keys::RECENTER;
                return;
            }
        }
        if self.area.interaction.held().is_none()
            && input.player_released & player_keys::TAKE_OFF != 0
        {
            let rules = &self.area.wear.rules;
            if let Some(item) = self
                .area
                .wear
                .worn
                .take_off_next(|w| rules[w.rule].consumable)
            {
                self.take_off(item.id, true);
            }
        }
    }
}
