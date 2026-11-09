use serde_json::{json, Value};
use sol_adaptor_moose::physics_oci::{docker_command, IMAGE};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const REQUEST: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");

fn preflight_with_profile(profile: &str, conflicting_executable: Option<&str>) -> Value {
    let request: Value = serde_json::from_str(REQUEST).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_sol-adaptor-moose"));
    command
        .env("SOL_MOOSE_BACKEND_PROFILE", profile)
        .env_remove("SOL_MOOSE_CONDA_PREFIX")
        .env_remove("SOL_MOOSE_EXECUTABLE")
        .env_remove("SOL_MOOSE_WORKSPACE_ROOT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(executable) = conflicting_executable {
        command.env("SOL_MOOSE_EXECUTABLE", executable);
    }
    let mut child = command.spawn().unwrap();
    let frame = json!({
        "jsonrpc": "2.0",
        "method": "validate_plan",
        "params": request,
        "id": 1,
    });
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{}\n", frame).as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "transport process should complete");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn unknown_or_conflicting_backend_profile_rejects_before_execution() {
    let wrong = preflight_with_profile("unrecognised_backend", None);
    assert_eq!(wrong["result"]["kind"], "protocol_success");
    assert_eq!(wrong["result"]["payload"]["preflight"], "rejected");
    assert_eq!(wrong["result"]["payload"]["target_compatible"], false);

    let mixed = preflight_with_profile(
        "physics_oci_thermal_v1",
        Some("/tmp/fake-conda/bin/moose-opt"),
    );
    assert_eq!(mixed["result"]["kind"], "protocol_success");
    assert_eq!(mixed["result"]["payload"]["preflight"], "rejected");
    assert_eq!(mixed["result"]["payload"]["target_compatible"], false);
}

#[test]
fn docker_command_has_exact_no_pull_no_network_no_shell_contract() {
    let command = docker_command(
        &["--check-input", "-i", "input.i"],
        None,
        Path::new("/tmp/bounded-oci-cid-test"),
        "sol-adaptor-moose-oci-00000000000000000000000000000000",
    )
    .unwrap();
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert_eq!(command.get_program(), "docker");
    for flag in [
        "run",
        "--rm",
        "--pull=never",
        "--network=none",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
    ] {
        assert!(args.iter().any(|arg| arg == flag), "missing {flag}");
    }
    assert!(args.iter().any(|arg| arg == IMAGE));
    assert_eq!(args.last().unwrap(), "input.i");
    assert!(!args.iter().any(|arg| arg == "/bin/sh"));
}
