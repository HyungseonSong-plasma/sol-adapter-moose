//! B1 differential characterization against an immutable Physics Python source.
//! The exact Python SHA is checked out by the dedicated CI parity job.

use serde_json::{json, Value};
use sol_adaptor_moose::hit::{
    append_top_level_block, get_parameter, remove_block, remove_parameter, replace_block,
    upsert_parameter, HitError, MooseInput,
};
use std::io::Write;
use std::process::{Command, Stdio};

fn legacy(case: &Value, root: &str) -> Value {
    let mut child = Command::new("python3")
        .arg("tests/parity/b1_legacy_oracle.py")
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("launch pinned Python oracle");
    child
        .stdin
        .take()
        .expect("oracle stdin")
        .write_all(case.to_string().as_bytes())
        .expect("write case");
    let result = child.wait_with_output().expect("wait for Python oracle");
    assert!(
        result.status.success(),
        "oracle crashed for {case}: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).expect("oracle JSON")
}

fn error_name(err: &HitError) -> &'static str {
    match err {
        HitError::UnmatchedClose { .. } => "UnmatchedClose",
        HitError::UnclosedBlock { .. } => "UnclosedBlock",
        HitError::BlockNotFound { .. } => "BlockNotFound",
        HitError::AmbiguousBlock { .. } => "AmbiguousBlock",
        HitError::AmbiguousParameter { .. } => "AmbiguousParameter",
        HitError::InvalidMutation { .. } => "InvalidMutation",
    }
}

fn native(case: &Value) -> Value {
    let str_field = |key: &str| case[key].as_str().unwrap_or("");
    let text = str_field("text");
    let path = str_field("path");
    let name = str_field("name");
    let value = str_field("value");
    let payload = str_field("payload");
    let outcome: Result<Value, HitError> = match str_field("op") {
        "paths" => MooseInput::parse(text).map(|doc| {
            json!(doc
                .blocks()
                .iter()
                .map(|span| span.path.as_str())
                .collect::<Vec<_>>())
        }),
        "unique" => MooseInput::parse(text)
            .and_then(|doc| doc.unique(path).map(|span| json!(span.path))),
        "children" => MooseInput::parse(text).map(|doc| json!(doc.direct_children(path))),
        "get" => get_parameter(text, path, name).map(|value| json!(value)),
        "upsert" => upsert_parameter(text, path, name, value).map(|value| json!(value)),
        "remove_param" => remove_parameter(text, path, name).map(|value| json!(value)),
        "remove_block" => remove_block(text, path).map(|value| json!(value)),
        "replace_block" => replace_block(text, path, payload).map(|value| json!(value)),
        "append" => append_top_level_block(text, payload).map(|value| json!(value)),
        "insert_child" => MooseInput::parse(text)
            .and_then(|doc| doc.insert_before_close(path, payload).map(|value| json!(value))),
        other => panic!("unexpected parity operation: {other}"),
    };
    match outcome {
        Ok(value) => json!({"status": "ok", "value": value}),
        Err(err) => json!({"status": "error", "kind": error_name(&err)}),
    }
}

#[test]
fn pinned_python_rust_shared_contract_differential() {
    let Ok(root) = std::env::var("PHYSICS_B1_SOURCE_ROOT") else {
        eprintln!("B1 oracle checkout absent: dedicated hit-parity CI validates differential");
        return;
    };
    let base = "[A]\n  p = old # comment\n  [./B]\n    q = child\n  [../]\n[]\n";
    let cases = [
        json!({"op": "paths", "text": "# α retained\n[A]\n  [./B]\n  [../]\n[]\n"}),
        json!({"op": "unique", "text": base, "path": "A/B"}),
        json!({"op": "children", "text": base, "path": "A"}),
        json!({"op": "get", "text": base, "path": "A", "name": "p"}),
        json!({"op": "upsert", "text": base, "path": "A", "name": "p", "value": "new"}),
        json!({"op": "upsert", "text": base, "path": "A", "name": "added", "value": "7"}),
        json!({"op": "remove_param", "text": base, "path": "A", "name": "p"}),
        json!({"op": "remove_param", "text": base, "path": "A", "name": "missing"}),
        json!({"op": "remove_block", "text": base, "path": "A/B"}),
        json!({"op": "replace_block", "text": base, "path": "A/B", "payload": "  [./C]\n    q = new\n  [../]"}),
        json!({"op": "append", "text": base, "payload": "[Other]\n  k = 1\n[]"}),
        json!({"op": "insert_child", "text": base, "path": "A", "payload": "  [./C]\n  [../]"}),
        json!({"op": "get", "text": "[A]\r\n  p = old # comment\r\n[]\r\n", "path": "A", "name": "p"}),
        json!({"op": "upsert", "text": "[A]\r\n  p = old # comment\r\n[]\r\n", "path": "A", "name": "p", "value": "new"}),
        json!({"op": "unique", "text": "[A]\n[]\n[A]\n[]\n", "path": "A"}),
        json!({"op": "upsert", "text": "[A]\n  p = 1\n  p = 2\n[]\n", "path": "A", "name": "p", "value": "3"}),
        json!({"op": "paths", "text": "[]\n"}),
        json!({"op": "paths", "text": "[A]\n"}),
    ];
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(
            native(case),
            legacy(case, &root),
            "B1 supported parity case #{index}: {case}"
        );
    }
}

#[test]
fn pinned_legacy_deltas_are_explicitly_not_parity_passes() {
    let Ok(root) = std::env::var("PHYSICS_B1_SOURCE_ROOT") else {
        return;
    };
    let nested = json!({
        "op": "get",
        "text": "[A]\n  [./B]\n    value = child\n  [../]\n[]\n",
        "path": "A",
        "name": "value"
    });
    assert_eq!(native(&nested), json!({"status": "ok", "value": null}));
    assert_eq!(
        legacy(&nested, &root),
        json!({"status": "ok", "value": "child"})
    );

    let commented_close = json!({
        "op": "upsert",
        "text": "[A]\n[] # closure note\n",
        "path": "A",
        "name": "p",
        "value": "1"
    });
    assert_eq!(native(&commented_close)["status"], "ok");
    assert_eq!(
        legacy(&commented_close, &root),
        json!({"status": "error", "kind": "InvalidMutation"})
    );
}
