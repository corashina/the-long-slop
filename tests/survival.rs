mod common;

use my_summer_slop::reference::ConsumptionEffect;
use my_summer_slop::survival::{SurvivalState, ToiletInput};

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}

#[test]
fn source_rates_and_queue_order() {
    let r = common::reference().survival;
    for (temp, food, water) in [
        (20., 0.7496, 1.7996),
        (-5., 0.7488, 1.79988),
        (40., 0.7496, 1.7996),
    ] {
        let mut state = SurvivalState::new(&r);
        let out = state.advance(&r, 1., temp, 1.);
        close(out.needs.food, food);
        close(out.needs.water, water);
    }
    let mut empty = r.clone();
    empty.initial.food = 0.;
    empty.initial.water = 0.;
    let mut state = SurvivalState::new(&empty);
    close(state.advance(&empty, 1., 20., 1.).needs.hp, 4.99765);
    let mut queued = SurvivalState::new(&empty);
    queued.queue(ConsumptionEffect {
        food: 0.2,
        water: 0.4,
        bowel: 0.,
        hp: 0.,
    });
    let out = queued.advance(&empty, 0.25, 20., 1.);
    close(out.needs.food, 0.1);
    close(out.needs.water, 0.2);
    close(queued.pending().food, 0.1);
    close(queued.pending().water, 0.2);

    let mut full = r.clone();
    full.food_loss = 0.;
    full.initial.food = full.maximum.food;
    full.initial.water = full.maximum.water;
    full.initial.hp = 4.;
    full.initial.bowel = 0.;
    full.initial_bladder = [0.; 6];
    let mut state = SurvivalState::new(&full);
    close(state.advance(&full, 1., 20., 1.).needs.hp, 4.0035);
    full.initial.bowel = full.maximum.bowel;
    let mut state = SurvivalState::new(&full);
    close(state.advance(&full, 1., 20., 1.).needs.hp, 4.00175);

    let mut excess = full.clone();
    excess.initial.food = 0.9;
    excess.initial.water = 2.9;
    excess.initial.bowel = 0.;
    excess.initial_bladder = [0.25, 0.25, 0., 0., 0., 0.];
    let mut state = SurvivalState::new(&excess);
    state.queue(ConsumptionEffect {
        food: 0.3,
        water: 0.4,
        bowel: 0.,
        hp: 0.,
    });
    let out = state.advance(&excess, 0.5, 20., 1.);
    close(out.needs.food, 1.);
    close(out.needs.water, 3.);
    close(out.needs.bowel, 0.2);
    close(out.bladder_liters, 0.7653847);
}

#[test]
fn toilet_fullness_and_gate() {
    let mut r = common::reference().survival;
    r.initial.bowel = r.maximum.bowel;
    r.initial_bladder = [0.; 6];
    let mut state = SurvivalState::new(&r);
    assert!(!state.output().can_eat);
    let input = ToiletInput {
        burp_pressed: true,
        ..Default::default()
    };
    let first = state.toilet_step(&r, input, 0.1, [0.1, 0.9], 4.0);
    assert!(first.defecated);
    close(state.output().needs.bowel, 1.4);
    assert!(state.output().can_eat);
    assert!(!state.toilet_step(&r, input, 0.1, [0.1, 0.9], 4.0).defecated);
    assert!(!state.toilet_step(&r, input, 3.8, [0.1, 0.9], 4.0).defecated);
    assert!(state.toilet_step(&r, input, 0.2, [0.1, 0.9], 4.0).defecated);

    let mut state = SurvivalState::new(&r);
    assert!(state.toilet_step(&r, input, 0.1, [0.1, 0.5], 1.0).fart);
    assert!(state.toilet_step(&r, input, 0.1, [0.9, 0.9], 1.0).burp);
    assert!(!state.toilet_step(&r, input, 0.1, [0.9, 0.9], 1.0).burp);
    assert!(state.toilet_step(&r, input, 1., [0.9, 0.9], 1.0).burp);

    r.initial_bladder = [0., 0., 0., 0.01, 0., 0.];
    let mut state = SurvivalState::new(&r);
    let held = ToiletInput {
        urinate: true,
        ..Default::default()
    };
    let first_pour = state
        .toilet_step(&r, held, 1. / 60., [0.; 2], 0.)
        .poured_liters;
    assert!(first_pour > 0.);
    state.apply_drink(&r, [0., 0., 1., 0., 0., 0.], 0.1);
    let latched_pour = state
        .toilet_step(&r, held, 1. / 60., [0.; 2], 0.)
        .poured_liters;
    assert!(latched_pour < first_pour);
    state.toilet_step(&r, ToiletInput::default(), 1., [0.; 2], 0.);
    assert!(state.toilet_step(&r, held, 1., [0.; 2], 0.).poured_liters > 0.);
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
fn feedback_timer_continues_during_death() {
    let mut r = common::reference().survival;
    r.food_loss = 0.;
    let mut state = SurvivalState::new(&r);
    state.show_stats();
    state.damage_instant(&r, r.initial.hp);
    assert!(state.output().dead);
    close(state.output().opacity, 1.);
    state.advance(&r, 0., 20., 1.);
    close(state.advance(&r, 4.5, 20., 1.).opacity, 0.5);
    close(state.advance(&r, 0.5, 20., 1.).opacity, 0.);
    assert!(!state.output().menu_due);
}

#[test]
fn drinking_uses_all_six_source_fractions() {
    let mut r = common::reference().survival;
    r.initial.water = 1.;
    r.initial.hp = 4.;
    r.initial_bladder = [0.; 6];
    let mut state = SurvivalState::new(&r);
    state.apply_drink(&r, [0.1, 0.1, 0.2, 0.3, 0.2, 0.1], 1.);
    close(state.output().needs.water, 0.6);
    close(state.output().bladder_liters, 0.34);
    let pending = state.pending();
    close(pending.water, 0.3);
    close(pending.hp, 0.1);
    state.advance(&r, 0.5, 20., 1.);
    close(state.pending().hp, 0.);
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
                0.
            )
            .poured_liters,
        0.
    );
    assert_eq!(state.output(), before);
}

#[test]
fn warning_and_feedback_boundaries() {
    let mut r = common::reference().survival;
    r.food_loss = 0.;
    r.max_healing_speed = 0.;
    r.initial.hp = 1.25;
    let mut exact = SurvivalState::new(&r);
    close(exact.advance(&r, 1., 20., 1.).opacity, 0.);
    r.initial.hp = 1.249;
    let mut below = SurvivalState::new(&r);
    close(below.advance(&r, 1., 20., 1.).opacity, 1.);
    close(below.advance(&r, 4.5, 20., 1.).opacity, 0.5);
    below.show_stats();
    close(below.output().opacity, 1.);
}

#[test]
fn bladder_capacity_paths_and_signed_effects() {
    let mut r = common::reference().survival;
    r.food_loss = 0.;
    r.initial.water = r.maximum.water;
    r.initial_bladder = [0., 0., 0., 2., 0., 0.];
    let mut state = SurvivalState::new(&r);
    state.apply_drink(&r, [0., 0., 1., 0., 0., 0.], 0.1);
    close(state.output().needs.water, 2.8);
    close(state.output().bladder_liters, 2.);
    state.queue(ConsumptionEffect {
        water: 1.,
        ..Default::default()
    });
    state.advance(&r, 0.5, 20., 1.);
    close(state.output().needs.water, 3.);
    close(state.output().bladder_liters, 2.8);
    state.queue(ConsumptionEffect {
        water: -0.5,
        food: 0.2,
        ..Default::default()
    });
    state.advance(&r, 0.5, 20., 1.);
    close(state.output().needs.water, 2.5);
    close(state.output().needs.food, 0.95);
}
