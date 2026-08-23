use serde::{Deserialize, Serialize};
use serde_json::Value;
use sol_public_contract::{
    BackendTargetDto, BackendTargetDtoV02, Diagnostic, DiagnosticSeverity, MappingPlanDto,
    MappingPlanDtoV02, MappingSubjectDto, RealizationEffectDto, RealizationSpecDtoV02,
    ValidationReport, PUBLIC_CONTRACT_VERSION, PUBLIC_CONTRACT_VERSION_0_2,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    canonicalize_value, reject_transport_markers, ActionExecutionReport, AdapterProtocolVersion,
    ExecutePlanRequest, ExecutePlanResponse, ExecutionOutcome, ExecutionProvenance, Extensions,
    PreflightOutcome, ProtocolError, ADAPTER_PROTOCOL_VERSION_0_2, DIAGNOSTIC_MISSING_CAPABILITY,
    DIAGNOSTIC_PRECONDITION_REJECTED, DIAGNOSTIC_TARGET_MISMATCH, DIAGNOSTIC_TRANSIENT_UNAVAILABLE,
    DIAGNOSTIC_UNSUPPORTED_ACTION,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatePlanRequestV02 {
    pub adapter_protocol_version: String,
    pub target: BackendTargetDtoV02,
    pub plan: MappingPlanDtoV02,
    pub realization_spec: RealizationSpecDtoV02,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ValidatePlanRequestV02 {
    pub fn new(
        target: BackendTargetDtoV02,
        plan: MappingPlanDtoV02,
        realization_spec: RealizationSpecDtoV02,
    ) -> Result<Self, RealizationRequestError> {
        let mut request = Self {
            adapter_protocol_version: ADAPTER_PROTOCOL_VERSION_0_2.to_owned(),
            target,
            plan,
            realization_spec,
            extensions: BTreeMap::new(),
        };
        request.normalize()?;
        Ok(request)
    }

    pub fn from_json(input: &str) -> Result<Self, RealizationRequestError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| RealizationRequestError::InvalidRequest(error.to_string()))?;
        reject_transport_markers(&value).map_err(RealizationRequestError::Protocol)?;
        reject_v02_forbidden_fields(&value).map_err(RealizationRequestError::ForbiddenField)?;
        let mut request: Self = serde_json::from_value(value)
            .map_err(|error| RealizationRequestError::InvalidRequest(error.to_string()))?;
        request.normalize()?;
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationRequestError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        canonical_request_json(&normalized)
    }

    pub fn canonical_plan_identity(&self) -> Result<String, RealizationRequestError> {
        self.plan.to_canonical_json().map_err(|error| {
            RealizationRequestError::InvalidPublicPayload {
                field: "plan",
                detail: error.to_string(),
            }
        })
    }

    pub fn canonical_realization_identity(&self) -> Result<String, RealizationRequestError> {
        self.realization_spec.to_canonical_json().map_err(|error| {
            RealizationRequestError::InvalidPublicPayload {
                field: "realization_spec",
                detail: error.to_string(),
            }
        })
    }

    fn normalize(&mut self) -> Result<(), RealizationRequestError> {
        normalize_realization_request(
            &self.adapter_protocol_version,
            &mut self.target,
            &mut self.plan,
            &mut self.realization_spec,
            &self.extensions,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutePlanRequestV02 {
    pub adapter_protocol_version: String,
    pub target: BackendTargetDtoV02,
    pub plan: MappingPlanDtoV02,
    pub realization_spec: RealizationSpecDtoV02,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ExecutePlanRequestV02 {
    pub fn new(
        target: BackendTargetDtoV02,
        plan: MappingPlanDtoV02,
        realization_spec: RealizationSpecDtoV02,
    ) -> Result<Self, RealizationRequestError> {
        let mut request = Self {
            adapter_protocol_version: ADAPTER_PROTOCOL_VERSION_0_2.to_owned(),
            target,
            plan,
            realization_spec,
            extensions: BTreeMap::new(),
        };
        request.normalize()?;
        Ok(request)
    }

    pub fn from_json(input: &str) -> Result<Self, RealizationRequestError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| RealizationRequestError::InvalidRequest(error.to_string()))?;
        reject_transport_markers(&value).map_err(RealizationRequestError::Protocol)?;
        reject_v02_forbidden_fields(&value).map_err(RealizationRequestError::ForbiddenField)?;
        let mut request: Self = serde_json::from_value(value)
            .map_err(|error| RealizationRequestError::InvalidRequest(error.to_string()))?;
        request.normalize()?;
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationRequestError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        canonical_request_json(&normalized)
    }

    pub fn canonical_plan_identity(&self) -> Result<String, RealizationRequestError> {
        self.plan.to_canonical_json().map_err(|error| {
            RealizationRequestError::InvalidPublicPayload {
                field: "plan",
                detail: error.to_string(),
            }
        })
    }

    pub fn canonical_realization_identity(&self) -> Result<String, RealizationRequestError> {
        self.realization_spec.to_canonical_json().map_err(|error| {
            RealizationRequestError::InvalidPublicPayload {
                field: "realization_spec",
                detail: error.to_string(),
            }
        })
    }

    fn normalize(&mut self) -> Result<(), RealizationRequestError> {
        normalize_realization_request(
            &self.adapter_protocol_version,
            &mut self.target,
            &mut self.plan,
            &mut self.realization_spec,
            &self.extensions,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatePlanResponseV02 {
    pub adapter_protocol_version: String,
    pub target_compatible: bool,
    pub capabilities_satisfied: bool,
    pub preflight: PreflightOutcome,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ValidatePlanResponseV02 {
    pub fn new(
        target_compatible: bool,
        capabilities_satisfied: bool,
        preflight: PreflightOutcome,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Self, RealizationResponseError> {
        let mut response = Self {
            adapter_protocol_version: ADAPTER_PROTOCOL_VERSION_0_2.to_owned(),
            target_compatible,
            capabilities_satisfied,
            preflight,
            diagnostics,
            extensions: BTreeMap::new(),
        };
        response.normalize()?;
        Ok(response)
    }

    pub fn accepted() -> Self {
        Self::new(true, true, PreflightOutcome::Accepted, Vec::new())
            .expect("accepted Protocol 0.2 preflight shape is internally consistent")
    }

    pub fn from_json(input: &str) -> Result<Self, RealizationResponseError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))?;
        reject_transport_markers(&value).map_err(RealizationResponseError::Protocol)?;
        reject_v02_forbidden_fields(&value).map_err(RealizationResponseError::ForbiddenField)?;
        let mut response: Self = serde_json::from_value(value)
            .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))?;
        response.normalize()?;
        Ok(response)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationResponseError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        canonical_response_json(&normalized)
    }

    fn normalize(&mut self) -> Result<(), RealizationResponseError> {
        require_v02_response_version(&self.adapter_protocol_version)?;
        reject_v02_response_value(self)?;
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            diagnostic.validate_shape().map_err(|error| {
                RealizationResponseError::MalformedDiagnostic {
                    index,
                    detail: error.to_string(),
                }
            })?;
        }
        self.diagnostics = ValidationReport::new(self.diagnostics.clone()).diagnostics;
        validate_v02_preflight_consistency(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutePlanResponseV02 {
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

impl ExecutePlanResponseV02 {
    pub fn from_json(input: &str) -> Result<Self, RealizationResponseError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))?;
        reject_transport_markers(&value).map_err(RealizationResponseError::Protocol)?;
        reject_v02_forbidden_fields(&value).map_err(RealizationResponseError::ForbiddenField)?;
        let mut response: Self = serde_json::from_value(value)
            .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))?;
        response.normalize_local()?;
        Ok(response)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationResponseError> {
        let mut normalized = self.clone();
        normalized.normalize_local()?;
        canonical_response_json(&normalized)
    }

    pub fn validate_against(
        &mut self,
        request: &ExecutePlanRequestV02,
    ) -> Result<(), RealizationResponseError> {
        self.normalize_local()?;
        let shadow_request = shadow_execute_request_v01(request)?;
        let mut shadow_response = self.shadow_v01()?;
        shadow_response
            .validate_against(&shadow_request)
            .map_err(|error| {
                RealizationResponseError::InheritedExecutionInvariant(error.to_string())
            })?;
        self.copy_normalized_shadow(&shadow_response);
        validate_realization_provenance_separation(self, request)
    }

    fn normalize_local(&mut self) -> Result<(), RealizationResponseError> {
        require_v02_response_version(&self.adapter_protocol_version)?;
        reject_v02_response_value(self)?;
        let shadow = self.shadow_v01()?;
        self.copy_normalized_shadow(&shadow);
        Ok(())
    }

    fn shadow_v01(&self) -> Result<ExecutePlanResponse, RealizationResponseError> {
        let mut value = serde_json::to_value(self)
            .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))?;
        value["adapter_protocol_version"] = Value::String("0.1".to_owned());
        ExecutePlanResponse::from_json(
            &serde_json::to_string(&value)
                .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))?,
        )
        .map_err(|error| RealizationResponseError::InheritedExecutionInvariant(error.to_string()))
    }

    fn copy_normalized_shadow(&mut self, shadow: &ExecutePlanResponse) {
        self.execution = shadow.execution;
        self.execution_batches = shadow.execution_batches.clone();
        self.action_reports = shadow.action_reports.clone();
        self.effects = shadow.effects.clone();
        self.diagnostics = shadow.diagnostics.clone();
        self.provenance = shadow.provenance.clone();
        self.extensions = shadow.extensions.clone();
    }
}

fn normalize_realization_request(
    adapter_protocol_version: &str,
    target: &mut BackendTargetDtoV02,
    plan: &mut MappingPlanDtoV02,
    realization_spec: &mut RealizationSpecDtoV02,
    extensions: &Extensions,
) -> Result<(), RealizationRequestError> {
    require_v02_protocol_version(adapter_protocol_version)?;

    let extension_value = Value::Object(
        extensions
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    reject_transport_markers(&extension_value).map_err(RealizationRequestError::Protocol)?;
    reject_v02_forbidden_fields(&extension_value)
        .map_err(RealizationRequestError::ForbiddenField)?;

    if target.public_contract_version != plan.public_contract_version
        || target.public_contract_version != realization_spec.public_contract_version
    {
        return Err(RealizationRequestError::IncoherentPublicContractVersions {
            target: target.public_contract_version.clone(),
            plan: plan.public_contract_version.clone(),
            realization_spec: realization_spec.public_contract_version.clone(),
        });
    }
    if target.public_contract_version != PUBLIC_CONTRACT_VERSION_0_2 {
        return Err(RealizationRequestError::UnsupportedPublicContractVersion(
            target.public_contract_version.clone(),
        ));
    }

    *target = BackendTargetDtoV02::from_json(&target.to_canonical_json().map_err(|error| {
        RealizationRequestError::InvalidPublicPayload {
            field: "target",
            detail: error.to_string(),
        }
    })?)
    .map_err(|error| RealizationRequestError::InvalidPublicPayload {
        field: "target",
        detail: error.to_string(),
    })?;

    *plan = MappingPlanDtoV02::from_json(&plan.to_canonical_json().map_err(|error| {
        RealizationRequestError::InvalidPublicPayload {
            field: "plan",
            detail: error.to_string(),
        }
    })?)
    .map_err(|error| RealizationRequestError::InvalidPublicPayload {
        field: "plan",
        detail: error.to_string(),
    })?;

    *realization_spec =
        RealizationSpecDtoV02::from_json(&realization_spec.to_canonical_json().map_err(
            |error| RealizationRequestError::InvalidPublicPayload {
                field: "realization_spec",
                detail: error.to_string(),
            },
        )?)
        .map_err(|error| RealizationRequestError::InvalidPublicPayload {
            field: "realization_spec",
            detail: error.to_string(),
        })?;

    realization_spec
        .validate_against_plan(plan)
        .map_err(|error| RealizationRequestError::PlanRealizationMismatch(error.to_string()))
}

fn require_v02_protocol_version(version: &str) -> Result<(), RealizationRequestError> {
    let parsed =
        AdapterProtocolVersion::parse(version).map_err(RealizationRequestError::Protocol)?;
    if parsed != AdapterProtocolVersion::realization_v02() {
        return Err(RealizationRequestError::UnsupportedProtocolVersion(
            version.to_owned(),
        ));
    }
    Ok(())
}

fn require_v02_response_version(version: &str) -> Result<(), RealizationResponseError> {
    let parsed =
        AdapterProtocolVersion::parse(version).map_err(RealizationResponseError::Protocol)?;
    if parsed != AdapterProtocolVersion::realization_v02() {
        return Err(RealizationResponseError::UnsupportedProtocolVersion(
            version.to_owned(),
        ));
    }
    Ok(())
}

fn validate_v02_preflight_consistency(
    response: &ValidatePlanResponseV02,
) -> Result<(), RealizationResponseError> {
    let error_codes = response
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    if !response.target_compatible {
        if response.preflight != PreflightOutcome::Rejected {
            return Err(RealizationResponseError::InconsistentPreflight(
                "target mismatch requires preflight=rejected".to_owned(),
            ));
        }
        if !error_codes.contains(&DIAGNOSTIC_TARGET_MISMATCH) {
            return Err(RealizationResponseError::InconsistentPreflight(
                "target mismatch requires adapter.target_mismatch diagnostic".to_owned(),
            ));
        }
    }

    if !response.capabilities_satisfied {
        if response.preflight != PreflightOutcome::Rejected {
            return Err(RealizationResponseError::InconsistentPreflight(
                "missing capability requires preflight=rejected".to_owned(),
            ));
        }
        if !error_codes.contains(&DIAGNOSTIC_MISSING_CAPABILITY) {
            return Err(RealizationResponseError::InconsistentPreflight(
                "missing capability requires adapter.missing_capability diagnostic".to_owned(),
            ));
        }
    }

    match response.preflight {
        PreflightOutcome::Accepted => {
            if !response.target_compatible || !response.capabilities_satisfied {
                return Err(RealizationResponseError::InconsistentPreflight(
                    "accepted preflight requires compatible target and satisfied capabilities"
                        .to_owned(),
                ));
            }
            if !error_codes.is_empty() {
                return Err(RealizationResponseError::InconsistentPreflight(
                    "accepted preflight cannot contain error diagnostics".to_owned(),
                ));
            }
        }
        PreflightOutcome::Rejected => {
            if error_codes.is_empty() {
                return Err(RealizationResponseError::InconsistentPreflight(
                    "rejected preflight requires an error diagnostic".to_owned(),
                ));
            }
            if response.target_compatible
                && response.capabilities_satisfied
                && !error_codes.iter().any(|code| {
                    matches!(
                        *code,
                        DIAGNOSTIC_UNSUPPORTED_ACTION | DIAGNOSTIC_PRECONDITION_REJECTED
                    )
                })
            {
                return Err(RealizationResponseError::InconsistentPreflight(
                    "adapter-side rejection requires unsupported-action or precondition diagnostic"
                        .to_owned(),
                ));
            }
        }
        PreflightOutcome::Unavailable => {
            if !response.target_compatible || !response.capabilities_satisfied {
                return Err(RealizationResponseError::InconsistentPreflight(
                    "unavailable preflight still requires compatible target and satisfied capabilities"
                        .to_owned(),
                ));
            }
            if !error_codes.contains(&DIAGNOSTIC_TRANSIENT_UNAVAILABLE) {
                return Err(RealizationResponseError::InconsistentPreflight(
                    "unavailable preflight requires adapter.transient_unavailable diagnostic"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn shadow_execute_request_v01(
    request: &ExecutePlanRequestV02,
) -> Result<ExecutePlanRequest, RealizationResponseError> {
    let target = shadow_public_contract_v01::<BackendTargetDto>(&request.target, "target")?;
    let plan = shadow_public_contract_v01::<MappingPlanDto>(&request.plan, "plan")?;
    ExecutePlanRequest::new(target, plan)
        .map_err(|error| RealizationResponseError::InheritedExecutionInvariant(error.to_string()))
}

fn shadow_public_contract_v01<T>(
    value: &impl Serialize,
    field: &'static str,
) -> Result<T, RealizationResponseError>
where
    T: for<'de> Deserialize<'de>,
{
    let mut value = serde_json::to_value(value).map_err(|error| {
        RealizationResponseError::InvalidPublicPayload {
            field,
            detail: error.to_string(),
        }
    })?;
    value["public_contract_version"] = Value::String(PUBLIC_CONTRACT_VERSION.to_owned());
    serde_json::from_value(value).map_err(|error| RealizationResponseError::InvalidPublicPayload {
        field,
        detail: error.to_string(),
    })
}

fn validate_realization_provenance_separation(
    response: &ExecutePlanResponseV02,
    request: &ExecutePlanRequestV02,
) -> Result<(), RealizationResponseError> {
    let semantic_references = realization_semantic_references(request);
    let mut opaque_references = Vec::new();
    collect_opaque_references(response.provenance.as_ref(), &mut opaque_references);
    for report in &response.action_reports {
        collect_opaque_references(report.provenance.as_ref(), &mut opaque_references);
    }
    for reference in opaque_references {
        if semantic_references.contains(reference.as_str()) {
            return Err(RealizationResponseError::OpaqueReferenceUsedAsSemanticIdentity(reference));
        }
    }
    Ok(())
}

fn realization_semantic_references(request: &ExecutePlanRequestV02) -> BTreeSet<String> {
    let mut references = BTreeSet::new();
    references.insert(request.target.target.clone());
    references.insert(request.realization_spec.source_model.clone());
    references.extend(request.plan.actions.iter().map(|action| action.id.clone()));
    for entity in &request.realization_spec.entities {
        references.insert(entity.id.clone());
        for parameter in &entity.parameters {
            references.insert(parameter.semantic_parameter.clone());
            references.insert(parameter.quantity.unit.clone());
        }
    }
    for scope in &request.realization_spec.scopes {
        references.insert(scope.id.clone());
        references.extend(scope.members.iter().cloned());
    }
    for relation in &request.realization_spec.relations {
        references.insert(relation.source.clone());
        references.insert(relation.target.clone());
    }
    for binding in &request.realization_spec.action_bindings {
        references.insert(binding.action_id.clone());
        references.extend(binding.scopes.iter().cloned());
        for subject in &binding.subjects {
            match subject {
                MappingSubjectDto::Entity { id, .. } => {
                    references.insert(id.clone());
                }
                MappingSubjectDto::Relation { source, target, .. } => {
                    references.insert(source.clone());
                    references.insert(target.clone());
                }
            }
        }
    }
    references
}

fn collect_opaque_references(
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

fn reject_v02_response_value<T: Serialize>(response: &T) -> Result<(), RealizationResponseError> {
    let value = serde_json::to_value(response)
        .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))?;
    reject_transport_markers(&value).map_err(RealizationResponseError::Protocol)?;
    reject_v02_forbidden_fields(&value).map_err(RealizationResponseError::ForbiddenField)
}

fn reject_v02_forbidden_fields(value: &Value) -> Result<(), String> {
    const FORBIDDEN: &[&str] = &[
        "validation_token",
        "acceptance_id",
        "lease_id",
        "plan_hash",
        "plan_digest",
        "execution_authority",
        "replay_authority",
        "retry_authority",
        "backend_native_id",
        "backend_object",
        "native_object",
        "solver_object",
    ];

    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if FORBIDDEN.contains(&key.as_str()) {
                    return Err(key.clone());
                }
                reject_v02_forbidden_fields(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_v02_forbidden_fields(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn canonical_request_json<T: Serialize>(value: &T) -> Result<String, RealizationRequestError> {
    let value = serde_json::to_value(value)
        .map_err(|error| RealizationRequestError::InvalidRequest(error.to_string()))?;
    serde_json::to_string(&canonicalize_value(value))
        .map_err(|error| RealizationRequestError::InvalidRequest(error.to_string()))
}

fn canonical_response_json<T: Serialize>(value: &T) -> Result<String, RealizationResponseError> {
    let value = serde_json::to_value(value)
        .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))?;
    serde_json::to_string(&canonicalize_value(value))
        .map_err(|error| RealizationResponseError::InvalidResponse(error.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealizationRequestError {
    Protocol(ProtocolError),
    InvalidRequest(String),
    InvalidPublicPayload {
        field: &'static str,
        detail: String,
    },
    UnsupportedProtocolVersion(String),
    UnsupportedPublicContractVersion(String),
    IncoherentPublicContractVersions {
        target: String,
        plan: String,
        realization_spec: String,
    },
    PlanRealizationMismatch(String),
    ForbiddenField(String),
}

impl Display for RealizationRequestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(error) => write!(formatter, "{error}"),
            Self::InvalidRequest(detail) => {
                write!(formatter, "invalid Adapter Protocol 0.2 realization request: {detail}")
            }
            Self::InvalidPublicPayload { field, detail } => {
                write!(formatter, "invalid Public Contract 0.2 {field}: {detail}")
            }
            Self::UnsupportedProtocolVersion(version) => {
                write!(formatter, "unsupported Adapter Protocol version for realization request: {version}")
            }
            Self::UnsupportedPublicContractVersion(version) => {
                write!(formatter, "unsupported Public Contract version for realization request: {version}")
            }
            Self::IncoherentPublicContractVersions {
                target,
                plan,
                realization_spec,
            } => write!(
                formatter,
                "Adapter Protocol 0.2 request Public Contract versions differ: target={target}, plan={plan}, realization_spec={realization_spec}"
            ),
            Self::PlanRealizationMismatch(detail) => {
                write!(formatter, "MappingPlan/RealizationSpec mismatch: {detail}")
            }
            Self::ForbiddenField(field) => write!(
                formatter,
                "field is forbidden from Adapter Protocol 0.2 realization request semantics: {field}"
            ),
        }
    }
}

impl Error for RealizationRequestError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealizationResponseError {
    Protocol(ProtocolError),
    InvalidResponse(String),
    InvalidPublicPayload { field: &'static str, detail: String },
    UnsupportedProtocolVersion(String),
    MalformedDiagnostic { index: usize, detail: String },
    InconsistentPreflight(String),
    InheritedExecutionInvariant(String),
    OpaqueReferenceUsedAsSemanticIdentity(String),
    ForbiddenField(String),
}

impl Display for RealizationResponseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(error) => write!(formatter, "{error}"),
            Self::InvalidResponse(detail) => {
                write!(formatter, "invalid Adapter Protocol 0.2 realization response: {detail}")
            }
            Self::InvalidPublicPayload { field, detail } => {
                write!(formatter, "invalid inherited Public Contract payload {field}: {detail}")
            }
            Self::UnsupportedProtocolVersion(version) => {
                write!(formatter, "unsupported Adapter Protocol version for realization response: {version}")
            }
            Self::MalformedDiagnostic { index, detail } => {
                write!(formatter, "malformed Protocol 0.2 diagnostic at index {index}: {detail}")
            }
            Self::InconsistentPreflight(detail) => {
                write!(formatter, "inconsistent Protocol 0.2 validate_plan response: {detail}")
            }
            Self::InheritedExecutionInvariant(detail) => {
                write!(formatter, "Protocol 0.2 execution violates inherited operation semantics: {detail}")
            }
            Self::OpaqueReferenceUsedAsSemanticIdentity(reference) => write!(
                formatter,
                "opaque execution reference is reused as Public Contract 0.2 semantic identity: {reference}"
            ),
            Self::ForbiddenField(field) => write!(
                formatter,
                "field is forbidden from Adapter Protocol 0.2 realization response semantics: {field}"
            ),
        }
    }
}

impl Error for RealizationResponseError {}
