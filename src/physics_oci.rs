//! Explicit Physics OCI runtime profile for bounded AP/PC 0.2 Thermal execution.
//!
//! This is **not** the Conda moose=2026.08.12=mpich identity. It does not
//! advertise a new SOL capability or imply equivalence with the Conda release.

use crate::backend::BackendError;
use serde_json::Value;
use std::fs;
use std::io::{ErrorKind, Read};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
    container_name: &str,
) -> Result<Command, BackendError> {
    if !valid_container_name(container_name) {
        return Err(BackendError::Process(
            "invalid private Physics OCI container name".to_owned(),
        ));
    }
    let mut command = Command::new("docker");
    command.args([
        "run",
        "--rm",
        "--pull=never",
        "--network=none",
        "--cap-drop=ALL",
        "--security-opt=no-new-privileges",
        "--pids-limit=128",
        "--tmpfs",
        "/tmp:rw,nosuid,nodev,mode=1777",
        "--cidfile",
    ]);
    command.arg(cid_file);
    command.args(["--name", container_name]);
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
            command
                .arg("--user")
                .arg(format!("{}:{}", owner.uid(), owner.gid()));
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

// A 128-bit, OS-random container name and an atomically created mode-0700
// directory are needed for both CID-file security and orphan cleanup.
// PID+counter filenames in a world-traversable tmp directory are insufficient.
const OCI_CONTAINER_PREFIX: &str = "sol-adaptor-moose-oci-";
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(10);

fn valid_container_name(name: &str) -> bool {
    name.starts_with(OCI_CONTAINER_PREFIX)
        && name.len() == OCI_CONTAINER_PREFIX.len() + 32
        && name[OCI_CONTAINER_PREFIX.len()..]
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

pub struct OciContainerLease {
    private_dir: PathBuf,
    cid_file: PathBuf,
    container_name: String,
}

impl OciContainerLease {
    pub fn new() -> Result<Self, BackendError> {
        #[cfg(not(unix))]
        {
            return Err(BackendError::Process(
                "Physics OCI lifecycle requires a Unix host".to_owned(),
            ));
        }
        #[cfg(unix)]
        {
            let mut random = [0u8; 16];
            for _attempt in 0..4 {
                fs::File::open("/dev/urandom")
                    .and_then(|mut f| f.read_exact(&mut random))
                    .map_err(|e| BackendError::Io {
                        context: "get kernel randomness for private OCI container".to_owned(),
                        detail: e.to_string(),
                    })?;
                let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
                let container_name = format!("{OCI_CONTAINER_PREFIX}{nonce}");
                let dir = std::env::temp_dir().join(&container_name);
                let result = fs::DirBuilder::new().mode(0o700).create(&dir);
                match result {
                    Ok(()) => {
                        return Ok(Self {
                            cid_file: dir.join("container.cid"),
                            private_dir: dir,
                            container_name,
                        })
                    }
                    Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                    Err(e) => {
                        return Err(BackendError::Io {
                            context: format!("create private OCI directory {}", dir.display()),
                            detail: e.to_string(),
                        })
                    }
                }
            }
            Err(BackendError::Process(
                "cannot allocate collision-free private OCI container directory".to_owned(),
            ))
        }
    }

    pub fn cid_file(&self) -> &Path {
        &self.cid_file
    }

    pub fn container_name(&self) -> &str {
        &self.container_name
    }

    fn read_cid(&self) -> Result<Option<String>, BackendError> {
        let metadata = match fs::symlink_metadata(&self.cid_file) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(BackendError::Process(format!(
                    "inspect private Docker CID file: {error}"
                )))
            }
        };
        if !metadata.file_type().is_file() {
            return Err(BackendError::Process(
                "Docker CID file must be a regular file, not a symlink".to_owned(),
            ));
        }
        let cid = fs::read_to_string(&self.cid_file)
            .map_err(|e| BackendError::Process(format!("read private Docker CID: {e}")))?;
        let cid = cid.trim();
        if cid.len() != 64 || !cid.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(BackendError::Process(
                "Docker CID is not a 64-digit hex container identity".to_owned(),
            ));
        }
        Ok(Some(cid.to_ascii_lowercase()))
    }

    fn docker_lifecycle(&self, args: &[&str]) -> Result<String, BackendError> {
        let mut child = Command::new("docker")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| BackendError::Process(format!("spawn OCI lifecycle command: {e}")))?;
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let output = child.wait_with_output().map_err(|e| {
                        BackendError::Process(format!("read OCI lifecycle status: {e}"))
                    })?;
                    if !status.success() {
                        return Err(BackendError::Process(format!(
                            "Docker lifecycle {:?} failed: {}",
                            args,
                            String::from_utf8_lossy(&output.stderr)
                        )));
                    }
                    return String::from_utf8(output.stdout).map_err(|e| {
                        BackendError::Process(format!("invalid Docker lifecycle output: {e}"))
                    });
                }
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(BackendError::Process(
                        "Docker lifecycle/cleanup command timed out; container state UNKNOWN"
                            .to_owned(),
                    ));
                }
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(BackendError::Process(format!(
                        "cannot check Docker lifecycle process: {e}"
                    )));
                }
            }
        }
    }

    fn listed_cid(&self) -> Result<Option<String>, BackendError> {
        let filter = format!("name=^/{}$", self.container_name);
        let stdout = self.docker_lifecycle(&[
            "container", "ls", "--all", "--no-trunc", "--filter", &filter, "--format",
            "{{.ID}}",
        ])?;
        let ids = stdout.lines().filter(|x| !x.trim().is_empty()).collect::<Vec<_>>();
        match ids.as_slice() {
            [] => Ok(None),
            [id] if id.len() == 64 && id.chars().all(|c| c.is_ascii_hexdigit()) => {
                Ok(Some(id.to_ascii_lowercase()))
            }
            _ => Err(BackendError::Process(
                "ambiguous Docker container identity for private name".to_owned(),
            )),
        }
    }

    /// Called after the Docker CLI has exited or been killed. A missing CID
    /// is not grounds to ignore a container: the cryptographically random
    /// Docker name permits a name-based orphan search as a second witness.
    /// Any unexpected leftover becomes an error even when removal succeeds.
    pub fn finalize(self) -> Result<(), BackendError> {
        let recorded = self.read_cid()?;
        let existing = self.listed_cid()?;
        if let (Some(recorded), Some(existing)) = (&recorded, &existing) {
            if recorded != existing {
                return Err(BackendError::Process(
                    "Docker CID file disagrees with private Docker container name".to_owned(),
                ));
            }
        }
        if existing.is_some() {
            self.docker_lifecycle(&["container", "rm", "--force", &self.container_name])?;
            if self.listed_cid()?.is_some() {
                return Err(BackendError::Process(
                    "orphan OCI container still present after forced removal".to_owned(),
                ));
            }
            return Err(BackendError::Process(
                "orphan OCI container detected after Docker client exit; forcibly removed"
                    .to_owned(),
            ));
        }
        // No residual container; the temporary CID may be removed safely.
        fs::remove_dir_all(&self.private_dir).map_err(|e| BackendError::Io {
            context: format!("remove private OCI CID directory {}", self.private_dir.display()),
            detail: e.to_string(),
        })?;
        Ok(())
    }
}

