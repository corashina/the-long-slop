use rapier3d::math::Pose;
use rapier3d::parry::partitioning::Bvh;
use rapier3d::prelude::{ColliderHandle, PhysicsWorld, RigidBodyHandle, SharedShape};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct QueryBounds {
    pub bvh: Bvh,
    states: Vec<Option<ColliderState>>,
    changed: Vec<(ColliderHandle, Pose)>,
    pending_bodies: HandleQueue<RigidBodyHandle>,
    pending_colliders: HandleQueue<ColliderHandle>,
    geometry_changed: Vec<bool>,
    removals: HandleQueue<ColliderHandle>,
    step_bodies: HandleQueue<RigidBodyHandle>,
    step_colliders: HandleQueue<ColliderHandle>,
}

#[derive(Default)]
struct HandleQueue<H> {
    handles: Vec<H>,
    markers: Vec<Option<(H, usize)>>,
}
impl<H: Copy + Eq> HandleQueue<H> {
    fn mark(&mut self, handle: H, slot: usize) -> usize {
        if self.markers.len() <= slot {
            self.markers.resize_with(slot + 1, || None);
        }
        if let Some((previous, index)) = self.markers[slot] {
            if previous == handle {
                return index;
            }
        }
        let index = self.handles.len();
        self.handles.push(handle);
        self.markers[slot] = Some((handle, index));
        index
    }
    fn clear(&mut self, slot: impl Fn(H) -> usize) {
        for &handle in &self.handles {
            let marker = &mut self.markers[slot(handle)];
            if marker.is_some_and(|(previous, _)| previous == handle) {
                *marker = None;
            }
        }
        self.handles.clear();
    }
    fn retained_bytes(&self) -> usize {
        self.handles.capacity() * std::mem::size_of::<H>()
            + self.markers.capacity() * std::mem::size_of::<Option<(H, usize)>>()
    }
}

struct ColliderState {
    handle: ColliderHandle,
    position: Pose,
    parent: Option<(RigidBodyHandle, Pose)>,
    enabled: bool,
    // Owning the prior shape forces shape_mut to copy before changing its geometry.
    shape: SharedShape,
}

impl QueryBounds {
    pub(super) fn mark_body(&mut self, body: RigidBodyHandle) {
        let slot = body.into_raw_parts().0 as usize;
        self.pending_bodies.mark(body, slot);
        self.step_bodies.mark(body, slot);
    }
    pub(super) fn mark_collider(&mut self, collider: ColliderHandle, geometry_changed: bool) {
        let slot = collider.into_raw_parts().0 as usize;
        let index = self.pending_colliders.mark(collider, slot);
        if self.geometry_changed.len() <= index {
            self.geometry_changed.push(geometry_changed);
        } else {
            self.geometry_changed[index] |= geometry_changed;
        }
        self.step_colliders.mark(collider, slot);
    }
    pub(super) fn retire_collider(&mut self, collider: ColliderHandle) {
        self.removals
            .mark(collider, collider.into_raw_parts().0 as usize);
    }
    fn clear_pending(&mut self) {
        self.pending_bodies.clear(|h| h.into_raw_parts().0 as usize);
        self.pending_colliders
            .clear(|h| h.into_raw_parts().0 as usize);
        self.removals.clear(|h| h.into_raw_parts().0 as usize);
        self.geometry_changed.clear();
    }
    pub(super) fn clear_step(&mut self) {
        self.step_bodies.clear(|h| h.into_raw_parts().0 as usize);
        self.step_colliders.clear(|h| h.into_raw_parts().0 as usize);
    }
    #[cfg(test)]
    pub(super) fn pending_counts(&self) -> (usize, usize, usize) {
        (
            self.pending_bodies.handles.len(),
            self.pending_colliders.handles.len(),
            self.removals.handles.len(),
        )
    }
    #[cfg(test)]
    pub(super) fn step_counts(&self) -> (usize, usize) {
        (
            self.step_bodies.handles.len(),
            self.step_colliders.handles.len(),
        )
    }

    pub fn retained_bytes(&self) -> usize {
        self.pending_bodies.retained_bytes()
            + self.pending_colliders.retained_bytes()
            + self.removals.retained_bytes()
            + self.step_bodies.retained_bytes()
            + self.step_colliders.retained_bytes()
            + self.geometry_changed.capacity() * std::mem::size_of::<bool>()
            + self.bvh.heap_memory_size()
            + self.states.capacity() * std::mem::size_of::<Option<ColliderState>>()
            + self.changed.capacity() * std::mem::size_of::<(ColliderHandle, Pose)>()
    }

    pub fn refresh(&mut self, physics: &mut PhysicsWorld) -> usize {
        // Keep body, island and collision notifications for PhysicsPipeline::step.
        physics
            .bodies
            .propagate_modified_body_positions_to_colliders(&mut physics.colliders);
        self.changed.clear();
        // Remove old generations before a new collider can reuse the same tree leaf.
        for state in &mut self.states {
            if state
                .as_ref()
                .is_some_and(|state| !physics.colliders.contains(state.handle))
            {
                self.bvh
                    .remove(state.as_ref().unwrap().handle.into_raw_parts().0);
                *state = None;
            }
        }
        for (handle, collider) in physics.colliders.iter() {
            let slot = handle.into_raw_parts().0 as usize;
            let previous = self.states.get(slot).and_then(Option::as_ref);
            let parent = collider
                .parent()
                .zip(collider.position_wrt_parent().copied());
            let enabled = collider.is_enabled()
                && parent.is_none_or(|(handle, _)| physics.bodies[handle].is_enabled());
            // A local attachment edit does not move its body. Synchronize that edit here.
            let position = if previous.is_none_or(|state| state.parent != parent) {
                parent.map_or(*collider.position(), |(handle, local)| {
                    *physics.bodies[handle].position() * local
                })
            } else {
                *collider.position()
            };
            if previous.is_none_or(|state| {
                state.handle != handle
                    || state.position != position
                    || state.parent != parent
                    || state.enabled != enabled
                    || !Arc::ptr_eq(&state.shape.0, &collider.shared_shape().0)
            }) {
                self.changed.push((handle, position));
            }
        }
        let mut updates = 0;
        for &(handle, position) in &self.changed {
            if physics.colliders[handle].position() != &position {
                physics.colliders[handle].set_position(position);
            }
            let collider = &physics.colliders[handle];
            let parent = collider
                .parent()
                .zip(collider.position_wrt_parent().copied());
            let enabled = collider.is_enabled()
                && parent.is_none_or(|(handle, _)| physics.bodies[handle].is_enabled());
            let slot = handle.into_raw_parts().0 as usize;
            if enabled {
                self.bvh.reinsert_or_update_with_change_detection(
                    collider.compute_aabb(),
                    slot as u32,
                    0.0,
                );
                updates += 1;
            } else {
                self.bvh.remove(slot as u32);
            }
            if self.states.len() <= slot {
                self.states.resize_with(slot + 1, || None);
            }
            self.states[slot] = Some(ColliderState {
                handle,
                position,
                parent,
                enabled,
                shape: collider.shared_shape().clone(),
            });
        }
        self.clear_pending();
        updates
    }
}

#[cfg(test)]
mod reference;
#[cfg(test)]
mod tests;
