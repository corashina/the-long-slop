mod common;

use my_summer_slop::reference::ConsumptionEffect;
use my_summer_slop::survival::{SurvivalState, ToiletInput};

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}

#[test]
fn death_does_not_heal() {
    let mut r = common::reference().survival;
    r.food_loss = 0.;
    r.initial.food = r.maximum.food;
    r.initial.water = r.maximum.water;
    r.initial.bowel = 0.;
    r.initial_bladder = [0.; 6];
    r.initial.hp = 0.001;
    let mut state = SurvivalState::new(&r);
    state.queue(ConsumptionEffect {
        hp: 1.,
        ..Default::default()
    });
    state.damage_instant(&r, 0.001);
    assert!(state.output().dead);
    close(state.advance(&r, 1., 20., 1.).needs.hp, 0.);
    close(state.output().death_seconds as f32, 1.);
    assert!(!state.advance(&r, 12.999, 20., 1.).menu_due);
    assert!(state.advance(&r, 0.001, 20., 1.).menu_due);
    close(state.output().needs.hp, 0.);
}

#[test]
fn invalid_deltas_leave_state_unchanged() {
    let r = common::reference().survival;
    let mut state = SurvivalState::new(&r);
    let before = state.output();
    assert_eq!(state.advance(&r, f32::NAN, 20., 1.), before);
    assert_eq!(state.advance(&r, -1., 20., 1.), before);
    assert_eq!(state.advance(&r, 1., f32::INFINITY, 1.), before);
    assert_eq!(state.advance(&r, 1., 20., -1.), before);
    assert_eq!(
        state
            .toilet_step(
                &r,
                ToiletInput {
                    urinate: true,
                    ..Default::default()
                },
                f32::NAN,
                [0.; 2],
                0.,
                1.
            )
            .poured_liters,
        0.
    );
    assert_eq!(state.output(), before);
}
