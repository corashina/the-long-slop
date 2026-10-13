//! Session changes keyed by source identity. Records contain no runtime handles.
use super::{AbsolutePose, ObjectKey, WorldBounds};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSnapshot {
    pub pose: AbsolutePose,
    pub condition: f32,
    pub temperature: Option<f32>,
    pub fluid_quantities: Option<[f32; 6]>,
    pub valve: Option<f32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectChange {
    pub snapshot: ObjectSnapshot,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeRecords {
    records: Vec<(ObjectKey, ObjectChange)>,
}
impl ChangeRecords {
    pub fn record(&mut self, key: ObjectKey, current: ObjectSnapshot, baseline: &ObjectSnapshot) {
        if &current == baseline {
            self.remove(key);
            return;
        }
        if let Some((_, change)) = self.records.iter_mut().find(|(id, _)| id == &key) {
            change.snapshot = current;
        } else {
            self.records.push((key, ObjectChange { snapshot: current }));
        }
    }
    pub fn remove(&mut self, key: ObjectKey) {
        if let Some(index) = self.records.iter().position(|(id, _)| *id == key) {
            self.records.swap_remove(index);
        }
    }
    pub fn get(&self, key: ObjectKey) -> Option<&ObjectChange> {
        self.records
            .iter()
            .find(|(id, _)| *id == key)
            .map(|(_, change)| change)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&ObjectKey, &ObjectChange)> {
        self.records.iter().map(|(key, change)| (key, change))
    }
    pub fn changes_in(&self, bounds: WorldBounds, output: &mut Vec<ObjectKey>) {
        output.clear();
        output.extend(
            self.records
                .iter()
                .filter(|(_, change)| {
                    (0..3).all(|i| {
                        change.snapshot.pose.position[i] >= bounds.min[i]
                            && change.snapshot.pose.position[i] <= bounds.max[i]
                    })
                })
                .map(|(key, _)| key.clone()),
        );
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.records.capacity() * std::mem::size_of::<(ObjectKey, ObjectChange)>()
            + self
                .records
                .iter()
                .map(|(key, _)| match key {
                    ObjectKey::Imported(_) => 0,
                    ObjectKey::Generated(key) => {
                        key.generator.file.capacity()
                            + key.children.capacity()
                                * std::mem::size_of::<(crate::reference::SourceId, u32)>()
                            + key
                                .children
                                .iter()
                                .map(|(id, _)| id.file.capacity())
                                .sum::<usize>()
                    }
                })
                .sum::<usize>()
    }
}
