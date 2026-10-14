mod common;
use my_summer_slop::conditions::{ConditionStore, RepairResult};

#[test]
fn repair_threshold_resets_without_overflow() {
    let mut r = common::reference();
    let s = &mut r.conditions.surfaces[0];
    s.startup.state = 2;
    s.startup.repair_progress = 0.;
    let id = s.id;
    assert!(r.conditions.repair_tables[s.permission_index as usize][2][2]);
    let mut store = ConditionStore::new(&r).unwrap();
    assert_eq!(store.repair(id, 2, 0.75), RepairResult::None);
    assert_eq!(store.repair(id, 2, 0.5), RepairResult::Improved);
    assert_eq!(store.get(id).state, 1);
    assert_eq!(store.get(id).repair_progress, 0.);
}

#[test]
fn damage_precedes_repair() {
    let mut r = common::reference();
    let s = &mut r.conditions.surfaces[0];
    s.startup.state = 2;
    let id = s.id;
    r.conditions.repair_tables[s.permission_index as usize][2][3] = true;
    r.conditions.repair_tables[s.permission_index as usize][2][1] = true;
    let mut store = ConditionStore::new(&r).unwrap();
    assert_eq!(store.repair(id, 1, 1.), RepairResult::Worsened);
    assert_eq!(store.get(id).state, 3);
    assert_eq!(store.get(id).repair_progress, 0.);
}

#[test]
fn tool_wear_disables_after_state_four() {
    let mut r = common::reference();
    let id = r.conditions.brushes[0].own_condition;
    r.conditions
        .surfaces
        .iter_mut()
        .find(|s| s.id == id)
        .unwrap()
        .startup
        .state = 4;
    let mut store = ConditionStore::new(&r).unwrap();
    assert!(store.wear(id));
    assert_eq!(store.get(id).state, 5);
    assert!(!store.paint(id, [0.; 4]));
}

#[test]
fn children_follow_only_source_flags() {
    let mut r = common::reference();
    let mut child = r.conditions.surfaces[0].clone();
    child.id.0 = 800;
    child.source.path_id = 800;
    child.item = None;
    child.is_child = true;
    child.startup.color = [0.2, 0.3, 0.4, 1.];
    let mut independent = child.clone();
    independent.id.0 = 801;
    independent.source.path_id = 801;
    independent.is_child = false;
    r.conditions.surfaces[0].children = vec![child.id, independent.id];
    r.conditions.surfaces[0].startup.state = 2;
    let id = r.conditions.surfaces[0].id;
    r.conditions
        .surfaces
        .extend([child.clone(), independent.clone()]);
    let mut store = ConditionStore::new(&r).unwrap();
    store.repair(id, 2, 1.);
    assert_eq!(store.get(child.id).state, 1);
    assert_eq!(store.get(child.id).color, child.startup.color);
    assert_eq!(store.get(independent.id).state, independent.startup.state);
    assert_eq!(store.get(independent.id).color, independent.startup.color);
    store.paint(id, [0.8, 0.1, 0.2, 1.]);
    store.paint(id, [0.7, 0.1, 0.2, 1.]);
    assert_eq!(store.get(child.id).color, child.startup.color);
    r.conditions.surfaces[0].no_inherit_state = true;
    let mut blocked = ConditionStore::new(&r).unwrap();
    blocked.repair(id, 2, 1.);
    assert_eq!(blocked.get(child.id).state, child.startup.state);
    r.conditions
        .surfaces
        .iter_mut()
        .find(|s| s.id == independent.id)
        .unwrap()
        .same_state_as_parent = true;
    let mut same = ConditionStore::new(&r).unwrap();
    same.repair(id, 2, 1.);
    assert_eq!(same.get(independent.id).state, 1);
    let mut changed = Vec::new();
    while let Some(id) = store.take_changed() {
        assert!(!changed.contains(&id));
        changed.push(id);
    }
}

#[test]
fn decay_clamps_both_signs_and_pause_preserves_progress() {
    let mut r = common::reference();
    r.conditions.surfaces[0].startup.repair_progress = 0.005;
    r.conditions.surfaces[1].startup.repair_progress = -0.005;
    let mut store = ConditionStore::new(&r).unwrap();
    store.update(1., false);
    assert_eq!(
        store.get(r.conditions.surfaces[0].id).repair_progress,
        0.005
    );
    assert_eq!(
        store.get(r.conditions.surfaces[1].id).repair_progress,
        -0.005
    );
    store.update(1., true);
    assert_eq!(store.get(r.conditions.surfaces[0].id).repair_progress, 0.);
    assert_eq!(store.get(r.conditions.surfaces[1].id).repair_progress, 0.);
}
