//! Checked source `mountStuff` storage volumes, policy, and item flags.
use super::{
    ColliderId, ItemId, NodeId, PartSlotId, ReferenceError, ReferenceManifest, SourceId, require,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageRecord {
    pub policy: StoragePolicy,
    pub volumes: Vec<StorageVolumeReference>,
    pub prototype_volumes: Vec<PrototypeVolumeReference>,
    pub flags: Vec<StorageFlag>,
    pub prototype_flags: Vec<PrototypeStorageFlag>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoragePolicy {
    pub settings_source: SourceId,
    pub default_setting: u8,
    pub mount_mask: u32,
    pub drop_delay_seconds: f32,
    pub scan_interval_seconds: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageVolumeReference {
    pub source: SourceId,
    pub node: NodeId,
    pub owner: ItemId,
    pub collider: ColliderId,
    pub center: [f32; 3],
    pub half_extents: [f32; 3],
    pub storage_type: u8,
    pub only_manual: bool,
    pub no_physics_lock: bool,
    pub owner_pickup: Option<SourceId>,
    pub dont_lock_if_owner_picked_up: bool,
    pub slots: Vec<PartSlotId>,
    pub usables: Vec<SourceId>,
    pub active: bool,
}

/// Storage volume in a world prototype. The node is an index in the prototype.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypeVolumeReference {
    pub prototype: SourceId,
    pub source: SourceId,
    pub node: u32,
    /// Box centre in the prototype root frame, the frame of `WorldPrototypeBody::pose`.
    pub center: [f32; 3],
    /// Half extents with the node chain scale applied.
    pub half_extents: [f32; 3],
    /// Box rotation in the prototype root frame.
    pub rotation: [f32; 4],
    pub storage_type: u8,
    /// Gate `usablescript` sources. Each is a world mechanism of the same prototype.
    pub usables: Vec<SourceId>,
    pub dont_lock_if_owner_picked_up: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageFlag {
    pub item: ItemId,
    pub dont_mount: bool,
    pub dont_mount_till_pickup: bool,
}

/// Pickupable flags of a world prototype root.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrototypeStorageFlag {
    pub prototype: SourceId,
    pub dont_mount: bool,
    pub dont_mount_till_pickup: bool,
}

/// Returns true if setting 2 lets this storage type lock items.
/// Setting 2 blocks `passengerRoom` (5) and `outside` (7).
pub fn stores(storage_type: u8) -> bool {
    !matches!(storage_type, 5 | 7)
}

fn positive(half_extents: &[f32; 3]) -> bool {
    half_extents
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
}

impl StorageRecord {
    pub(super) fn validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError> {
        let policy = &self.policy;
        require(
            policy.default_setting == 2
                && policy.mount_mask == 8_388_609
                && policy.drop_delay_seconds == 1.0
                && policy.scan_interval_seconds == 1.5
                && self.volumes.len() == 8,
            "storage policy",
        )?;
        for volume in &self.volumes {
            require(
                manifest.items.iter().any(|item| item.id == volume.owner)
                    && manifest.colliders.iter().any(|collider| {
                        collider.id == volume.collider
                            && (collider.node == volume.node
                                || manifest.nodes.iter().any(|node| {
                                    node.id == collider.node && node.parent == Some(volume.node)
                                }))
                    })
                    && volume
                        .slots
                        .iter()
                        .all(|id| manifest.part_slots.iter().any(|slot| slot.id == *id))
                    && volume.center.iter().all(|value| value.is_finite())
                    && positive(&volume.half_extents)
                    && !volume.only_manual
                    && !volume.no_physics_lock,
                "storage volume",
            )?;
        }
        let world = &manifest.world;
        for volume in &self.prototype_volumes {
            let prototype = world
                .prototypes
                .iter()
                .find(|prototype| prototype.source == volume.prototype);
            require(
                prototype.is_some_and(|prototype| (volume.node as usize) < prototype.nodes.len())
                    && volume.usables.iter().all(|usable| {
                        world.mechanisms.iter().any(|mechanism| {
                            mechanism.prototype == volume.prototype
                                && mechanism.recipe.source == *usable
                        })
                    })
                    && volume.center.iter().all(|value| value.is_finite())
                    && positive(&volume.half_extents)
                    && volume.rotation.iter().all(|value| value.is_finite()),
                "storage prototype volume",
            )?;
        }
        for flag in &self.prototype_flags {
            require(
                (flag.dont_mount || flag.dont_mount_till_pickup)
                    && world
                        .prototypes
                        .iter()
                        .any(|prototype| prototype.source == flag.prototype),
                "storage prototype flag",
            )?;
        }
        for flag in &self.flags {
            require(
                (flag.dont_mount || flag.dont_mount_till_pickup)
                    && manifest.items.iter().any(|item| item.id == flag.item),
                "storage flag item",
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn setting_two_blocks_passenger_room_and_outside() {
        let allowed: Vec<bool> = (0..=7).map(super::stores).collect();
        assert_eq!(allowed, [true, true, true, true, true, false, true, false]);
    }
}
