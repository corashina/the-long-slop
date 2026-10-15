mod common;
use my_summer_slop::interaction::items::ItemStore;
use my_summer_slop::interaction::{InteractionAction, InteractionState, LightAim, ResolvedAim};
use my_summer_slop::lighting::LightingState;
use my_summer_slop::player::PlayerState;
use my_summer_slop::reference::{LightControlId, LightControllerKind};
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};
fn pose() -> VehiclePose {
    VehiclePose {
        position: [10., 2., 0.],
        rotation: [0., 0., 0., 1.],
    }
}
fn target(id: u32) -> ResolvedAim {
    let r = common::reference();
    let c = r
        .lighting
        .controllers
        .iter()
        .find(|c| c.id == LightControlId(id))
        .unwrap();
    ResolvedAim {
        light_target: Some(LightAim {
            control: r
                .lighting
                .controllers
                .iter()
                .filter(|other| other.aim_colliders.contains(&c.aim_colliders[0]))
                .map(|other| other.id)
                .min_by_key(|id| id.0)
                .unwrap(),
            node: r
                .colliders
                .iter()
                .find(|record| record.id == c.aim_colliders[0])
                .unwrap()
                .node,
            collider: c.aim_colliders[0],
            distance: 1.,
            point: [10., 2., -1.],
        }),
        ..Default::default()
    }
}
fn fixture() -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    let r = common::reference();
    let mut p = common::world();
    let player = PlayerState::new(&r.player, pose(), &mut p);
    (
        InteractionState::new(&r).unwrap(),
        ItemStore::new(&r).unwrap(),
        p,
        player,
    )
}
fn step(
    i: &mut InteractionState,
    t: ResolvedAim,
    p: &mut PlayerState,
    s: &mut ItemStore,
    w: &mut PhysicsWorld,
    pressed: u32,
) -> Option<InteractionAction> {
    i.step(
        FrameInput {
            held: pressed,
            pressed,
            ..Default::default()
        },
        t,
        p,
        s,
        w,
        0.016,
        Default::default(),
        1.,
    )
}
#[test]
fn physical_headlight_switch_cycles_the_shared_car_request_once() {
    let r = common::reference();
    let (mut i, mut items, mut w, mut p) = fixture();
    let a = i.available_actions(&target(9), &p, &items, &w, Default::default());
    assert_eq!(a.light_control, Some(LightControlId(8)));
    let action = step(
        &mut i,
        target(9),
        &mut p,
        &mut items,
        &mut w,
        keys::USE | keys::PRIMARY,
    );
    assert_eq!(
        action,
        Some(InteractionAction::LightControl(LightControlId(8)))
    );
    let mut state = LightingState::new(&r.lighting);
    state.use_control(&r.lighting, a.light_control.unwrap());
    for _ in 0..3 {
        state.advance_fades(&r.lighting, 0.1);
    }
    for c in r
        .lighting
        .controllers
        .iter()
        .filter(|c| c.kind == LightControllerKind::Headlight)
    {
        assert!(
            state
                .light_outputs()
                .iter()
                .find(|o| o.id == c.states[1].lights[0])
                .unwrap()
                .enabled
        );
        assert!(
            !state
                .light_outputs()
                .iter()
                .find(|o| o.id == c.states[2].lights[0])
                .unwrap()
                .enabled
        );
    }
    let mut conflict = r.clone();
    conflict.lighting.controllers[6].aim_colliders =
        r.lighting.controllers[0].aim_colliders.clone();
    assert!(InteractionState::new(&conflict).is_err());
}
