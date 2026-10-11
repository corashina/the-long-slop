use std::{
    io::Write,
    process::{Command, Stdio},
};

fn validate(input: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_validate-reference"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn stdin_accepts_current_manifest() {
    let result = validate(include_bytes!("common/reference.json"));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn stdin_rejects_missing_records_wrong_build_and_invalid_links() {
    let fixture: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for field in fixture.as_object().unwrap().keys() {
        let mut invalid = fixture.clone();
        invalid.as_object_mut().unwrap().remove(field);
        let result = validate(&serde_json::to_vec(&invalid).unwrap());
        assert!(!result.status.success(), "accepted missing {field}");
        assert!(String::from_utf8_lossy(&result.stderr).contains("Runtime manifest contract"));
    }
    let mut invalid = fixture.clone();
    invalid["build_id"] = serde_json::json!(0);
    assert!(
        !validate(&serde_json::to_vec(&invalid).unwrap())
            .status
            .success()
    );
    invalid = fixture.clone();
    invalid["nodes"][0]["parent"] = serde_json::json!(999999);
    assert!(
        !validate(&serde_json::to_vec(&invalid).unwrap())
            .status
            .success()
    );
    invalid = fixture;
    invalid["obsolete"] = serde_json::json!({});
    assert!(
        !validate(&serde_json::to_vec(&invalid).unwrap())
            .status
            .success()
    );
}
