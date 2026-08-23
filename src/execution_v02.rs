use crate::backend::{
    probe_exact_phase1_target, BackendError, MooseProcessRunner, WorkspaceLayout,
    PHASE1_MOOSE_PACKAGE_FILENAME,
};
use crate::realization_v02::translate_steady_thermal_v02;
use sol_adapter_protocol::{
    protocol_diagnostic, ActionExecutionReport, ActionExecutionState,
    AdapterProtocolDiagnosticContext, ExecutePlanRequestV02, ExecutePlanResponseV02,
    ExecutionOutcome, ExecutionProvenance, OpaqueExecutionReference, PreflightOutcome,
    ProtocolFailure, ProtocolOperation, SideEffectEvidence, ValidatePlanRequestV02,
    ValidatePlanResponseV02, ADAPTER_PROTOCOL_VERSION_0_2, DIAGNOSTIC_EXECUTION_REJECTED,
    DIAGNOSTIC_MISSING_CAPABILITY, DIAGNOSTIC_PRECONDITION_REJECTED,
    DIAGNOSTIC_TARGET_MISMATCH, DIAGNOSTIC_TRANSIENT_UNAVAILABLE,
};
use sol_public_contract::{
    Diagnostic, EntityKindDto, MappingSubjectDto, RealizationEffectDto, RealizationQualityDto,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MOOSE_TARGET: &str = "moose";
pub const THERMAL_CAPABILITY_V02: &str = "thermal.steady_conduction";

const DEFAULT_BACKEND_TIMEOUT: Duration = Duration::from_secs(120);
static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct ExecutionEnvironment {
    runner: Option<MooseProcessRunner>,
    workspace_root: PathBuf,
}

impl ExecutionEnvironment {
    pub fn from_environment() -> Self {
        let runner = std::env::var_os("SOL_MOOSE_EXECUTABLE")
            .map(|path| MooseProcessRunner::new(PathBuf::from(path), DEFAULT_BACKEND_TIMEOUT));
        let workspace_root = std::env::var_os("SOL_MOOSE_WORKSPACE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("sol-adaptor-moose-runtime"));
        Self {
            runner,
            workspace_root,
        }
    }

    pub fn configured(
        executable: impl Into<PathBuf>,
        workspace_root: impl Into<PathBuf>,
        timeout: Duration,
    ) -> Self {
        Self {
            runner: Some(MooseProcessRunner::new(executable, timeout)),
            workspace_root: workspace_root.into(),
        }
    }

    pub fn unavailable(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            runner: None,
            workspace_root: workspace_root.into(),
        }
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn validate_plan_v02(
        &self,
        request: &ValidatePlanRequestV02,
    ) -> Result<ValidatePlanResponseV02, ProtocolFailure> {
        let request = normalize_validate_request(request)?;

        if request.target.target != MOOSE_TARGET {
            return rejected_preflight(
                false,
                required_capabilities_satisfied(&request.target.required_capabilities),
                DIAGNOSTIC_TARGET_MISMATCH,
                format!(
                    "A0.1 supports only backend target `{MOOSE_TARGET}`, got `{}`",
                    request.target.target
                ),
                AdapterProtocolDiagnosticContext {
                    target: Some(request.target.target.clone()),
                    ..Default::default()
                },
            );
        }
        if !required_capabilities_satisfied(&request.target.required_capabilities) {
            return rejected_preflight(
                true,
                false,
                DIAGNOSTIC_MISSING_CAPABILITY,
                format!(
                    "A0.1 Protocol 0.2 execution supports exactly `{THERMAL_CAPABILITY_V02}`"
                ),
                AdapterProtocolDiagnosticContext {
                    capability: Some(THERMAL_CAPABILITY_V02.to_owned()),
                    ..Default::default()
                },
            );
        }

        if let Err(error) = translate_steady_thermal_v02(&request) {
            return rejected_preflight(
                true,
                true,
                DIAGNOSTIC_PRECONDITION_REJECTED,
                error.to_string(),
                AdapterProtocolDiagnosticContext::default(),
            );
        }

        match self.backend_readiness() {
            BackendReadiness::Ready => Ok(ValidatePlanResponseV02::accepted()),
            BackendReadiness::Unavailable(detail) => {
                let diagnostic = protocol_diagnostic(
                    DIAGNOSTIC_TRANSIENT_UNAVAILABLE,
                    detail,
                    AdapterProtocolDiagnosticContext {
                        target: Some(MOOSE_TARGET.to_owned()),
                        ..Default::default()
                    },
                )
                .map_err(|error| protocol_operation_failure(
                    ProtocolOperation::ValidatePlan,
                    error,
                    SideEffectEvidence::None,
                ))?;
                ValidatePlanResponseV02::new(
                    true,
                    true,
                    PreflightOutcome::Unavailable,
                    vec![diagnostic],
                )
                .map_err(|error| protocol_operation_failure(
                    ProtocolOperation::ValidatePlan,
                    error,
                    SideEffectEvidence::None,
                ))
            }
            BackendReadiness::Incompatible(detail) => rejected_preflight(
                false,
                true,
                DIAGNOSTIC_TARGET_MISMATCH,
                detail,
                AdapterProtocolDiagnosticContext {
                    target: Some(MOOSE_TARGET.to_owned()),
                    ..Default::default()
                },
            ),
        }
    }

    pub fn execute_plan_v02(
        &self,
        request: &ExecutePlanRequestV02,
    ) -> Result<ExecutePlanResponseV02, ProtocolFailure> {
        let run_key = fresh_run_key();
        self.execute_plan_v02_with_run_key(request, &run_key)
    }

    pub fn execute_plan_v02_with_run_key(
        &self,
        request: &ExecutePlanRequestV02,
        run_key: &str,
    ) -> Result<ExecutePlanResponseV02, ProtocolFailure> {
        let request = normalize_execute_request(request)?;

        if request.target.target != MOOSE_TARGET {
            return rejected_execution(
                &request,
                vec![Diagnostic::error(
                    DIAGNOSTIC_EXECUTION_REJECTED,
                    None,
                    format!(
                        "authoritative execution rejected: unsupported backend target `{}`",
                        request.target.target
                    ),
                )],
            );
        }
        if !required_capabilities_satisfied(&request.target.required_capabilities) {
            return rejected_execution(
                &request,
                vec![Diagnostic::error(
                    DIAGNOSTIC_EXECUTION_REJECTED,
                    None,
                    format!(
                        "authoritative execution rejected: required capability set is outside `{THERMAL_CAPABILITY_V02}`"
                    ),
                )],
            );
        }

        let validate_request = ValidatePlanRequestV02::new(
            request.target.clone(),
            request.plan.clone(),
            request.realization_spec.clone(),
        )
        .map_err(|error| invalid_request_failure(error.to_string()))?;
        let model = match translate_steady_thermal_v02(&validate_request) {
            Ok(model) => model,
            Err(error) => {
                return rejected_execution(
                    &request,
                    vec![Diagnostic::error(
                        DIAGNOSTIC_PRECONDITION_REJECTED,
                        None,
                        error.to_string(),
                    )],
                )
            }
        };
        let input = model.to_moose_input().map_err(|error| {
            protocol_operation_failure(
                ProtocolOperation::ExecutePlan,
                error,
                SideEffectEvidence::None,
            )
        })?;

        match self.backend_readiness() {
            BackendReadiness::Ready => {}
            BackendReadiness::Unavailable(detail) => {
                return unavailable_execution(&request, detail)
            }
            BackendReadiness::Incompatible(detail) => {
                return rejected_execution(
                    &request,
                    vec![Diagnostic::error(
                        DIAGNOSTIC_EXECUTION_REJECTED,
                        None,
                        format!("authoritative execution rejected by exact-target check: {detail}"),
                    )],
                )
            }
        }

        self.execute_after_verified_backend(&request, &input, run_key)
    }

    fn execute_after_verified_backend(
        &self,
        request: &ExecutePlanRequestV02,
        input: &str,
        run_key: &str,
    ) -> Result<ExecutePlanResponseV02, ProtocolFailure> {
        let runner = self.runner.as_ref().ok_or_else(|| {
            protocol_operation_failure(
                ProtocolOperation::ExecutePlan,
                "backend runner disappeared after readiness check",
                SideEffectEvidence::None,
            )
        })?;
        let layout = WorkspaceLayout::new(&self.workspace_root, run_key).map_err(|error| {
            protocol_operation_failure(
                ProtocolOperation::ExecutePlan,
                error,
                SideEffectEvidence::None,
            )
        })?;

        layout.prepare().map_err(|error| post_start_failure(error.to_string()))?;
        fs::write(layout.input_path(), input.as_bytes())
            .map_err(|error| post_start_failure(format!("write generated input: {error}")))?;

        let check = runner
            .run_in_dir(&["-i", "input.i", "--check-input"], &layout.run_dir())
            .map_err(|error| post_start_failure(format!("MOOSE --check-input: {error}")))?;
        persist_runtime_artifact(&layout.run_dir().join("check.stdout.log"), &check.stdout)?;
        persist_runtime_artifact(&layout.run_dir().join("check.stderr.log"), &check.stderr)?;
        if !check.success() {
            return Err(post_start_failure(format!(
                "MOOSE --check-input failed: status={:?}, timed_out={}, stderr={}",
                check.status_code,
                check.timed_out,
                check.stderr_text()
            )));
        }

        let output = runner
            .run_in_dir(&["-i", "input.i"], &layout.run_dir())
            .map_err(|error| post_start_failure(format!("MOOSE execution: {error}")))?;
        persist_runtime_artifact(&layout.stdout_path(), &output.stdout)?;
        persist_runtime_artifact(&layout.stderr_path(), &output.stderr)?;
        if !output.success() {
            return Err(post_start_failure(format!(
                "MOOSE execution failed: status={:?}, timed_out={}, stderr={}",
                output.status_code,
                output.timed_out,
                output.stderr_text()
            )));
        }

        completed_execution_response(request, runner, &layout, run_key)
    }

    fn backend_readiness(&self) -> BackendReadiness {
        let Some(runner) = self.runner.as_ref() else {
            return BackendReadiness::Unavailable(
                "SOL_MOOSE_EXECUTABLE is not configured for the current adapter process"
                    .to_owned(),
            );
        };
        let Some(prefix) = runner.inferred_conda_prefix() else {
            return BackendReadiness::Incompatible(format!(
                "exact MOOSE package identity cannot be inferred from executable `{}`",
                runner.executable().display()
            ));
        };
        match probe_exact_phase1_target(runner, &prefix) {
            Ok(_) => BackendReadiness::Ready,
            Err(error) => classify_backend_error(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BackendReadiness {
    Ready,
    Unavailable(String),
    Incompatible(String),
}

fn classify_backend_error(error: BackendError) -> BackendReadiness {
    let detail = error.to_string();
    match error {
        BackendError::Spawn { .. } | BackendError::Process(_) | BackendError::Io { .. } => {
            BackendReadiness::Unavailable(detail)
        }
        BackendError::InvalidMetadata(_)
        | BackendError::IdentityMismatch { .. }
        | BackendError::Probe(_)
        | BackendError::InvalidRunKey(_) => BackendReadiness::Incompatible(detail),
    }
}

fn normalize_validate_request(
    request: &ValidatePlanRequestV02,
) -> Result<ValidatePlanRequestV02, ProtocolFailure> {
    let canonical = request
        .to_canonical_json()
        .map_err(|error| invalid_request_failure(error.to_string()))?;
    ValidatePlanRequestV02::from_json(&canonical)
        .map_err(|error| invalid_request_failure(error.to_string()))
}

fn normalize_execute_request(
    request: &ExecutePlanRequestV02,
) -> Result<ExecutePlanRequestV02, ProtocolFailure> {
    let canonical = request
        .to_canonical_json()
        .map_err(|error| invalid_request_failure(error.to_string()))?;
    ExecutePlanRequestV02::from_json(&canonical)
        .map_err(|error| invalid_request_failure(error.to_string()))
}

fn required_capabilities_satisfied(capabilities: &[String]) -> bool {
    capabilities == [THERMAL_CAPABILITY_V02]
}

fn rejected_preflight(
    target_compatible: bool,
    capabilities_satisfied: bool,
    code: &str,
    detail: String,
    context: AdapterProtocolDiagnosticContext,
) -> Result<ValidatePlanResponseV02, ProtocolFailure> {
    let diagnostic = protocol_diagnostic(code, detail, context).map_err(|error| {
        protocol_operation_failure(
            ProtocolOperation::ValidatePlan,
            error,
            SideEffectEvidence::None,
        )
    })?;
    ValidatePlanResponseV02::new(
        target_compatible,
        capabilities_satisfied,
        PreflightOutcome::Rejected,
        vec![diagnostic],
    )
    .map_err(|error| {
        protocol_operation_failure(
            ProtocolOperation::ValidatePlan,
            error,
            SideEffectEvidence::None,
        )
    })
}

fn rejected_execution(
    request: &ExecutePlanRequestV02,
    diagnostics: Vec<Diagnostic>,
) -> Result<ExecutePlanResponseV02, ProtocolFailure> {
    let mut response = ExecutePlanResponseV02 {
        adapter_protocol_version: ADAPTER_PROTOCOL_VERSION_0_2.to_owned(),
        execution: ExecutionOutcome::Rejected,
        execution_batches: Vec::new(),
        action_reports: request
            .plan
            .actions
            .iter()
            .map(|action| ActionExecutionReport {
                action_id: action.id.clone(),
                state: ActionExecutionState::NotStarted,
                effects: Vec::new(),
                diagnostics: Vec::new(),
                provenance: None,
                extensions: Default::default(),
            })
            .collect(),
        effects: Vec::new(),
        diagnostics,
        provenance: None,
        extensions: Default::default(),
    };
    response.validate_against(request).map_err(|error| {
        protocol_operation_failure(
            ProtocolOperation::ExecutePlan,
            error,
            SideEffectEvidence::None,
        )
    })?;
    Ok(response)
}

fn unavailable_execution(
    request: &ExecutePlanRequestV02,
    detail: String,
) -> Result<ExecutePlanResponseV02, ProtocolFailure> {
    let action_diagnostic = Diagnostic::error(DIAGNOSTIC_TRANSIENT_UNAVAILABLE, None, detail.clone());
    let mut response = ExecutePlanResponseV02 {
        adapter_protocol_version: ADAPTER_PROTOCOL_VERSION_0_2.to_owned(),
        execution: ExecutionOutcome::Unavailable,
        execution_batches: Vec::new(),
        action_reports: request
            .plan
            .actions
            .iter()
            .map(|action| ActionExecutionReport {
                action_id: action.id.clone(),
                state: ActionExecutionState::Unavailable,
                effects: Vec::new(),
                diagnostics: vec![action_diagnostic.clone()],
                provenance: None,
                extensions: Default::default(),
            })
            .collect(),
        effects: Vec::new(),
        diagnostics: vec![Diagnostic::error(
            DIAGNOSTIC_TRANSIENT_UNAVAILABLE,
            None,
            detail,
        )],
        provenance: None,
        extensions: Default::default(),
    };
    response.validate_against(request).map_err(|error| {
        protocol_operation_failure(
            ProtocolOperation::ExecutePlan,
            error,
            SideEffectEvidence::None,
        )
    })?;
    Ok(response)
}

fn completed_execution_response(
    request: &ExecutePlanRequestV02,
    runner: &MooseProcessRunner,
    layout: &WorkspaceLayout,
    run_key: &str,
) -> Result<ExecutePlanResponseV02, ProtocolFailure> {
    let order = request
        .plan
        .topological_order()
        .map_err(|error| invalid_request_failure(error.to_string()))?;

    let mut action_reports = Vec::with_capacity(order.len());
    for action_id in &order {
        let effect = representative_effect(request, action_id)?;
        action_reports.push(ActionExecutionReport {
            action_id: action_id.clone(),
            state: ActionExecutionState::Completed,
            effects: vec![effect],
            diagnostics: Vec::new(),
            provenance: None,
            extensions: Default::default(),
        });
    }
    let effects = action_reports
        .iter()
        .flat_map(|report| report.effects.iter().cloned())
        .collect::<Vec<_>>();
    let provenance = ExecutionProvenance {
        producer: "sol.adapter.moose".to_owned(),
        opaque_references: vec![
            OpaqueExecutionReference {
                namespace: "moose.workspace".to_owned(),
                reference: run_key.to_owned(),
                extensions: Default::default(),
            },
            OpaqueExecutionReference {
                namespace: "moose.package".to_owned(),
                reference: PHASE1_MOOSE_PACKAGE_FILENAME.to_owned(),
                extensions: Default::default(),
            },
            OpaqueExecutionReference {
                namespace: "moose.executable".to_owned(),
                reference: runner.executable().display().to_string(),
                extensions: Default::default(),
            },
            OpaqueExecutionReference {
                namespace: "moose.input".to_owned(),
                reference: layout.input_path().display().to_string(),
                extensions: Default::default(),
            },
            OpaqueExecutionReference {
                namespace: "moose.stdout".to_owned(),
                reference: layout.stdout_path().display().to_string(),
                extensions: Default::default(),
            },
            OpaqueExecutionReference {
                namespace: "moose.stderr".to_owned(),
                reference: layout.stderr_path().display().to_string(),
                extensions: Default::default(),
            },
        ],
        extensions: Default::default(),
    };

    let mut response = ExecutePlanResponseV02 {
        adapter_protocol_version: ADAPTER_PROTOCOL_VERSION_0_2.to_owned(),
        execution: ExecutionOutcome::Completed,
        execution_batches: order.into_iter().map(|action| vec![action]).collect(),
        action_reports,
        effects,
        diagnostics: Vec::new(),
        provenance: Some(provenance),
        extensions: Default::default(),
    };
    response.validate_against(request).map_err(|error| {
        post_start_failure(format!("constructed Protocol 0.2 execution response is invalid: {error}"))
    })?;
    Ok(response)
}

fn representative_effect(
    request: &ExecutePlanRequestV02,
    action_id: &str,
) -> Result<RealizationEffectDto, ProtocolFailure> {
    let binding = request
        .realization_spec
        .action_bindings
        .iter()
        .find(|binding| binding.action_id == action_id)
        .ok_or_else(|| invalid_request_failure(format!("missing action binding for {action_id}")))?;

    let candidates = binding
        .subjects
        .iter()
        .filter(|subject| is_supported_effect_subject(request, subject))
        .cloned()
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [subject] => Ok(RealizationEffectDto::new(
            subject.clone(),
            RealizationQualityDto::Exact,
        )),
        [] => Err(protocol_operation_failure(
            ProtocolOperation::ExecutePlan,
            format!(
                "action binding `{action_id}` has no supported canonical realization-effect subject"
            ),
            SideEffectEvidence::MayHaveOccurred,
        )),
        _ => Err(protocol_operation_failure(
            ProtocolOperation::ExecutePlan,
            format!(
                "action binding `{action_id}` has multiple representative realization-effect subjects"
            ),
            SideEffectEvidence::MayHaveOccurred,
        )),
    }
}

fn is_supported_effect_subject(request: &ExecutePlanRequestV02, subject: &MappingSubjectDto) -> bool {
    let MappingSubjectDto::Entity { id, .. } = subject else {
        return false;
    };
    request
        .realization_spec
        .entities
        .iter()
        .find(|entity| entity.id == *id)
        .map(|entity| {
            matches!(
                (entity.kind, entity.semantic_type.as_str()),
                (EntityKindDto::SpatialModel, "LineDomain1D")
                    | (EntityKindDto::MaterialModel, "ThermalConductivity")
                    | (EntityKindDto::MathematicalModel, "Field")
            )
        })
        .unwrap_or(false)
}

fn persist_runtime_artifact(path: &Path, bytes: &[u8]) -> Result<(), ProtocolFailure> {
    fs::write(path, bytes)
        .map_err(|error| post_start_failure(format!("write {}: {error}", path.display())))
}

fn fresh_run_key() -> String {
    let counter = RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("run-{}-{nanos}-{counter}", process::id())
}

fn invalid_request_failure(detail: String) -> ProtocolFailure {
    ProtocolFailure::invalid_request(detail).expect("non-empty invalid request detail")
}

fn post_start_failure(detail: String) -> ProtocolFailure {
    protocol_operation_failure(
        ProtocolOperation::ExecutePlan,
        detail,
        SideEffectEvidence::MayHaveOccurred,
    )
}

fn protocol_operation_failure(
    operation: ProtocolOperation,
    error: impl ToString,
    side_effects: SideEffectEvidence,
) -> ProtocolFailure {
    let failure = ProtocolFailure::operational(error.to_string(), side_effects)
        .expect("adapter operation failure detail is non-empty");
    failure
        .validate_for(operation)
        .expect("side-effect evidence is valid for operation");
    failure
}

#[cfg(test)]
mod tests {
    use super::*;
    use sol_adapter_protocol::FailureCategory;
    use std::io::Write;

    const REQUEST: &str = include_str!("../tests/fixtures/sol/0.2/thermal-realization-request.json");

    fn execute_request() -> ExecutePlanRequestV02 {
        ExecutePlanRequestV02::from_json(REQUEST).unwrap()
    }

    #[test]
    fn missing_backend_returns_unavailable_without_workspace_side_effect() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/phase4-unit/missing-backend");
        let _ = fs::remove_dir_all(&root);
        let environment = ExecutionEnvironment::unavailable(&root);
        let response = environment
            .execute_plan_v02_with_run_key(&execute_request(), "missing-backend")
            .unwrap();
        assert_eq!(response.execution, ExecutionOutcome::Unavailable);
        assert!(response
            .action_reports
            .iter()
            .all(|report| report.state == ActionExecutionState::Unavailable));
        assert!(!root.exists());
    }

    #[test]
    fn unsupported_semantics_are_rejected_before_backend_access() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/phase4-unit/rejected-semantics");
        let _ = fs::remove_dir_all(&root);
        let mut value: serde_json::Value = serde_json::from_str(REQUEST).unwrap();
        value["realization_spec"]["entities"][9]["semantic_type"] =
            "TransientThermalTransport".into();
        let request = ExecutePlanRequestV02::from_json(&value.to_string()).unwrap();
        let environment = ExecutionEnvironment::unavailable(&root);
        let response = environment
            .execute_plan_v02_with_run_key(&request, "rejected-semantics")
            .unwrap();
        assert_eq!(response.execution, ExecutionOutcome::Rejected);
        assert!(response
            .action_reports
            .iter()
            .all(|report| report.state == ActionExecutionState::NotStarted));
        assert!(!root.exists());
    }

    #[cfg(unix)]
    #[test]
    fn backend_failure_after_start_is_conservative_and_not_replayed() {
        use std::os::unix::fs::PermissionsExt;

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/phase4-unit/post-start-failure");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script = root.join("fake-moose.sh");
        let count = root.join("solve-count.txt");
        let mut file = fs::File::create(&script).unwrap();
        writeln!(
            file,
            "#!/bin/sh\ncase \" $* \" in\n  *\" --check-input \"*) echo checked; exit 0;;\nesac\necho run >> '{}'\necho backend-failed >&2\nexit 9",
            count.display()
        )
        .unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).unwrap();

        let environment = ExecutionEnvironment::configured(&script, &root, Duration::from_secs(5));
        let request = execute_request();
        let validate_request = ValidatePlanRequestV02::new(
            request.target.clone(),
            request.plan.clone(),
            request.realization_spec.clone(),
        )
        .unwrap();
        let input = translate_steady_thermal_v02(&validate_request)
            .unwrap()
            .to_moose_input()
            .unwrap();

        let failure = environment
            .execute_after_verified_backend(&request, &input, "post-start-failure")
            .unwrap_err();
        assert_eq!(failure.category, FailureCategory::Operational);
        assert_eq!(failure.side_effects, SideEffectEvidence::MayHaveOccurred);
        assert_eq!(fs::read_to_string(&count).unwrap().lines().count(), 1);

        let layout = WorkspaceLayout::new(&root, "post-start-failure").unwrap();
        assert!(layout.input_path().is_file());
        assert!(layout.run_dir().join("check.stdout.log").is_file());
        assert!(layout.run_dir().join("check.stderr.log").is_file());
        assert!(layout.stdout_path().is_file());
        assert!(layout.stderr_path().is_file());
    }
}
