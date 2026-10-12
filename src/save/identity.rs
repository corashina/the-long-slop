use super::SaveError;
use crate::reference::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceKey {
    pub file: String,
    pub path_id: String,
}
impl From<&SourceId> for SourceKey {
    fn from(source: &SourceId) -> Self {
        Self {
            file: source.file.clone(),
            path_id: source.path_id.to_string(),
        }
    }
}
impl SourceKey {
    pub fn source_id(&self) -> Result<SourceId, SaveError> {
        if self.file.is_empty() {
            return Err(SaveError::InvalidLinks("Empty source file".into()));
        }
        Ok(SourceId {
            file: self.file.clone(),
            path_id: signed(&self.path_id)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstanceId {
    Initial {
        source: SourceKey,
    },
    Station {
        seed: String,
        index: String,
        role: String,
    },
}
impl InstanceId {
    pub fn initial(source: &SourceId) -> Self {
        Self::Initial {
            source: source.into(),
        }
    }
    pub fn station(&self) -> Result<(u64, i64), SaveError> {
        match self {
            Self::Station { seed, index, role } if role == "pump" => {
                Ok((unsigned(seed)?, signed(index)?))
            }
            _ => Err(SaveError::InvalidLinks(
                "Unknown station role or identity".into(),
            )),
        }
    }
}

/// Reference types have separate maps because source objects can have several roles.
pub struct IdentityMap {
    pub items: HashMap<SourceKey, ItemId>,
    pub bodies: HashMap<SourceKey, BodyId>,
    pub nodes: HashMap<SourceKey, NodeId>,
    pub anchors: HashMap<SourceKey, AnchorId>,
    pub part_slots: HashMap<SourceKey, PartSlotId>,
    pub wheel_slots: HashMap<SourceKey, WheelSlotId>,
    pub seats: HashMap<SourceKey, SeatId>,
    pub mechanisms: HashMap<SourceKey, MechanismId>,
    pub openings: HashMap<SourceKey, OpeningId>,
    pub lights: HashMap<SourceKey, LightId>,
    pub controllers: HashMap<SourceKey, LightControlId>,
    pub vehicle: InstanceId,
}
impl IdentityMap {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, SaveError> {
        Ok(Self {
            items: index(reference.items.iter().map(|r| (&r.source, r.id)))?,
            bodies: index(reference.bodies.iter().map(|r| (&r.source, r.id)))?,
            nodes: index(reference.nodes.iter().map(|r| (&r.source, r.id)))?,
            anchors: index(reference.anchors.iter().map(|r| (&r.source, r.id)))?,
            part_slots: index(reference.part_slots.iter().map(|r| (&r.source, r.id)))?,
            wheel_slots: index(reference.wheel_slots.iter().map(|r| (&r.source, r.id)))?,
            seats: index(reference.seats.iter().map(|r| (&r.source, r.id)))?,
            mechanisms: index(reference.mechanisms.iter().map(|r| (&r.source, r.id)))?,
            openings: index(reference.openings.iter().map(|r| (&r.source, r.id)))?,
            lights: index(reference.lighting.lights.iter().map(|r| (&r.source, r.id)))?,
            controllers: index(
                reference
                    .lighting
                    .controllers
                    .iter()
                    .map(|r| (&r.source, r.id)),
            )?,
            vehicle: InstanceId::initial(&reference.vehicle.source),
        })
    }
    pub fn item_instance(&self, id: ItemId) -> Result<InstanceId, SaveError> {
        Ok(InstanceId::Initial {
            source: source_key(&self.items, id)?,
        })
    }
    pub fn item(&self, instance: &InstanceId) -> Result<ItemId, SaveError> {
        match instance {
            InstanceId::Initial { source } => self
                .items
                .get(source)
                .copied()
                .ok_or_else(|| SaveError::InvalidLinks(format!("Unknown item {source:?}"))),
            InstanceId::Station { .. } => Err(SaveError::InvalidLinks(
                "Station item needs a generated runtime binding".into(),
            )),
        }
    }
}
pub(crate) fn source_key<T: Copy + Eq>(
    map: &HashMap<SourceKey, T>,
    id: T,
) -> Result<SourceKey, SaveError> {
    map.iter()
        .find(|(_, value)| **value == id)
        .map(|(key, _)| key.clone())
        .ok_or_else(|| SaveError::InvalidLinks("Missing runtime source identity".into()))
}
fn index<'a, T: Copy + Eq + Hash>(
    records: impl Iterator<Item = (&'a SourceId, T)>,
) -> Result<HashMap<SourceKey, T>, SaveError> {
    let mut result = HashMap::new();
    let mut ids = std::collections::HashSet::new();
    for (source, id) in records {
        let key = SourceKey::from(source);
        key.source_id()?;
        if result.insert(key, id).is_some() || !ids.insert(id) {
            return Err(SaveError::InvalidLinks(
                "Duplicate source identity or runtime ID".into(),
            ));
        }
    }
    Ok(result)
}
pub(super) fn signed(value: &str) -> Result<i64, SaveError> {
    value
        .parse::<i64>()
        .ok()
        .filter(|number| number.to_string() == value)
        .ok_or_else(|| SaveError::InvalidLinks("Invalid signed decimal identity".into()))
}
pub(crate) fn unsigned(value: &str) -> Result<u64, SaveError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|number| number.to_string() == value)
        .ok_or_else(|| SaveError::InvalidLinks("Invalid unsigned decimal identity".into()))
}
