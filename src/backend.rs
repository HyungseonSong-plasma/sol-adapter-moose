use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const PHASE1_MOOSE_PACKAGE_NAME: &str = "moose";
pub const PHASE1_MOOSE_PACKAGE_VERSION: &str = "2026.08.12";
pub const PHASE1_MOOSE_PACKAGE_BUILD: &str = "mpich";
pub const PHASE1_MOOSE_PACKAGE_SUBDIR: &str = "linux-64";
pub const PHASE1_MOOSE_PACKAGE_FILENAME: &str = "moose-2026.08.12-mpich.conda";
pub const PHASE1_MOOSE_PACKAGE_SHA256: &str =
    "d7b2eec6b958f11455c903b70a16ea319b7edb68b4499537313c06785abd05b3";
pub const PHASE1_REQUIRED_COPYABLE_INPUT: &str = "heat_transfer";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MooseCondaIdentity {
    pub name: String,
    pub version: String,
    pub build: String,
    pub subdir: Option<String>,
    pub filename: Option<String>,
    pub sha256: Option<String>,
    pub channel: Option<String>,
    pub url: Option<String>,
}

impl MooseCondaIdentity {
    pub fn load_from_prefix(prefix: &Path) -> Result<Self, BackendError> {
        let metadata_dir = prefix.join("conda-meta");
        let entries = fs::read_dir(&metadata_dir).map_err(|error| BackendError::Io {
            context: format!("read {}", metadata_dir.display()),
            detail: error.to_string(),
        })?;

        let mut matches = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| BackendError::Io {
                context: format!("read entry in {}", metadata_dir.display()),
                detail: error.to_string(),
            })?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let content = fs::read_to_string(&path).map_err(|error| BackendError::Io {
                context: format!("read {}", path.display()),
                detail: error.to_string(),
            })?;
            let value: Value = serde_json::from_str(&content).map_err(|error| {
                BackendError::InvalidMetadata(format!("{}: {error}", path.display()))
            })?;
            if value.get("name").and_then(Value::as_str) == Some(PHASE1_MOOSE_PACKAGE_NAME) {
                matches.push(Self::from_conda_value(&value)?);
            }
        }

        match matches.len() {
            1 => Ok(matches.remove(0)),
            0 => Err(BackendError::InvalidMetadata(format!(
                "no `{PHASE1_MOOSE_PACKAGE_NAME}` Conda record in {}",
                metadata_dir.display()
            ))),
            count => Err(BackendError::InvalidMetadata(format!(
                "found {count} `{PHASE1_MOOSE_PACKAGE_NAME}` Conda records in {}",
                metadata_dir.display()
            ))),
        }
    }

    fn from_conda_value(value: &Value) -> Result<Self, BackendError> {
        let required = |field: &'static str| {
            value
                .get(field)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| BackendError::InvalidMetadata(format!("missing string `{field}`")))
        };
        let optional =
            |field: &'static str| value.get(field).and_then(Value::as_str).map(str::to_owned);

        Ok(Self {
            name: required("name")?,
            version: required("version")?,
            build: required("build")?,
            subdir: optional("subdir"),
            filename: optional("fn"),
            sha256: optional("sha256"),
            channel: optional("channel"),
            url: optional("url"),
        })
    }

    pub fn validate_phase1_policy(&self) -> Result<(), BackendError> {
        require_equal("package name", &self.name, PHASE1_MOOSE_PACKAGE_NAME)?;
        require_equal(
            "package version",
            &self.version,
            PHASE1_MOOSE_PACKAGE_VERSION,
        )?;
        require_equal("package build", &self.build, PHASE1_MOOSE_PACKAGE_BUILD)?;
        if let Some(subdir) = &self.subdir {
            require_equal("package subdir", subdir, PHASE1_MOOSE_PACKAGE_SUBDIR)?;
        }
        if let Some(filename) = &self.filename {
            require_equal("package filename", filename, PHASE1_MOOSE_PACKAGE_FILENAME)?;
        }
        if let Some(sha256) = &self.sha256 {
            require_equal("package sha256", sha256, PHASE1_MOOSE_PACKAGE_SHA256)?;
        }
        Ok(())
    }
}

fn require_equal(label: &'static str, actual: &str, expected: &str) -> Result<(), BackendError> {
    if actual == expected {
        Ok(())
    } else {
        Err(BackendError::IdentityMismatch {
            label,
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

impl ProcessOutput {
    pub fn success(&self) -> bool {
        !self.timed_out && self.status_code == Some(0)
    }

    pub fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

#[derive(Debug, Clone)]
pub struct MooseProcessRunner {
    executable: PathBuf,
    timeout: Duration,
}

impl MooseProcessRunner {
    pub fn new(executable: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            executable: executable.into(),
            timeout,
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn inferred_conda_prefix(&self) -> Option<PathBuf> {
        let bin = self.executable.parent()?;
        if bin.file_name().and_then(|name| name.to_str()) != Some("bin") {
            return None;
        }
        bin.parent().map(Path::to_path_buf)
    }

    pub fn run(&self, args: &[&str]) -> Result<ProcessOutput, BackendError> {
        self.run_command(args, None)
    }

    pub fn run_in_dir(
        &self,
        args: &[&str],
        working_dir: &Path,
    ) -> Result<ProcessOutput, BackendError> {
        self.run_command(args, Some(working_dir))
    }

    fn run_command(
        &self,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<ProcessOutput, BackendError> {
        let mut command = Command::new(&self.executable);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(working_dir) = working_dir {
            command.current_dir(working_dir);
        }

        let mut child = command.spawn().map_err(|error| BackendError::Spawn {
            executable: self.executable.clone(),
            detail: error.to_string(),
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| BackendError::Process("stdout pipe unavailable".to_owned()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| BackendError::Process("stderr pipe unavailable".to_owned()))?;
        let stdout_reader = thread::spawn(move || read_all(stdout));
        let stderr_reader = thread::spawn(move || read_all(stderr));

        let deadline = Instant::now() + self.timeout;
        let (status, timed_out) = loop {
            match child
                .try_wait()
                .map_err(|error| BackendError::Process(error.to_string()))?
            {
                Some(status) => break (status, false),
                None if Instant::now() >= deadline => {
                    child.kill().map_err(|error| {
                        BackendError::Process(format!("kill timed-out child: {error}"))
                    })?;
                    let status = child.wait().map_err(|error| {
                        BackendError::Process(format!("wait after timeout: {error}"))
                    })?;
                    break (status, true);
                }
                None => thread::sleep(Duration::from_millis(20)),
            }
        };

        let stdout = join_reader(stdout_reader, "stdout")?;
        let stderr = join_reader(stderr_reader, "stderr")?;
        Ok(ProcessOutput {
            status_code: exit_code(status),
            stdout,
            stderr,
            timed_out,
        })
    }
}

fn read_all(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_reader(
    handle: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stream: &'static str,
) -> Result<Vec<u8>, BackendError> {
    handle
        .join()
        .map_err(|_| BackendError::Process(format!("{stream} reader thread panicked")))?
        .map_err(|error| BackendError::Process(format!("read {stream}: {error}")))
}

fn exit_code(status: ExitStatus) -> Option<i32> {
    status.code()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MooseProbeEvidence {
    pub identity: MooseCondaIdentity,
    pub version_output: String,
    pub application_type: String,
    pub copyable_inputs: String,
    pub help_output: String,
}

pub fn probe_exact_phase1_target(
    runner: &MooseProcessRunner,
    conda_prefix: &Path,
) -> Result<MooseProbeEvidence, BackendError> {
    let identity = MooseCondaIdentity::load_from_prefix(conda_prefix)?;
    identity.validate_phase1_policy()?;

    let version_output = require_success_text(runner.run(&["--version"])?, "--version")?;
    let application_type = require_success_text(runner.run(&["--show-type"])?, "--show-type")?;
    let copyable_inputs = require_success_text(
        runner.run(&["--show-copyable-inputs"])?,
        "--show-copyable-inputs",
    )?;
    if !copyable_inputs
        .split_whitespace()
        .any(|entry| entry == PHASE1_REQUIRED_COPYABLE_INPUT)
    {
        return Err(BackendError::Probe(format!(
            "`--show-copyable-inputs` did not report `{PHASE1_REQUIRED_COPYABLE_INPUT}`"
        )));
    }
    let help_output = require_success_text(runner.run(&["--help"])?, "--help")?;
    for option in ["--check-input", "--json", "--version"] {
        if !help_output.contains(option) {
            return Err(BackendError::Probe(format!(
                "`--help` did not report required interface `{option}`"
            )));
        }
    }

    Ok(MooseProbeEvidence {
        identity,
        version_output,
        application_type,
        copyable_inputs,
        help_output,
    })
}

pub fn check_backend_input(
    runner: &MooseProcessRunner,
    input: &Path,
) -> Result<ProcessOutput, BackendError> {
    let input = input
        .to_str()
        .ok_or_else(|| BackendError::Process("input path is not valid UTF-8".to_owned()))?;
    let output = runner.run(&["-i", input, "--check-input"])?;
    if output.success() {
        Ok(output)
    } else {
        Err(BackendError::Probe(format!(
            "MOOSE --check-input failed: status={:?}, timed_out={}, stderr={}",
            output.status_code,
            output.timed_out,
            output.stderr_text()
        )))
    }
}

fn require_success_text(output: ProcessOutput, operation: &str) -> Result<String, BackendError> {
    if !output.success() {
        return Err(BackendError::Probe(format!(
            "MOOSE {operation} failed: status={:?}, timed_out={}, stderr={}",
            output.status_code,
            output.timed_out,
            output.stderr_text()
        )));
    }
    let mut text = output.stdout_text();
    if text.trim().is_empty() {
        text = output.stderr_text();
    }
    if text.trim().is_empty() {
        return Err(BackendError::Probe(format!(
            "MOOSE {operation} produced no observable text"
        )));
    }
    Ok(text)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLayout {
    root: PathBuf,
    run_key: String,
}

impl WorkspaceLayout {
    pub fn new(root: impl Into<PathBuf>, run_key: impl Into<String>) -> Result<Self, BackendError> {
        let run_key = run_key.into();
        let valid = !run_key.is_empty()
            && run_key.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
            });
        if !valid {
            return Err(BackendError::InvalidRunKey(run_key));
        }
        Ok(Self {
            root: root.into(),
            run_key,
        })
    }

    pub fn run_dir(&self) -> PathBuf {
        self.root.join("runs").join(&self.run_key)
    }

    pub fn input_path(&self) -> PathBuf {
        self.run_dir().join("input.i")
    }

    pub fn stdout_path(&self) -> PathBuf {
        self.run_dir().join("stdout.log")
    }

    pub fn stderr_path(&self) -> PathBuf {
        self.run_dir().join("stderr.log")
    }

    pub fn prepare(&self) -> Result<(), BackendError> {
        fs::create_dir_all(self.run_dir()).map_err(|error| BackendError::Io {
            context: format!("create {}", self.run_dir().display()),
            detail: error.to_string(),
        })
    }
}

pub fn execute_backend_input(
    runner: &MooseProcessRunner,
    layout: &WorkspaceLayout,
    input: &str,
) -> Result<ProcessOutput, BackendError> {
    layout.prepare()?;
    write_artifact(&layout.input_path(), input.as_bytes())?;

    let output = runner.run_in_dir(&["-i", "input.i"], &layout.run_dir())?;
    write_artifact(&layout.stdout_path(), &output.stdout)?;
    write_artifact(&layout.stderr_path(), &output.stderr)?;
    Ok(output)
}

fn write_artifact(path: &Path, bytes: &[u8]) -> Result<(), BackendError> {
    fs::write(path, bytes).map_err(|error| BackendError::Io {
        context: format!("write {}", path.display()),
        detail: error.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    Io {
        context: String,
        detail: String,
    },
    InvalidMetadata(String),
    IdentityMismatch {
        label: &'static str,
        expected: String,
        actual: String,
    },
    Spawn {
        executable: PathBuf,
        detail: String,
    },
    Process(String),
    Probe(String),
    InvalidRunKey(String),
}

impl Display for BackendError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { context, detail } => write!(formatter, "{context}: {detail}"),
            Self::InvalidMetadata(detail) => {
                write!(formatter, "invalid MOOSE package metadata: {detail}")
            }
            Self::IdentityMismatch {
                label,
                expected,
                actual,
            } => write!(
                formatter,
                "MOOSE {label} mismatch: expected `{expected}`, observed `{actual}`"
            ),
            Self::Spawn { executable, detail } => {
                write!(
                    formatter,
                    "failed to spawn {}: {detail}",
                    executable.display()
                )
            }
            Self::Process(detail) => write!(formatter, "MOOSE process error: {detail}"),
            Self::Probe(detail) => write!(formatter, "MOOSE probe error: {detail}"),
            Self::InvalidRunKey(key) => write!(formatter, "invalid deterministic run key: {key}"),
        }
    }
}

impl std::error::Error for BackendError {}
