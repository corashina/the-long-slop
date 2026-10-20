//! Critical G04 fluid checks against the installed single-Lada export.
use my_summer_slop::{reference::ReferenceManifest, starting_area::GarageRuntime};

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
thread_local! {
    static TRACK_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct AllocationCounter;
fn record_allocation() {
    if TRACK_ALLOCATIONS.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for AllocationCounter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: AllocationCounter = AllocationCounter;

#[test]
#[ignore = "requires the installed single-Lada export"]
fn g04_ordered_snapshot_capture_and_copy_do_not_allocate() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let r = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let run = GarageRuntime::new(&r).unwrap();
    let oil = r.vehicles[0].engine.oil_tank;
    let warm = run.area.items.generated_snapshot(oil, None).unwrap();
    assert!(warm.fluids.is_some());
    ALLOCATIONS.with(|count| count.set(0));
    TRACK_ALLOCATIONS.with(|track| track.set(true));
    for _ in 0..256 {
        let snapshot = std::hint::black_box(run.area.items.generated_snapshot(oil, None).unwrap());
        let copy = std::hint::black_box(snapshot.clone());
        std::hint::black_box(snapshot == copy);
    }
    TRACK_ALLOCATIONS.with(|track| track.set(false));
    let count = ALLOCATIONS.with(Cell::get);
    assert_eq!(
        count, 0,
        "repeated fluid snapshot capture and copy must not allocate"
    );
}

fn cap_aim(
    run: &GarageRuntime,
    r: &ReferenceManifest,
    cap: my_summer_slop::reference::ItemId,
) -> my_summer_slop::interaction::ResolvedAim {
    use my_summer_slop::interaction::{AimTarget, ResolvedAim};
    let binding = r
        .action_bindings
        .iter()
        .find(|b| b.use_owner == Some(cap))
        .unwrap();
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == binding.collider.0)
        .unwrap()
        .0;
    let target = AimTarget {
        collider,
        item: cap,
        distance: 1.,
        point: run.area.items.get(cap).unwrap().pose.position,
        slot: None,
    };
    ResolvedAim {
        use_target: Some(target),
        filling_target: Some(target),
        ..ResolvedAim::default()
    }
}

fn fluid_step(
    run: &mut GarageRuntime,
    input: my_summer_slop::simulation::FrameInput,
    aim: my_summer_slop::interaction::ResolvedAim,
) -> Option<my_summer_slop::interaction::InteractionAction> {
    run.area.interaction.step(
        input,
        aim,
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        my_summer_slop::simulation::FIXED_DT,
        run.survival.output(),
        1.,
    )
}

fn hold_tank(run: &mut GarageRuntime, tank: my_summer_slop::reference::ItemId) {
    use my_summer_slop::vehicle::to_rapier;
    use rapier3d::math::Vector;
    let body = run.area.items.get(tank).unwrap().body.unwrap();
    let view = to_rapier(run.area.player.view_pose(&run.physics));
    let point = view.translation + view.rotation * Vector::NEG_Z;
    run.physics
        .edit_body(body, |body| body.set_translation(point, true));
    run.area.items.sync_poses(&run.physics);
    run.area
        .interaction
        .pickup_item(
            tank,
            point.to_array(),
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
}

fn set_cap(
    run: &mut GarageRuntime,
    r: &ReferenceManifest,
    id: my_summer_slop::reference::OpeningId,
    open: bool,
) {
    use my_summer_slop::{
        interaction::{InteractionAction, InteractionSound},
        simulation::{FrameInput, keys},
    };
    let opening = r.openings.iter().find(|o| o.id == id).unwrap();
    if run.area.interaction.opening_is_open(id) == open {
        return;
    }
    while run.area.interaction.pop_sound().is_some() {}
    let input = if opening.can_toggle {
        FrameInput {
            pressed: keys::USE,
            held: keys::USE,
            ..FrameInput::default()
        }
    } else {
        assert!(opening.can_tune);
        FrameInput {
            wheel: if open { 1. } else { -1. },
            ..FrameInput::default()
        }
    };
    assert_eq!(
        fluid_step(run, input, cap_aim(run, r, opening.cap)),
        Some(InteractionAction::Cap(id))
    );
    assert_eq!(run.area.interaction.opening_is_open(id), open);
    assert!(
        matches!(run.area.interaction.pop_sound(), Some(InteractionSound::Cap { id: sound_id, .. }) if sound_id == id)
    );
}

fn fill_step(
    run: &mut GarageRuntime,
    r: &ReferenceManifest,
    cap: my_summer_slop::reference::ItemId,
) -> Option<my_summer_slop::interaction::InteractionAction> {
    use my_summer_slop::{
        interaction::{InteractionAction, feedback::ActionMask},
        simulation::{FrameInput, keys},
    };
    let aim = cap_aim(run, r, cap);
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    let feedback = run
        .area
        .interaction
        .feedback(&available, &run.area.items, r);
    let eligible = feedback.actions.0 & ActionMask::FILL != 0;
    assert_eq!(
        available.fill_reservation.is_some(),
        eligible,
        "fill reservation and feedback disagree"
    );
    let fill = fluid_step(
        run,
        FrameInput {
            held: keys::PRIMARY,
            ..FrameInput::default()
        },
        aim,
    )
    .filter(|action| matches!(action, InteractionAction::Fill { .. }));
    assert_eq!(
        fill.is_some(),
        eligible,
        "fill feedback and transfer disagree"
    );
    fill
}

#[test]
#[ignore = "requires the installed single-Lada export"]
fn g04_caps_and_direct_fill() {
    use my_summer_slop::{
        fluids::{FluidContainer, FluidEntry},
        interaction::{InteractionAction, feedback::ActionMask},
        reference::{ItemId, TankRole},
        simulation::{FrameInput, keys},
    };
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let mut r = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    assert_eq!(r.build_id, 11140890);
    assert_eq!(r.vehicles.len(), 1);
    let source = ItemId(116);
    let receiver = r.vehicles[0].engine.fuel_tank;
    let source_cap = r
        .openings
        .iter()
        .find(|o| o.tank == source)
        .unwrap()
        .clone();
    let receiver_cap = r
        .openings
        .iter()
        .find(|o| o.tank == receiver)
        .unwrap()
        .clone();
    let supply = r
        .tanks
        .iter()
        .find(|t| t.role == TankRole::Supply)
        .unwrap()
        .item;
    let supply_cap = r
        .openings
        .iter()
        .find(|o| o.tank == supply)
        .unwrap()
        .clone();
    let mut run = GarageRuntime::new(&r).unwrap();
    hold_tank(&mut run, source);
    assert!(!run.area.interaction.opening_is_open(source_cap.id));

    let closed_receiver_fill = fill_step(&mut run, &r, receiver_cap.cap);
    assert_eq!(closed_receiver_fill, None);
    set_cap(&mut run, &r, receiver_cap.id, true);
    let source_before = run
        .area
        .items
        .get(source)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .total();
    let receiver_before = run
        .area
        .items
        .get(receiver)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .total();
    let open_receiver_fill = fill_step(&mut run, &r, receiver_cap.cap);
    assert!(
        matches!(open_receiver_fill, Some(InteractionAction::Fill { source: s, destination: d, .. }) if s == source && d == receiver)
    );
    let source_after = run
        .area
        .items
        .get(source)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .total();
    let receiver_after = run
        .area
        .items
        .get(receiver)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .total();
    assert!(source_after < source_before);
    assert!(receiver_after > receiver_before);
    let expected = receiver_cap.flow_liters_per_second * my_summer_slop::simulation::FIXED_DT;
    assert!((source_before - source_after - expected).abs() < 1e-6);
    assert!((receiver_after - receiver_before - expected).abs() < 1e-6);
    assert_eq!(
        run.area.interaction.opening_audio_flags(receiver_cap.id),
        (true, false)
    );

    // The receiver closes after this frame's feedback but before transfer.
    let aim = cap_aim(&run, &r, receiver_cap.cap);
    assert_ne!(
        run.area
            .interaction
            .available_actions(
                &aim,
                &run.area.player,
                &run.area.items,
                &run.physics,
                run.survival.output()
            )
            .actions
            .0
            & ActionMask::FILL,
        0
    );
    let before = run.area.items.get(source).unwrap().fluid.clone();
    let receiver_before = run.area.items.get(receiver).unwrap().fluid.clone();
    let action = fluid_step(
        &mut run,
        FrameInput {
            held: keys::USE | keys::PRIMARY,
            pressed: keys::USE,
            ..FrameInput::default()
        },
        aim,
    );
    assert_eq!(action, Some(InteractionAction::Cap(receiver_cap.id)));
    assert_eq!(run.area.items.get(source).unwrap().fluid, before);
    assert_eq!(run.area.items.get(receiver).unwrap().fluid, receiver_before);
    assert!(!run.area.interaction.opening_is_open(receiver_cap.id));
    assert_eq!(fill_step(&mut run, &r, receiver_cap.cap), None);
    set_cap(&mut run, &r, receiver_cap.id, true);

    // A supply is a receiver too. Its open valve also spills through the existing path.
    set_cap(&mut run, &r, supply_cap.id, true);
    let source_before = run
        .area
        .items
        .get(source)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .total();
    let gas_before = run
        .area
        .items
        .get(supply)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture()
        .amount(0);
    let supply_fill = fill_step(&mut run, &r, supply_cap.cap);
    assert!(
        matches!(supply_fill, Some(InteractionAction::Fill { source: s, destination: d, .. }) if s == source && d == supply),
        "supply targets receive fluid from the held tank"
    );
    assert!(
        run.area
            .items
            .get(source)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .total()
            < source_before
    );
    assert!(
        run.area
            .items
            .get(supply)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture()
            .amount(0)
            > gas_before
    );
    let Some(InteractionAction::Fill { liters, .. }) = supply_fill else {
        unreachable!()
    };
    let expected = supply_cap.flow_liters_per_second
        * run.area.interaction.opening_valve(supply_cap.id)
        * my_summer_slop::simulation::FIXED_DT;
    assert!((liters - expected).abs() < 1e-6);
    assert!(run.area.interaction.opening_audio_flags(supply_cap.id).0);
    run.area.items.get_mut(supply).unwrap().fluid = Some(FluidContainer::new(1000., &[]));
    assert!(
        matches!(fill_step(&mut run, &r, supply_cap.cap), Some(InteractionAction::Fill { source: s, destination: d, .. }) if s == source && d == supply)
    );

    run.area.items.get_mut(source).unwrap().fluid = Some(FluidContainer::new(20., &[]));
    let empty_source_fill = fill_step(&mut run, &r, receiver_cap.cap);
    assert_eq!(empty_source_fill, None);
    run.area.items.get_mut(source).unwrap().fluid = Some(FluidContainer::new(
        20.,
        &[FluidEntry {
            fluid_type: 0,
            amount: 2.,
        }],
    ));
    let capacity = run
        .area
        .items
        .get(receiver)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .capacity_liters;
    run.area.items.get_mut(receiver).unwrap().fluid = Some(FluidContainer::new(
        capacity,
        &[FluidEntry {
            fluid_type: 0,
            amount: capacity,
        }],
    ));
    let full_receiver_fill = fill_step(&mut run, &r, receiver_cap.cap);
    assert_eq!(full_receiver_fill, None);
    set_cap(&mut run, &r, source_cap.id, true);
    assert_eq!(fill_step(&mut run, &r, source_cap.cap), None);
    set_cap(&mut run, &r, source_cap.id, false);

    // Oil flow uses the receiver's existing slowdown rule.
    run.area
        .interaction
        .drop_held(&mut run.area.items, &mut run.physics);
    hold_tank(&mut run, ItemId(120));
    run.area.items.get_mut(receiver).unwrap().fluid = Some(FluidContainer::new(
        capacity,
        &[FluidEntry {
            fluid_type: 0,
            amount: 0.5,
        }],
    ));
    let oil_fill = fill_step(&mut run, &r, receiver_cap.cap);
    let Some(InteractionAction::Fill {
        liters,
        source: oil,
        destination,
    }) = oil_fill
    else {
        panic!("oil fill required")
    };
    assert_eq!(oil, ItemId(120));
    assert_eq!(destination, receiver);
    let requested = receiver_cap.flow_liters_per_second * my_summer_slop::simulation::FIXED_DT;
    let expected = (requested * 0.5 * r.fluid_rules.slow_oil_modifier)
        .max(0.0001)
        .min(requested);
    assert!((liters - expected).abs() < 1e-6);
    set_cap(&mut run, &r, supply_cap.id, false);

    for (no_fill_from, no_fill_this) in [(true, false), (false, true)] {
        r.tanks
            .iter_mut()
            .find(|t| t.item == source)
            .unwrap()
            .no_fill_from = no_fill_from;
        r.tanks
            .iter_mut()
            .find(|t| t.item == receiver)
            .unwrap()
            .no_fill_this = no_fill_this;
        let mut blocked = GarageRuntime::new(&r).unwrap();
        hold_tank(&mut blocked, source);
        set_cap(&mut blocked, &r, receiver_cap.id, true);
        let before = blocked.area.items.get(source).unwrap().fluid.clone();
        assert_eq!(fill_step(&mut blocked, &r, receiver_cap.cap), None);
        assert_eq!(blocked.area.items.get(source).unwrap().fluid, before);
    }
}

#[test]
#[ignore = "requires the installed single-Lada export"]
fn g04_particle_receipt() {
    use my_summer_slop::{
        fluids::{FluidContainer, FluidEntry, spill},
        interaction::ResolvedAim,
        reference::ItemId,
        simulation::{FIXED_DT, FrameInput},
        vehicle::to_rapier,
    };
    use rapier3d::math::{Pose, Vector};
    let mut r = ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let source = ItemId(116);
    let receiver = r.vehicles[0].engine.fuel_tank;
    let probe = GarageRuntime::new(&r).unwrap();
    let chassis = probe.physics.chassis_handle(0);
    let collider = probe
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| c.parent() == Some(chassis) && c.is_enabled() && !c.is_sensor())
        .unwrap()
        .1;
    let bounds = collider.compute_aabb();
    let emitter = Pose::from_translation(Vector::new(
        (bounds.mins.x + bounds.maxs.x) * 0.5,
        bounds.maxs.y + 0.05,
        (bounds.mins.z + bounds.maxs.z) * 0.5,
    ));
    let source_cap = r
        .openings
        .iter()
        .find(|o| o.tank == source)
        .unwrap()
        .clone();
    let source_item = r.items.iter().find(|i| i.id == source).unwrap();
    let body = r.bodies.iter().find(|b| b.id == source_item.body).unwrap();
    let anchor = r
        .anchors
        .iter()
        .find(|a| a.id == source_cap.anchor)
        .unwrap();
    let pose = |t: &my_summer_slop::reference::Transform| {
        to_rapier(my_summer_slop::vehicle::VehiclePose {
            position: t.translation,
            rotation: t.rotation,
        })
    };
    let local = (pose(&body.pose) * pose(&anchor.transform)).inverse() * emitter;
    for opening in &mut r.openings {
        opening.valve = 0.;
        if opening.tank == source {
            opening.angle_disabled = true;
            opening.spill_disabled = false;
            opening.tube_connected = false;
            opening.tilt_axis = [0., -1., 0.];
            opening.flow_liters_per_second = 1.;
            opening.particles.local_pose.translation = local.translation.to_array();
            opening.particles.local_pose.rotation = local.rotation.to_array();
            opening.particles.cone_degrees = 0.;
            opening.particles.cone_radius = 0.;
            opening.particles.gravity_multiplier = 0.;
            opening.particles.max_speed = 20.;
            opening.particles.min_rate = 600.;
            opening.particles.max_rate = 600.;
        }
        if opening.tank == receiver {
            opening.flow_radius = 100.;
            opening.spill_disabled = true;
        }
    }
    let receiver_cap = r
        .openings
        .iter()
        .find(|o| o.tank == receiver)
        .unwrap()
        .clone();
    let mut run = GarageRuntime::new(&r).unwrap();
    set_cap(&mut run, &r, receiver_cap.id, true);
    set_cap(&mut run, &r, source_cap.id, true);
    run.area.items.get_mut(source).unwrap().fluid = Some(FluidContainer::new(
        4.,
        &[
            FluidEntry {
                fluid_type: 0,
                amount: 2.,
            },
            FluidEntry {
                fluid_type: 3,
                amount: 2.,
            },
        ],
    ));
    run.area.items.get_mut(receiver).unwrap().fluid = Some(FluidContainer::new(40., &[]));
    let mut expected = run.area.items.get(source).unwrap().fluid.clone().unwrap();
    let mut received = false;
    for _ in 0..20 {
        spill(&mut expected, FIXED_DT * 2.);
        run.area.interaction.step(
            FrameInput::default(),
            ResolvedAim::default(),
            &mut run.area.player,
            &mut run.area.items,
            &mut run.physics,
            FIXED_DT,
            run.survival.output(),
            2.,
        );
        assert_eq!(
            run.area.items.get(source).unwrap().fluid.as_ref().unwrap(),
            &expected,
            "receipt must not debit its source"
        );
        let fluid = run
            .area
            .items
            .get(receiver)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap();
        if fluid.total() > 0. {
            let total = expected.total();
            assert!(
                (fluid.mixture().amount(0) - FIXED_DT * 2. * expected.mixture().amount(0) / total)
                    .abs()
                    < 1e-5
            );
            assert!(
                (fluid.mixture().amount(3) - FIXED_DT * 2. * expected.mixture().amount(3) / total)
                    .abs()
                    < 1e-5
            );
            received = true;
            break;
        }
    }
    assert!(
        received,
        "actual source particles must fill installed receiver geometry"
    );
}

fn sync_fluid_mass(run: &mut GarageRuntime) {
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    run.wheels
        .sync(&mut run.area.items, &mut run.physics)
        .unwrap();
    run.area.sync_portable_mass(&mut run.physics);
}

fn tank_total(run: &GarageRuntime, tank: my_summer_slop::reference::ItemId) -> f32 {
    run.area
        .items
        .get(tank)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .total()
}

fn mass_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.005,
        "mass {actual} != {expected}"
    );
}

#[test]
#[ignore = "requires the installed single-Lada export"]
fn g04_detached_tanks_and_save() {
    use my_summer_slop::{
        interaction::{InteractionAction, ResolvedAim, items::ItemLocation},
        reference::ItemId,
        simulation::FrameInput,
        vehicle::to_rapier,
    };
    use rapier3d::math::{Rotation, Vector};
    let r = ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    assert_eq!(r.build_id, 11140890);
    assert_eq!(r.vehicles.len(), 1);
    let mut run = GarageRuntime::new(&r).unwrap();
    sync_fluid_mass(&mut run);
    // The carry anchor is created once on first pickup.
    hold_tank(&mut run, ItemId(120));
    run.area
        .interaction
        .drop_held(&mut run.area.items, &mut run.physics);
    let chassis = run.physics.chassis_handle(0);
    let body_count = run.physics.rapier().bodies.len();
    let collider_count = run.physics.rapier().colliders.len();
    let engine = &r.vehicles[0].engine;
    for (part, tank, source) in [
        (engine.item, engine.oil_tank, ItemId(120)),
        (engine.coolant_tank, engine.coolant_tank, ItemId(118)),
    ] {
        let slot = r
            .part_slots
            .iter()
            .find(|s| s.part == Some(part))
            .unwrap()
            .id;
        let opening = r.openings.iter().find(|o| o.tank == tank).unwrap();
        let caps: Vec<_> = r
            .openings
            .iter()
            .filter(|o| o.tank == tank)
            .map(|o| o.cap)
            .collect();
        let part_reference = r.parts.iter().find(|p| p.item == part).unwrap();
        let handles: Vec<_> = run
            .physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, c)| {
                part_reference
                    .colliders
                    .iter()
                    .any(|id| id.0 == (c.user_data >> 64) as u32)
            })
            .map(|(handle, collider)| (handle, (collider.user_data >> 64) as u32))
            .collect();
        assert!(!handles.is_empty());
        for cycle in 0..3 {
            let original = run.area.items.get(tank).unwrap().fluid.clone();
            let installed_mass = run.physics.rapier().bodies[chassis].mass();
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
            sync_fluid_mass(&mut run);
            let loose = run.area.items.get(part).unwrap().body.unwrap();
            assert_eq!(
                run.area.items.get(part).unwrap().location,
                ItemLocation::World
            );
            assert_ne!(loose, chassis);
            assert_eq!(run.area.items.get(tank).unwrap().fluid, original);
            let detached_mass = run.physics.rapier().bodies[loose].mass();
            let detached_total = tank_total(&run, tank);
            let remaining_mass = run.physics.rapier().bodies[chassis].mass();
            mass_near(remaining_mass + detached_mass, installed_mass);
            for id in caps.iter().copied().chain([tank]) {
                assert_eq!(run.area.items.get(id).unwrap().id, id);
                assert_eq!(run.area.items.get(id).unwrap().body, Some(loose));
            }
            for &(handle, identity) in &handles {
                let collider = &run.physics.rapier().colliders[handle];
                assert_eq!((collider.user_data >> 64) as u32, identity);
                assert_eq!(collider.parent(), Some(loose));
            }
            let cap_before = run.area.items.get(opening.cap).unwrap().pose;
            set_cap(&mut run, &r, opening.id, true);
            // Move the detached opening upward to prevent free spill during direct fill.
            let forward = to_rapier(run.area.items.get(opening.cap).unwrap().pose).rotation
                * Vector::from_array(opening.tilt_axis);
            run.physics.edit_body(loose, |body| {
                let mut pose = *body.position();
                pose.rotation =
                    Rotation::from_rotation_arc(forward.normalize(), Vector::Y) * pose.rotation;
                pose.translation += Vector::new(0., 5. + cycle as f32, 0.);
                body.set_position(pose, true);
            });
            run.area.items.sync_poses(&run.physics);
            sync_fluid_mass(&mut run);
            assert_ne!(run.area.items.get(opening.cap).unwrap().pose, cap_before);
            hold_tank(&mut run, source);
            let before_fill = tank_total(&run, tank);
            assert!(
                matches!(fill_step(&mut run, &r, opening.cap), Some(InteractionAction::Fill { destination, .. }) if destination == tank)
            );
            let after_fill = tank_total(&run, tank);
            assert!(after_fill > before_fill);
            sync_fluid_mass(&mut run);
            mass_near(
                run.physics.rapier().bodies[loose].mass(),
                detached_mass + after_fill - detached_total,
            );
            mass_near(run.physics.rapier().bodies[chassis].mass(), remaining_mass);
            run.area
                .interaction
                .drop_held(&mut run.area.items, &mut run.physics);
            let forward = to_rapier(run.area.items.get(opening.cap).unwrap().pose).rotation
                * Vector::from_array(opening.tilt_axis);
            run.physics.edit_body(loose, |body| {
                let mut pose = *body.position();
                pose.rotation =
                    Rotation::from_rotation_arc(forward.normalize(), -Vector::Y) * pose.rotation;
                body.set_position(pose, true);
            });
            run.area.items.sync_poses(&run.physics);
            sync_fluid_mass(&mut run);
            fluid_step(&mut run, FrameInput::default(), ResolvedAim::default());
            let after_spill = tank_total(&run, tank);
            assert!(
                after_spill < after_fill,
                "detached tank must spill through its open cap"
            );
            sync_fluid_mass(&mut run);
            mass_near(
                run.physics.rapier().bodies[loose].mass(),
                detached_mass + after_spill - detached_total,
            );
            set_cap(&mut run, &r, opening.id, false);
            let final_fluid = run.area.items.get(tank).unwrap().fluid.clone();
            let final_total = tank_total(&run, tank);
            sync_fluid_mass(&mut run);
            mass_near(
                run.physics.rapier().bodies[loose].mass(),
                detached_mass + final_total - detached_total,
            );
            run.area
                .interaction
                .parts
                .install(part, slot, &mut run.area.items, &mut run.physics)
                .unwrap();
            run.area
                .interaction
                .parts
                .mount_mechanism(part, &mut run.area.interaction.mechanisms);
            sync_fluid_mass(&mut run);
            assert_eq!(run.area.interaction.parts.slot_item(slot), Some(part));
            assert!(!run.physics.rapier().bodies[loose].is_enabled());
            assert_eq!(run.area.items.get(tank).unwrap().fluid, final_fluid);
            mass_near(
                run.physics.rapier().bodies[chassis].mass(),
                installed_mass + final_total - detached_total,
            );
            for id in caps.iter().copied().chain([tank]) {
                assert_eq!(run.area.items.get(id).unwrap().id, id);
                assert_eq!(run.area.items.get(id).unwrap().body, Some(chassis));
            }
            for &(handle, identity) in &handles {
                let collider = &run.physics.rapier().colliders[handle];
                assert_eq!((collider.user_data >> 64) as u32, identity);
                assert_eq!(collider.parent(), Some(chassis));
            }
            assert_eq!(run.physics.rapier().bodies.len(), body_count);
            assert_eq!(run.physics.rapier().colliders.len(), collider_count);
        }
    }
    // Exact signed capture and completed deferred restore share the existing private test:
    // world::stream::tests::g04_ordered_save_load_cleans_once_before_detached_mass_sync.
}
