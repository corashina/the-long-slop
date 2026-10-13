//! Checked portable item and storage bindings.
use super::{
    ColliderId, ItemId, LightControlId, LightControllerKind, NodeId, PartSlotId, ReferenceError,
    ReferenceManifest, SourceId, require,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlashlightRecord {
    pub models: Vec<FlashlightModelReference>,
    pub instances: Vec<FlashlightInstanceReference>,
    pub storage: StorageLockRecord,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlashlightModelReference {
    pub source: SourceId,
    pub controller: SourceId,
    pub node: NodeId,
    pub meshes: Vec<SourceId>,
    pub materials: Vec<SourceId>,
    pub renderers: Vec<SourceId>,
    pub lights: Vec<SourceId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlashlightInstanceReference {
    pub model: SourceId,
    pub item: ItemId,
    pub node: NodeId,
    pub pickup: SourceId,
    pub save: SourceId,
    pub controller: LightControlId,
    pub startup_active: bool,
    pub colliders: Vec<ColliderId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageLockRecord {
    pub settings_source: SourceId,
    pub default_setting: u8,
    pub mount_mask: u32,
    pub drop_delay_seconds: f32,
    pub scan_interval_seconds: f32,
    pub volumes: Vec<StorageVolumeReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageVolumeReference {
    pub source: SourceId,
    pub node: NodeId,
    pub owner: ItemId,
    pub collider: ColliderId,
    pub storage_type: u8,
    pub only_manual: bool,
    pub no_physics_lock: bool,
    pub owner_pickup: Option<SourceId>,
    pub dont_lock_if_owner_picked_up: bool,
    pub slots: Vec<PartSlotId>,
    pub usables: Vec<SourceId>,
    pub active: bool,
}

impl FlashlightRecord {
    pub(super) fn validate(&self, manifest: &ReferenceManifest) -> Result<(), ReferenceError> {
        require(
            self.models.len() == 2
                && self.instances.len() == 2
                && self
                    .instances
                    .iter()
                    .filter(|item| item.startup_active)
                    .count()
                    == 1,
            "flashlight model and startup count",
        )?;
        for (model, instance) in self.models.iter().zip(&self.instances) {
            require(
                model.source == instance.model
                    && model.node == instance.node
                    && !model.meshes.is_empty()
                    && !model.materials.is_empty()
                    && !model.renderers.is_empty()
                    && !model.lights.is_empty(),
                "flashlight model binding",
            )?;
            let item = manifest.items.iter().find(|item| item.id == instance.item);
            let node = manifest.nodes.iter().find(|node| node.id == instance.node);
            let body =
                item.and_then(|item| manifest.bodies.iter().find(|body| body.id == item.body));
            let control = manifest
                .lighting
                .controllers
                .iter()
                .find(|control| control.id == instance.controller);
            require(
                item.is_some_and(|item| item.node == instance.node && item.pickup.is_some())
                    && node.is_some_and(|node| node.active == instance.startup_active)
                    && body.is_some_and(|body| body.dynamic && body.mass_kg == 1.0)
                    && control.is_some_and(|control| {
                        control.node == instance.node
                            && control.kind == LightControllerKind::Switch
                            && !control.startup_on
                            && control.enabled == instance.startup_active
                            && control.states.len() == 1
                    })
                    && !instance.colliders.is_empty()
                    && instance.colliders.iter().all(|id| {
                        manifest.colliders.iter().any(|collider| {
                            collider.id == *id && collider.enabled == instance.startup_active
                        })
                    }),
                "flashlight item, body, controller, or collider",
            )?;
        }
        let storage = &self.storage;
        require(
            storage.default_setting == 2
                && storage.mount_mask == 8_388_609
                && storage.drop_delay_seconds == 1.0
                && storage.scan_interval_seconds == 1.5
                && storage.volumes.len() == 8,
            "flashlight storage policy",
        )?;
        for volume in &storage.volumes {
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
                    && !volume.only_manual
                    && !volume.no_physics_lock,
                "flashlight storage volume",
            )?;
        }
        Ok(())
    }
}
