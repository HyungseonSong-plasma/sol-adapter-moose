//! Candidate official INL MOOSE installed OCI profile for AP/PC 0.2 Thermal.
//! Distinct backend provenance: OpenMPI, image SHA, and MOOSE source.
//! NOT a Conda moose=2026.08.12=mpich replacement or a new SOL capability.

use crate::backend::BackendError;
use crate::physics_oci;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const PROFILE: &str = "official_moose_oci_thermal_v1";
pub const IMAGE: &str =
    "idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452";
pub const CONFIG_ID: &str =
    "sha256:8e2d6d65ef05e3a7deb15fe29d5475c95f67a50b9d5dd265b74fa435d382ad96";
pub const MOOSE_SOURCE_SHA: &str = "167ee97da204df5e8643695e18b9b28910f0014b";
pub const MOOSE_VERSION: &str = "167ee97";
pub const EXECUTABLE: &str = "/opt/moose/bin/moose-opt";

fn field<'a>(value: &'a Value, segments: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in segments {
        cursor = cursor.get(*segment)?;
    }
    cursor.as_str()
}

/// Require the locally pulled, historically observed exact OCI image.
/// Never fetch, run a container, or allocate an Adapter workspace in readiness.
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
            "pinned official MOOSE OCI image is unavailable; pull exact digest explicitly: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let documents: Vec<Value> = serde_json::from_slice(&output.stdout).map_err(|error| {
        BackendError::InvalidMetadata(format!("official OCI image inspect JSON: {error}"))
    })?;
    let image = match documents.as_slice() {
        [image] => image,
        _ => {
            return Err(BackendError::InvalidMetadata(
                "expected exactly one official OCI image".to_owned(),
            ))
        }
    };
    let digests = image
        .get("RepoDigests")
        .and_then(Value::as_array)
        .ok_or_else(|| BackendError::InvalidMetadata("missing image RepoDigests".to_owned()))?;
    if !digests.iter().any(|digest| digest.as_str() == Some(IMAGE)) {
        return Err(BackendError::InvalidMetadata(
            "official image does not retain exact repository manifest digest".to_owned(),
        ));
    }
    let required = [
        (&["Id"][..], CONFIG_ID),
        (&["Os"][..], "linux"),
        (&["Architecture"][..], "amd64"),
        (
            &["Config", "Labels", "moose-openmpi-x86_64.moose.sha"][..],
            MOOSE_SOURCE_SHA,
        ),
        (
            &["Config", "Labels", "moose-openmpi-x86_64.version"][..],
            MOOSE_VERSION,
        ),
        (
            &["Config", "Labels", "org.label-schema.build-arch"][..],
            "amd64",
        ),
        (
            &["Config", "Labels", "moose-dev-openmpi-x86_64.version"][..],
            "2026.07.30",
        ),
    ];
    for (path, expected) in required {
        let actual = field(image, path).unwrap_or("<missing>");
        if actual != expected {
            return Err(BackendError::InvalidMetadata(format!(
                "official OCI identity {:?} mismatch: expected {}, observed {}",
                path, expected, actual
            )));
        }
    }
    // Actual pinned OCI manifest observed in CI: a login Bash entrypoint,
    // not the Physics image direct executable ENTRYPOINT.
    let entrypoint = image
        .pointer("/Config/Entrypoint")
        .and_then(Value::as_array)
        .ok_or_else(|| BackendError::InvalidMetadata("missing OCI entrypoint".to_owned()))?;
    let expected = ["/bin/bash", "-l", "-c"];
    if entrypoint.len() != expected.len()
        || entrypoint
            .iter()
            .zip(expected)
            .any(|(actual, expected)| actual.as_str() != Some(expected))
    {
        return Err(BackendError::InvalidMetadata(
            "unexpected official OCI shell entrypoint".to_owned(),
        ));
    }
    Ok(CONFIG_ID.to_owned())
}

/// Reuse the hardened mount, no-network, capability-drop, and CID contract.
/// The Bash script is a fixed literal; external MOOSE args are positional only.
pub fn docker_command(
    args: &[&str],
    working_dir: Option<&Path>,
    cid_file: &Path,
) -> Result<Command, BackendError> {
    let mut command = physics_oci::docker_base_command(working_dir, cid_file)?;
    command.args(["--entrypoint", "/bin/bash"]);
    command.arg(IMAGE);
    command.args([
        "-lc",
        "set -eo pipefail; if [[ -f /environment ]]; then source /environment; fi; set -u; exec moose-opt \"$@\"",
        "bash",
    ]);
    command.args(args);
    Ok(command)
}
