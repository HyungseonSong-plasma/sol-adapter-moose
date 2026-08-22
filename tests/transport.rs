use serde_json::Value;
use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_adapter(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sol-adaptor-moose"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("adapter binary starts");

    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("request writes");
    drop(child.stdin.take());
    child.wait_with_output().expect("adapter exits")
}

#[test]
fn describe_adapter_emits_one_protocol_frame_and_no_stderr() {
    let output = run_adapter("{\"jsonrpc\":\"2.0\",\"method\":\"describe_adapter\",\"id\":1}\n");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);

    let response: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["kind"], "protocol_success");
    assert_eq!(
        response["result"]["payload"]["bootstrap"]["adapter_id"],
        "sol.adapter.moose"
    );
    assert_eq!(
        response["result"]["payload"]["bootstrap"]["supported_adapter_protocol_versions"][0],
        "0.1"
    );
    assert_eq!(
        response["result"]["payload"]["bootstrap"]["supported_public_contract_versions"][0],
        "0.1"
    );
    assert_eq!(
        response["result"]["payload"]["targets"],
        Value::Array(vec![])
    );
}

#[test]
fn malformed_json_is_a_transport_error_not_a_protocol_failure() {
    let output = run_adapter("{not-json}\n");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], Value::Null);
    assert_eq!(response["error"]["code"], -32700);
    assert!(response.get("result").is_none());
}

#[test]
fn invalid_validate_payload_stays_in_protocol_result_channel() {
    let request = concat!(
        "{\"jsonrpc\":\"2.0\",\"method\":\"validate_plan\",\"params\":",
        "{\"adapter_protocol_version\":\"0.1\"},\"id\":7}\n"
    );
    let output = run_adapter(request);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["id"], 7);
    assert_eq!(response["result"]["kind"], "protocol_failure");
    assert_eq!(
        response["result"]["payload"]["code"],
        "protocol.invalid_request"
    );
    assert!(response.get("error").is_none());
}
