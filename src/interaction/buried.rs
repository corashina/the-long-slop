//! Source tosaveitemscript buried state: IsBuried, IsBuriedRay, UnBury and IsRecentlyUnburied.
use super::items::ItemStore;
use crate::reference::ItemId;
use crate::vehicle::PhysicsWorld;
use rapier3d::math::Vector;
use rapier3d::prelude::{QueryFilter, Ray, RigidBodyHandle, RigidBodyType};

/// IsRecentlyUnburied window: `Time.time < unburiedTime + 1f`.
pub const RECENT_SECONDS: f32 = 1.;
/// Collider layer that IsBuriedRay skips for the lowest point (player layer).
const SKIPPED_LAYER: u32 = 1 << 9;

/// Items unburied in the last second, with their remaining window.
pub struct BuriedItems {
    recent: Vec<(ItemId, f32)>,
}
impl Default for BuriedItems {
    fn default() -> Self {
        Self {
            recent: Vec::with_capacity(8),
        }
    }
}
impl BuriedItems {
    pub fn tick(&mut self, dt: f32) {
        if self.recent.is_empty() {
            return;
        }
        for (_, left) in &mut self.recent {
            *left -= dt;
        }
        self.recent.retain(|(_, left)| *left > 0.);
    }
    pub fn recently_unburied(&self, id: ItemId) -> bool {
        self.recent.iter().any(|(item, _)| *item == id)
    }
    /// UnBury: a dynamic body that can be picked up again.
    pub fn unbury(&mut self, id: ItemId, items: &mut ItemStore, physics: &mut PhysicsWorld) {
        let Some(item) = items.get_mut(id) else {
            return;
        };
        if item.buried {
            self.recent.retain(|(item, _)| *item != id);
            self.recent.push((id, RECENT_SECONDS));
        }
        item.buried = false;
        if let Some(body) = item.body {
            physics.edit_body(body, |body| {
                body.set_body_type(RigidBodyType::Dynamic, true)
            });
        }
        items.mark_dirty(id);
    }
    /// IsBuried: a buried item with a dynamic body or ground below it unburies.
    pub fn is_buried(
        &mut self,
        id: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        player: Option<RigidBodyHandle>,
    ) -> bool {
        let Some(item) = items.get(id) else {
            return false;
        };
        if !item.buried {
            return false;
        }
        let dynamic = item
            .body
            .and_then(|b| physics.rapier().bodies.get(b))
            .is_some_and(|b| b.is_dynamic());
        if dynamic || !buried_ray(id, items, physics, player) {
            self.unbury(id, items, physics);
            return false;
        }
        true
    }
}

/// Bury: a kinematic body that digging lifts.
pub fn bury(id: ItemId, items: &mut ItemStore, physics: &mut PhysicsWorld) {
    let Some(item) = items.get_mut(id) else {
        return;
    };
    item.buried = true;
    if let Some(body) = item.body {
        physics.edit_body(body, |body| {
            body.set_body_type(RigidBodyType::KinematicPositionBased, true)
        });
    }
    items.mark_dirty(id);
}

/// IsBuriedRay: true while no other collider lies below the item's lowest point.
/// Other buried items and the player do not count. Triggers count (queriesHitTriggers).
pub fn buried_ray(
    id: ItemId,
    items: &ItemStore,
    physics: &PhysicsWorld,
    player: Option<RigidBodyHandle>,
) -> bool {
    let Some(handle) = items.get(id).and_then(|i| i.body) else {
        return true;
    };
    let rapier = physics.rapier();
    let Some(body) = rapier.bodies.get(handle) else {
        return true;
    };
    let origin = body.translation();
    // ClosestPoint to a point far below is the lowest point of a convex collider.
    let lowest = body
        .colliders()
        .iter()
        .filter_map(|c| rapier.colliders.get(*c))
        .filter(|c| {
            c.is_enabled()
                && !c.is_sensor()
                && c.collision_groups().memberships.bits() != SKIPPED_LAYER
        })
        .map(|c| c.compute_aabb().mins.y)
        .fold(f32::INFINITY, f32::min);
    let ray = Ray::new(origin, -Vector::Y);
    let filter = QueryFilter::default().exclude_rigid_body(handle);
    !physics
        .queries(filter)
        .intersect_ray(ray, f32::MAX, true)
        .any(|(_, collider, hit)| {
            let parent = collider.parent();
            origin.y - hit.time_of_impact < lowest
                && (parent.is_none() || parent != player)
                && !parent
                    .is_some_and(|parent| items.iter().any(|i| i.buried && i.body == Some(parent)))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vehicle::VehiclePose;
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    const ITEM: ItemId = ItemId(1);
    const OTHER: ItemId = ItemId(2);

    fn item(physics: &mut PhysicsWorld, items: &mut ItemStore, id: ItemId, y: f32) {
        let body =
            physics.insert_body(RigidBodyBuilder::dynamic().translation(Vector::new(0.0, y, 0.0)));
        physics.insert_collider_with_parent(ColliderBuilder::cuboid(0.1, 0.1, 0.1), body);
        let pose = VehiclePose {
            position: [0.0, y, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        };
        items.insert_spawned(id, "test", pose, body).unwrap();
        bury(id, items, physics);
    }
    /// Ground slab with its top at y = 0.
    fn ground(physics: &mut PhysicsWorld) {
        physics.insert_collider(
            ColliderBuilder::cuboid(5.0, 0.5, 5.0).translation(Vector::new(0.0, -0.5, 0.0)),
        );
    }
    fn dynamic(physics: &PhysicsWorld, items: &ItemStore, id: ItemId) -> bool {
        physics.rapier().bodies[items.get(id).unwrap().body.unwrap()].is_dynamic()
    }

    #[test]
    fn item_under_ground_stays_buried_and_item_above_ground_unburies_for_one_second() {
        let mut physics = PhysicsWorld::empty_for_tests();
        let mut items = ItemStore::from_states(Vec::new());
        let mut buried = BuriedItems::default();
        ground(&mut physics);
        item(&mut physics, &mut items, ITEM, -0.3);
        physics.refresh_queries();
        assert!(buried.is_buried(ITEM, &mut items, &mut physics, None));
        assert!(!dynamic(&physics, &items, ITEM));

        // Digging lifts the item above the ground top.
        let body = items.get(ITEM).unwrap().body.unwrap();
        physics.edit_body(body, |b| {
            b.set_translation(Vector::new(0.0, 0.3, 0.0), true)
        });
        physics.refresh_queries();
        assert!(!buried.is_buried(ITEM, &mut items, &mut physics, None));
        assert!(!items.get(ITEM).unwrap().buried && dynamic(&physics, &items, ITEM));
        assert!(buried.recently_unburied(ITEM));
        buried.tick(1.0);
        assert!(!buried.recently_unburied(ITEM));
    }

    #[test]
    fn buried_item_below_does_not_unbury() {
        let mut physics = PhysicsWorld::empty_for_tests();
        let mut items = ItemStore::from_states(Vec::new());
        item(&mut physics, &mut items, ITEM, 0.0);
        item(&mut physics, &mut items, OTHER, -0.5);
        physics.refresh_queries();
        assert!(buried_ray(ITEM, &items, &physics, None));
        items.get_mut(OTHER).unwrap().buried = false;
        assert!(!buried_ray(ITEM, &items, &physics, None));
    }

    #[test]
    fn buried_state_round_trips_through_the_save_snapshot() {
        let mut physics = PhysicsWorld::empty_for_tests();
        let mut items = ItemStore::from_states(Vec::new());
        item(&mut physics, &mut items, ITEM, 0.0);
        let json = serde_json::to_string(&items.generated_snapshot(ITEM, None).unwrap()).unwrap();
        items.get_mut(ITEM).unwrap().buried = false;
        let snapshot: crate::world::ObjectSnapshot = serde_json::from_str(&json).unwrap();
        items.restore_generated(ITEM, &snapshot).unwrap();
        assert!(items.get(ITEM).unwrap().buried);
    }
}
