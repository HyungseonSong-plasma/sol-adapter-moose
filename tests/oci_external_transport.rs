//! Experimental real subprocess/stdio JSON-RPC consumer test.
//! Not a claim that an arbitrary OCI executable is securely bound to its digest.

use serde_json::{json, Value};
use sol_adapter_protocol::{
    ExecutePlanRequestV02, ExecutePlanResponseV02, ExecutionOutcome, PreflightOutcome,
    ValidatePlanResponseV02,
};
use sol_adaptor_moose::oci_profile::APPROVED_CANDIDATE_IMAGE;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const REQUEST: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");

fn rpc(name: &str, params: Value, id: u64) -> String {
    serde_json::to_string(&json!({
        "jsonrpc":"2.0", "id":id, "method":name, "params":params
    }))
    .unwrap()
        + "\n"
}

fn invoke(
    frames: &[String],
    root: &PathBuf,
    executable: &std::ffi::OsStr,
    image: &str,
    docker_pin: &str,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sol-adaptor-moose"));
    command
        .env_remove("SOL_MOOSE_BACKEND")
        .env_remove("SOL_MOOSE_OCI_CANDIDATE_IMAGE")
        .env("SOL_MOOSE_OCI_EXECUTABLE", executable)
        .env("SOL_MOOSE_WORKSPACE_ROOT", root)
        .env("MOOSE_DOCKER_PIN", docker_pin)
        .env_remove("SOL_MOOSE_EXECUTABLE")
        .env_remove("SOL_MOOSE_CONDA_PREFIX")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !image.is_empty() {
        command.env("SOL_MOOSE_OCI_CANDIDATE_IMAGE", image);
    }
    let mut child = command.spawn().expect("start real Adapter transport");
    {
        let input = child.stdin.as_mut().expect("stdin pipe");
        for frame in frames {
            input
                .write_all(frame.as_bytes())
                .expect("write request frame");
        }
    }
    drop(child.stdin.take());
    child
        .wait_with_output()
        .expect("wait for adapter subprocess")
}

fn parse(output: Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "transport exited nonzero: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "protocol adapter emitted stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect()
}

fn payload(value: &Value) -> &Value {
    assert_eq!(value["result"]["kind"], "protocol_success");
    &value["result"]["payload"]
}

#[test]
fn external_oci_protocol_v02_serialization_and_independent_runs() {
    let Some(executable) = std::env::var_os("SOL_MOOSE_OCI_EXECUTABLE") else {
        eprintln!("experimental external OCI process test is not enabled");
        return;
    };
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/oci-external-transport-evidence");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let raw: Value = serde_json::from_str(REQUEST).unwrap();
    let frames = [
        rpc("validate_plan", raw.clone(), 71),
        rpc("execute_plan", raw.clone(), 72),
        rpc("execute_plan", raw, 73),
    ];
    let observations = parse(invoke(
        &frames,
        &root,
        executable.as_os_str(),
        "", // OCI is the consumer's default; no backend or image selection is set.
        APPROVED_CANDIDATE_IMAGE,
    ));
    assert_eq!(observations.len(), 3);
    let validate = ValidatePlanResponseV02::from_json(
        &serde_json::to_string(payload(&observations[0])).unwrap(),
    )
    .unwrap();
    assert_eq!(validate.preflight, PreflightOutcome::Accepted);
    let request = ExecutePlanRequestV02::from_json(REQUEST).unwrap();
    let mut seen_run_keys = Vec::new();
    let mut observed_input = None;
    for (index, response) in observations.iter().enumerate().skip(1) {
        assert_eq!(response["id"], json!((index as u64) + 71));
        let mut result =
            ExecutePlanResponseV02::from_json(&serde_json::to_string(payload(response)).unwrap())
                .unwrap();
        result.validate_against(&request).unwrap();
        assert_eq!(result.execution, ExecutionOutcome::Completed);
        let provenance = result.provenance.expect("provenance");
        assert!(provenance.opaque_references.iter().any(|x| {
            x.namespace == "moose.oci.image" && x.reference == APPROVED_CANDIDATE_IMAGE
        }));
        assert!(!provenance
            .opaque_references
            .iter()
            .any(|x| x.namespace == "moose.package"));
        let run_key = provenance
            .opaque_references
            .iter()
            .find(|x| x.namespace == "moose.workspace")
            .unwrap()
            .reference
            .clone();
        let run_dir = root.join("runs").join(&run_key);
        for name in [
            "input.i",
            "check.stdout.log",
            "check.stderr.log",
            "stdout.log",
            "stderr.log",
        ] {
            assert!(
                run_dir.join(name).is_file(),
                "missing {name} at {}",
                run_dir.display()
            );
        }
        let input = fs::read(run_dir.join("input.i")).unwrap();
        if let Some(previous) = &observed_input {
            assert_eq!(&input, previous, "replay changed the canonical input bytes");
        } else {
            observed_input = Some(input);
        }
        seen_run_keys.push(run_key);
    }
    assert_ne!(
        seen_run_keys[0], seen_run_keys[1],
        "two runs must not reuse a workspace"
    );
    fs::write(
        root.join("external-protocol-report.json"),
        serde_json::to_vec_pretty(&json!({
            "classification": "EXPERIMENTAL_EXTERNAL_TRANSPORT_RUNS_PASS",
            "image": APPROVED_CANDIDATE_IMAGE,
            "distinct_run_keys": seen_run_keys,
            "request_count": frames.len(),
            "conda_package_parity": "NOT_EVALUATED",
            "identity_of_supplied_executable": "TRUSTED_CI_WRAPPER_ONLY"
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn default_oci_without_launcher_preserves_describe_and_fails_closed_on_execute() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/oci-default-without-launcher");
    let _ = fs::remove_dir_all(&root);
    let mut command = Command::new(env!("CARGO_BIN_EXE_sol-adaptor-moose"));
    command
        .env_remove("SOL_MOOSE_BACKEND")
        .env_remove("SOL_MOOSE_OCI_EXECUTABLE")
        .env_remove("SOL_MOOSE_EXECUTABLE")
        .env_remove("MOOSE_DOCKER_PIN")
        .env("SOL_MOOSE_WORKSPACE_ROOT", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(rpc("execute_plan", serde_json::from_str(REQUEST).unwrap(), 99).as_bytes())
            .unwrap();
    }
    drop(child.stdin.take());
    let response = parse(child.wait_with_output().unwrap());
    assert_eq!(response.len(), 1);
    assert_eq!(payload(&response[0])["execution"], "unavailable");
    assert!(
        response[0].to_string().contains("SOL_MOOSE_OCI_EXECUTABLE"),
        "missing launcher diagnostics must identify the selected OCI backend"
    );
    assert!(
        !root.exists(),
        "missing OCI launcher must not create a workspace"
    );
}

#[test]
fn explicit_conda_selection_does_not_fall_through_to_oci() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/oci-conda-explicit");
    let _ = fs::remove_dir_all(&root);
    let mut command = Command::new(env!("CARGO_BIN_EXE_sol-adaptor-moose"));
    command
        .env("SOL_MOOSE_BACKEND", "conda")
        .env_remove("SOL_MOOSE_EXECUTABLE")
        .env_remove("SOL_MOOSE_CONDA_PREFIX")
        .env("SOL_MOOSE_OCI_EXECUTABLE", "/usr/bin/false")
        .env("SOL_MOOSE_WORKSPACE_ROOT", &root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(rpc("execute_plan", serde_json::from_str(REQUEST).unwrap(), 101).as_bytes())
        .unwrap();
    drop(child.stdin.take());
    let frames = parse(child.wait_with_output().unwrap());
    assert_eq!(frames.len(), 1);
    assert_eq!(payload(&frames[0])["execution"], "unavailable");
    assert!(!root.exists());
}

#[test]
fn unrecognized_backend_setting_rejected_before_json_rpc_dispatch() {
    let output = Command::new(env!("CARGO_BIN_EXE_sol-adaptor-moose"))
        .env("SOL_MOOSE_BACKEND", "automatic")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected oci or conda"));
}

#[test]
fn external_oci_rejects_failed_probe_without_workspace_creation() {
    let Some(_) = std::env::var_os("SOL_MOOSE_OCI_EXECUTABLE") else {
        return;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/oci-external-failed-probe");
    let _ = fs::remove_dir_all(&root);
    let output = invoke(
        &[rpc(
            "execute_plan",
            serde_json::from_str(REQUEST).unwrap(),
            81,
        )],
        &root,
        std::ffi::OsStr::new("/usr/bin/false"),
        APPROVED_CANDIDATE_IMAGE,
        APPROVED_CANDIDATE_IMAGE,
    );
    let frames = parse(output);
    assert_eq!(frames.len(), 1);
    let response = &frames[0];
    let mut result =
        ExecutePlanResponseV02::from_json(&serde_json::to_string(payload(response)).unwrap())
            .unwrap();
    let request = ExecutePlanRequestV02::from_json(REQUEST).unwrap();
    result.validate_against(&request).unwrap();
    assert_eq!(result.execution, ExecutionOutcome::Rejected);
    assert!(result.provenance.is_none());
    assert!(!root.exists(), "failed probe created workspace");
}

#[test]
fn external_oci_launch_rejects_digest_mismatch_before_any_side_effects() {
    let Some(executable) = std::env::var_os("SOL_MOOSE_OCI_EXECUTABLE") else {
        return;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/oci-external-transport-mismatched-identity");
    let _ = fs::remove_dir_all(&root);
    let output = invoke(
        &[rpc(
            "execute_plan",
            serde_json::from_str(REQUEST).unwrap(),
            91,
        )],
        &root,
        executable.as_os_str(),
        APPROVED_CANDIDATE_IMAGE,
        "idaholab/moose:latest",
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("pin disagree"));
    assert!(
        !root.exists(),
        "identity rejection must be side-effect free"
    );
}
