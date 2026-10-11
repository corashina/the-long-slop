//! Hinged doors, hood, and trunk.

use super::*;

impl InteractionState {
    pub fn hinge_angle(&self, id: ItemId) -> Option<f32> {
        self.hinges
            .iter()
            .find(|hinge| hinge.reference.item == id)
            .map(|hinge| hinge.angle)
    }
    /// A fitted part returns to its slot closed.
    pub fn close_hinge(&mut self, item: ItemId) {
        if let Some(hinge) = self
            .hinges
            .iter_mut()
            .find(|hinge| hinge.reference.item == item)
        {
            hinge.angle = hinge.reference.closed_degrees;
            hinge.target = hinge.reference.closed_degrees;
        }
    }
    pub fn dragging_hinge(&self) -> Option<ItemId> {
        self.latched_hinge
            .map(|(index, _)| self.hinges[index].reference.item)
    }
    pub fn sync_hinges(&self, physics: &mut PhysicsWorld) {
        self.place_hinges(physics, true);
    }
    pub fn hinge_local_pose(&self, body: BodyId) -> Option<Pose> {
        self.hinges
            .iter()
            .find(|hinge| hinge.reference.body == body)
            .map(local_hinge_pose)
    }
    pub(super) fn place_hinges(&self, physics: &mut PhysicsWorld, commit: bool) {
        for hinge in &self.hinges {
            let Some(handle) = self.bodies.get(&hinge.reference.body).copied() else {
                continue;
            };
            let parent = if let Some(parent) = hinge.parent {
                let Some(body) = self
                    .bodies
                    .get(&parent)
                    .and_then(|handle| physics.rapier().bodies.get(*handle))
                else {
                    continue;
                };
                *body.position()
            } else {
                Pose::IDENTITY
            };
            let pose = parent * local_hinge_pose(hinge);
            let body = &mut physics.rapier_mut().bodies[handle];
            if commit {
                body.set_position(pose, false);
            }
            body.set_next_kinematic_position(pose);
        }
    }
}

fn local_hinge_pose(hinge: &HingeState) -> Pose {
    let rotation = Pose::from_rotation(Rotation::from_axis_angle(
        hinge.axis,
        (hinge.angle - hinge.reference.closed_degrees).to_radians(),
    ));
    hinge.closed_pose * hinge.anchor * rotation * hinge.anchor_inverse
}
