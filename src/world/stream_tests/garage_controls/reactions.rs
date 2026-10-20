use super::*;
use crate::interaction::mechanisms::MechanismState;
use crate::reference::{EndpointReactionReference, MechanismReference, NodeId};
use crate::simulation::FIXED_DT;
use rapier3d::prelude::{RigidBody, RigidBodyHandle};

fn door(reference: &ReferenceManifest) -> MechanismReference {
    reference
        .mechanisms
        .iter()
        .find(|m| {
            m.parent_item.is_some()
                && matches!(
                    m.reaction,
                    EndpointReactionReference::Rotation { force: 10000., .. }
                )
                && m.rotation
                    .as_ref()
                    .is_some_and(|r| r.limits_degrees[1][0] != r.limits_degrees[1][1])
        })
        .expect("installed source door reaction")
        .clone()
}

#[track_caller]
fn near(actual: Vector, expected: Vector) {
    assert!(
        (actual - expected).length() < 0.02,
        "{actual:?} != {expected:?}"
    );
}

fn momentum(body: &RigidBody) -> (Vector, Vector) {
    (
        body.linvel() * body.mass(),
        body.mass_properties().effective_angular_inertia() * body.angvel(),
    )
}

fn nonzero_inertia(body: &RigidBody) {
    assert!(body.is_dynamic() && body.is_enabled() && body.mass() > 0.);
    for axis in [Vector::X, Vector::Y, Vector::Z] {
        assert!((body.mass_properties().effective_angular_inertia() * axis).length() > 0.);
        assert!((body.mass_properties().effective_world_inv_inertia * axis).length() > 0.);
    }
}

fn zero(run: &mut GarageRuntime, body: RigidBodyHandle) {
    run.physics.edit_body(body, |b| {
        b.set_linvel(Vector::ZERO, true);
        b.set_angvel(Vector::ZERO, true);
    });
}

fn restore(run: &mut GarageRuntime, item: ItemId, degrees: [f32; 2]) {
    let m = &mut run.area.interaction.mechanisms;
    let mut saved = m.snapshot(item).unwrap();
    saved.degrees = degrees;
    m.restore(item, saved).unwrap();
    m.sync(&mut run.physics, true);
}

fn update(run: &mut GarageRuntime, item: ItemId, input: [f32; 2], dt: f32) {
    let m = &mut run.area.interaction.mechanisms;
    m.step(
        item,
        FrameInput {
            held: keys::USE,
            mouse: [input[0], -input[1]],
            ..FrameInput::default()
        },
        VehiclePose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        },
        dt,
    );
    m.sync(&mut run.physics, true);
}

fn source_endpoint(run: &mut GarageRuntime, control: &MechanismReference) {
    assert!(matches!(
        control.drag.mode,
        crate::reference::DragMode::Relative
    ));
    let r = control.rotation.as_ref().unwrap();
    let EndpointReactionReference::Rotation { events, .. } = control.reaction else {
        unreachable!()
    };
    let endpoint = if events[0] { 0 } else { 1 };
    let input =
        10000. * if endpoint == 0 { -1. } else { 1. } * if r.inverted[1] { -1. } else { 1. };
    let pose = run
        .area
        .interaction
        .mechanisms
        .node_pose(control.drag.node)
        .unwrap();
    let direction = Rotation::from_array(pose.rotation) * Vector::from_array(control.drag.axis);
    run.area.interaction.mechanisms.step(
        control.item,
        FrameInput {
            held: keys::USE,
            mouse: [0., -input],
            ..FrameInput::default()
        },
        VehiclePose {
            position: [0.; 3],
            rotation: Rotation::from_rotation_arc(-Vector::Z, direction.normalize()).to_array(),
        },
        0.02,
    );
    run.area.interaction.mechanisms.sync(&mut run.physics, true);
    assert_eq!(
        run.area
            .interaction
            .mechanisms
            .rotation_degrees(control.item)
            .unwrap()[1],
        r.limits_degrees[1][endpoint]
    );
}

fn install_test_control(
    reference: &mut ReferenceManifest,
    run: &mut GarageRuntime,
    events: [bool; 4],
    axes: [bool; 3],
    inverted: bool,
) -> (ItemId, NodeId) {
    let mut control = door(reference);
    let EndpointReactionReference::Rotation { node, .. } = control.reaction else {
        unreachable!()
    };
    control.enabled = true;
    control.drag.mode = crate::reference::DragMode::Direct;
    let r = control.rotation.as_mut().unwrap();
    r.limits_degrees = [[0., 1.]; 2];
    r.initial_degrees = [0.5; 2];
    r.inverted = [true, false];
    r.sensitivity = 1.;
    control.reaction = EndpointReactionReference::Rotation {
        node,
        axes,
        inverted,
        force: 10000.,
        events,
    };
    let item = control.item;
    reference
        .nodes
        .iter_mut()
        .find(|n| n.id == node)
        .unwrap()
        .transform
        .scale = [2., 3., 4.];
    reference.mechanisms = vec![control];
    run.area.interaction.mechanisms = MechanismState::new(reference).unwrap();
    run.area
        .interaction
        .mechanisms
        .bind_body(reference.vehicles[0].body, run.physics.chassis_handle(0));
    run.area.interaction.mechanisms.sync(&mut run.physics, true);
    (item, node)
}

fn assert_impulse(
    run: &mut GarageRuntime,
    node: NodeId,
    body: RigidBodyHandle,
    axes: Vector,
    force: f32,
) {
    let pose = run.area.interaction.mechanisms.node_pose(node).unwrap();
    let impulse = Rotation::from_array(pose.rotation) * axes * (force * FIXED_DT);
    let torque = (Vector::from_array(pose.position)
        - run.physics.rapier().bodies[body].center_of_mass())
    .cross(impulse);
    let properties = run.physics.rapier().bodies[body].mass_properties();
    let expected_spin = properties.effective_world_inv_inertia * torque;
    let expected_angular = properties.effective_angular_inertia() * expected_spin;
    if body == run.physics.chassis_handle(0) && force != 0. {
        assert!(
            torque.length() > 0.001,
            "chassis probe must apply point torque"
        );
        near(expected_angular, torque);
    }
    run.area
        .interaction
        .mechanisms
        .apply_reactions(&mut run.physics);
    let (linear, angular) = momentum(&run.physics.rapier().bodies[body]);
    near(run.physics.rapier().bodies[body].angvel(), expected_spin);
    near(run.physics.rapier().bodies[body].user_force(), Vector::ZERO);
    near(
        run.physics.rapier().bodies[body].user_torque(),
        Vector::ZERO,
    );
    near(linear, impulse);
    near(angular, expected_angular);
    run.area
        .interaction
        .mechanisms
        .apply_reactions(&mut run.physics);
    let (again, spin) = momentum(&run.physics.rapier().bodies[body]);
    near(again, linear);
    near(spin, angular);
}

#[test]
fn g10_endpoint_reactions() {
    let (mut reference, mut run) = load_garage();
    let chassis = run.physics.chassis_handle(0);
    for inverted in [false, true] {
        for axis in 0..2 {
            for event in 0..4 {
                let mut events = [false; 4];
                events[event] = true;
                let (item, node) =
                    install_test_control(&mut reference, &mut run, events, [true; 3], inverted);
                for value in [0.5, 2.] {
                    zero(&mut run, chassis);
                    let mut previous = [0.5; 2];
                    previous[axis] = match event {
                        2 => 0.,
                        3 => 1.,
                        _ => 0.5,
                    };
                    restore(&mut run, item, previous);
                    let mut input = [0.; 2];
                    input[axis] = value * if event == 0 || event == 3 { -1. } else { 1. };
                    update(&mut run, item, input, 0.02);
                    let force = if value == 0.5 { 2500. } else { 10000. }
                        * if event < 2 { 1. } else { -1. }
                        * if inverted { -1. } else { 1. };
                    assert_impulse(&mut run, node, chassis, Vector::new(1., 1., -1.), force);
                }
            }
        }
    }
    let (item, node) = install_test_control(
        &mut reference,
        &mut run,
        [true; 4],
        [true, false, false],
        false,
    );
    restore(&mut run, item, [0.; 2]);
    zero(&mut run, chassis);
    update(&mut run, item, [2.; 2], 0.02);
    assert_eq!(
        run.area.interaction.mechanisms.rotation_degrees(item),
        Some([1.; 2])
    );
    assert_impulse(&mut run, node, chassis, Vector::X, 0.);
    for (input, dt) in [
        ([2.; 2], 0.02),
        ([-2.; 2], 0.),
        ([-2.; 2], -1.),
        ([f32::NAN; 2], 0.02),
    ] {
        zero(&mut run, chassis);
        update(&mut run, item, input, dt);
        assert_impulse(&mut run, node, chassis, Vector::X, 0.);
    }
}

#[test]
fn g10_endpoint_root_and_restore() {
    let (reference, mut run) = load_garage();
    let control = door(&reference);
    let item = control.item;
    let part = control.parent_item.unwrap();
    let part_node = reference
        .parts
        .iter()
        .find(|p| p.item == part)
        .unwrap()
        .node;
    let slot = reference
        .part_slots
        .iter()
        .find(|s| s.part == Some(part))
        .unwrap();
    let mount = reference
        .anchors
        .iter()
        .find(|a| a.id == slot.mount_anchor)
        .unwrap()
        .node;
    let EndpointReactionReference::Rotation {
        node,
        axes,
        inverted,
        force,
        ..
    } = control.reaction
    else {
        unreachable!()
    };
    let r = control.rotation.as_ref().unwrap();
    let chassis = run.physics.chassis_handle(0);
    let driven = r.targets[0].node;
    let driven_body = reference
        .bodies
        .iter()
        .find(|body| body.node == driven)
        .unwrap()
        .id;
    let collider = reference
        .colliders
        .iter()
        .find(|c| c.body == driven_body && !c.sensor)
        .unwrap()
        .id;
    let collider_handle = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == collider.0)
        .unwrap()
        .0;
    run.area.interaction.mechanisms.sync(&mut run.physics, true);
    let before_pose =
        crate::vehicle::to_rapier(run.area.interaction.mechanisms.node_pose(driven).unwrap());
    run.area.sync_compound(&mut run.physics);
    run.physics.refresh_queries();
    let collider_local =
        before_pose.inverse() * *run.physics.rapier().colliders[collider_handle].position();
    let mut saved = run.area.interaction.mechanisms.snapshot(item).unwrap();
    saved.degrees[1] = (r.limits_degrees[1][0] + r.limits_degrees[1][1]) * 0.5;
    run.area
        .interaction
        .mechanisms
        .restore(item, saved.clone())
        .unwrap();
    run.area.interaction.mechanisms.sync(&mut run.physics, true);
    run.area.sync_compound(&mut run.physics);
    run.physics.refresh_queries();
    let before_collider = *run.physics.rapier().colliders[collider_handle].position();
    zero(&mut run, chassis);
    source_endpoint(&mut run, &control);
    run.area.sync_compound(&mut run.physics);
    run.physics.refresh_queries();
    let expected_collider =
        crate::vehicle::to_rapier(run.area.interaction.mechanisms.node_pose(driven).unwrap())
            * collider_local;
    let actual_collider = run.physics.rapier().colliders[collider_handle].position();
    near(actual_collider.translation, expected_collider.translation);
    assert!(
        actual_collider
            .rotation
            .dot(expected_collider.rotation)
            .abs()
            > 0.9999
    );
    assert!(actual_collider.rotation.dot(before_collider.rotation).abs() < 0.999);
    let local_axes = Vector::new(
        axes[0] as u8 as f32,
        axes[1] as u8 as f32,
        -(axes[2] as u8 as f32),
    ) * if inverted { -1. } else { 1. };
    assert_impulse(&mut run, node, chassis, local_axes, force);

    // The production detach path keeps cached parents and installs an external root boundary.
    run.area
        .interaction
        .parts
        .can_remove(part, &run.area.player, &run.area.items)
        .unwrap();
    run.area
        .interaction
        .parts
        .remove(part, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.area.interaction.parts.detach_mechanism(
        part,
        &mut run.area.interaction.mechanisms,
        &mut run.physics,
    );
    let detached = run.area.items.get(part).unwrap().body.unwrap();
    assert_ne!(detached, chassis);
    nonzero_inertia(&run.physics.rapier().bodies[detached]);
    assert!(run.physics.body_active(detached));
    // The source disables this usable on detach. Enable only this controlled physics probe.
    saved.enabled = true;
    run.area
        .interaction
        .mechanisms
        .restore(item, saved.clone())
        .unwrap();
    zero(&mut run, chassis);
    zero(&mut run, detached);
    source_endpoint(&mut run, &control);
    // Repeated frame preparation must preserve this update's pending reaction.
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    assert_impulse(&mut run, node, detached, local_axes, force);
    near(
        momentum(&run.physics.rapier().bodies[chassis]).0,
        Vector::ZERO,
    );

    for lifecycle in 0..6 {
        run.area
            .interaction
            .mechanisms
            .set_part_root(part_node, Some(detached));
        run.area.interaction.mechanisms.sync(&mut run.physics, true);
        run.area
            .interaction
            .mechanisms
            .restore(item, saved.clone())
            .unwrap();
        zero(&mut run, detached);
        zero(&mut run, chassis);
        source_endpoint(&mut run, &control);
        match lifecycle {
            0 => run
                .area
                .interaction
                .mechanisms
                .restore(item, saved.clone())
                .unwrap(),
            1 => run.area.interaction.mechanisms.cancel_input(),
            2 => run
                .area
                .interaction
                .mechanisms
                .bind_body(reference.vehicles[0].body, chassis),
            3 => run.area.interaction.mechanisms.set_mounted(part, false),
            4 => run
                .area
                .interaction
                .mechanisms
                .set_part_root(part_node, None),
            _ => run
                .area
                .interaction
                .mechanisms
                .mount_part(part_node, mount, [0., 0., 0., 1.]),
        }
        run.area
            .interaction
            .mechanisms
            .apply_reactions(&mut run.physics);
        if lifecycle == 0 {
            assert_eq!(
                run.area.interaction.mechanisms.rotation_degrees(item),
                Some(saved.degrees)
            );
        }
        near(
            momentum(&run.physics.rapier().bodies[detached]).0,
            Vector::ZERO,
        );
        near(
            momentum(&run.physics.rapier().bodies[detached]).1,
            Vector::ZERO,
        );
        near(
            momentum(&run.physics.rapier().bodies[chassis]).0,
            Vector::ZERO,
        );
        near(
            momentum(&run.physics.rapier().bodies[chassis]).1,
            Vector::ZERO,
        );
    }
    // A source root without a Rigidbody cannot borrow a child's body.
    let mut rootless = reference.clone();
    let root_body = rootless.vehicles[0].body;
    rootless.bodies.retain(|b| b.id != root_body);
    rootless.mechanisms = vec![door(&rootless)];
    let nested_body = rootless
        .bodies
        .iter()
        .find(|b| b.node == part_node)
        .unwrap()
        .id;
    run.area.interaction.mechanisms = MechanismState::new(&rootless).unwrap();
    run.area
        .interaction
        .mechanisms
        .bind_body(nested_body, chassis);
    restore(&mut run, item, saved.degrees);
    source_endpoint(&mut run, &control);
    zero(&mut run, chassis);
    assert_impulse(&mut run, node, chassis, Vector::X, 0.);
}

#[test]
fn g10_generated_endpoint_root() {
    use crate::reference::DragMode;
    use crate::world::{StructureRecipe, WorldOrigin};
    let (reference, mut run) = load_garage();
    let row = reference.world.mechanisms.iter().find(|row| {
        matches!(row.recipe.drag.mode, DragMode::Direct)
            && matches!(&row.recipe.reaction, EndpointReactionReference::Rotation { force, events, .. }
                if *force > 0. && *force != 10000. && (events[0] || events[1]))
            && row.recipe.rotation.as_ref().is_some_and(|r| r.limits_degrees[0][0] == r.limits_degrees[0][1]
                && r.limits_degrees[1][0] != r.limits_degrees[1][1])
            && reference.world.prototypes.iter().find(|p| p.source == row.prototype).is_some_and(|p| {
                p.bodies.len() == 1 && p.bodies[0].node == 0 && !p.bodies[0].kinematic && p.bodies[0].constraints == 0
            })
    }).expect("generated source reaction with its own root body");
    let source_prototype = reference
        .world
        .prototypes
        .iter()
        .find(|p| p.source == row.prototype)
        .unwrap();
    let EndpointReactionReference::Rotation {
        node,
        axes,
        inverted,
        force,
        events,
    } = &row.recipe.reaction
    else {
        unreachable!()
    };
    let rotation = row.recipe.rotation.as_ref().unwrap();
    let rules = crate::world::spawn_rules::SourceRules::new(&reference.world);
    for present in [true, false] {
        let mut prototype = source_prototype.clone();
        let mut placement = super::super::replay_placement(
            &prototype.source,
            if present { 9821 } else { 9822 },
            [50.; 3],
        );
        placement.scale = prototype.prefab_pose.scale;
        let recipe = StructureRecipe::new(&reference.world, &prototype).unwrap();
        let mut instance = recipe
            .publish(
                placement.clone(),
                WorldOrigin::default(),
                false,
                &mut run.physics,
            )
            .unwrap();
        let root_body = instance.bodies[0];
        run.physics.step_physics();
        let body = &run.physics.rapier().bodies[root_body];
        nonzero_inertia(body);
        assert!(run.physics.body_active(root_body));
        assert_eq!(body.mass(), prototype.bodies[0].mass_kg);
        let mut content = crate::world::contents::ContentInstance::for_structure(placement.clone());
        let root_item = run
            .area
            .items
            .insert_generated(placement.id, &prototype, placement.pose)
            .unwrap();
        assert!(run.area.items.get(root_item).unwrap().body.is_none());
        content.item = Some(root_item);
        if !present {
            // A live item body must not substitute for an absent source root body.
            prototype.bodies.clear();
            run.area.items.get_mut(root_item).unwrap().body = Some(root_body);
        }
        crate::world::content_mechanisms::bind(
            &mut content,
            &mut instance,
            &prototype,
            &reference,
            &rules,
            &mut run.area.items,
            &mut run.area.interaction,
            &mut run.physics,
        )
        .unwrap();
        let item = if row.node == 0 {
            root_item
        } else {
            content
                .bindings
                .iter()
                .find(|b| b.source == row.recipe.source)
                .unwrap()
                .item
        };
        if present {
            assert_ne!(
                run.area.items.get(root_item).unwrap().body,
                Some(root_body),
                "binding must preserve existing item ownership"
            );
        }
        let control_node = content.nodes[node.0 as usize - 1];
        let mut saved = run.area.interaction.mechanisms.snapshot(item).unwrap();
        saved.degrees[1] = (rotation.limits_degrees[1][0] + rotation.limits_degrees[1][1]) * 0.5;
        saved.enabled = true;
        run.area
            .interaction
            .mechanisms
            .restore(item, saved)
            .unwrap();
        run.area.interaction.mechanisms.sync(&mut run.physics, true);
        zero(&mut run, root_body);
        let input =
            10000. * if events[0] { -1. } else { 1. } * if rotation.inverted[1] { -1. } else { 1. };
        update(&mut run, item, [0., input], 0.02);
        let local_axes = Vector::new(
            axes[0] as u8 as f32,
            axes[1] as u8 as f32,
            -(axes[2] as u8 as f32),
        ) * if *inverted { -1. } else { 1. };
        assert_eq!(
            run.area
                .interaction
                .mechanisms
                .rotation_degrees(item)
                .unwrap()[1],
            rotation.limits_degrees[1][if events[0] { 0 } else { 1 }]
        );
        assert_impulse(
            &mut run,
            control_node,
            root_body,
            local_axes,
            if present { *force } else { 0. },
        );
    }
}

#[test]
fn g10_world_sample_source_identity() {
    let reference = super::super::reference();
    let capture = reference.world.capture_library.manifest_sha256.clone();
    let package_hash = "f".repeat(64);
    let mut republished = reference.clone();
    republished.library.manifest_sha256 = package_hash.clone();
    std::sync::Arc::make_mut(&mut republished.world)
        .library
        .manifest_sha256 = package_hash;
    republished.world.validate(&republished).unwrap();
    assert_eq!(republished.world.capture_library.manifest_sha256, capture);

    let mut manifest_only = reference.clone();
    manifest_only.library.manifest_sha256 = "e".repeat(64);
    assert!(manifest_only.world.validate(&manifest_only).is_err());
    let mut runtime_only = reference.clone();
    std::sync::Arc::make_mut(&mut runtime_only.world)
        .library
        .manifest_sha256 = "e".repeat(64);
    assert!(runtime_only.world.validate(&runtime_only).is_err());

    for changed_source in [true, false] {
        let mut changed = republished.clone();
        let world = std::sync::Arc::make_mut(&mut changed.world);
        if changed_source {
            changed.library.source_manifest_sha256 = "0".repeat(64);
            world.library.source_manifest_sha256 = "0".repeat(64);
        } else {
            changed.library.catalog_sha256 = "0".repeat(64);
            world.library.catalog_sha256 = "0".repeat(64);
        }
        assert!(
            changed
                .world
                .validate(&changed)
                .unwrap_err()
                .to_string()
                .contains("world capture library receipt")
        );
    }

    let mut malformed_runtime = reference.clone();
    std::sync::Arc::make_mut(&mut malformed_runtime.world)
        .library
        .manifest_sha256 = "invalid".into();
    assert!(
        malformed_runtime
            .world
            .validate(&malformed_runtime)
            .unwrap_err()
            .to_string()
            .contains("world library receipt")
    );

    for changed_source in [true, false] {
        let mut changed = reference.clone();
        let capture = &mut std::sync::Arc::make_mut(&mut changed.world).capture_library;
        if changed_source {
            capture.source_manifest_sha256 = "0".repeat(64);
        } else {
            capture.catalog_sha256 = "0".repeat(64);
        }
        assert!(
            changed
                .world
                .validate(&changed)
                .unwrap_err()
                .to_string()
                .contains("world capture library receipt")
        );
    }
    let mut changed_capture = reference.clone();
    std::sync::Arc::make_mut(&mut changed_capture.world)
        .capture_library
        .manifest_sha256 = "0".repeat(64);
    assert!(
        changed_capture
            .world
            .validate(&changed_capture)
            .unwrap_err()
            .to_string()
            .contains("world capture library receipt")
    );
}

#[test]
fn g10_reaction_schema() {
    let (reference, _) = load_garage();
    let control = door(&reference);
    let mut row = serde_json::to_value(&control).unwrap();
    row.as_object_mut().unwrap().remove("reaction");
    assert!(serde_json::from_value::<MechanismReference>(row).is_err());
    for force in [f32::NAN, f32::INFINITY, -1.] {
        let mut invalid = reference.clone();
        let reaction = &mut invalid
            .mechanisms
            .iter_mut()
            .find(|m| m.item == control.item)
            .unwrap()
            .reaction;
        let EndpointReactionReference::Rotation { force: value, .. } = reaction else {
            unreachable!()
        };
        *value = force;
        assert!(
            invalid
                .validate()
                .unwrap_err()
                .to_string()
                .contains("mechanism endpoint reaction")
        );
    }
    let mut invalid = reference.clone();
    let EndpointReactionReference::Rotation { node, .. } = &mut invalid
        .mechanisms
        .iter_mut()
        .find(|m| m.item == control.item)
        .unwrap()
        .reaction
    else {
        unreachable!()
    };
    *node = NodeId(u32::MAX);
    assert!(
        invalid
            .validate()
            .unwrap_err()
            .to_string()
            .contains("mechanism endpoint reaction")
    );
    let mut generated = reference.clone();
    let generated_rotation = std::sync::Arc::make_mut(&mut generated.world)
        .mechanisms
        .iter_mut()
        .find(|row| row.recipe.rotation.is_some())
        .unwrap();
    generated_rotation.recipe.reaction = EndpointReactionReference::Rotation {
        node: NodeId(u32::MAX),
        axes: [false; 3],
        inverted: false,
        force: 0.,
        events: [false; 4],
    };
    assert!(
        generated
            .validate()
            .unwrap_err()
            .to_string()
            .contains("world mechanism endpoint reaction")
    );
    let absent = reference
        .mechanisms
        .iter()
        .find(|m| matches!(m.reaction, EndpointReactionReference::None))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&absent.reaction).unwrap(),
        serde_json::json!({"kind": "none"})
    );
}
