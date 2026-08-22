use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{canonicalize_value, reject_transport_markers, Extensions};

pub const FAILURE_COMPATIBILITY_NOT_ESTABLISHED: &str = "protocol.compatibility_not_established";
pub const FAILURE_UNSUPPORTED_ADAPTER_PROTOCOL: &str = "protocol.unsupported_adapter_protocol";
pub const FAILURE_UNSUPPORTED_PUBLIC_CONTRACT: &str = "protocol.unsupported_public_contract";
pub const FAILURE_MALFORMED_BOOTSTRAP: &str = "protocol.malformed_bootstrap";
pub const FAILURE_INVALID_REQUEST: &str = "protocol.invalid_request";
pub const FAILURE_OPERATIONAL: &str = "adapter.operational_failure";

pub const PRECONDITION_ALREADY_REALIZED: &str = "already_realized";
pub const PRECONDITION_PARTIAL_PRIOR_EXECUTION: &str = "partial_prior_execution";
pub const PRECONDITION_UNRESOLVED_PREREQUISITE: &str = "unresolved_prerequisite";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Compatibility,
    InvalidRequest,
    Operational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectEvidence {
    None,
    MayHaveOccurred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolOperation {
    DescribeAdapter,
    ValidatePlan,
    ExecutePlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanOperation {
    ValidatePlan,
    ExecutePlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanOperationIdempotency {
    IdempotentForEquivalentInputAndState,
    NonIdempotentByDefault,
}

pub const fn plan_operation_idempotency(operation: PlanOperation) -> PlanOperationIdempotency {
    match operation {
        PlanOperation::ValidatePlan => {
            PlanOperationIdempotency::IdempotentForEquivalentInputAndState
        }
        PlanOperation::ExecutePlan => PlanOperationIdempotency::NonIdempotentByDefault,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolFailure {
    pub category: FailureCategory,
    pub code: String,
    pub detail: String,
    pub side_effects: SideEffectEvidence,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ProtocolFailure {
    pub fn new(
        category: FailureCategory,
        code: impl Into<String>,
        detail: impl Into<String>,
        side_effects: SideEffectEvidence,
    ) -> Result<Self, ProtocolFailureError> {
        let mut failure = Self {
            category,
            code: code.into(),
            detail: detail.into(),
            side_effects,
            extensions: Extensions::new(),
        };
        failure.normalize()?;
        Ok(failure)
    }

    pub fn compatibility_not_established(
        detail: impl Into<String>,
    ) -> Result<Self, ProtocolFailureError> {
        Self::new(
            FailureCategory::Compatibility,
            FAILURE_COMPATIBILITY_NOT_ESTABLISHED,
            detail,
            SideEffectEvidence::None,
        )
    }

    pub fn unsupported_adapter_protocol(
        detail: impl Into<String>,
    ) -> Result<Self, ProtocolFailureError> {
        Self::new(
            FailureCategory::Compatibility,
            FAILURE_UNSUPPORTED_ADAPTER_PROTOCOL,
            detail,
            SideEffectEvidence::None,
        )
    }

    pub fn unsupported_public_contract(
        detail: impl Into<String>,
    ) -> Result<Self, ProtocolFailureError> {
        Self::new(
            FailureCategory::Compatibility,
            FAILURE_UNSUPPORTED_PUBLIC_CONTRACT,
            detail,
            SideEffectEvidence::None,
        )
    }

    pub fn malformed_bootstrap(detail: impl Into<String>) -> Result<Self, ProtocolFailureError> {
        Self::new(
            FailureCategory::InvalidRequest,
            FAILURE_MALFORMED_BOOTSTRAP,
            detail,
            SideEffectEvidence::None,
        )
    }

    pub fn invalid_request(detail: impl Into<String>) -> Result<Self, ProtocolFailureError> {
        Self::new(
            FailureCategory::InvalidRequest,
            FAILURE_INVALID_REQUEST,
            detail,
            SideEffectEvidence::None,
        )
    }

    pub fn operational(
        detail: impl Into<String>,
        side_effects: SideEffectEvidence,
    ) -> Result<Self, ProtocolFailureError> {
        Self::new(
            FailureCategory::Operational,
            FAILURE_OPERATIONAL,
            detail,
            side_effects,
        )
    }

    pub fn from_json(input: &str) -> Result<Self, ProtocolFailureError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| ProtocolFailureError::InvalidJson(error.to_string()))?;
        reject_transport_markers(&value)
            .map_err(|error| ProtocolFailureError::ForbiddenField(error.to_string()))?;
        reject_failure_root_fields(&value)?;
        let mut failure: Self = serde_json::from_value(value)
            .map_err(|error| ProtocolFailureError::InvalidShape(error.to_string()))?;
        failure.normalize()?;
        Ok(failure)
    }

    pub fn to_canonical_json(&self) -> Result<String, ProtocolFailureError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        let value = serde_json::to_value(normalized)
            .map_err(|error| ProtocolFailureError::InvalidShape(error.to_string()))?;
        serde_json::to_string(&canonicalize_value(value))
            .map_err(|error| ProtocolFailureError::InvalidShape(error.to_string()))
    }

    pub fn validate_for(&self, operation: ProtocolOperation) -> Result<(), ProtocolFailureError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        if normalized.category == FailureCategory::Operational
            && operation != ProtocolOperation::ExecutePlan
            && normalized.side_effects != SideEffectEvidence::None
        {
            return Err(ProtocolFailureError::InconsistentSideEffectEvidence(
                "describe_adapter/validate_plan operational failures must prove side_effects=none"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    fn normalize(&mut self) -> Result<(), ProtocolFailureError> {
        require_failure_code(&self.code)?;
        if self.detail.is_empty() {
            return Err(ProtocolFailureError::EmptyDetail);
        }
        reject_failure_extension_fields(&self.extensions)?;
        reject_transport_markers(
            &serde_json::to_value(&self.extensions)
                .map_err(|error| ProtocolFailureError::InvalidShape(error.to_string()))?,
        )
        .map_err(|error| ProtocolFailureError::ForbiddenField(error.to_string()))?;

        if matches!(
            self.category,
            FailureCategory::Compatibility | FailureCategory::InvalidRequest
        ) && self.side_effects != SideEffectEvidence::None
        {
            return Err(ProtocolFailureError::InconsistentSideEffectEvidence(
                "compatibility/invalid-request failure must guarantee side_effects=none".to_owned(),
            ));
        }

        validate_known_code_category(self.category, &self.code)
    }
}

pub fn conservative_execute_side_effect_evidence(
    observed_failure: Option<&ProtocolFailure>,
) -> SideEffectEvidence {
    match observed_failure {
        Some(failure) if failure.side_effects == SideEffectEvidence::None => {
            SideEffectEvidence::None
        }
        _ => SideEffectEvidence::MayHaveOccurred,
    }
}

fn validate_known_code_category(
    category: FailureCategory,
    code: &str,
) -> Result<(), ProtocolFailureError> {
    let expected = match code {
        FAILURE_COMPATIBILITY_NOT_ESTABLISHED
        | FAILURE_UNSUPPORTED_ADAPTER_PROTOCOL
        | FAILURE_UNSUPPORTED_PUBLIC_CONTRACT => Some(FailureCategory::Compatibility),
        FAILURE_MALFORMED_BOOTSTRAP | FAILURE_INVALID_REQUEST => {
            Some(FailureCategory::InvalidRequest)
        }
        FAILURE_OPERATIONAL => Some(FailureCategory::Operational),
        _ => None,
    };

    if let Some(expected) = expected {
        if category != expected {
            return Err(ProtocolFailureError::CodeCategoryMismatch {
                code: code.to_owned(),
                category,
                expected,
            });
        }
    }
    Ok(())
}

fn require_failure_code(code: &str) -> Result<(), ProtocolFailureError> {
    let valid = code.contains('.')
        && code.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        });
    if valid {
        Ok(())
    } else {
        Err(ProtocolFailureError::InvalidCode(code.to_owned()))
    }
}

fn reject_failure_root_fields(value: &Value) -> Result<(), ProtocolFailureError> {
    let object = value.as_object().ok_or_else(|| {
        ProtocolFailureError::InvalidShape("failure root must be an object".to_owned())
    })?;
    for key in object.keys() {
        if failure_forbidden_root_field(key) {
            return Err(ProtocolFailureError::ForbiddenField(key.clone()));
        }
    }
    Ok(())
}

fn reject_failure_extension_fields(extensions: &Extensions) -> Result<(), ProtocolFailureError> {
    for key in extensions.keys() {
        if failure_forbidden_root_field(key) {
            return Err(ProtocolFailureError::ForbiddenField(key.clone()));
        }
    }
    Ok(())
}

fn failure_forbidden_root_field(key: &str) -> bool {
    matches!(
        key,
        "adapter_protocol_version"
            | "retryable"
            | "safe_to_retry"
            | "idempotency_key"
            | "deduplication_token"
            | "dedup_token"
            | "resume_token"
            | "validation_token"
            | "acceptance_id"
            | "lease_id"
            | "replay"
            | "status"
            | "evaluation_status"
            | "lifecycle_status"
            | "comparison"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolFailureError {
    InvalidJson(String),
    InvalidShape(String),
    EmptyDetail,
    InvalidCode(String),
    ForbiddenField(String),
    CodeCategoryMismatch {
        code: String,
        category: FailureCategory,
        expected: FailureCategory,
    },
    InconsistentSideEffectEvidence(String),
}

impl Display for ProtocolFailureError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(detail) => write!(formatter, "invalid protocol failure JSON: {detail}"),
            Self::InvalidShape(detail) => write!(formatter, "invalid protocol failure shape: {detail}"),
            Self::EmptyDetail => write!(formatter, "protocol failure detail must not be empty"),
            Self::InvalidCode(code) => write!(formatter, "invalid protocol failure code: {code}"),
            Self::ForbiddenField(field) => {
                write!(formatter, "field is forbidden from Protocol 0.1 failure semantics: {field}")
            }
            Self::CodeCategoryMismatch {
                code,
                category,
                expected,
            } => write!(
                formatter,
                "protocol failure code/category mismatch: code={code}, category={category:?}, expected={expected:?}"
            ),
            Self::InconsistentSideEffectEvidence(detail) => {
                write!(formatter, "inconsistent protocol failure side-effect evidence: {detail}")
            }
        }
    }
}

impl Error for ProtocolFailureError {}
