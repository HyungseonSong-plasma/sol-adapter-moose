use serde::{Deserialize, Serialize};
use serde_json::Value;
use sol_public_contract::{
    BackendTargetDto, Diagnostic, DiagnosticSeverity, Extensions, MappingPlanDto,
    MappingSubjectDto, RealizationEffectDocumentDto, RealizationEffectDto, ValidationReport,
    PUBLIC_CONTRACT_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    canonicalize_value, reject_transport_markers, AdapterProtocolVersion, ProtocolError,
    ADAPTER_PROTOCOL_VERSION, DIAGNOSTIC_PRECONDITION_REJECTED, DIAGNOSTIC_TRANSIENT_UNAVAILABLE,
};

pub const DIAGNOSTIC_EXECUTION_FAILED: &str = "adapter.execution_failed";
pub const DIAGNOSTIC_DEPENDENCY_SKIPPED: &str = "adapter.dependency_skipped";
pub const DIAGNOSTIC_EXECUTION_REJECTED: &str = "adapter.execution_rejected";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutePlanRequest {
    pub adapter_protocol_version: String,
    pub target: BackendTargetDto,
    pub plan: MappingPlanDto,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ExecutePlanRequest {
    pub fn new(target: BackendTargetDto, plan: MappingPlanDto) -> Result<Self, ExecutionError> {
        let mut request = Self {
            adapter_protocol_version: ADAPTER_PROTOCOL_VERSION.to_owned(),
            target,
            plan,
            extensions: BTreeMap::new(),
        };
        request.normalize()?;
        Ok(request)
    }

    pub fn from_json(input: &str) -> Result<Self, ExecutionError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| ExecutionError::InvalidRequest(error.to_string()))?;
        reject_transport_markers(&value).map_err(ExecutionError::Protocol)?;
        reject_execution_forbidden_fields(&value)?;
        let mut request: Self = serde_json::from_value(value)
            .map_err(|error| ExecutionError::InvalidRequest(error.to_string()))?;
        request.normalize()?;
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, ExecutionError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        let value = serde_json::to_value(normalized)
            .map_err(|error| ExecutionError::InvalidRequest(error.to_string()))?;
        serde_json::to_string(&canonicalize_value(value))
            .map_err(|error| ExecutionError::InvalidRequest(error.to_string()))
    }

    pub fn canonical_plan_identity(&self) -> Result<String, ExecutionError> {
        self.plan
            .to_canonical_json()
            .map_err(|error| ExecutionError::InvalidPublicPayload(error.to_string()))
    }

    fn normalize(&mut self) -> Result<(), ExecutionError> {
        require_current_protocol_version(&self.adapter_protocol_version)?;
        reject_execution_forbidden_fields(
            &serde_json::to_value(&*self)
                .map_err(|error| ExecutionError::InvalidRequest(error.to_string()))?,
        )?;

        self.target = BackendTargetDto::from_json(
            &self
                .target
                .to_canonical_json()
                .map_err(|error| ExecutionError::InvalidPublicPayload(error.to_string()))?,
        )
        .map_err(|error| ExecutionError::InvalidPublicPayload(error.to_string()))?;
        self.plan = MappingPlanDto::from_json(
            &self
                .plan
                .to_canonical_json()
                .map_err(|error| ExecutionError::InvalidPublicPayload(error.to_string()))?,
        )
        .map_err(|error| ExecutionError::InvalidPublicPayload(error.to_string()))?;

        if self.target.public_contract_version != self.plan.public_contract_version {
            return Err(ExecutionError::IncoherentPublicContractVersions {
                target: self.target.public_contract_version.clone(),
                plan: self.plan.public_contract_version.clone(),
            });
        }
        if self.target.public_contract_version != PUBLIC_CONTRACT_VERSION {
            return Err(ExecutionError::UnsupportedPublicContractVersion(
                self.target.public_contract_version.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionExecutionState {
    Completed,
    Failed,
    Unavailable,
    SkippedDependency,
    NotStarted,
}

impl ActionExecutionState {
    fn started(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpaqueExecutionReference {
    pub namespace: String,
    pub reference: String,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl OpaqueExecutionReference {
    fn normalize(&mut self) -> Result<(), ExecutionError> {
        if self.namespace.is_empty() {
            return Err(ExecutionError::EmptyField("opaque reference namespace"));
        }
        if self.reference.is_empty() {
            return Err(ExecutionError::EmptyField("opaque reference"));
        }
        reject_execution_forbidden_fields(
            &serde_json::to_value(&*self)
                .map_err(|error| ExecutionError::InvalidResponse(error.to_string()))?,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionProvenance {
    pub producer: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opaque_references: Vec<OpaqueExecutionReference>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ExecutionProvenance {
    fn normalize(&mut self) -> Result<(), ExecutionError> {
        if self.producer.is_empty() {
            return Err(ExecutionError::EmptyField("execution provenance producer"));
        }
        reject_execution_forbidden_fields(
            &serde_json::to_value(&self.extensions)
                .map_err(|error| ExecutionError::InvalidResponse(error.to_string()))?,
        )?;
        for reference in &mut self.opaque_references {
            reference.normalize()?;
        }
        self.opaque_references.sort_by(|left, right| {
            left.namespace
                .cmp(&right.namespace)
                .then_with(|| left.reference.cmp(&right.reference))
                .then_with(|| stable_json(&left.extensions).cmp(&stable_json(&right.extensions)))
        });
        self.opaque_references.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionExecutionReport {
    pub action_id: String,
    pub state: ActionExecutionState,
    #[serde(default)]
    pub effects: Vec<RealizationEffectDto>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<ExecutionProvenance>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ActionExecutionReport {
    fn normalize(&mut self) -> Result<(), ExecutionError> {
        require_stable_symbol("action report id", &self.action_id)?;
        reject_execution_forbidden_fields(
            &serde_json::to_value(&self.extensions)
                .map_err(|error| ExecutionError::InvalidResponse(error.to_string()))?,
        )?;
        normalize_effects(&mut self.effects)?;
        normalize_diagnostics(&mut self.diagnostics)?;
        if let Some(provenance) = &mut self.provenance {
            provenance.normalize()?;
        }

        let error_codes: Vec<_> = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        match self.state {
            ActionExecutionState::Completed => {
                if !error_codes.is_empty() {
                    return Err(ExecutionError::InconsistentActionReport {
                        action: self.action_id.clone(),
                        reason: "completed action cannot contain error diagnostics".to_owned(),
                    });
                }
            }
            ActionExecutionState::Failed => {
                if error_codes.is_empty() {
                    return Err(ExecutionError::InconsistentActionReport {
                        action: self.action_id.clone(),
                        reason: "failed action requires an error diagnostic".to_owned(),
                    });
                }
            }
            ActionExecutionState::Unavailable => {
                if !self.effects.is_empty() {
                    return Err(ExecutionError::InconsistentActionReport {
                        action: self.action_id.clone(),
                        reason: "unavailable action cannot report realization effects".to_owned(),
                    });
                }
                if !error_codes.contains(&DIAGNOSTIC_TRANSIENT_UNAVAILABLE) {
                    return Err(ExecutionError::InconsistentActionReport {
                        action: self.action_id.clone(),
                        reason:
                            "unavailable action requires adapter.transient_unavailable diagnostic"
                                .to_owned(),
                    });
                }
            }
            ActionExecutionState::SkippedDependency => {
                if !self.effects.is_empty() {
                    return Err(ExecutionError::InconsistentActionReport {
                        action: self.action_id.clone(),
                        reason: "dependency-skipped action cannot report realization effects"
                            .to_owned(),
                    });
                }
                if !error_codes.contains(&DIAGNOSTIC_DEPENDENCY_SKIPPED) {
                    return Err(ExecutionError::InconsistentActionReport {
                        action: self.action_id.clone(),
                        reason: "dependency-skipped action requires adapter.dependency_skipped diagnostic"
                            .to_owned(),
                    });
                }
            }
            ActionExecutionState::NotStarted => {
                if !self.effects.is_empty() {
                    return Err(ExecutionError::InconsistentActionReport {
                        action: self.action_id.clone(),
                        reason: "not-started action cannot report realization effects".to_owned(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcome {
    Completed,
    Partial,
    Failed,
    Rejected,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutePlanResponse {
    pub adapter_protocol_version: String,
    pub execution: ExecutionOutcome,
    #[serde(default)]
    pub execution_batches: Vec<Vec<String>>,
    pub action_reports: Vec<ActionExecutionReport>,
    #[serde(default)]
    pub effects: Vec<RealizationEffectDto>,
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<ExecutionProvenance>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ExecutePlanResponse {
    pub fn from_json(input: &str) -> Result<Self, ExecutionError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| ExecutionError::InvalidResponse(error.to_string()))?;
        reject_transport_markers(&value).map_err(ExecutionError::Protocol)?;
        reject_execution_forbidden_fields(&value)?;
        let mut response: Self = serde_json::from_value(value)
            .map_err(|error| ExecutionError::InvalidResponse(error.to_string()))?;
        response.normalize_local()?;
        Ok(response)
    }

    pub fn to_canonical_json(&self) -> Result<String, ExecutionError> {
        let mut normalized = self.clone();
        normalized.normalize_local()?;
        let value = serde_json::to_value(normalized)
            .map_err(|error| ExecutionError::InvalidResponse(error.to_string()))?;
        serde_json::to_string(&canonicalize_value(value))
            .map_err(|error| ExecutionError::InvalidResponse(error.to_string()))
    }

    pub fn validate_against(&mut self, request: &ExecutePlanRequest) -> Result<(), ExecutionError> {
        self.normalize_local()?;
        validate_action_coverage_and_schedule(self, request)?;
        validate_aggregate_effects(self)?;
        validate_execution_outcome(self)?;
        validate_provenance_semantic_separation(self, request)?;
        Ok(())
    }

    fn normalize_local(&mut self) -> Result<(), ExecutionError> {
        require_current_protocol_version(&self.adapter_protocol_version)?;
        reject_execution_forbidden_fields(
            &serde_json::to_value(&*self)
                .map_err(|error| ExecutionError::InvalidResponse(error.to_string()))?,
        )?;

        for batch in &mut self.execution_batches {
            if batch.is_empty() {
                return Err(ExecutionError::EmptyExecutionBatch);
            }
            for action_id in batch.iter() {
                require_stable_symbol("execution batch action id", action_id)?;
            }
            batch.sort();
            for pair in batch.windows(2) {
                if pair[0] == pair[1] {
                    return Err(ExecutionError::DuplicateScheduledAction(pair[0].clone()));
                }
            }
        }

        for report in &mut self.action_reports {
            report.normalize()?;
        }
        self.action_reports
            .sort_by(|left, right| left.action_id.cmp(&right.action_id));
        for pair in self.action_reports.windows(2) {
            if pair[0].action_id == pair[1].action_id {
                return Err(ExecutionError::DuplicateActionReport(
                    pair[0].action_id.clone(),
                ));
            }
        }

        normalize_effects(&mut self.effects)?;
        normalize_diagnostics(&mut self.diagnostics)?;
        if let Some(provenance) = &mut self.provenance {
            provenance.normalize()?;
        }
        Ok(())
    }
}

fn validate_action_coverage_and_schedule(
    response: &ExecutePlanResponse,
    request: &ExecutePlanRequest,
) -> Result<(), ExecutionError> {
    let plan_actions: BTreeMap<_, _> = request
        .plan
        .actions
        .iter()
        .map(|action| (action.id.as_str(), action))
        .collect();
    let report_actions: BTreeSet<_> = response
        .action_reports
        .iter()
        .map(|report| report.action_id.as_str())
        .collect();

    let expected: BTreeSet<_> = plan_actions.keys().copied().collect();
    if report_actions != expected {
        let missing = expected
            .difference(&report_actions)
            .map(|value| (*value).to_owned())
            .collect();
        let unknown = report_actions
            .difference(&expected)
            .map(|value| (*value).to_owned())
            .collect();
        return Err(ExecutionError::ActionReportCoverage { missing, unknown });
    }

    let reports: BTreeMap<_, _> = response
        .action_reports
        .iter()
        .map(|report| (report.action_id.as_str(), report))
        .collect();

    let mut batch_index = BTreeMap::new();
    for (index, batch) in response.execution_batches.iter().enumerate() {
        for action_id in batch {
            if batch_index.insert(action_id.as_str(), index).is_some() {
                return Err(ExecutionError::DuplicateScheduledAction(action_id.clone()));
            }
            let Some(report) = reports.get(action_id.as_str()) else {
                return Err(ExecutionError::UnknownScheduledAction(action_id.clone()));
            };
            if !report.state.started() {
                return Err(ExecutionError::InconsistentSchedule {
                    action: action_id.clone(),
                    reason: "only completed/failed actions may appear in execution batches"
                        .to_owned(),
                });
            }
        }
    }

    for report in &response.action_reports {
        if report.state.started() && !batch_index.contains_key(report.action_id.as_str()) {
            return Err(ExecutionError::InconsistentSchedule {
                action: report.action_id.clone(),
                reason: "started action is missing from execution_batches".to_owned(),
            });
        }
        if !report.state.started() && batch_index.contains_key(report.action_id.as_str()) {
            return Err(ExecutionError::InconsistentSchedule {
                action: report.action_id.clone(),
                reason: "non-started action appears in execution_batches".to_owned(),
            });
        }
    }

    for (action_id, action) in &plan_actions {
        let report = reports
            .get(*action_id)
            .expect("action report coverage was checked");

        if report.state == ActionExecutionState::SkippedDependency {
            let all_completed = action.dependencies.iter().all(|dependency| {
                reports
                    .get(dependency.as_str())
                    .expect("plan dependency was validated by Public Contract")
                    .state
                    == ActionExecutionState::Completed
            });
            if all_completed {
                return Err(ExecutionError::InconsistentActionReport {
                    action: (*action_id).to_owned(),
                    reason: "skipped_dependency requires at least one non-completed dependency"
                        .to_owned(),
                });
            }
        }

        for dependency in &action.dependencies {
            let dependency_report = reports
                .get(dependency.as_str())
                .expect("plan dependency was validated by Public Contract");
            if report.state.started() {
                if dependency_report.state != ActionExecutionState::Completed {
                    return Err(ExecutionError::DependencyViolation {
                        action: (*action_id).to_owned(),
                        dependency: dependency.clone(),
                        reason: "started action requires completed dependency".to_owned(),
                    });
                }
                let dependency_batch = batch_index[dependency.as_str()];
                let action_batch = batch_index[action_id];
                if dependency_batch >= action_batch {
                    return Err(ExecutionError::DependencyViolation {
                        action: (*action_id).to_owned(),
                        dependency: dependency.clone(),
                        reason: "dependency must appear in an earlier execution batch".to_owned(),
                    });
                }
            }
        }
    }

    Ok(())
}

fn validate_aggregate_effects(response: &ExecutePlanResponse) -> Result<(), ExecutionError> {
    let mut union = response
        .action_reports
        .iter()
        .flat_map(|report| report.effects.iter().cloned())
        .collect::<Vec<_>>();
    normalize_effects(&mut union)?;
    if union != response.effects {
        return Err(ExecutionError::AggregateEffectsMismatch);
    }
    Ok(())
}

fn validate_execution_outcome(response: &ExecutePlanResponse) -> Result<(), ExecutionError> {
    let states = response
        .action_reports
        .iter()
        .map(|report| report.state)
        .collect::<Vec<_>>();
    let completed = states
        .iter()
        .filter(|state| **state == ActionExecutionState::Completed)
        .count();
    let failed = states
        .iter()
        .filter(|state| **state == ActionExecutionState::Failed)
        .count();
    let unavailable = states
        .iter()
        .filter(|state| **state == ActionExecutionState::Unavailable)
        .count();
    let not_started = states
        .iter()
        .filter(|state| **state == ActionExecutionState::NotStarted)
        .count();

    let expected = if completed == states.len() {
        ExecutionOutcome::Completed
    } else if completed > 0 {
        ExecutionOutcome::Partial
    } else if failed > 0 {
        ExecutionOutcome::Failed
    } else if not_started == states.len() {
        ExecutionOutcome::Rejected
    } else if unavailable > 0
        && states.iter().all(|state| {
            matches!(
                state,
                ActionExecutionState::Unavailable | ActionExecutionState::NotStarted
            )
        })
    {
        ExecutionOutcome::Unavailable
    } else {
        return Err(ExecutionError::InconsistentExecutionOutcome(
            "terminal action states do not map to a valid Protocol 0.1 execution outcome"
                .to_owned(),
        ));
    };

    if response.execution != expected {
        return Err(ExecutionError::ExecutionOutcomeMismatch {
            declared: response.execution,
            expected,
        });
    }

    let top_error_codes: Vec<_> = response
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .map(|diagnostic| diagnostic.code.as_str())
        .collect();
    if response.execution == ExecutionOutcome::Completed && !top_error_codes.is_empty() {
        return Err(ExecutionError::InconsistentExecutionOutcome(
            "completed execution cannot contain top-level error diagnostics".to_owned(),
        ));
    }
    if response.execution == ExecutionOutcome::Rejected
        && !top_error_codes.iter().any(|code| {
            matches!(
                *code,
                DIAGNOSTIC_EXECUTION_REJECTED | DIAGNOSTIC_PRECONDITION_REJECTED
            )
        })
    {
        return Err(ExecutionError::InconsistentExecutionOutcome(
            "rejected execution requires authoritative rejection diagnostic".to_owned(),
        ));
    }
    if response.execution == ExecutionOutcome::Unavailable
        && !top_error_codes.contains(&DIAGNOSTIC_TRANSIENT_UNAVAILABLE)
    {
        return Err(ExecutionError::InconsistentExecutionOutcome(
            "unavailable execution requires adapter.transient_unavailable diagnostic".to_owned(),
        ));
    }
    Ok(())
}

fn validate_provenance_semantic_separation(
    response: &ExecutePlanResponse,
    request: &ExecutePlanRequest,
) -> Result<(), ExecutionError> {
    let mut references = Vec::new();
    collect_provenance_references(response.provenance.as_ref(), &mut references);
    for report in &response.action_reports {
        collect_provenance_references(report.provenance.as_ref(), &mut references);
    }

    let semantic_values = semantic_identity_values(response, request);
    for reference in references {
        if semantic_values.contains(reference.as_str()) {
            return Err(ExecutionError::OpaqueReferenceUsedAsSemanticIdentity(
                reference,
            ));
        }
    }
    Ok(())
}

fn collect_provenance_references(
    provenance: Option<&ExecutionProvenance>,
    references: &mut Vec<String>,
) {
    if let Some(provenance) = provenance {
        references.extend(
            provenance
                .opaque_references
                .iter()
                .map(|reference| reference.reference.clone()),
        );
    }
}

fn semantic_identity_values(
    response: &ExecutePlanResponse,
    request: &ExecutePlanRequest,
) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    values.insert(request.target.target.clone());
    values.extend(request.plan.actions.iter().map(|action| action.id.clone()));
    for effect in response.effects.iter().chain(
        response
            .action_reports
            .iter()
            .flat_map(|report| report.effects.iter()),
    ) {
        match &effect.subject {
            MappingSubjectDto::Entity { id, .. } => {
                values.insert(id.clone());
            }
            MappingSubjectDto::Relation { source, target, .. } => {
                values.insert(source.clone());
                values.insert(target.clone());
            }
        }
    }
    values
}

fn normalize_effects(effects: &mut Vec<RealizationEffectDto>) -> Result<(), ExecutionError> {
    for effect in effects.iter() {
        let document = RealizationEffectDocumentDto {
            public_contract_version: PUBLIC_CONTRACT_VERSION.to_owned(),
            effect: effect.clone(),
            extensions: BTreeMap::new(),
        };
        document
            .to_canonical_json()
            .map_err(|error| ExecutionError::InvalidPublicPayload(error.to_string()))?;
    }
    effects.sort_by_key(stable_json);
    effects.dedup();
    Ok(())
}

fn normalize_diagnostics(diagnostics: &mut Vec<Diagnostic>) -> Result<(), ExecutionError> {
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        diagnostic
            .validate_shape()
            .map_err(|error| ExecutionError::MalformedDiagnostic {
                index,
                reason: error.to_string(),
            })?;
    }
    *diagnostics = ValidationReport::new(diagnostics.clone()).diagnostics;
    Ok(())
}

fn require_current_protocol_version(version: &str) -> Result<(), ExecutionError> {
    let parsed = AdapterProtocolVersion::parse(version).map_err(ExecutionError::Protocol)?;
    if parsed != AdapterProtocolVersion::current() {
        return Err(ExecutionError::UnsupportedProtocolVersion(
            version.to_owned(),
        ));
    }
    Ok(())
}

fn require_stable_symbol(field: &'static str, value: &str) -> Result<(), ExecutionError> {
    let valid = !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        });
    if valid {
        Ok(())
    } else {
        Err(ExecutionError::InvalidSymbol {
            field,
            value: value.to_owned(),
        })
    }
}

fn reject_execution_forbidden_fields(value: &Value) -> Result<(), ExecutionError> {
    const FORBIDDEN: &[&str] = &[
        "validation_token",
        "acceptance_id",
        "lease_id",
        "plan_hash",
        "plan_digest",
        "backend_native_id",
        "backend_object",
        "native_object",
        "solver_object",
        "vendor_object",
        "selection_id",
        "job_id",
        "artifact_id",
        "retry",
        "retry_policy",
        "replay",
    ];

    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if FORBIDDEN.contains(&key.as_str()) {
                    return Err(ExecutionError::ForbiddenField(key.clone()));
                }
                reject_execution_forbidden_fields(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_execution_forbidden_fields(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn stable_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    Protocol(ProtocolError),
    InvalidRequest(String),
    InvalidResponse(String),
    InvalidPublicPayload(String),
    UnsupportedProtocolVersion(String),
    UnsupportedPublicContractVersion(String),
    IncoherentPublicContractVersions {
        target: String,
        plan: String,
    },
    EmptyField(&'static str),
    InvalidSymbol {
        field: &'static str,
        value: String,
    },
    MalformedDiagnostic {
        index: usize,
        reason: String,
    },
    DuplicateActionReport(String),
    ActionReportCoverage {
        missing: Vec<String>,
        unknown: Vec<String>,
    },
    EmptyExecutionBatch,
    DuplicateScheduledAction(String),
    UnknownScheduledAction(String),
    InconsistentSchedule {
        action: String,
        reason: String,
    },
    DependencyViolation {
        action: String,
        dependency: String,
        reason: String,
    },
    InconsistentActionReport {
        action: String,
        reason: String,
    },
    AggregateEffectsMismatch,
    InconsistentExecutionOutcome(String),
    ExecutionOutcomeMismatch {
        declared: ExecutionOutcome,
        expected: ExecutionOutcome,
    },
    OpaqueReferenceUsedAsSemanticIdentity(String),
    ForbiddenField(String),
}

impl Display for ExecutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(error) => write!(formatter, "{error}"),
            Self::InvalidRequest(detail) => {
                write!(formatter, "invalid execute_plan request: {detail}")
            }
            Self::InvalidResponse(detail) => {
                write!(formatter, "invalid execute_plan response: {detail}")
            }
            Self::InvalidPublicPayload(detail) => {
                write!(
                    formatter,
                    "invalid reused Public Contract payload: {detail}"
                )
            }
            Self::UnsupportedProtocolVersion(version) => {
                write!(formatter, "unsupported Adapter Protocol version: {version}")
            }
            Self::UnsupportedPublicContractVersion(version) => {
                write!(formatter, "unsupported Public Contract version: {version}")
            }
            Self::IncoherentPublicContractVersions { target, plan } => write!(
                formatter,
                "execute_plan payload Public Contract versions differ: target={target}, plan={plan}"
            ),
            Self::EmptyField(field) => write!(formatter, "{field} must not be empty"),
            Self::InvalidSymbol { field, value } => {
                write!(formatter, "invalid {field} symbol: {value}")
            }
            Self::MalformedDiagnostic { index, reason } => {
                write!(
                    formatter,
                    "malformed execution diagnostic at index {index}: {reason}"
                )
            }
            Self::DuplicateActionReport(action) => {
                write!(formatter, "duplicate action execution report: {action}")
            }
            Self::ActionReportCoverage { missing, unknown } => write!(
                formatter,
                "action report coverage mismatch: missing={missing:?}, unknown={unknown:?}"
            ),
            Self::EmptyExecutionBatch => write!(formatter, "execution batch must not be empty"),
            Self::DuplicateScheduledAction(action) => {
                write!(
                    formatter,
                    "action appears more than once in execution batches: {action}"
                )
            }
            Self::UnknownScheduledAction(action) => {
                write!(
                    formatter,
                    "execution batch references unknown action: {action}"
                )
            }
            Self::InconsistentSchedule { action, reason } => {
                write!(
                    formatter,
                    "inconsistent execution schedule for {action}: {reason}"
                )
            }
            Self::DependencyViolation {
                action,
                dependency,
                reason,
            } => write!(
                formatter,
                "execution dependency violation: action={action}, dependency={dependency}: {reason}"
            ),
            Self::InconsistentActionReport { action, reason } => {
                write!(
                    formatter,
                    "inconsistent action report for {action}: {reason}"
                )
            }
            Self::AggregateEffectsMismatch => write!(
                formatter,
                "top-level realization effects do not equal normalized union of action effects"
            ),
            Self::InconsistentExecutionOutcome(reason) => {
                write!(formatter, "inconsistent execution outcome: {reason}")
            }
            Self::ExecutionOutcomeMismatch { declared, expected } => write!(
                formatter,
                "execution outcome mismatch: declared={declared:?}, expected={expected:?}"
            ),
            Self::OpaqueReferenceUsedAsSemanticIdentity(reference) => write!(
                formatter,
                "opaque provenance reference is reused as canonical semantic identity: {reference}"
            ),
            Self::ForbiddenField(field) => write!(
                formatter,
                "field is forbidden from Protocol 0.1 execution semantics: {field}"
            ),
        }
    }
}

impl Error for ExecutionError {}
