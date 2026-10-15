mod common;
use my_summer_slop::reference::ConditionId;

#[test]
fn child_cycle_is_rejected() {
    let mut r = common::reference();
    let id = r.conditions.surfaces[0].id;
    let source = r.conditions.surfaces[0].source.clone();
    r.conditions.surfaces[0].children.push(ConditionId(id.0));
    let error = r.validate().unwrap_err().to_string();
    assert!(error.contains(&format!("{}:{}", source.file, source.path_id)));
    assert!(error.contains("children"));
}
