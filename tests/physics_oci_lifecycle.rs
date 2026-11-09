#![cfg(unix)]
// This one-test integration harness owns its PATH and never needs a real Docker daemon.
// It checks successful cleanup and negative failure/timeout/orphan/mismatch paths.
use sol_adaptor_moose::backend::MooseProcessRunner;
use sol_adaptor_moose::physics_oci::{docker_command, OciContainerLease};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;
use std::time::Duration;

const MOCK_DOCKER: &str = r#"#!/usr/bin/env python3
import os
import pathlib
import sys
import time

a = sys.argv[1:]
root = pathlib.Path(os.environ["OCI_MOCK_STATE"])
mode = os.environ.get("OCI_MOCK_MODE", "normal")
if a[:2] == ["container", "ls"]:
    f = next(item for item in a if item.startswith("name=^/"))
    name = f[len("name=^/"):-1]
    p = root / (name + ".state")
    if p.exists():
        print(p.read_text())
    sys.exit(0)
if a[:2] == ["container", "rm"]:
    if mode == "rm_failure":
        print("simulated Docker cleanup failure", file=sys.stderr)
        sys.exit(17)
    name = a[-1]
    (root / (name + ".state")).unlink(missing_ok=True)
    sys.exit(0)
if a[0] == "run":
    name = a[a.index("--name") + 1]
    cidfile = pathlib.Path(a[a.index("--cidfile") + 1])
    state = root / (name + ".state")
    state.write_text("a" * 64)
    if mode == "wrong_cid":
        cidfile.write_text("b" * 64)
        sys.exit(2)
    if mode != "no_cid":
        cidfile.write_text("a" * 64)
    if mode == "normal":
        state.unlink()
        print("mock Physics runtime completed")
        sys.exit(0)
    time.sleep(30)
    sys.exit(99)
print("unsupported mock Docker invocation", file=sys.stderr)
sys.exit(98)
"#;

fn assert_no_mock_containers(root: &Path) {
    let names = fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|item| item.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("state"))
        .collect::<Vec<_>>();
    assert!(names.is_empty(), "mock Docker containers leaked: {names:?}");
}

#[test]
fn docker_lease_normal_timeout_missing_cid_and_cleanup_failure() {
    let state = std::env::temp_dir().join(format!("oci-lifecycle-mock-{}", std::process::id()));
    assert!(!state.exists(), "mock directory collision");
    fs::create_dir(&state).unwrap();
    let mock_dir = state.join("bin");
    fs::create_dir(&mock_dir).unwrap();
    let mock = mock_dir.join("docker");
    fs::write(&mock, MOCK_DOCKER).unwrap();
    fs::set_permissions(&mock, fs::Permissions::from_mode(0o755)).unwrap();

    let original_path = std::env::var_os("PATH").unwrap();
    let mut new_path = mock_dir.as_os_str().to_os_string();
    new_path.push(":");
    new_path.push(&original_path);
    std::env::set_var("PATH", &new_path);
    std::env::set_var("OCI_MOCK_STATE", &state);

    let lease = OciContainerLease::new().unwrap();
    let private_dir = lease.cid_file().parent().unwrap();
    assert_eq!(
        fs::metadata(private_dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let command = docker_command(
        &["--check-input"],
        None,
        lease.cid_file(),
        lease.container_name(),
    )
    .unwrap();
    let args = command
        .get_args()
        .map(|v| v.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(args.iter().any(|arg| arg == "--name"));
    assert!(args.iter().any(|arg| arg == lease.container_name()));
    lease.finalize().unwrap();

    std::env::set_var("OCI_MOCK_MODE", "normal");
    let runner = MooseProcessRunner::physics_oci(Duration::from_secs(3));
    let success = runner.run(&["--check-input"]).unwrap();
    assert!(success.success());
    assert_no_mock_containers(&state);

    // Docker's client exits in response to a timeout but the mock container
    // is still present. The runner must forcibly remove it and report error.
    std::env::set_var("OCI_MOCK_MODE", "timeout");
    let runner = MooseProcessRunner::physics_oci(Duration::from_millis(150));
    let error = runner.run(&["-i", "input.i"]).unwrap_err();
    assert!(
        error.to_string().contains("orphan OCI container"),
        "{error}"
    );
    assert_no_mock_containers(&state);

    // Docker never wrote a CID, but the private random name still identifies
    // the orphan; cleanup must occur and the outcome is still an error.
    std::env::set_var("OCI_MOCK_MODE", "no_cid");
    let error = runner.run(&["-i", "input.i"]).unwrap_err();
    assert!(
        error.to_string().contains("orphan OCI container"),
        "{error}"
    );
    assert_no_mock_containers(&state);

    // A cleanup failure must NOT be treated as successful or self-healing.
    std::env::set_var("OCI_MOCK_MODE", "rm_failure");
    let error = runner.run(&["-i", "input.i"]).unwrap_err();
    assert!(
        error.to_string().contains("Docker lifecycle") || error.to_string().contains("cleanup"),
        "{error}"
    );
    let mut leftovers = fs::read_dir(&state)
        .unwrap()
        .filter_map(Result::ok)
        .map(|item| item.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("state"))
        .collect::<Vec<_>>();
    assert_eq!(
        leftovers.len(),
        1,
        "failed cleanup must retain truthful evidence"
    );
    fs::remove_file(leftovers.pop().unwrap()).unwrap();

    // A forged CID is never trusted for docker kill/rm. Fail closed.
    std::env::set_var("OCI_MOCK_MODE", "wrong_cid");
    let error = runner.run(&["-i", "input.i"]).unwrap_err();
    assert!(error.to_string().contains("disagrees"), "{error}");
    let leftovers = fs::read_dir(&state)
        .unwrap()
        .filter_map(Result::ok)
        .map(|item| item.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("state"))
        .collect::<Vec<_>>();
    assert_eq!(leftovers.len(), 1);
    fs::remove_file(&leftovers[0]).unwrap();

    // A symlink in place of the CID must never be followed.
    let lease = OciContainerLease::new().unwrap();
    symlink("/etc/passwd", lease.cid_file()).unwrap();
    let dir = lease.cid_file().parent().unwrap().to_path_buf();
    let error = lease.finalize().unwrap_err();
    assert!(error.to_string().contains("regular file"), "{error}");
    fs::remove_dir_all(dir).unwrap();

    std::env::set_var("PATH", original_path);
    std::env::remove_var("OCI_MOCK_MODE");
    std::env::remove_var("OCI_MOCK_STATE");
    fs::remove_dir_all(state).unwrap();
}
