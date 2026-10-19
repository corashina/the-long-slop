//! Active source lamp controls retain state independently of render proxy slots.
use super::WorldStream;
use crate::interaction::InteractionState;
use crate::reference::Transform;
use crate::reference::{ColliderId, LightControlId, ReferenceError, SourceId};
use crate::vehicle::PhysicsWorld;
use crate::world::spawn_rules::*;
use crate::world::{GeneratedId, ObjectKey, ObjectSnapshot};
pub(super) struct ButtonPose {
    pub(super) node: SourceId,
    pub(super) pose: Transform,
    target: Transform,
    progress: f32,
    moving: bool,
}

pub(super) struct LampBinding {
    pub(super) button: Option<ButtonPose>,
    collision_dirty: bool,
    pub parent: GeneratedId,
    pub source: SourceId,
    pub control: LightControlId,
    pub baseline: ObjectSnapshot,
}
impl LampBinding {
    pub fn heap_bytes(&self) -> usize {
        crate::world::contents::identity_heap(&self.parent)
            + self.source.file.capacity()
            + self.button.as_ref().map_or(0, |b| b.node.file.capacity())
    }
}
impl WorldStream {
    pub(super) fn sync_lamp_bindings(
        &mut self,
        interaction: &mut InteractionState,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let mut i = 0;
        while i < self.lamp_bindings.len() {
            if self
                .structures
                .iter()
                .any(|s| s.placement.id == self.lamp_bindings[i].parent)
            {
                i += 1;
                continue;
            }
            let binding = self.lamp_bindings.swap_remove(i);
            interaction.remove_generated_light(binding.control);
            self.lamp_binding_revision += 1;
            let mut key = binding.parent;
            key.children.push((binding.source, 0));
            if self.changes.forget(&ObjectKey::Generated(key)) {
                self.invalidate_changed_selection();
            }
            self.changes_prune_due = true;
        }
        loop {
            let next = self
                .contents
                .lamp_states()
                .filter(|(parent, source, _)| {
                    !self
                        .lamp_bindings
                        .iter()
                        .any(|b| &b.parent == parent && &b.source == source)
                        && self.structures.iter().any(|s| &s.placement.id == parent)
                        && self.bindings_ready(parent)
                })
                .next()
                .cloned();
            let Some((parent, source, on)) = next else {
                break;
            };
            if self
                .lamp_bindings
                .iter()
                .any(|b| b.parent == parent && b.source == source)
            {
                continue;
            }
            let Some(instance) = self.structures.iter().find(|s| s.placement.id == parent) else {
                continue;
            };
            let prototype = self
                .reference
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .unwrap();
            let root = self.contents.rules.root(&self.reference, &source)?;
            let root_index = prototype
                .nodes
                .iter()
                .position(|n| &n.source == root)
                .ok_or_else(|| ReferenceError("Generated lamp controller root missing".into()))?;
            let baseline = ObjectSnapshot {
                pose: instance.placement.pose,
                condition: 1.,
                food: None,
                temperature: None,
                fluid_quantities: None,
                valve: None,
                lamp: Some(on),
                linear_velocity: [0.; 3],
                angular_velocity: [0.; 3],
                sleeping: false,
                held: false,
                attachment: None,
                mechanism: None,
            };
            let mut key = parent.clone();
            key.children.push((source.clone(), 0));
            if let Some(changed) = self
                .changes
                .get(ObjectKey::Generated(key))
                .and_then(|r| r.snapshot.as_ref())
                .and_then(|s| s.lamp)
            {
                self.contents.set_lamp(
                    &self.reference,
                    &parent,
                    &source,
                    changed,
                    self.available_staging_bytes(),
                )?;
            }
            let f = self.contents.rules.get(&self.reference, &source)?.fields;
            let button = if let Some(node) = optional(f, "GButton")? {
                let selected_on = self
                    .contents
                    .lamp_states()
                    .find(|(p, s, _)| p == &parent && s == &source)
                    .unwrap()
                    .2;
                let target = link(
                    f,
                    if selected_on ^ flag(f, "reverseSwitch")? {
                        "GButtonOn"
                    } else {
                        "GButtonOff"
                    },
                )?;
                Some(ButtonPose {
                    node: node.clone(),
                    pose: self.contents.rules.node(&self.reference, node)?.pose,
                    target: self.contents.rules.node(&self.reference, target)?.pose,
                    progress: 0.,
                    moving: true,
                })
            } else {
                None
            };
            let instance = self
                .structures
                .iter()
                .find(|s| s.placement.id == parent)
                .unwrap();
            let prototype = self
                .reference
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .unwrap();
            let control = interaction.allocate_generated_light()?;
            for (source, handle) in &instance.colliders {
                let collider = prototype
                    .colliders
                    .iter()
                    .find(|c| &c.source == source)
                    .unwrap();
                let mut node = Some(collider.node);
                let mut belongs = false;
                while let Some(n) = node {
                    if n as usize == root_index {
                        belongs = true;
                        break;
                    }
                    node = prototype.nodes[n as usize].parent;
                }
                if !belongs {
                    continue;
                }
                let mut id =
                    ColliderId((physics.rapier().colliders[*handle].user_data >> 64) as u32);
                if id.0 == 0 {
                    id = match interaction.allocate_generated_collider() {
                        Ok(id) => id,
                        Err(error) => {
                            interaction.remove_generated_light(control);
                            return Err(error);
                        }
                    };
                    physics.rapier_mut().colliders[*handle].user_data |= (id.0 as u128) << 64;
                }
                interaction.bind_generated_light(control, id);
            }
            self.lamp_bindings.push(LampBinding {
                button,
                collision_dirty: true,
                parent,
                source,
                control,
                baseline,
            });
            self.lamp_binding_revision += 1;
        }
        for binding in &mut self.lamp_bindings {
            if !binding.collision_dirty
                || self
                    .contents
                    .active
                    .iter()
                    .chain(&self.static_controls)
                    .any(|c| c.content.placement.id == binding.parent && !c.restored)
            {
                continue;
            }
            let Some(instance) = self
                .structures
                .iter()
                .find(|s| s.placement.id == binding.parent)
            else {
                continue;
            };
            let prototype = self
                .reference
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .unwrap();
            let appearance = self.contents.appearance(&binding.parent);
            for (source, handle) in &instance.colliders {
                let collider = prototype
                    .colliders
                    .iter()
                    .find(|c| &c.source == source)
                    .unwrap();
                physics.rapier_mut().colliders[*handle].set_enabled(
                    collider.enabled
                        && super::super::spawn_appearance::node_active(
                            prototype,
                            collider.node as usize,
                            appearance,
                        ),
                );
            }
            binding.collision_dirty = false;
        }
        self.refresh_lamp_outputs();
        Ok(())
    }
    pub fn use_generated_light(&mut self, control: LightControlId) -> Result<bool, ReferenceError> {
        let Some(binding) = self.lamp_bindings.iter().find(|b| b.control == control) else {
            return Ok(false);
        };
        let on = self
            .contents
            .lamp_states()
            .find(|(p, s, _)| p == &binding.parent && s == &binding.source)
            .ok_or_else(|| ReferenceError("Generated lamp state missing".into()))?
            .2;
        self.contents.set_lamp(
            &self.reference,
            &binding.parent,
            &binding.source,
            !on,
            self.available_staging_bytes(),
        )?;
        let mut key = binding.parent.clone();
        key.children.push((binding.source.clone(), 0));
        let mut snapshot = binding.baseline.clone();
        snapshot.lamp = Some(!on);
        let object = ObjectKey::Generated(key);
        self.changes
            .record(object.clone(), snapshot, &binding.baseline);
        if let Some(origin) = self.contents.replay_origin(&binding.parent).or_else(|| {
            self.structures
                .iter()
                .find(|s| s.placement.id == binding.parent)
                .map(|s| &s.placement)
        }) {
            self.changes.set_replay(&object, origin, &binding.parent);
        }
        let binding = self
            .lamp_bindings
            .iter_mut()
            .find(|b| b.control == control)
            .unwrap();
        binding.collision_dirty = true;
        if let Some(button) = &mut binding.button {
            let f = self
                .contents
                .rules
                .get(&self.reference, &binding.source)?
                .fields;
            let target = link(
                f,
                if !on ^ flag(f, "reverseSwitch")? {
                    "GButtonOn"
                } else {
                    "GButtonOff"
                },
            )?;
            button.target = self.contents.rules.node(&self.reference, target)?.pose;
            if !button.moving {
                button.progress = 0.;
                button.moving = true;
            }
        }
        self.refresh_lamp_outputs();
        Ok(true)
    }
    pub(super) fn sync_moving_lamps(&mut self, items: &crate::interaction::items::ItemStore) {
        for content in &self.contents.active {
            let Some(item) = content.item.and_then(|id| items.get(id)) else {
                continue;
            };
            let old = content.content.placement.pose;
            let old_inverse = [
                -old.rotation[0],
                -old.rotation[1],
                -old.rotation[2],
                old.rotation[3],
            ];
            let rotation = crate::world::placement::multiply(item.pose.rotation, old_inverse);
            let position = self.origin.absolute(item.pose.position);
            for source in &content.content.lights {
                if let Some(light) = self.lights.iter_mut().find(|l| l.id == source.id) {
                    let offset = crate::world::placement::rotate(
                        rotation,
                        std::array::from_fn(|i| (source.pose.position[i] - old.position[i]) as f32),
                    );
                    light.pose.position = std::array::from_fn(|i| position[i] + offset[i] as f64);
                    light.pose.rotation =
                        crate::world::placement::multiply(rotation, source.pose.rotation);
                }
            }
        }
    }
    pub fn advance_generated_lamps(&mut self, dt: f32) {
        if !dt.is_finite() || dt < 0. {
            return;
        }
        for binding in &mut self.lamp_bindings {
            let Some(button) = &mut binding.button else {
                continue;
            };
            if !button.moving {
                continue;
            }
            let t = button.progress.min(1.);
            button.pose.translation = std::array::from_fn(|i| {
                button.pose.translation[i]
                    + (button.target.translation[i] - button.pose.translation[i]) * t
            });
            let a = rapier3d::math::Rotation::from_array(button.pose.rotation);
            let b = rapier3d::math::Rotation::from_array(button.target.rotation);
            button.pose.rotation = a.lerp(b, t).normalize().to_array();
            if button.progress >= 1. {
                button.pose = button.target;
                button.moving = false;
                button.progress = 0.;
            } else {
                button.progress += dt * 25.;
            }
        }
    }
    pub fn lamp_button_pose(&self, parent: &GeneratedId, node: &SourceId) -> Option<Transform> {
        self.lamp_bindings
            .iter()
            .filter(|b| &b.parent == parent)
            .filter_map(|b| b.button.as_ref())
            .find(|b| &b.node == node)
            .map(|b| b.pose)
    }
    fn refresh_lamp_outputs(&mut self) {
        for content in self.contents.instances() {
            for light in &content.lights {
                if let Some(current) = self.lights.iter_mut().find(|l| l.id == light.id) {
                    current.enabled = light.enabled;
                    current.intensity = light.intensity;
                    current.pose = light.pose;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::content_index::ContentIndex;
    use super::*;
    use crate::world::{AbsolutePose, WorldLayer};

    #[test]
    fn cached_buttons_keep_order_and_read_live_pose() {
        let node = SourceId {
            file: "test.assets".into(),
            path_id: 1,
        };
        let parent = GeneratedId {
            layer: WorldLayer::Building,
            generator: node.clone(),
            unit: [0; 2],
            ordinal: 0,
            children: Vec::new(),
        };
        let mut other = parent.clone();
        other.children.push((node.clone(), 0));
        let pose = |x| Transform {
            translation: [x, 0., 0.],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        };
        let binding = |parent: GeneratedId, x| LampBinding {
            parent,
            source: node.clone(),
            control: LightControlId(x as u32),
            collision_dirty: false,
            button: Some(ButtonPose {
                node: node.clone(),
                pose: pose(x),
                target: pose(x + 1.),
                progress: 0.,
                moving: true,
            }),
            baseline: ObjectSnapshot {
                pose: AbsolutePose {
                    position: [0.; 3],
                    rotation: [0., 0., 0., 1.],
                },
                condition: 1.,
                food: None,
                temperature: None,
                fluid_quantities: None,
                valve: None,
                lamp: Some(false),
                linear_velocity: [0.; 3],
                angular_velocity: [0.; 3],
                sleeping: false,
                held: false,
                attachment: None,
                mechanism: None,
            },
        };
        let mut lamps = vec![
            binding(other.clone(), 9.),
            binding(parent.clone(), 1.),
            binding(other.clone(), 8.),
            binding(parent.clone(), 2.),
        ];
        let mut no_button = binding(parent.clone(), 3.);
        no_button.button = None;
        lamps.push(no_button);
        let mut index = ContentIndex::default();
        index.update_lamps((1, 0), &lamps);
        assert_eq!(index.rebuilds, [0, 1]);
        let positions = |index: &ContentIndex, lamps: &[LampBinding]| {
            index
                .buttons(lamps, &parent)
                .map(|(_, pose)| pose.translation[0])
                .collect::<Vec<_>>()
        };
        assert_eq!(positions(&index, &lamps), [1., 2.]);
        assert_eq!(index.buttons(&lamps, &parent).next().unwrap().0, &node);
        let mut missing = parent.clone();
        missing.ordinal = 9;
        assert_eq!(index.buttons(&lamps, &missing).count(), 0);
        lamps[1].button.as_mut().unwrap().pose = pose(2.);
        lamps[3].button.as_mut().unwrap().pose = pose(3.);
        index.update_lamps((1, 0), &lamps);
        assert_eq!(index.rebuilds, [0, 1]);
        assert_eq!(positions(&index, &lamps), [2., 3.]);

        lamps.swap_remove(1);
        lamps.push(binding(parent.clone(), 5.));
        index.update_lamps((1, 1), &lamps);
        assert_eq!(index.rebuilds, [0, 2]);
        assert_eq!(positions(&index, &lamps), [3., 5.]);
        lamps[0].parent = parent.clone();
        index.update_lamps((2, 1), &lamps);
        assert_eq!(index.rebuilds, [0, 3]);
        assert_eq!(positions(&index, &lamps), [9., 3., 5.]);
        index.update_lamps((2, 1), &lamps);
        assert_eq!(index.rebuilds, [0, 3]);
    }
}
