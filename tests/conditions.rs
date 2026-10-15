mod common;
use my_summer_slop::conditions::{ConditionStore, RepairResult};

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
