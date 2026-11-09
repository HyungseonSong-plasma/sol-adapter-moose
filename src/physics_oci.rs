//! Explicit Physics OCI runtime profile for bounded AP/PC 0.2 Thermal execution.
//!
//! This is **not** the Conda moose=2026.08.12=mpich identity. It does not
//! advertise a new SOL capability or imply equivalence with the Conda release.

use crate::backend::BackendError;
use serde_json::Value;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const IMAGE: &str = "ghcr.io/hyungseonsong-plasma/physics-runtime@sha256:cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c";
pub const BUILD_BASE: &str = "ghcr.io/hyungseonsong-plasma/physics-build-base@sha256:2c3352da4f5c1b2ae98475fc01463ad0cdbc6ce7d221ce55055ddea152182008";
pub const MOOSE_SOURCE: &str = "9f388366ccf38b9c34542ec5561198249fde0ac9";
pub const EXECUTABLE: &str = "/opt/physics/physics-opt";
pub const PROFILE: &str = "physics_oci_thermal_v1";

fn image_field<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

/// No pull, no network, no container creation. Check the *local* image digest
/// and source provenance before any generated workspace or backend side effect.
pub fn verify_local_image() -> Result<String, BackendError> {
    let output = Command::new("docker")
        .args(["image", "inspect", IMAGE])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| BackendError::Spawn {
            executable: PathBuf::from("docker"),
            detail: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(BackendError::Process(format!(
            "pinned Physics OCI image is not locally installed; explicitly pull exact digest before running: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let decoded: Vec<Value> = serde_json::from_slice(&output.stdout).map_err(|error| {
        BackendError::InvalidMetadata(format!("docker image inspect JSON: {error}"))
    })?;
    let image = match decoded.as_slice() {
        [image] => image,
        _ => {
            return Err(BackendError::InvalidMetadata(
                "docker image inspect must identify exactly one local image".to_owned(),
            ))
        }
    };
    let digests = image
        .get("RepoDigests")
        .and_then(Value::as_array)
        .ok_or_else(|| BackendError::InvalidMetadata("missing RepoDigests".to_owned()))?;
    if !digests.iter().any(|digest| digest.as_str() == Some(IMAGE)) {
        return Err(BackendError::InvalidMetadata(
            "local OCI image lacks the exact repository digest".to_owned(),
        ));
    }
    let required = [
        (
            &["Config", "Labels", "io.physics.moose.sha"][..],
            MOOSE_SOURCE,
        ),
        (
            &["Config", "Labels", "io.physics.build-base.ref"][..],
            BUILD_BASE,
        ),
        (
            &["Config", "Labels", "io.physics.source.owner"][..],
            "moose-test-repo/physics_app",
        ),
        (&["Os"][..], "linux"),
        (&["Architecture"][..], "amd64"),
    ];
    for (path, expected) in required {
        let actual = image_field(image, path).unwrap_or("<missing>");
        if actual != expected {
            return Err(BackendError::InvalidMetadata(format!(
                "Physics OCI identity {:?} mismatch: expected {expected}, got {actual}",
                path
            )));
        }
    }
    let entrypoint = image
        .pointer("/Config/Entrypoint/0")
        .and_then(Value::as_str)
        .unwrap_or("<missing>");
    if entrypoint != EXECUTABLE {
        return Err(BackendError::InvalidMetadata(format!(
            "Physics OCI executable mismatch: expected {EXECUTABLE}, got {entrypoint}"
        )));
    }
    let id = image_field(image, &["Id"]).unwrap_or("");
    if !id.starts_with("sha256:") || id.len() != 71 {
        return Err(BackendError::InvalidMetadata(
            "Physics OCI image missing validated local content ID".to_owned(),
        ));
    }
    Ok(id.to_owned())
}

/// Docker is only a transport. Backend args are passed directly to the
/// immutable image ENTRYPOINT; no shell, user-supplied image or host network.
pub fn docker_command(
    args: &[&str],
    work_dir: Option<&Path>,
    cid_file: &Path,
) -> Result<Command, BackendError> {
    let mut command = Command::new("docker");
    command.args([
        "run",
        "--rm",
        "--pull=never",
        "--network=none",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
        "--pids-limit=128",
        "--cidfile",
    ]);
    command.arg(cid_file);
    if let Some(work_dir) = work_dir {
        let real = work_dir.canonicalize().map_err(|error| BackendError::Io {
            context: format!("canonicalize OCI workspace {}", work_dir.display()),
            detail: error.to_string(),
        })?;
        if !real.is_dir() {
            return Err(BackendError::Process(
                "Physics OCI workspace mount is not a directory".to_owned(),
            ));
        }
        let path = real.to_str().ok_or_else(|| {
            BackendError::Process("Physics OCI workspace path is not UTF-8".to_owned())
        })?;
        if path.contains(',') || path.contains(':') || path.contains('\n') {
            return Err(BackendError::Process(
                "Physics OCI workspace path contains an unsupported mount character".to_owned(),
            ));
        }
        // The OCI runtime drops ALL capabilities. Its default root identity
        // cannot open a runner-owned mode-0755 bind mount for CSV/Exodus output.
        // Select the host directory owner instead of granting DAC_OVERRIDE.
        #[cfg(unix)]
        {
            let owner = fs::metadata(&real).map_err(|error| BackendError::Io {
                context: format!("read OCI workspace ownership {}", real.display()),
                detail: error.to_string(),
            })?;
            command.arg("--user").arg(format!("{}:{}", owner.uid(), owner.gid()));
        }
        #[cfg(not(unix))]
        return Err(BackendError::Process(
            "Physics OCI workspace execution requires a verified Unix host UID/GID".to_owned(),
        ));
        command
            .arg("--mount")
            .arg(format!("type=bind,src={path},dst=/work"))
            .args(["--workdir", "/work"]);
    }
    command.arg(IMAGE);
    command.args(args);
    Ok(command)
}

/// A killed Docker *client* does not guarantee a killed container. On timeout
/// explicitly kill the recorded container; uncertain cleanup is a hard failure.
pub fn terminate_container(cid_file: &Path) -> Result<(), BackendError> {
    let id = fs::read_to_string(cid_file).map_err(|error| {
        BackendError::Process(format!(
            "cannot establish Physics OCI container ID on timeout: {error}"
        ))
    })?;
    let id = id.trim();
    if id.len() != 64 || !id.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(BackendError::Process(
            "invalid Physics OCI container ID on timeout".to_owned(),
        ));
    }
    let output = Command::new("docker")
        .args(["kill", id])
        .output()
        .map_err(|error| BackendError::Process(format!("OCI timeout kill: {error}")))?;
    if !output.status.success() {
        return Err(BackendError::Process(format!(
            "OCI timeout kill did not confirm termination: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}
