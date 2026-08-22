use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::model::{is_canonical_reference, EntityKindDto, Extensions, SimulationDto};
use crate::{CanonicalDocument, ContractDocumentError, PUBLIC_CONTRACT_VERSION};

pub const DIAGNOSTIC_INVALID_CANONICAL_ID: &str = "model.invalid_canonical_id";
pub const DIAGNOSTIC_DUPLICATE_ID: &str = "model.duplicate_id";
pub const DIAGNOSTIC_EMPTY_SCOPE: &str = "model.empty_scope";
pub const DIAGNOSTIC_DUPLICATE_SCOPE_MEMBER: &str = "model.duplicate_scope_member";
pub const DIAGNOSTIC_UNRESOLVED_SCOPE_MEMBER: &str = "model.unresolved_scope_member";
pub const DIAGNOSTIC_NON_SPATIAL_SCOPE_MEMBER: &str = "model.non_spatial_scope_member";
pub const DIAGNOSTIC_UNRESOLVED_RELATION_ENDPOINT: &str = "model.unresolved_relation_endpoint";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl DiagnosticSeverity {
    fn rank(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warning => 1,
            Self::Info => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    pub detail: String,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        subject: Option<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: DiagnosticSeverity::Error,
            subject,
            detail: detail.into(),
            extensions: BTreeMap::new(),
        }
    }

    pub fn validate_shape(&self) -> Result<(), DiagnosticShapeError> {
        if !valid_diagnostic_code(&self.code) {
            return Err(DiagnosticShapeError::InvalidCode(self.code.clone()));
        }
        if self.detail.is_empty() {
            return Err(DiagnosticShapeError::EmptyDetail);
        }
        if let Some(subject) = &self.subject {
            if !is_canonical_reference(subject) {
                return Err(DiagnosticShapeError::InvalidSubject(subject.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticShapeError {
    InvalidCode(String),
    EmptyDetail,
    InvalidSubject(String),
}

impl Display for DiagnosticShapeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCode(code) => write!(formatter, "invalid diagnostic code: {code}"),
            Self::EmptyDetail => write!(formatter, "diagnostic detail must not be empty"),
            Self::InvalidSubject(subject) => {
                write!(formatter, "invalid diagnostic subject reference: {subject}")
            }
        }
    }
}

impl Error for DiagnosticShapeError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub public_contract_version: String,
    pub valid: bool,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ValidationReport {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        let diagnostics = normalize_diagnostics(diagnostics);
        let valid = !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);

        Self {
            public_contract_version: PUBLIC_CONTRACT_VERSION.to_owned(),
            valid,
            diagnostics,
            extensions: BTreeMap::new(),
        }
    }

    pub fn from_json(input: &str) -> Result<Self, ValidationReportError> {
        let document = CanonicalDocument::parse(input).map_err(ValidationReportError::Contract)?;
        let mut report: Self = serde_json::from_value(document.value().clone())
            .map_err(|error| ValidationReportError::InvalidDto(error.to_string()))?;

        for (index, diagnostic) in report.diagnostics.iter().enumerate() {
            diagnostic.validate_shape().map_err(|error| {
                ValidationReportError::MalformedDiagnostic {
                    index,
                    reason: error.to_string(),
                }
            })?;
        }

        let normalized = normalize_diagnostics(report.diagnostics);
        let derived_valid = !normalized
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);
        if report.valid != derived_valid {
            return Err(ValidationReportError::InconsistentValidity {
                declared: report.valid,
                derived: derived_valid,
            });
        }

        report.diagnostics = normalized;
        Ok(report)
    }

    pub fn to_canonical_json(&self) -> Result<String, ValidationReportError> {
        let mut report = self.clone();
        for (index, diagnostic) in report.diagnostics.iter().enumerate() {
            diagnostic.validate_shape().map_err(|error| {
                ValidationReportError::MalformedDiagnostic {
                    index,
                    reason: error.to_string(),
                }
            })?;
        }
        report.diagnostics = normalize_diagnostics(report.diagnostics);
        report.valid = !report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error);

        let json = serde_json::to_string(&report)
            .map_err(|error| ValidationReportError::InvalidDto(error.to_string()))?;
        let document = CanonicalDocument::parse(&json).map_err(ValidationReportError::Contract)?;
        document
            .to_canonical_json()
            .map_err(|error| ValidationReportError::InvalidDto(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationReportError {
    Contract(ContractDocumentError),
    InvalidDto(String),
    MalformedDiagnostic { index: usize, reason: String },
    InconsistentValidity { declared: bool, derived: bool },
}

impl Display for ValidationReportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "{error}"),
            Self::InvalidDto(detail) => {
                write!(formatter, "invalid validation report DTO: {detail}")
            }
            Self::MalformedDiagnostic { index, reason } => {
                write!(formatter, "malformed diagnostic at index {index}: {reason}")
            }
            Self::InconsistentValidity { declared, derived } => write!(
                formatter,
                "validation report validity is inconsistent: declared={declared}, derived={derived}"
            ),
        }
    }
}

impl Error for ValidationReportError {}

pub fn validate_simulation(simulation: &SimulationDto) -> ValidationReport {
    let mut diagnostics = Vec::new();
    let mut nodes = BTreeMap::new();

    register_id(
        &simulation.model.id,
        PublicNodeKind::Other,
        &mut nodes,
        &mut diagnostics,
    );

    for entity in simulation
        .model
        .physics
        .iter()
        .chain(&simulation.model.mathematical)
        .chain(&simulation.model.constitutive)
        .chain(&simulation.model.spatial)
        .chain(&simulation.model.material)
        .chain(&simulation.model.conditions)
        .chain(&simulation.model.numerical)
        .chain(&simulation.model.observations)
    {
        let kind = if entity.kind == EntityKindDto::SpatialModel {
            PublicNodeKind::Spatial
        } else {
            PublicNodeKind::Other
        };
        register_id(&entity.id, kind, &mut nodes, &mut diagnostics);
    }

    for scope in &simulation.model.scopes {
        register_id(
            &scope.id,
            PublicNodeKind::Scope,
            &mut nodes,
            &mut diagnostics,
        );
    }

    for task in &simulation.tasks {
        register_id(
            &task.id,
            PublicNodeKind::Other,
            &mut nodes,
            &mut diagnostics,
        );
        for entity in task.analyses.iter().chain(&task.solver_configurations) {
            register_id(
                &entity.id,
                PublicNodeKind::Other,
                &mut nodes,
                &mut diagnostics,
            );
        }
    }

    for scope in &simulation.model.scopes {
        validate_scope(scope.id.as_str(), &scope.members, &nodes, &mut diagnostics);
    }

    for relation in &simulation.relations {
        validate_relation_endpoint(relation.source.as_str(), "source", &nodes, &mut diagnostics);
        validate_relation_endpoint(relation.target.as_str(), "target", &nodes, &mut diagnostics);
    }

    ValidationReport::new(diagnostics)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublicNodeKind {
    Spatial,
    Scope,
    Other,
}

fn register_id(
    raw_id: &str,
    kind: PublicNodeKind,
    nodes: &mut BTreeMap<String, PublicNodeKind>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_canonical_reference(raw_id) {
        diagnostics.push(Diagnostic::error(
            DIAGNOSTIC_INVALID_CANONICAL_ID,
            None,
            format!("invalid canonical id: {raw_id}"),
        ));
        return;
    }

    if nodes.contains_key(raw_id) {
        diagnostics.push(Diagnostic::error(
            DIAGNOSTIC_DUPLICATE_ID,
            Some(raw_id.to_owned()),
            format!("duplicate canonical id: {raw_id}"),
        ));
        return;
    }

    nodes.insert(raw_id.to_owned(), kind);
}

fn validate_scope(
    scope_id: &str,
    members: &[String],
    nodes: &BTreeMap<String, PublicNodeKind>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let subject = is_canonical_reference(scope_id).then(|| scope_id.to_owned());
    if members.is_empty() {
        diagnostics.push(Diagnostic::error(
            DIAGNOSTIC_EMPTY_SCOPE,
            subject.clone(),
            format!("spatial scope has no members: {scope_id}"),
        ));
        return;
    }

    let mut seen = BTreeSet::new();
    for member in members {
        if !is_canonical_reference(member) {
            diagnostics.push(Diagnostic::error(
                DIAGNOSTIC_INVALID_CANONICAL_ID,
                subject.clone(),
                format!("invalid spatial scope member reference {member} in {scope_id}"),
            ));
            continue;
        }

        if !seen.insert(member.as_str()) {
            diagnostics.push(Diagnostic::error(
                DIAGNOSTIC_DUPLICATE_SCOPE_MEMBER,
                subject.clone(),
                format!("duplicate spatial scope member {member} in {scope_id}"),
            ));
            continue;
        }

        match nodes.get(member) {
            None => diagnostics.push(Diagnostic::error(
                DIAGNOSTIC_UNRESOLVED_SCOPE_MEMBER,
                subject.clone(),
                format!("unresolved spatial scope member {member} in {scope_id}"),
            )),
            Some(PublicNodeKind::Spatial) => {}
            Some(_) => diagnostics.push(Diagnostic::error(
                DIAGNOSTIC_NON_SPATIAL_SCOPE_MEMBER,
                subject.clone(),
                format!("non-spatial scope member {member} in {scope_id}"),
            )),
        }
    }
}

fn validate_relation_endpoint(
    endpoint: &str,
    endpoint_name: &str,
    nodes: &BTreeMap<String, PublicNodeKind>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_canonical_reference(endpoint) {
        diagnostics.push(Diagnostic::error(
            DIAGNOSTIC_INVALID_CANONICAL_ID,
            None,
            format!("invalid relation {endpoint_name} reference: {endpoint}"),
        ));
        return;
    }

    if !nodes.contains_key(endpoint) {
        diagnostics.push(Diagnostic::error(
            DIAGNOSTIC_UNRESOLVED_RELATION_ENDPOINT,
            Some(endpoint.to_owned()),
            format!("unresolved relation {endpoint_name} endpoint: {endpoint}"),
        ));
    }
}

fn normalize_diagnostics(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort_by(compare_diagnostics);
    diagnostics.dedup();
    diagnostics
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    left.severity
        .rank()
        .cmp(&right.severity.rank())
        .then_with(|| left.code.cmp(&right.code))
        .then_with(|| left.subject.cmp(&right.subject))
        .then_with(|| left.detail.cmp(&right.detail))
        .then_with(|| extension_key(&left.extensions).cmp(&extension_key(&right.extensions)))
}

fn extension_key(extensions: &BTreeMap<String, Value>) -> String {
    serde_json::to_string(extensions).unwrap_or_default()
}

fn valid_diagnostic_code(code: &str) -> bool {
    !code.is_empty()
        && code.split('.').all(|segment| {
            !segment.is_empty()
                && segment.chars().all(|character| {
                    character.is_ascii_lowercase()
                        || character.is_ascii_digit()
                        || matches!(character, '_' | '-')
                })
        })
}
