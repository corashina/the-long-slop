//! Checked prototype collision recipes, shared by source instances.
use super::{Placement, WorldOrigin};
use crate::reference::{
    ReferenceError, SourceId, WorldPrototypeReference, WorldReference, WorldValue,
};
use crate::vehicle::{ColliderHandle, PhysicsWorld, VehiclePose, collider_builder, to_rapier};
use rapier3d::math::Pose;
use rapier3d::prelude::{
    Collider, Group, InteractionGroups, InteractionTestMode, RigidBodyBuilder, RigidBodyHandle,
};
use std::collections::HashMap;

struct BodyRecipe {
    builder: RigidBodyBuilder,
    pose: Pose,
}
struct ColliderRecipe {
    source: SourceId,
    body: Option<usize>,
    collider: Collider,
}

pub struct StructureRecipe {
    source: SourceId,
    bodies: Vec<BodyRecipe>,
    colliders: Vec<ColliderRecipe>,
}

pub struct StructureInstance {
    pub placement: Placement,
    pub bodies: Vec<RigidBodyHandle>,
    pub colliders: Vec<(SourceId, ColliderHandle)>,
}
impl StructureRecipe {
    pub fn new(
        reference: &WorldReference,
        prototype: &WorldPrototypeReference,
    ) -> Result<Self, ReferenceError> {
        let records: HashMap<_, _> = reference
            .records
            .iter()
            .map(|record| (&record.source, record))
            .collect();
        let active = |mut node: usize| {
            for _ in 0..prototype.nodes.len() {
                let record = &prototype.nodes[node];
                if !record.active {
                    return false;
                }
                let Some(parent) = record.parent else {
                    return true;
                };
                node = parent as usize;
            }
            false
        };
        let mut bodies = Vec::with_capacity(prototype.bodies.len());
        for body in &prototype.bodies {
            let constraints = body.constraints;
            let builder = if body.kinematic {
                RigidBodyBuilder::kinematic_position_based()
            } else {
                RigidBodyBuilder::dynamic().ccd_enabled(true)
            }
            .enabled(active(body.node as usize))
            .additional_mass(body.mass_kg)
            .linear_damping(body.linear_damping)
            .angular_damping(body.angular_damping)
            .gravity_scale(if body.use_gravity { 1.0 } else { 0.0 })
            .enabled_translations(
                constraints & 2 == 0,
                constraints & 4 == 0,
                constraints & 8 == 0,
            )
            .enabled_rotations(
                constraints & 16 == 0,
                constraints & 32 == 0,
                constraints & 64 == 0,
            );
            bodies.push(BodyRecipe {
                builder,
                pose: to_rapier(VehiclePose {
                    position: body.pose.translation,
                    rotation: body.pose.rotation,
                }),
            });
        }
        let mut colliders = Vec::with_capacity(prototype.colliders.len());
        for record in &prototype.colliders {
            if record.source_empty_mesh.is_some() {
                continue;
            }
            let geometry = record.geometry.as_ref().ok_or_else(|| {
                ReferenceError(format!(
                    "Structure collider {}:{} has no checked geometry",
                    record.source.file, record.source.path_id
                ))
            })?;
            let (friction, restitution) = if let Some(source) = &record.material {
                let material = records.get(source).ok_or_else(|| {
                    ReferenceError(format!(
                        "Missing physical material {}:{}",
                        source.file, source.path_id
                    ))
                })?;
                let number = |name: &str| match material.fields.get(name) {
                    Some(WorldValue::Number(value)) => Ok(*value as f32),
                    Some(WorldValue::Integer(value)) => Ok(*value as f32),
                    _ => Err(ReferenceError(format!("Physical material field {name}"))),
                };
                (number("m_DynamicFriction")?, number("m_Bounciness")?)
            } else {
                (0.6, 0.0)
            };
            let layer = prototype.nodes[record.node as usize].layer as usize;
            let collider = collider_builder(geometry)?
                .friction(friction)
                .restitution(restitution)
                .density(0.0)
                .enabled(record.enabled && active(record.node as usize))
                .sensor(record.trigger)
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_truncate(1u32 << layer),
                    Group::from_bits_truncate(reference.collision_rules.layers[layer]),
                    InteractionTestMode::And,
                ))
                .build();
            colliders.push(ColliderRecipe {
                source: record.source.clone(),
                body: record.body.map(|i| i as usize),
                collider,
            });
        }
        Ok(Self {
            source: prototype.source.clone(),
            bodies,
            colliders,
        })
    }

    /// All source validation finishes before any live physics state changes.
    pub fn publish(
        &self,
        placement: Placement,
        origin: WorldOrigin,
        physics: &mut PhysicsWorld,
    ) -> Result<StructureInstance, ReferenceError> {
        if placement.prototype != self.source || placement.scale != [1.0; 3] {
            return Err(ReferenceError(
                "Structure placement does not match its checked unit-scale recipe".into(),
            ));
        }
        let root = to_rapier(VehiclePose {
            position: origin.local(placement.pose.position),
            rotation: placement.pose.rotation,
        });
        let mut bodies = Vec::with_capacity(self.bodies.len());
        let mut colliders = Vec::with_capacity(self.colliders.len());
        let world = physics.rapier_mut();
        for body in &self.bodies {
            bodies.push(world.insert_body(body.builder.clone().pose(root * body.pose)));
        }
        for source in &self.colliders {
            let mut collider = source.collider.clone();
            let handle = if let Some(body) = source.body {
                world
                    .colliders
                    .insert_with_parent(collider, bodies[body], &mut world.bodies)
            } else {
                collider.set_position(root * *collider.position());
                world.colliders.insert(collider)
            };
            colliders.push((source.source.clone(), handle));
        }
        physics.refresh_queries();
        Ok(StructureInstance {
            placement,
            bodies,
            colliders,
        })
    }
}
impl StructureInstance {
    /// The caller removes item and interaction bindings before retiring this instance.
    pub fn retire(self, physics: &mut PhysicsWorld) {
        for (_, collider) in self.colliders {
            physics.remove_collider(collider);
        }
        for body in self.bodies {
            physics.rapier_mut().remove_body(body);
        }
        physics.refresh_queries();
    }
}
