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
    pub cpu_bytes: usize,
    source: SourceId,
    scale: [f32; 3],
    bodies: Vec<BodyRecipe>,
    colliders: Vec<ColliderRecipe>,
}

pub struct StructureInstance {
    pub cpu_bytes: usize,
    pub placement: Placement,
    pub bodies: Vec<RigidBodyHandle>,
    pub colliders: Vec<(SourceId, ColliderHandle)>,
}
impl StructureRecipe {
    /// Collider construction and retained shape bounds include scaled input copies and topology storage.
    pub fn reservation(prototype: &WorldPrototypeReference) -> usize {
        use crate::reference::ColliderGeometry::*;
        std::mem::size_of::<Self>()
            + prototype.bodies.len() * std::mem::size_of::<BodyRecipe>()
            + prototype
                .colliders
                .iter()
                .map(|c| {
                    std::mem::size_of::<ColliderRecipe>()
                        + c.source.file.len()
                        + match &c.geometry {
                            Some(Convex { points }) => points.len() * 512,
                            Some(Trimesh { points, triangles }) => {
                                points.len() * 48 + triangles.len() * 256
                            }
                            Some(_) => 512,
                            None => 0,
                        }
                })
                .sum::<usize>()
    }
    pub fn instance_bytes(&self) -> usize {
        std::mem::size_of::<StructureInstance>()
            + self.bodies.len() * (std::mem::size_of::<rapier3d::prelude::RigidBody>() + 512)
            + self
                .colliders
                .iter()
                .map(|c| std::mem::size_of::<Collider>() + 512 + c.source.file.capacity())
                .sum::<usize>()
    }

    pub fn new(
        reference: &WorldReference,
        prototype: &WorldPrototypeReference,
    ) -> Result<Self, ReferenceError> {
        let scale = prototype.prefab_pose.scale;
        if scale[0] <= 0. || scale[0] != scale[1] || scale[1] != scale[2] {
            return Err(ReferenceError(
                "Unresolved nonuniform source structure collision scale".into(),
            ));
        }
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
                    position: body.pose.translation.map(|v| v * scale[0]),
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
                let material = reference
                    .records
                    .iter()
                    .find(|r| r.source == *source)
                    .ok_or_else(|| {
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
            let scaled = scaled_geometry(geometry, scale[0]);
            let collider = collider_builder(&scaled)?
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
            cpu_bytes: Self::reservation(prototype),
            source: prototype.source.clone(),
            scale,
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
        if placement.prototype != self.source {
            return Err(ReferenceError(
                "Structure placement does not match its checked recipe".into(),
            ));
        }
        if placement.scale != self.scale {
            return Err(ReferenceError(
                "Source structure placement scale changed".into(),
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
            cpu_bytes: self.instance_bytes(),
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

fn scaled_geometry(
    source: &crate::reference::ColliderGeometry,
    scale: f32,
) -> crate::reference::ColliderGeometry {
    use crate::reference::ColliderGeometry::*;
    let point = |p: [f32; 3]| p.map(|v| v * scale);
    match source {
        Box {
            center,
            half_extents,
            rotation,
        } => Box {
            center: point(*center),
            half_extents: point(*half_extents),
            rotation: *rotation,
        },
        Sphere { center, radius } => Sphere {
            center: point(*center),
            radius: radius * scale,
        },
        Capsule { a, b, radius } => Capsule {
            a: point(*a),
            b: point(*b),
            radius: radius * scale,
        },
        Convex { points } => Convex {
            points: points.iter().copied().map(point).collect(),
        },
        Trimesh { points, triangles } => Trimesh {
            points: points.iter().copied().map(point).collect(),
            triangles: triangles.clone(),
        },
    }
}
