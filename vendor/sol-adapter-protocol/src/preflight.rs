use serde::{Deserialize, Serialize};
use serde_json::Value;
use sol_public_contract::{
    BackendTargetDto, Diagnostic, DiagnosticSeverity, Extensions, MappingPlanDto, ValidationReport,
    PUBLIC_CONTRACT_VERSION,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    canonicalize_value, reject_transport_markers, AdapterProtocolVersion, ProtocolError,
    ADAPTER_PROTOCOL_VERSION,
};

pub const DIAGNOSTIC_TARGET_MISMATCH: &str = "adapter.target_mismatch";
pub const DIAGNOSTIC_MISSING_CAPABILITY: &str = "adapter.missing_capability";
pub const DIAGNOSTIC_UNSUPPORTED_ACTION: &str = "adapter.unsupported_action";
pub const DIAGNOSTIC_PRECONDITION_REJECTED: &str = "adapter.precondition_rejected";
pub const DIAGNOSTIC_TRANSIENT_UNAVAILABLE: &str = "adapter.transient_unavailable";
pub const DIAGNOSTIC_CONTEXT_EXTENSION: &str = "sol_adapter_protocol_context";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AdapterProtocolDiagnosticContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precondition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatePlanRequest {
    pub adapter_protocol_version: String,
    pub target: BackendTargetDto,
    pub plan: MappingPlanDto,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ValidatePlanRequest {
    pub fn new(target: BackendTargetDto, plan: MappingPlanDto) -> Result<Self, PreflightError> {
        let mut request = Self {
            adapter_protocol_version: ADAPTER_PROTOCOL_VERSION.to_owned(),
            target,
            plan,
            extensions: BTreeMap::new(),
        };
        request.normalize()?;
        Ok(request)
    }

    pub fn from_json(input: &str) -> Result<Self, PreflightError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| PreflightError::InvalidRequest(error.to_string()))?;
        reject_transport_markers(&value).map_err(PreflightError::Protocol)?;
        reject_authority_and_backend_native_fields(&value)?;
        let mut request: Self = serde_json::from_value(value)
            .map_err(|error| PreflightError::InvalidRequest(error.to_string()))?;
        request.normalize()?;
        Ok(request)
    }

    pub fn to_canonical_json(&self) -> Result<String, PreflightError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        let value = serde_json::to_value(normalized)
            .map_err(|error| PreflightError::InvalidRequest(error.to_string()))?;
        serde_json::to_string(&canonicalize_value(value))
            .map_err(|error| PreflightError::InvalidRequest(error.to_string()))
    }

    pub fn canonical_plan_identity(&self) -> Result<String, PreflightError> {
        self.plan
            .to_canonical_json()
            .map_err(|error| PreflightError::InvalidPublicPayload(error.to_string()))
    }

    fn normalize(&mut self) -> Result<(), PreflightError> {
        require_current_protocol_version(&self.adapter_protocol_version)?;
        reject_authority_and_backend_native_fields(
            &serde_json::to_value(&*self)
                .map_err(|error| PreflightError::InvalidRequest(error.to_string()))?,
        )?;

        self.target = BackendTargetDto::from_json(
            &self
                .target
                .to_canonical_json()
                .map_err(|error| PreflightError::InvalidPublicPayload(error.to_string()))?,
        )
        .map_err(|error| PreflightError::InvalidPublicPayload(error.to_string()))?;
        self.plan = MappingPlanDto::from_json(
            &self
                .plan
                .to_canonical_json()
                .map_err(|error| PreflightError::InvalidPublicPayload(error.to_string()))?,
        )
        .map_err(|error| PreflightError::InvalidPublicPayload(error.to_string()))?;

        if self.target.public_contract_version != self.plan.public_contract_version {
            return Err(PreflightError::IncoherentPublicContractVersions {
                target: self.target.public_contract_version.clone(),
                plan: self.plan.public_contract_version.clone(),
            });
        }
        if self.target.public_contract_version != PUBLIC_CONTRACT_VERSION {
            return Err(PreflightError::UnsupportedPublicContractVersion(
                self.target.public_contract_version.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightOutcome {
    Accepted,
    Rejected,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatePlanResponse {
    pub adapter_protocol_version: String,
    pub target_compatible: bool,
    pub capabilities_satisfied: bool,
    pub preflight: PreflightOutcome,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ValidatePlanResponse {
    pub fn new(
        target_compatible: bool,
        capabilities_satisfied: bool,
        preflight: PreflightOutcome,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Self, PreflightError> {
        let mut response = Self {
            adapter_protocol_version: ADAPTER_PROTOCOL_VERSION.to_owned(),
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
            .expect("accepted preflight shape is internally consistent")
    }

    pub fn from_json(input: &str) -> Result<Self, PreflightError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| PreflightError::InvalidResponse(error.to_string()))?;
        reject_transport_markers(&value).map_err(PreflightError::Protocol)?;
        reject_authority_and_backend_native_fields(&value)?;
        let mut response: Self = serde_json::from_value(value)
            .map_err(|error| PreflightError::InvalidResponse(error.to_string()))?;
        response.normalize()?;
        Ok(response)
    }

    pub fn to_canonical_json(&self) -> Result<String, PreflightError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        let value = serde_json::to_value(normalized)
            .map_err(|error| PreflightError::InvalidResponse(error.to_string()))?;
        serde_json::to_string(&canonicalize_value(value))
            .map_err(|error| PreflightError::InvalidResponse(error.to_string()))
    }

    fn normalize(&mut self) -> Result<(), PreflightError> {
        require_current_protocol_version(&self.adapter_protocol_version)?;
        reject_authority_and_backend_native_fields(
            &serde_json::to_value(&*self)
                .map_err(|error| PreflightError::InvalidResponse(error.to_string()))?,
        )?;

        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            diagnostic
                .validate_shape()
                .map_err(|error| PreflightError::MalformedDiagnostic {
                    index,
                    reason: error.to_string(),
                })?;
        }
        self.diagnostics = ValidationReport::new(self.diagnostics.clone()).diagnostics;
        validate_preflight_consistency(self)
    }
}

pub fn protocol_diagnostic(
    code: impl Into<String>,
    detail: impl Into<String>,
    context: AdapterProtocolDiagnosticContext,
) -> Result<Diagnostic, PreflightError> {
    let mut diagnostic = Diagnostic::error(code, None, detail);
    let value = serde_json::to_value(context)
        .map_err(|error| PreflightError::InvalidDiagnosticContext(error.to_string()))?;
    diagnostic
        .extensions
        .insert(DIAGNOSTIC_CONTEXT_EXTENSION.to_owned(), value);
    diagnostic
        .validate_shape()
        .map_err(|error| PreflightError::InvalidDiagnosticContext(error.to_string()))?;
    Ok(diagnostic)
}

pub fn diagnostic_context(
    diagnostic: &Diagnostic,
) -> Result<Option<AdapterProtocolDiagnosticContext>, PreflightError> {
    diagnostic
        .extensions
        .get(DIAGNOSTIC_CONTEXT_EXTENSION)
        .map(|value| {
            serde_json::from_value(value.clone())
                .map_err(|error| PreflightError::InvalidDiagnosticContext(error.to_string()))
        })
        .transpose()
}

fn require_current_protocol_version(version: &str) -> Result<(), PreflightError> {
    let parsed = AdapterProtocolVersion::parse(version).map_err(PreflightError::Protocol)?;
    if parsed != AdapterProtocolVersion::current() {
        return Err(PreflightError::UnsupportedProtocolVersion(
            version.to_owned(),
        ));
    }
    Ok(())
}

fn validate_preflight_consistency(response: &ValidatePlanResponse) -> Result<(), PreflightError> {
    let error_codes = response
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    if !response.target_compatible {
        if response.preflight != PreflightOutcome::Rejected {
            return Err(PreflightError::InconsistentResponse(
                "target mismatch requires preflight=rejected".to_owned(),
            ));
        }
        if !error_codes.contains(&DIAGNOSTIC_TARGET_MISMATCH) {
            return Err(PreflightError::InconsistentResponse(
                "target mismatch requires adapter.target_mismatch diagnostic".to_owned(),
            ));
        }
    }

    if !response.capabilities_satisfied {
        if response.preflight != PreflightOutcome::Rejected {
            return Err(PreflightError::InconsistentResponse(
                "missing capability requires preflight=rejected".to_owned(),
            ));
        }
        if !error_codes.contains(&DIAGNOSTIC_MISSING_CAPABILITY) {
            return Err(PreflightError::InconsistentResponse(
                "missing capability requires adapter.missing_capability diagnostic".to_owned(),
            ));
        }
    }

    match response.preflight {
        PreflightOutcome::Accepted => {
            if !response.target_compatible || !response.capabilities_satisfied {
                return Err(PreflightError::InconsistentResponse(
                    "accepted preflight requires compatible target and satisfied capabilities"
                        .to_owned(),
                ));
            }
            if !error_codes.is_empty() {
                return Err(PreflightError::InconsistentResponse(
                    "accepted preflight cannot contain error diagnostics".to_owned(),
                ));
            }
        }
        PreflightOutcome::Rejected => {
            if error_codes.is_empty() {
                return Err(PreflightError::InconsistentResponse(
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
                return Err(PreflightError::InconsistentResponse(
                    "adapter-side rejection requires unsupported-action or precondition diagnostic"
                        .to_owned(),
                ));
            }
        }
        PreflightOutcome::Unavailable => {
            if !response.target_compatible || !response.capabilities_satisfied {
                return Err(PreflightError::InconsistentResponse(
                    "unavailable preflight still requires compatible target and satisfied capabilities"
                        .to_owned(),
                ));
            }
            if !error_codes.contains(&DIAGNOSTIC_TRANSIENT_UNAVAILABLE) {
                return Err(PreflightError::InconsistentResponse(
                    "unavailable preflight requires adapter.transient_unavailable diagnostic"
                        .to_owned(),
                ));
            }
        }
    }

    Ok(())
}

fn reject_authority_and_backend_native_fields(value: &Value) -> Result<(), PreflightError> {
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
    ];

    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if FORBIDDEN.contains(&key.as_str()) {
                    return Err(PreflightError::ForbiddenField(key.clone()));
                }
                reject_authority_and_backend_native_fields(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_authority_and_backend_native_fields(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightError {
    Protocol(ProtocolError),
    InvalidRequest(String),
    InvalidResponse(String),
    InvalidPublicPayload(String),
    UnsupportedProtocolVersion(String),
    UnsupportedPublicContractVersion(String),
    IncoherentPublicContractVersions { target: String, plan: String },
    MalformedDiagnostic { index: usize, reason: String },
    InvalidDiagnosticContext(String),
    InconsistentResponse(String),
    ForbiddenField(String),
}

impl Display for PreflightError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Protocol(error) => write!(formatter, "{error}"),
            Self::InvalidRequest(detail) => write!(formatter, "invalid validate_plan request: {detail}"),
            Self::InvalidResponse(detail) => {
                write!(formatter, "invalid validate_plan response: {detail}")
            }
            Self::InvalidPublicPayload(detail) => {
                write!(formatter, "invalid reused Public Contract payload: {detail}")
            }
            Self::UnsupportedProtocolVersion(version) => {
                write!(formatter, "unsupported Adapter Protocol version: {version}")
            }
            Self::UnsupportedPublicContractVersion(version) => {
                write!(formatter, "unsupported Public Contract version: {version}")
            }
            Self::IncoherentPublicContractVersions { target, plan } => write!(
                formatter,
                "validate_plan payload Public Contract versions differ: target={target}, plan={plan}"
            ),
            Self::MalformedDiagnostic { index, reason } => {
                write!(formatter, "malformed preflight diagnostic at index {index}: {reason}")
            }
            Self::InvalidDiagnosticContext(detail) => {
                write!(formatter, "invalid adapter diagnostic context: {detail}")
            }
            Self::InconsistentResponse(detail) => {
                write!(formatter, "inconsistent validate_plan response: {detail}")
            }
            Self::ForbiddenField(field) => write!(
                formatter,
                "field is forbidden from Protocol 0.1 preflight semantics: {field}"
            ),
        }
    }
}

impl Error for PreflightError {}
