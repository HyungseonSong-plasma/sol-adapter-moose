use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Output, Stdio};

const THERMAL_REQUEST_V02: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");

fn run_adapter(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sol-adaptor-moose"))
        .env_remove("SOL_MOOSE_EXECUTABLE")
        .env_remove("SOL_MOOSE_WORKSPACE_ROOT")
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

fn plan_rpc(method: &str, params: Value, id: u64) -> String {
    format!(
        "{}\n",
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id
        }))
        .unwrap()
    )
}

#[test]
fn describe_adapter_emits_explicit_v01_v02_support_and_moose_target() {
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
    let payload = &response["result"]["payload"];
    assert_eq!(payload["bootstrap"]["adapter_id"], "sol.adapter.moose");
    assert_eq!(
        payload["bootstrap"]["supported_adapter_protocol_versions"],
        json!(["0.1", "0.2"])
    );
    assert_eq!(
        payload["bootstrap"]["supported_public_contract_versions"],
        json!(["0.1", "0.2"])
    );
    assert_eq!(payload["targets"].as_array().unwrap().len(), 1);
    assert_eq!(payload["targets"][0]["target"], "moose");
    assert_eq!(
        payload["targets"][0]["capabilities"][0]["capability"],
        "thermal.steady_conduction"
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
fn invalid_v01_validate_payload_stays_in_protocol_result_channel() {
    let request = plan_rpc(
        "validate_plan",
        json!({"adapter_protocol_version": "0.1"}),
        7,
    );
    let output = run_adapter(&request);
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

#[test]
fn explicit_v02_validate_dispatches_and_reports_backend_unavailable() {
    let params: Value = serde_json::from_str(THERMAL_REQUEST_V02).unwrap();
    let output = run_adapter(&plan_rpc("validate_plan", params, 8));
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["id"], 8);
    assert_eq!(response["result"]["kind"], "protocol_success");
    let payload = &response["result"]["payload"];
    assert_eq!(payload["adapter_protocol_version"], "0.2");
    assert_eq!(payload["target_compatible"], true);
    assert_eq!(payload["capabilities_satisfied"], true);
    assert_eq!(payload["preflight"], "unavailable");
    assert_eq!(
        payload["diagnostics"][0]["code"],
        "adapter.transient_unavailable"
    );
}

#[test]
fn explicit_v02_execute_dispatches_and_reports_backend_unavailable_without_side_effects() {
    let params: Value = serde_json::from_str(THERMAL_REQUEST_V02).unwrap();
    let output = run_adapter(&plan_rpc("execute_plan", params, 9));
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["id"], 9);
    assert_eq!(response["result"]["kind"], "protocol_success");
    let payload = &response["result"]["payload"];
    assert_eq!(payload["adapter_protocol_version"], "0.2");
    assert_eq!(payload["execution"], "unavailable");
    assert!(payload["execution_batches"].as_array().unwrap().is_empty());
    assert!(payload["effects"].as_array().unwrap().is_empty());
    assert!(payload["action_reports"]
        .as_array()
        .unwrap()
        .iter()
        .all(|report| report["state"] == "unavailable"));
}

#[test]
fn unsupported_protocol_version_is_a_compatibility_failure() {
    let output = run_adapter(&plan_rpc(
        "validate_plan",
        json!({"adapter_protocol_version": "9.9"}),
        10,
    ));
    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["result"]["kind"], "protocol_failure");
    assert_eq!(
        response["result"]["payload"]["code"],
        "protocol.unsupported_adapter_protocol"
    );
    assert_eq!(response["result"]["payload"]["side_effects"], "none");
}
