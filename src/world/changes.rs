//! Session edits and their destination index. Records contain no runtime handles.
use super::{AbsolutePose, ObjectKey, Placement, WorldBounds};
use crate::reference::SourceId;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentSnapshot {
    pub owner: Option<ObjectKey>,
    pub slot: Option<SourceId>,
    pub local_pose: AbsolutePose,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MechanismSnapshot {
    pub degrees: [f32; 2],
    pub slide: f32,
    pub enabled: bool,
    pub turn_state: u8,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoodSnapshot {
    pub source: SourceId,
    pub bite: u8,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSnapshot {
    pub pose: AbsolutePose,
    pub condition: f32,
    pub food: Option<FoodSnapshot>,
    pub temperature: Option<f32>,
    pub fluids: Option<Vec<crate::fluids::FluidEntry>>,
    pub valve: Option<f32>,
    pub lamp: Option<bool>,
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub sleeping: bool,
    pub held: bool,
    pub attachment: Option<AttachmentSnapshot>,
    pub mechanism: Option<MechanismSnapshot>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectChange {
    /// None is a retained removal, not an absent edit.
    pub snapshot: Option<ObjectSnapshot>,
    /// Original source caller placement. Initialized arrays are replayed.
    pub replay: Option<Placement>,
    pub instance: Option<super::GeneratedId>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChangeRecordIndex(usize);

#[derive(Clone, Debug, Default, Serialize)]
pub struct ChangeRecords {
    records: Vec<(ObjectKey, ObjectChange)>,
    #[serde(skip)]
    identities: HashMap<ObjectKey, usize>,
    #[serde(skip)]
    attachment_owners: HashMap<ObjectKey, usize>,
    #[serde(skip)]
    attachment_owner_bytes: usize,
    #[serde(skip)]
    regions: BTreeMap<(i64, i64), Vec<usize>>,
    #[serde(skip)]
    revision: u64,
    #[serde(skip)]
    payload_bytes: usize,
    #[serde(skip)]
    region_bytes: usize,
}
impl PartialEq for ChangeRecords {
    fn eq(&self, other: &Self) -> bool {
        self.records == other.records
    }
}
impl<'de> Deserialize<'de> for ChangeRecords {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Envelope {
            records: Vec<(ObjectKey, ObjectChange)>,
        }
        let envelope = Envelope::deserialize(deserializer)?;
        let mut result = Self::default();
        for (key, change) in envelope.records {
            if result.identities.contains_key(&key) {
                return Err(serde::de::Error::custom("Duplicate world change identity"));
            }
            result.insert(key, change);
        }
        Ok(result)
    }
}
fn region(p: [f64; 3]) -> (i64, i64) {
    ((p[0] / 256.).floor() as i64, (p[2] / 256.).floor() as i64)
}
impl ChangeRecords {
    pub(crate) fn generated_records(&self) -> Self {
        let mut result = Self::default();
        for (key, change) in self.iter() {
            if matches!(key, ObjectKey::Generated(_)) {
                result.insert(key.clone(), change.clone());
            }
        }
        result
    }
    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) fn has_attachment_dependents(&self, key: &ObjectKey) -> bool {
        self.attachment_owners.contains_key(key)
    }
    fn index_attachment_owner(&mut self, owner: &ObjectKey) {
        if let Some(count) = self.attachment_owners.get_mut(owner) {
            *count += 1;
        } else {
            self.attachment_owners.insert(owner.clone(), 1);
            self.attachment_owner_bytes += key_heap(owner);
        }
    }
    fn unindex_attachment_owner(&mut self, owner: &ObjectKey) {
        let count = self.attachment_owners.get_mut(owner).unwrap();
        *count -= 1;
        if *count == 0 {
            self.attachment_owners.remove(owner);
            self.attachment_owner_bytes =
                self.attachment_owner_bytes.saturating_sub(key_heap(owner));
        }
    }
    pub fn record(&mut self, key: ObjectKey, current: ObjectSnapshot, baseline: &ObjectSnapshot) {
        if &current == baseline {
            self.forget(&key);
            return;
        }
        if self
            .get(&key)
            .is_some_and(|c| c.snapshot.as_ref() == Some(&current))
        {
            return;
        }
        if let Some(&index) = self.identities.get(&key) {
            let old_owner = self.records[index]
                .1
                .snapshot
                .as_ref()
                .and_then(attachment_owner);
            let new_owner = attachment_owner(&current);
            if old_owner != new_owner {
                let old_owner = old_owner.cloned();
                let new_owner = new_owner.cloned();
                if let Some(owner) = old_owner.as_ref() {
                    self.unindex_attachment_owner(owner);
                }
                if let Some(owner) = new_owner.as_ref() {
                    self.index_attachment_owner(owner);
                }
            }
            let old_region = self.records[index]
                .1
                .snapshot
                .as_ref()
                .map(|s| region(s.pose.position));
            let new_region = region(current.pose.position);
            if old_region != Some(new_region) {
                self.unindex(index);
                self.index(new_region, index);
            }
            self.payload_bytes = self.payload_bytes.saturating_sub(
                self.records[index]
                    .1
                    .snapshot
                    .as_ref()
                    .map_or(0, ObjectSnapshot::heap_bytes),
            );
            self.payload_bytes += current.heap_bytes();
            self.records[index].1.snapshot = Some(current);
            self.revision = self.revision.wrapping_add(1);
        } else {
            self.insert(
                key,
                ObjectChange {
                    snapshot: Some(current),
                    replay: None,
                    instance: None,
                },
            );
        }
    }

    pub(crate) fn retain_dependency(&mut self, key: ObjectKey, snapshot: ObjectSnapshot) {
        if self.get(&key).is_none() {
            self.insert(
                key,
                ObjectChange {
                    snapshot: Some(snapshot),
                    replay: None,
                    instance: None,
                },
            );
        }
    }
    pub fn set_replay(
        &mut self,
        key: &ObjectKey,
        placement: &Placement,
        instance: &super::GeneratedId,
    ) {
        if let Some(index) = self.identities.get(key) {
            if self.records[*index].1.replay.is_none() {
                self.records[*index].1.replay = Some(placement.clone());
                self.records[*index].1.instance = Some(instance.clone());
                self.payload_bytes += super::contents::placement_heap(
                    self.records[*index].1.replay.as_ref().unwrap(),
                ) + super::contents::identity_heap(
                    self.records[*index].1.instance.as_ref().unwrap(),
                );
                self.revision = self.revision.wrapping_add(1);
            }
        }
    }
    pub fn remove(&mut self, key: ObjectKey) {
        let replay = self.get(&key).and_then(|c| c.replay.clone());
        let instance = self.get(&key).and_then(|c| c.instance.clone());
        self.insert(
            key,
            ObjectChange {
                snapshot: None,
                replay,
                instance,
            },
        );
    }
    fn unindex(&mut self, index: usize) {
        if let Some(snapshot) = &self.records[index].1.snapshot {
            let cell = region(snapshot.pose.position);
            if let Some(entries) = self.regions.get_mut(&cell) {
                entries.retain(|i| *i != index);
                if entries.is_empty() {
                    self.region_bytes = self
                        .region_bytes
                        .saturating_sub(entries.capacity() * std::mem::size_of::<usize>());
                    self.regions.remove(&cell);
                }
            }
        }
    }
    fn index(&mut self, cell: (i64, i64), index: usize) {
        let entries = self.regions.entry(cell).or_default();
        let before = entries.capacity();
        entries.push(index);
        self.region_bytes += (entries.capacity() - before) * std::mem::size_of::<usize>();
    }
    fn insert(&mut self, key: ObjectKey, change: ObjectChange) {
        let index = if let Some(&index) = self.identities.get(&key) {
            let old_owner = self.records[index]
                .1
                .snapshot
                .as_ref()
                .and_then(attachment_owner);
            let new_owner = change.snapshot.as_ref().and_then(attachment_owner);
            if old_owner != new_owner {
                let old_owner = old_owner.cloned();
                let new_owner = new_owner.cloned();
                if let Some(owner) = old_owner.as_ref() {
                    self.unindex_attachment_owner(owner);
                }
                if let Some(owner) = new_owner.as_ref() {
                    self.index_attachment_owner(owner);
                }
            }
            self.unindex(index);
            self.payload_bytes = self
                .payload_bytes
                .saturating_sub(change_heap(&self.records[index].0, &self.records[index].1));
            self.payload_bytes += change_heap(&self.records[index].0, &change);
            self.records[index].1 = change;
            index
        } else {
            if let Some(owner) = change.snapshot.as_ref().and_then(attachment_owner) {
                self.index_attachment_owner(owner);
            }
            let index = self.records.len();
            self.identities.insert(key.clone(), index);
            self.payload_bytes += change_heap(&key, &change);
            self.records.push((key, change));
            index
        };
        if let Some(snapshot) = &self.records[index].1.snapshot {
            self.index(region(snapshot.pose.position), index);
        }
        self.revision = self.revision.wrapping_add(1);
    }
    pub(crate) fn forget(&mut self, key: &ObjectKey) -> bool {
        let Some(index) = self.identities.remove(key) else {
            return false;
        };
        if let Some(owner) = self.records[index]
            .1
            .snapshot
            .as_ref()
            .and_then(attachment_owner)
            .cloned()
        {
            self.unindex_attachment_owner(&owner);
        }
        self.unindex(index);
        let last = self.records.len() - 1;
        if index != last {
            self.unindex(last);
        }
        self.payload_bytes = self
            .payload_bytes
            .saturating_sub(change_heap(&self.records[index].0, &self.records[index].1));
        self.records.swap_remove(index);
        if index != last {
            self.identities.insert(self.records[index].0.clone(), index);
            if let Some(snapshot) = &self.records[index].1.snapshot {
                self.index(region(snapshot.pose.position), index);
            }
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }
    pub(crate) fn retain(
        &mut self,
        mut keep: impl FnMut(&ObjectKey, &ObjectChange) -> bool,
    ) -> bool {
        let revision = self.revision;
        let mut index = 0;
        while index < self.records.len() {
            let (key, change) = &self.records[index];
            if keep(key, change) {
                index += 1;
            } else {
                let key = key.clone();
                self.forget(&key);
            }
        }
        self.revision != revision
    }
    pub fn get(&self, key: impl std::borrow::Borrow<ObjectKey>) -> Option<&ObjectChange> {
        self.identities
            .get(key.borrow())
            .map(|i| &self.records[*i].1)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&ObjectKey, &ObjectChange)> {
        self.records.iter().map(|(key, change)| (key, change))
    }
    pub(crate) fn query_reservation(&self, bounds: WorldBounds) -> (usize, usize) {
        let mut count = 0;
        let mut bytes = 0;
        let min = region(bounds.min);
        let max = region(bounds.max);
        for (_, entries) in self.regions.range((min.0, i64::MIN)..=(max.0, i64::MAX)) {
            for &index in entries {
                let (key, record) = &self.records[index];
                if record.snapshot.as_ref().is_some_and(|s| {
                    (0..3).all(|i| {
                        s.pose.position[i] >= bounds.min[i] && s.pose.position[i] <= bounds.max[i]
                    })
                }) {
                    count += 1;
                    bytes += std::mem::size_of::<ObjectKey>()
                        + key_heap(key)
                        + record.replay.as_ref().map_or(0, |p| {
                            std::mem::size_of::<Placement>() + super::contents::placement_heap(p)
                        });
                }
            }
        }
        (count, bytes)
    }
    pub(crate) fn get_indexed(&self, index: ChangeRecordIndex) -> (&ObjectKey, &ObjectChange) {
        let (key, change) = &self.records[index.0];
        (key, change)
    }
    pub(crate) fn indices_in(&self, bounds: WorldBounds, output: &mut Vec<ChangeRecordIndex>) {
        output.clear();
        let min = region(bounds.min);
        let max = region(bounds.max);
        for (_, indices) in self.regions.range((min.0, i64::MIN)..=(max.0, i64::MAX)) {
            for &index in indices {
                let (_, change) = &self.records[index];
                if change.snapshot.as_ref().is_some_and(|s| {
                    (0..3).all(|i| {
                        s.pose.position[i] >= bounds.min[i] && s.pose.position[i] <= bounds.max[i]
                    })
                }) {
                    output.push(ChangeRecordIndex(index));
                }
            }
        }
    }
    // Test oracle: the previous key query stays independent of indexed access.
    #[cfg(test)]
    pub(super) fn old_keys_in(&self, bounds: WorldBounds, output: &mut Vec<ObjectKey>) {
        output.clear();
        let min = region(bounds.min);
        let max = region(bounds.max);
        for (_, indices) in self.regions.range((min.0, i64::MIN)..=(max.0, i64::MAX)) {
            for &index in indices {
                let (key, change) = &self.records[index];
                if change.snapshot.as_ref().is_some_and(|s| {
                    (0..3).all(|i| {
                        s.pose.position[i] >= bounds.min[i] && s.pose.position[i] <= bounds.max[i]
                    })
                }) {
                    output.push(key.clone());
                }
            }
        }
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.records.capacity() * std::mem::size_of::<(ObjectKey, ObjectChange)>()
            + self.identities.capacity().saturating_mul(2)
                * (std::mem::size_of::<(ObjectKey, usize)>() + 1)
            + 16
            + self.attachment_owners.capacity().saturating_mul(2)
                * (std::mem::size_of::<(ObjectKey, usize)>() + 1)
            + 16
            + self.attachment_owner_bytes
            + self.payload_bytes
            + self.region_bytes
            + (self.regions.len() + 1)
                * (11 * std::mem::size_of::<((i64, i64), Vec<usize>)>()
                    + 12 * std::mem::size_of::<usize>()
                    + 64)
    }
}
fn attachment_owner(snapshot: &ObjectSnapshot) -> Option<&ObjectKey> {
    snapshot.attachment.as_ref()?.owner.as_ref()
}
/// Both source and destination must leave both horizontal focus windows.
pub(crate) fn roadside_change_expired(
    change: &ObjectChange,
    foci: [[f64; 3]; 2],
    radius_squared: f32,
) -> bool {
    let Some(source) = &change.replay else {
        return false;
    };
    foci.into_iter().all(|focus| {
        super::contents::distance(focus, source.pose.position) >= radius_squared
            && change.snapshot.as_ref().is_none_or(|snapshot| {
                super::contents::distance(focus, snapshot.pose.position) >= radius_squared
            })
    })
}
pub(crate) fn external_attachment(change: &ObjectChange, records: &ChangeRecords) -> bool {
    change
        .snapshot
        .as_ref()
        .and_then(|s| s.attachment.as_ref())
        .is_some_and(|attachment| {
            attachment.owner.as_ref().is_none_or(|owner| {
                records.get(owner).is_none_or(|parent| {
                    change.instance.is_none() || parent.instance != change.instance
                })
            })
        })
}
/// Keep each external child and every recorded owner up to the root.
pub(crate) fn external_attachment_chain(
    records: &ChangeRecords,
    dependencies: &mut HashSet<ObjectKey>,
) {
    dependencies.clear();
    for (key, change) in records.iter() {
        if !external_attachment(change, records) {
            continue;
        }
        let mut next = Some(key);
        while let Some(key) = next {
            if !dependencies.insert(key.clone()) {
                break;
            }
            next = records
                .get(key)
                .and_then(|record| record.snapshot.as_ref())
                .and_then(|snapshot| snapshot.attachment.as_ref())
                .and_then(|attachment| attachment.owner.as_ref());
        }
    }
}
pub(crate) fn key_heap(key: &ObjectKey) -> usize {
    match key {
        ObjectKey::Imported(_) => 0,
        ObjectKey::Generated(id) => super::contents::identity_heap(id),
    }
}

/// Capture active state before teardown. Owner identities never contain runtime handles.
pub(crate) fn capture(
    id: crate::reference::ItemId,
    items: &crate::interaction::items::ItemStore,
    interaction: &crate::interaction::InteractionState,
    physics: &crate::vehicle::PhysicsWorld,
    reference: &crate::reference::ReferenceManifest,
) -> Option<ObjectSnapshot> {
    use crate::interaction::items::ItemLocation;
    use crate::vehicle::{from_rapier, to_rapier};
    let state = items.get(id)?;
    let mut snapshot = items.generated_snapshot(id, interaction.generated_valve(id))?;
    snapshot.mechanism = interaction.mechanisms.snapshot(id);
    snapshot.food = state.food.map(|food| FoodSnapshot {
        source: reference.survival.foods[food.prototype_index]
            .source
            .clone(),
        bite: food.bite,
    });
    snapshot.held = state.location == ItemLocation::Held;
    if let Some(body) = state.body.and_then(|b| physics.rapier().bodies.get(b)) {
        snapshot.linear_velocity = body.linvel().to_array();
        snapshot.angular_velocity = body.angvel().to_array();
        snapshot.sleeping = body.is_sleeping();
    }
    let relation =
        match state.location {
            ItemLocation::Attached(parent) => Some((Some(parent), None)),
            ItemLocation::PartSlot(slot) => interaction
                .parts
                .slot_identity(slot)
                .map(|(owner, source)| (owner, Some(source.clone()))),
            ItemLocation::WheelSlot(slot) => reference
                .wheel_slots
                .iter()
                .find(|s| s.id == slot)
                .map(|s| {
                    (
                        reference
                            .items
                            .iter()
                            .find(|i| {
                                reference.vehicles.iter().any(|vehicle| {
                                    vehicle.wheel_slots.contains(&slot) && i.node == vehicle.root
                                })
                            })
                            .map(|i| i.id),
                        Some(s.source.clone()),
                    )
                }),
            _ => None,
        };
    if let Some((parent, slot)) = relation {
        let pose = if let Some(parent) = parent.and_then(|p| items.get(p)) {
            from_rapier(&(to_rapier(parent.pose).inverse() * to_rapier(state.pose)))
        } else {
            state.local_pose
        };
        snapshot.attachment = Some(AttachmentSnapshot {
            owner: parent.map(|p| items.object_key(p)),
            slot,
            local_pose: AbsolutePose {
                position: pose.position.map(f64::from),
                rotation: pose.rotation,
            },
        });
    }
    Some(snapshot)
}

pub(crate) fn restore_motion(
    id: crate::reference::ItemId,
    snapshot: &ObjectSnapshot,
    items: &crate::interaction::items::ItemStore,
    physics: &mut crate::vehicle::PhysicsWorld,
) {
    if let Some(item) = items.get(id) {
        if let Some(handle) = item.body {
            physics.try_edit_body(handle, |body| {
                body.set_position(
                    crate::vehicle::to_rapier(item.pose)
                        * crate::vehicle::to_rapier(item.local_pose).inverse(),
                    false,
                );
                body.set_linvel(snapshot.linear_velocity.into(), false);
                body.set_angvel(snapshot.angular_velocity.into(), false);
                if snapshot.sleeping {
                    body.sleep();
                } else {
                    body.wake_up(true);
                }
            });
        }
    }
}

impl ObjectSnapshot {
    pub(crate) fn heap_bytes(&self) -> usize {
        self.food.as_ref().map_or(0, |f| f.source.file.capacity())
            + self.fluids.as_ref().map_or(0, |entries| {
                entries.capacity() * std::mem::size_of::<crate::fluids::FluidEntry>()
            })
            + self.attachment.as_ref().map_or(0, |a| {
                a.owner.as_ref().map_or(0, key_heap)
                    + a.slot.as_ref().map_or(0, |s| s.file.capacity())
            })
    }
}

fn change_heap(key: &ObjectKey, change: &ObjectChange) -> usize {
    key_heap(key) * 2
        + change
            .replay
            .as_ref()
            .map_or(0, super::contents::placement_heap)
        + change
            .instance
            .as_ref()
            .map_or(0, super::contents::identity_heap)
        + change
            .snapshot
            .as_ref()
            .map_or(0, ObjectSnapshot::heap_bytes)
}
