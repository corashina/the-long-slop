mod common;
use my_summer_slop::interaction::{
    items::{ItemLocation, ItemStore},
    lighting::LightingState,
    mechanisms::{ControlAction, MechanismState},
};
use my_summer_slop::reference::*;
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::vehicle::VehiclePose;
fn group(source: SourceId, control: LampControl, count: usize) -> LampGroupReference {
    LampGroupReference {
        source,
        owner: None,
        control,
        electronics_source: None,
        electronics_output: 1.0,
        initial_on: false,
        initial_state: 0,
        initial_target: 0,
        initial_energy: 0.0,
        energy_move: 10.0,
        button: None,
        states: (0..count)
            .map(|_| LampStateReference {
                intensity: 1.0,
                lamps: vec![],
                renderers: vec![],
                material: None,
            })
            .collect(),
    }
}
#[test]
fn light_switch_respects_electronics_and_source_states() {
    let mut reference = common::reference();
    let owner = reference.items[0].clone();
    let body = reference.bodies[1].id.0;
    common::add_rotation(&mut reference, owner.id.0, body, [0.0, 90.0]);
    reference.lighting.door_items = vec![owner.id];
    let door = reference.mechanisms.len() - 1;
    let mut window = reference.mechanisms[door].clone();
    window.id = MechanismId(999);
    window.item = reference.items[1].id;
    window.child = None;
    reference.mechanisms[door].child = Some(window.item);
    reference.mechanisms.push(window);
    let mut a = group(owner.source.clone(), LampControl::Headlights, 3);
    a.owner = Some(owner.id);
    let b = group(
        SourceId {
            file: "fixture".into(),
            path_id: 200,
        },
        LampControl::Interior,
        2,
    );
    let mut c = group(
        SourceId {
            file: "fixture".into(),
            path_id: 201,
        },
        LampControl::Home,
        2,
    );
    c.initial_energy = 1.0;
    reference.lighting.groups = vec![a, b, c];
    let switch_item = reference.items[2].id;
    let mut switch = reference.mechanisms[door].clone();
    switch.id = MechanismId(1000);
    switch.item = switch_item;
    switch.child = None;
    switch.rotation = None;
    switch.driven_nodes.clear();
    switch.turn = Some(TurnReference {
        action: TurnAction::ToggleHomeLight {
            source: reference.lighting.groups[2].source.clone(),
        },
        max_state: 0,
        initial_state: 0,
    });
    reference.mechanisms.push(switch);
    let mut lighting = LightingState::new(&reference.lighting);
    let mut mechanisms = MechanismState::new(&reference).unwrap();
    let view = VehiclePose {
        position: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    assert!(mechanisms.primary_switch(switch_item));
    mechanisms.step(
        switch_item,
        FrameInput {
            pressed: keys::FILL,
            ..Default::default()
        },
        view,
        1.0 / 60.0,
    );
    assert_eq!(
        mechanisms.take_control_action(),
        Some(ControlAction::ToggleHomeLight(
            reference.lighting.groups[2].source.clone()
        ))
    );
    assert_eq!(mechanisms.take_control_action(), None);
    let mut items = ItemStore::new(&reference).unwrap();
    items.get_mut(owner.id).unwrap().location = ItemLocation::Attached(reference.items[1].id);
    let settle = |lighting: &mut LightingState, mechanisms: &MechanismState, items: &ItemStore| {
        for _ in 0..16 {
            lighting.step(mechanisms, items, 1.0 / 60.0);
        }
    };
    settle(&mut lighting, &mechanisms, &items);
    lighting.apply(&ControlAction::CycleHeadlights);
    lighting.step(&mechanisms, &items, 0.1);
    assert_eq!(lighting.groups[0].current, 0);
    assert_eq!(lighting.groups[0].energy, 0.0);
    lighting.step(&mechanisms, &items, 0.1);
    assert_eq!(lighting.groups[0].current, 1);
    assert_eq!(lighting.groups[0].energy, 0.0);
    lighting.step(&mechanisms, &items, 0.1);
    assert_eq!(lighting.groups[0].energy, 1.0);
    for expected in [2, 0, 1] {
        lighting.apply(&ControlAction::CycleHeadlights);
        settle(&mut lighting, &mechanisms, &items);
        assert_eq!(lighting.controls.headlights, expected);
        assert_eq!(lighting.groups[0].current, expected);
        assert!(lighting.output(0, usize::from(expected)) > 0.0);
        lighting.groups[0].electronics_output = 0.0;
        assert_eq!(lighting.output(0, usize::from(expected)), 0.0);
        lighting.groups[0].electronics_output = 1.0;
    }
    let view = VehiclePose {
        position: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    mechanisms.step(
        reference.items[1].id,
        FrameInput {
            held: keys::USE,
            mouse: [-45.0, 0.0],
            ..Default::default()
        },
        view,
        1.0 / 60.0,
    );
    assert!(mechanisms.value(owner.id, true).unwrap() > 0.02);
    assert_eq!(mechanisms.value(owner.id, false), Some(0.0));
    settle(&mut lighting, &mechanisms, &items);
    assert!(!lighting.controls.interior_light);
    mechanisms.step(
        owner.id,
        FrameInput {
            held: keys::USE,
            mouse: [-1.8, 0.0],
            ..Default::default()
        },
        view,
        1.0 / 60.0,
    );
    settle(&mut lighting, &mechanisms, &items);
    assert!(!lighting.controls.interior_light);
    mechanisms.step(
        owner.id,
        FrameInput {
            held: keys::USE,
            mouse: [-0.01, 0.0],
            ..Default::default()
        },
        view,
        1.0 / 60.0,
    );
    settle(&mut lighting, &mechanisms, &items);
    assert!(lighting.controls.interior_light);
    items.get_mut(owner.id).unwrap().location = ItemLocation::World;
    settle(&mut lighting, &mechanisms, &items);
    assert!(!lighting.controls.interior_light);
    for (mode, on) in [(1, true), (2, false), (0, false)] {
        lighting.apply(&ControlAction::InteriorLight(mode));
        settle(&mut lighting, &mechanisms, &items);
        assert_eq!(lighting.controls.interior_light, on);
        assert_eq!(lighting.groups[1].current, u8::from(on));
    }
    lighting.apply(&ControlAction::InteriorLight(1));
    lighting.groups[1].electronics_output = 0.0;
    settle(&mut lighting, &mechanisms, &items);
    assert_eq!(lighting.controls.interior_light_mode, 1);
    assert!(!lighting.controls.interior_light);
    lighting.groups[1].electronics_output = 1.0;
    settle(&mut lighting, &mechanisms, &items);
    assert!(lighting.controls.interior_light);
    lighting.apply(&ControlAction::ToggleHomeLight(
        reference.lighting.groups[2].source.clone(),
    ));
    settle(&mut lighting, &mechanisms, &items);
    assert_eq!(lighting.groups[2].current, 1);
    lighting.groups[2].electronics_output = 0.0;
    settle(&mut lighting, &mechanisms, &items);
    assert_eq!(lighting.groups[2].current, 0);
    assert!(lighting.groups[2].on);
    lighting.groups[2].electronics_output = 0.2;
    settle(&mut lighting, &mechanisms, &items);
    assert_eq!(lighting.groups[2].current, 0);
    lighting.groups[2].electronics_output = 0.21;
    settle(&mut lighting, &mechanisms, &items);
    assert_eq!(lighting.groups[2].current, 1);
    assert_eq!(lighting.output(2, 1), 1.0);
    lighting.groups[2].electronics_output = 1.0;
    settle(&mut lighting, &mechanisms, &items);
    assert_eq!(lighting.groups[2].current, 1);
}
