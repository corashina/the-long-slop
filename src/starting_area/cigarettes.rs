//! Cigarette packs, the mouth wear slot, and smoking. Source `fpscontroller` smoke paths.
use super::GarageRuntime;
use crate::interaction::cigarettes::{
    BurnRates, CigaretteState, ExhaleRates, ExhaleState, MouthSlot, PackState,
};
use crate::interaction::feedback::ActionMask;
use crate::interaction::items::node_pose;
use crate::reference::{ItemId, NodeId, ReferenceError, ReferenceManifest};
use crate::simulation::{FIXED_DT, FrameInput, keys, player_keys};
use crate::vehicle::{VehiclePose, from_rapier, to_rapier};
use rapier3d::math::Pose;

pub struct CigaretteRuntime {
    pub rates: BurnRates,
    pub exhale_rates: ExhaleRates,
    pub pack_prototype: usize,
    pub cigarette_prototype: usize,
    pub packs: Vec<(ItemId, PackState)>,
    pub cigarettes: Vec<(ItemId, CigaretteState)>,
    pub mouth: MouthSlot<ItemId>,
    pub exhale: ExhaleState,
    /// Mouth smoke emission rate. `None` disables the emitter.
    pub mouth_smoke: Option<f32>,
    pub mouth_anchor: NodeId,
    /// Worn body pose in the mouth anchor frame.
    worn_offset: Pose,
    show_stats: bool,
    hold_seconds: f32,
    tab_elapsed: Option<f32>,
}

impl CigaretteRuntime {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let a = &reference.acquisition;
        let prototype = |item: u32| {
            reference
                .item_prototypes
                .iter()
                .position(|p| p.item_id == Some(item))
                .ok_or_else(|| ReferenceError(format!("Missing item prototype {item}")))
        };
        let pack_prototype = prototype(a.pack_item_id)?;
        let cigarette_prototype = prototype(a.cigarette_item_id)?;
        let record = &reference.item_prototypes[cigarette_prototype];
        let consumable = record
            .consumable
            .as_ref()
            .ok_or_else(|| ReferenceError("Missing cigarette consumable".into()))?;
        let wearable_type = a
            .packs
            .first()
            .ok_or_else(|| ReferenceError("Missing cigarette pack rules".into()))?
            .wearable_type;
        let mouth_anchor = *reference
            .clothing
            .pose
            .wearable_anchors
            .get(wearable_type as usize)
            .ok_or_else(|| ReferenceError("Missing mouth wear anchor".into()))?;
        // Source Wear: the root, or the wearable CenterObj, takes the anchor pose.
        let root = to_rapier(node_pose(reference, record.root)?);
        let root_in_anchor = match reference.clothing.wearables.iter().find(|w| {
            w.prototype == record.id && w.type_index == wearable_type && w.use_center_object
        }) {
            Some(w) => {
                let center = w
                    .center
                    .ok_or_else(|| ReferenceError("Missing wearable center".into()))?;
                (root.inverse() * to_rapier(node_pose(reference, center)?)).inverse()
            }
            None => Pose::IDENTITY,
        };
        let body = &record.bodies[0];
        let body_local = to_rapier(VehiclePose {
            position: body.pose.translation,
            rotation: body.pose.rotation,
        });
        Ok(Self {
            rates: BurnRates::from(consumable),
            exhale_rates: ExhaleRates::from(&reference.small_items.exhale),
            pack_prototype,
            cigarette_prototype,
            packs: Vec::new(),
            cigarettes: Vec::new(),
            mouth: MouthSlot::default(),
            exhale: ExhaleState::default(),
            mouth_smoke: None,
            mouth_anchor,
            worn_offset: root_in_anchor * body_local,
            show_stats: !consumable.dont_show_stats,
            hold_seconds: reference.inventory.equip_hold_seconds,
            tab_elapsed: None,
        })
    }
    pub fn pack(&self, id: ItemId) -> Option<&PackState> {
        self.packs.iter().find(|(i, _)| *i == id).map(|(_, p)| p)
    }
    pub fn cigarette(&self, id: ItemId) -> Option<&CigaretteState> {
        self.cigarettes
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, c)| c)
    }
    fn cigarette_mut(&mut self, id: ItemId) -> Option<&mut CigaretteState> {
        self.cigarettes
            .iter_mut()
            .find(|(i, _)| *i == id)
            .map(|(_, c)| c)
    }
    /// HUD actions for the held item.
    pub fn actions(&self, held: Option<ItemId>) -> u32 {
        let Some(held) = held else { return 0 };
        let mut mask = 0;
        if self.pack(held).is_some_and(PackState::can_take) {
            mask |= ActionMask::TAKE_ONE;
        }
        if self.cigarette(held).is_some() {
            mask |= ActionMask::SMOKE;
            if self.mouth.item.is_none() {
                mask |= ActionMask::PUT_ON;
            }
        }
        mask
    }
}

fn strip(input: &mut FrameInput, bits: u32) {
    input.held &= !bits;
    input.pressed &= !bits;
    input.released &= !bits;
}

impl GarageRuntime {
    /// Source `pickedUp`, or the right-hand item when one is held there.
    fn smoke_held(&self) -> Option<ItemId> {
        self.area
            .interaction
            .equipment
            .item()
            .or(self.area.interaction.held())
    }

    fn spawn_cigarette(
        &mut self,
        reference: &ReferenceManifest,
        root: VehiclePose,
    ) -> Result<ItemId, ReferenceError> {
        let prototype = self.area.cigarettes.cigarette_prototype;
        let id = super::spawned::spawn_prototype(
            reference,
            &mut self.area,
            &mut self.physics,
            prototype,
            root,
        )?[0];
        let state = CigaretteState::new(&self.area.cigarettes.rates);
        self.area.cigarettes.cigarettes.push((id, state));
        Ok(id)
    }

    pub(crate) fn spawn_pack(
        &mut self,
        reference: &ReferenceManifest,
        root: VehiclePose,
        count: u32,
    ) -> Result<ItemId, ReferenceError> {
        let prototype = self.area.cigarettes.pack_prototype;
        let id = super::spawned::spawn_prototype(
            reference,
            &mut self.area,
            &mut self.physics,
            prototype,
            root,
        )?[0];
        self.area.cigarettes.packs.push((id, PackState { count }));
        Ok(id)
    }

    /// Source `Wear`: drop the item and attach it to the mouth anchor.
    fn wear_cigarette(&mut self, id: ItemId) {
        if !self.area.cigarettes.mouth.wear_if_empty(id) {
            return;
        }
        if self.area.interaction.held() == Some(id) {
            self.area
                .interaction
                .drop_held(&mut self.area.items, &mut self.physics);
        }
        if let Some(body) = self.area.items.get(id).and_then(|i| i.body) {
            self.physics.edit_body(body, |b| b.set_enabled(false));
        }
        self.sync_mouth();
    }

    /// Source `TakeOff`: the worn cigarette goes back to the hand.
    fn take_off_cigarette(&mut self) {
        let Some(id) = self.area.cigarettes.mouth.take_off() else {
            return;
        };
        let Some(body) = self.area.items.get(id).and_then(|i| i.body) else {
            return;
        };
        self.physics.edit_body(body, |b| b.set_enabled(true));
        self.physics.refresh_queries();
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

    /// Keep the worn cigarette on the mouth anchor. Run after the pose frames update.
    pub(super) fn sync_mouth(&mut self) {
        let Some(id) = self.area.cigarettes.mouth.item else {
            return;
        };
        let Some(anchor) = self.area.pose.anchor(self.area.cigarettes.mouth_anchor) else {
            return;
        };
        let pose = to_rapier(anchor) * self.area.cigarettes.worn_offset;
        if let Some(body) = self.area.items.get(id).and_then(|i| i.body) {
            self.physics.edit_body(body, |b| {
                b.set_position(pose, false);
                b.set_enabled(false);
            });
        }
        if let Some(state) = self.area.items.get_mut(id) {
            state.pose = from_rapier(&pose);
        }
    }

    /// Pack, wear, and smoke input. Consumed keys leave `input`.
    pub(super) fn step_cigarettes(
        &mut self,
        reference: &ReferenceManifest,
        input: &mut FrameInput,
    ) {
        let dt = FIXED_DT;
        let held = self.smoke_held();
        let mut smoking = false;
        // Short Tab release puts a held cigarette on (source canTab / puton).
        if input.pressed & keys::RECENTER != 0 {
            self.area.cigarettes.tab_elapsed = Some(0.);
        } else if let Some(t) = &mut self.area.cigarettes.tab_elapsed {
            *t += dt;
        }
        if input.released & keys::RECENTER != 0 {
            let short = self
                .area
                .cigarettes
                .tab_elapsed
                .take()
                .is_some_and(|t| t <= self.area.cigarettes.hold_seconds);
            if let Some(id) = self.area.interaction.held()
                && short
                && self.area.cigarettes.cigarette(id).is_some()
                && self.area.cigarettes.mouth.item.is_none()
            {
                self.wear_cigarette(id);
                strip(input, keys::RECENTER);
                return;
            }
        }
        if self.area.interaction.held().is_none()
            && input.player_released & player_keys::TAKE_OFF != 0
        {
            self.take_off_cigarette();
        }
        // Worn cigarette: Caps Lock.
        if input.player_held & player_keys::SMOKE != 0
            && let Some(id) = self.area.cigarettes.mouth.item
        {
            let rates = self.area.cigarettes.rates;
            let exhale = self.area.cigarettes.exhale_rates;
            let c = &mut self.area.cigarettes;
            let state = c
                .cigarettes
                .iter_mut()
                .find(|(i, _)| *i == id)
                .map(|(_, s)| s);
            if let Some(state) = state {
                if state.hp > 0. {
                    c.exhale.charge(&exhale, dt);
                }
                let effect = state.eat(&rates, dt, false);
                self.survival.queue(effect);
                if self.area.cigarettes.show_stats {
                    self.survival.show_stats();
                }
                smoking = true;
            }
        }
        if let Some(id) = held {
            // Held pack: E takes one.
            if self
                .area
                .cigarettes
                .pack(id)
                .is_some_and(PackState::can_take)
                && input.pressed & keys::USE != 0
            {
                strip(input, keys::USE);
                self.take_one(reference, id);
            }
            // Held cigarette: LMB.
            if self.area.cigarettes.cigarette(id).is_some() {
                if input.held & keys::PRIMARY != 0 {
                    let rates = self.area.cigarettes.rates;
                    let exhale = self.area.cigarettes.exhale_rates;
                    let state = self.area.cigarettes.cigarette_mut(id).unwrap();
                    let effect = state.eat(&rates, dt, false);
                    let lit = state.hp > 0.;
                    self.survival.queue(effect);
                    if self.area.cigarettes.show_stats {
                        self.survival.show_stats();
                    }
                    if lit {
                        smoking = true;
                        self.area.cigarettes.exhale.charge(&exhale, dt);
                    }
                }
                strip(input, keys::PRIMARY);
            }
        }
        let blocked = smoking
            || input.player_held & player_keys::SMOKE != 0
            || input.held & keys::PRIMARY != 0;
        let exhale = self.area.cigarettes.exhale_rates;
        self.area.cigarettes.mouth_smoke = self.area.cigarettes.exhale.emit(&exhale, blocked, dt);
    }

    fn take_one(&mut self, reference: &ReferenceManifest, pack: ItemId) {
        let Some(state) = self.area.items.get(pack) else {
            return;
        };
        let Some(spawned) = self.area.spawned.get(pack).copied() else {
            return;
        };
        let record = &reference.item_prototypes[spawned.prototype];
        let body = &record.bodies[0];
        // The cigarette prefab spawns at the pack pickupable transform.
        let pickup = (|| -> Result<Pose, ReferenceError> {
            let root = to_rapier(node_pose(reference, record.root)?);
            let node = to_rapier(node_pose(reference, record.pickups[spawned.pickup].node)?);
            Ok(to_rapier(state.pose)
                * to_rapier(VehiclePose {
                    position: body.pose.translation,
                    rotation: body.pose.rotation,
                })
                .inverse()
                * root.inverse()
                * node)
        })();
        let Ok(pickup) = pickup else {
            self.notice = "Cannot take a cigarette.";
            return;
        };
        let p = self
            .area
            .cigarettes
            .packs
            .iter_mut()
            .find(|(i, _)| *i == pack)
            .unwrap();
        if !p.1.take_one() {
            return;
        }
        match self.spawn_cigarette(reference, from_rapier(&pickup)) {
            Ok(id) => self.wear_cigarette(id),
            Err(_) => self.notice = "Cannot take a cigarette.",
        }
    }

    /// Automatic burn and ember decay of every cigarette.
    pub(super) fn advance_cigarettes(&mut self) {
        let rates = self.area.cigarettes.rates;
        for (_, c) in &mut self.area.cigarettes.cigarettes {
            c.update(&rates, FIXED_DT);
        }
    }
}

/// New game only: the 3 loose StartSpawnItems packs and the selected start car pack,
/// as physics-space root poses and counts.
pub(super) fn start_pack_poses(
    reference: &ReferenceManifest,
    startup: &crate::world::StartupTerrain,
    origin: [f64; 3],
    vehicle: usize,
) -> Result<Vec<(VehiclePose, u32)>, ReferenceError> {
    let local = |pose: VehiclePose| VehiclePose {
        position: std::array::from_fn(|i| (pose.position[i] as f64 - origin[i]) as f32),
        rotation: pose.rotation,
    };
    let mut packs = Vec::new();
    for v in &reference.small_items.start_packs {
        let pose = crate::world::source_world_pose(&reference.world, startup, &v.transform, &[])?;
        packs.push((local(pose), v.ammo.count));
    }
    // Source DestroyOthers keeps only the selected start car group.
    let car = &reference.vehicles[vehicle].source;
    for v in reference
        .acquisition
        .startup_packs
        .iter()
        .filter(|v| &v.vehicle == car)
    {
        let parent = v
            .group_pose
            .parent
            .as_ref()
            .ok_or_else(|| ReferenceError("Start car pack group parent".into()))?;
        let pose = crate::world::source_world_pose(
            &reference.world,
            startup,
            parent,
            &[v.group_pose.transform, v.pose.transform],
        )?;
        packs.push((local(pose), v.ammo.count));
    }
    Ok(packs)
}

impl CigaretteRuntime {
    pub fn capture_save(
        &self,
        spawned: &super::spawned::SpawnedItems,
    ) -> crate::save::CigarettesSnapshot {
        use crate::save::{CigaretteSnapshot, PackSnapshot};
        let serial = |id: ItemId| spawned.get(id).map(|s| s.serial);
        let mut packs: Vec<_> = self
            .packs
            .iter()
            .filter_map(|(id, p)| {
                Some(PackSnapshot {
                    serial: serial(*id)?,
                    count: p.count,
                })
            })
            .collect();
        packs.sort_by_key(|p| p.serial);
        let mut cigarettes: Vec<_> = self
            .cigarettes
            .iter()
            .filter_map(|(id, c)| {
                Some(CigaretteSnapshot {
                    serial: serial(*id)?,
                    hp: c.hp,
                    ate_once: c.ate_once,
                })
            })
            .collect();
        cigarettes.sort_by_key(|c| c.serial);
        crate::save::CigarettesSnapshot {
            packs,
            cigarettes,
            mouth: self.mouth.item.and_then(serial),
        }
    }
}

/// Saved records name spawned items of the matching prototype.
pub(crate) fn validate_save(
    reference: &ReferenceManifest,
    spawned: &[crate::save::SpawnedItemSnapshot],
    saved: &crate::save::CigarettesSnapshot,
) -> Result<(), crate::save::SaveError> {
    let invalid = |detail: &str| crate::save::SaveError::InvalidLinks(detail.into());
    let of = |serial: u32, item: u32| {
        spawned.iter().any(|s| {
            s.serial == serial
                && reference.item_prototypes.iter().any(|p| {
                    p.item_id == Some(item)
                        && crate::save::SourceKey::from(&p.source) == s.prototype
                })
        })
    };
    let a = &reference.acquisition;
    let mut seen = std::collections::HashSet::new();
    for p in &saved.packs {
        if !of(p.serial, a.pack_item_id) || !seen.insert(p.serial) {
            return Err(invalid("Pack save identity"));
        }
    }
    for c in &saved.cigarettes {
        if !of(c.serial, a.cigarette_item_id) || !seen.insert(c.serial) {
            return Err(invalid("Cigarette save identity"));
        }
        if !c.hp.is_finite() || !(0. ..=1.).contains(&c.hp) {
            return Err(crate::save::SaveError::CorruptData("Cigarette hp".into()));
        }
    }
    if saved
        .mouth
        .is_some_and(|m| !saved.cigarettes.iter().any(|c| c.serial == m))
    {
        return Err(invalid("Mouth slot item"));
    }
    Ok(())
}

impl GarageRuntime {
    /// Rebuild pack, cigarette, and mouth state after the spawned items exist.
    pub(super) fn restore_cigarettes(&mut self, saved: &crate::save::CigarettesSnapshot) {
        let item = |serial: u32, prototype: usize| {
            self.area
                .spawned
                .iter()
                .find(|s| s.serial == serial && s.prototype == prototype)
                .map(|s| s.id)
        };
        let c = &self.area.cigarettes;
        let packs: Vec<_> = saved
            .packs
            .iter()
            .filter_map(|p| {
                Some((
                    item(p.serial, c.pack_prototype)?,
                    PackState { count: p.count },
                ))
            })
            .collect();
        let cigarettes: Vec<_> = saved
            .cigarettes
            .iter()
            .filter_map(|s| {
                Some((
                    item(s.serial, c.cigarette_prototype)?,
                    CigaretteState::restored(&c.rates, s.hp, s.ate_once, FIXED_DT),
                ))
            })
            .collect();
        let mouth = saved
            .mouth
            .and_then(|serial| item(serial, c.cigarette_prototype));
        let c = &mut self.area.cigarettes;
        c.packs = packs;
        c.cigarettes = cigarettes;
        c.mouth = MouthSlot::default();
        if let Some(id) = mouth {
            self.wear_cigarette(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interaction::cigarettes::tests::cigarette;
    use crate::starting_area::spawned::{SpawnedItem, SpawnedItems};

    #[test]
    fn save_round_trip_keeps_hp_counts_and_mouth() {
        let rates = cigarette();
        let mut spawned = SpawnedItems::default();
        for (id, serial, prototype) in [(5, 0, 0), (6, 1, 1), (7, 2, 1)] {
            spawned.record(SpawnedItem {
                id: ItemId(id),
                serial,
                prototype,
                pickup: 0,
            });
        }
        let mut burnt = CigaretteState::new(&rates);
        burnt.eat(&rates, 10., false);
        let mut runtime = CigaretteRuntime {
            rates,
            exhale_rates: ExhaleRates {
                max_rate: 30.,
                charge_speed: 0.5,
            },
            pack_prototype: 0,
            cigarette_prototype: 1,
            packs: vec![(ItemId(5), PackState { count: 18 })],
            cigarettes: vec![(ItemId(7), burnt), (ItemId(6), CigaretteState::new(&rates))],
            mouth: MouthSlot::default(),
            exhale: ExhaleState::default(),
            mouth_smoke: None,
            mouth_anchor: NodeId(0),
            worn_offset: Pose::IDENTITY,
            show_stats: true,
            hold_seconds: 0.2,
            tab_elapsed: None,
        };
        runtime.mouth.wear_if_empty(ItemId(7));
        let saved = runtime.capture_save(&spawned);
        assert_eq!(saved.packs[0].count, 18);
        assert_eq!(saved.mouth, Some(2));
        assert_eq!(saved.cigarettes[1].serial, 2);
        assert!(saved.cigarettes[1].ate_once);
        let json = serde_json::to_string(&saved).unwrap();
        let loaded: crate::save::CigarettesSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, saved);
        let restored = CigaretteState::restored(&rates, loaded.cigarettes[1].hp, true, FIXED_DT);
        assert_eq!(restored.hp, burnt.hp);
        assert!(restored.ate_once);
    }
}
