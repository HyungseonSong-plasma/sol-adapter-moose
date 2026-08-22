use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    is_canonical_reference, CanonicalDocument, ContractDocumentError, Extensions, RelationKindDto,
    PUBLIC_CONTRACT_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "subject_kind", rename_all = "snake_case")]
pub enum MappingSubjectDto {
    Entity {
        id: String,
        #[serde(flatten, default)]
        extensions: Extensions,
    },
    Relation {
        source: String,
        relation_kind: RelationKindDto,
        target: String,
        #[serde(flatten, default)]
        extensions: Extensions,
    },
}

impl MappingSubjectDto {
    pub fn entity(id: impl Into<String>) -> Self {
        Self::Entity {
            id: id.into(),
            extensions: BTreeMap::new(),
        }
    }

    pub fn relation(
        source: impl Into<String>,
        relation_kind: RelationKindDto,
        target: impl Into<String>,
    ) -> Self {
        Self::Relation {
            source: source.into(),
            relation_kind,
            target: target.into(),
            extensions: BTreeMap::new(),
        }
    }

    fn validate(&self) -> Result<(), RealizationDtoError> {
        match self {
            Self::Entity { id, extensions } => {
                require_canonical_reference("mapping subject entity", id)?;
                reject_backend_native_fields(extensions)
            }
            Self::Relation {
                source,
                target,
                extensions,
                ..
            } => {
                require_canonical_reference("mapping relation source", source)?;
                require_canonical_reference("mapping relation target", target)?;
                reject_backend_native_fields(extensions)
            }
        }
    }

    fn sort_key(&self) -> String {
        match self {
            Self::Entity { id, .. } => format!("entity:{id}"),
            Self::Relation {
                source,
                relation_kind,
                target,
                ..
            } => format!(
                "relation:{source}:{}:{target}",
                relation_kind_key(*relation_kind)
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingEvidenceDto {
    pub source: String,
    pub detail: String,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingProvenanceDto {
    pub producer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingClaimDto {
    pub rule_id: String,
    pub subject: MappingSubjectDto,
    #[serde(default)]
    pub evidence: Vec<MappingEvidenceDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<MappingProvenanceDto>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl MappingClaimDto {
    fn normalize(&mut self) -> Result<(), RealizationDtoError> {
        require_stable_symbol("mapping rule id", &self.rule_id)?;
        self.subject.validate()?;
        reject_backend_native_fields(&self.extensions)?;

        for evidence in &self.evidence {
            if evidence.source.is_empty() {
                return Err(RealizationDtoError::EmptyField("mapping evidence source"));
            }
            if evidence.detail.is_empty() {
                return Err(RealizationDtoError::EmptyField("mapping evidence detail"));
            }
            reject_backend_native_fields(&evidence.extensions)?;
        }
        self.evidence.sort_by_key(evidence_key);
        self.evidence.dedup();

        if let Some(provenance) = &self.provenance {
            if provenance.producer.is_empty() {
                return Err(RealizationDtoError::EmptyField(
                    "mapping provenance producer",
                ));
            }
            reject_backend_native_fields(&provenance.extensions)?;
        }

        Ok(())
    }

    fn sort_key(&self) -> String {
        format!("{}:{}", self.rule_id, self.subject.sort_key())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingClaimsDto {
    pub public_contract_version: String,
    pub claims: Vec<MappingClaimDto>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl MappingClaimsDto {
    pub fn from_json(input: &str) -> Result<Self, RealizationDtoError> {
        let mut document: Self = parse_contract_document(input)?;
        document.normalize()?;
        Ok(document)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationDtoError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        canonical_json(&normalized)
    }

    fn normalize(&mut self) -> Result<(), RealizationDtoError> {
        reject_backend_native_fields(&self.extensions)?;
        for claim in &mut self.claims {
            claim.normalize()?;
        }
        self.claims.sort_by(|left, right| {
            left.sort_key()
                .cmp(&right.sort_key())
                .then_with(|| stable_json(left).cmp(&stable_json(right)))
        });
        self.claims.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanActionDto {
    pub id: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl PlanActionDto {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            dependencies: Vec::new(),
            extensions: BTreeMap::new(),
        }
    }

    pub fn depends_on(mut self, dependency: impl Into<String>) -> Self {
        self.dependencies.push(dependency.into());
        self
    }

    fn normalize(&mut self) -> Result<(), RealizationDtoError> {
        require_stable_symbol("plan action id", &self.id)?;
        reject_backend_native_fields(&self.extensions)?;
        for dependency in &self.dependencies {
            require_stable_symbol("plan dependency", dependency)?;
        }
        self.dependencies.sort();
        self.dependencies.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingPlanDto {
    pub public_contract_version: String,
    pub actions: Vec<PlanActionDto>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl MappingPlanDto {
    pub fn new(actions: Vec<PlanActionDto>) -> Result<Self, RealizationDtoError> {
        let mut plan = Self {
            public_contract_version: PUBLIC_CONTRACT_VERSION.to_owned(),
            actions,
            extensions: BTreeMap::new(),
        };
        plan.normalize()?;
        Ok(plan)
    }

    pub fn from_json(input: &str) -> Result<Self, RealizationDtoError> {
        let mut plan: Self = parse_contract_document(input)?;
        plan.normalize()?;
        Ok(plan)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationDtoError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        canonical_json(&normalized)
    }

    pub fn topological_order(&self) -> Result<Vec<String>, RealizationDtoError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        topological_order(&normalized.actions)
    }

    fn normalize(&mut self) -> Result<(), RealizationDtoError> {
        reject_backend_native_fields(&self.extensions)?;
        for action in &mut self.actions {
            action.normalize()?;
        }
        self.actions.sort_by(|left, right| left.id.cmp(&right.id));

        for pair in self.actions.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(RealizationDtoError::DuplicateAction(pair[0].id.clone()));
            }
        }

        topological_order(&self.actions)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendTargetDto {
    pub public_contract_version: String,
    pub target: String,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl BackendTargetDto {
    pub fn from_json(input: &str) -> Result<Self, RealizationDtoError> {
        let mut target: Self = parse_contract_document(input)?;
        target.normalize()?;
        Ok(target)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationDtoError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        canonical_json(&normalized)
    }

    fn normalize(&mut self) -> Result<(), RealizationDtoError> {
        require_stable_symbol("backend target", &self.target)?;
        reject_backend_native_fields(&self.extensions)?;
        for capability in &self.required_capabilities {
            require_stable_symbol("backend capability", capability)?;
        }
        self.required_capabilities.sort();
        self.required_capabilities.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealizationQualityDto {
    Exact,
    Compatible,
    Degraded,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationEffectDto {
    pub subject: MappingSubjectDto,
    pub quality: RealizationQualityDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl RealizationEffectDto {
    pub fn new(subject: MappingSubjectDto, quality: RealizationQualityDto) -> Self {
        Self {
            subject,
            quality,
            detail: None,
            extensions: BTreeMap::new(),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn validate(&self) -> Result<(), RealizationDtoError> {
        self.subject.validate()?;
        reject_backend_native_fields(&self.extensions)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationEffectDocumentDto {
    pub public_contract_version: String,
    pub effect: RealizationEffectDto,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl RealizationEffectDocumentDto {
    pub fn from_json(input: &str) -> Result<Self, RealizationDtoError> {
        let document: Self = parse_contract_document(input)?;
        document.effect.validate()?;
        reject_backend_native_fields(&document.extensions)?;
        Ok(document)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationDtoError> {
        self.effect.validate()?;
        reject_backend_native_fields(&self.extensions)?;
        canonical_json(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticComparisonDto {
    Exact,
    Compatible,
    Degraded,
    Unsupported,
    Unknown,
    SubjectMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationStatusDto {
    Pass,
    Fail,
    Blocked,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvaluationResultDto {
    pub public_contract_version: String,
    pub intended_subject: MappingSubjectDto,
    pub effect: RealizationEffectDto,
    pub comparison: SemanticComparisonDto,
    pub status: EvaluationStatusDto,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl EvaluationResultDto {
    pub fn evaluate(
        intended_subject: MappingSubjectDto,
        effect: RealizationEffectDto,
    ) -> Result<Self, RealizationDtoError> {
        intended_subject.validate()?;
        effect.validate()?;
        let comparison = compare_subject_and_effect(&intended_subject, &effect);
        let status = classify_comparison(comparison);
        Ok(Self {
            public_contract_version: PUBLIC_CONTRACT_VERSION.to_owned(),
            intended_subject,
            effect,
            comparison,
            status,
            extensions: BTreeMap::new(),
        })
    }

    pub fn from_json(input: &str) -> Result<Self, RealizationDtoError> {
        let result: Self = parse_contract_document(input)?;
        result.validate_consistency()?;
        Ok(result)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationDtoError> {
        self.validate_consistency()?;
        canonical_json(self)
    }

    fn validate_consistency(&self) -> Result<(), RealizationDtoError> {
        self.intended_subject.validate()?;
        self.effect.validate()?;
        reject_backend_native_fields(&self.extensions)?;

        let expected_comparison = compare_subject_and_effect(&self.intended_subject, &self.effect);
        let expected_status = classify_comparison(expected_comparison);
        if self.comparison != expected_comparison || self.status != expected_status {
            return Err(RealizationDtoError::InconsistentEvaluation {
                expected_comparison,
                declared_comparison: self.comparison,
                expected_status,
                declared_status: self.status,
            });
        }
        Ok(())
    }
}

pub fn compare_subject_and_effect(
    intended_subject: &MappingSubjectDto,
    effect: &RealizationEffectDto,
) -> SemanticComparisonDto {
    if !same_semantic_subject(intended_subject, &effect.subject) {
        return SemanticComparisonDto::SubjectMismatch;
    }

    match effect.quality {
        RealizationQualityDto::Exact => SemanticComparisonDto::Exact,
        RealizationQualityDto::Compatible => SemanticComparisonDto::Compatible,
        RealizationQualityDto::Degraded => SemanticComparisonDto::Degraded,
        RealizationQualityDto::Unsupported => SemanticComparisonDto::Unsupported,
        RealizationQualityDto::Unknown => SemanticComparisonDto::Unknown,
    }
}

pub fn classify_comparison(comparison: SemanticComparisonDto) -> EvaluationStatusDto {
    match comparison {
        SemanticComparisonDto::Exact | SemanticComparisonDto::Compatible => {
            EvaluationStatusDto::Pass
        }
        SemanticComparisonDto::Degraded | SemanticComparisonDto::SubjectMismatch => {
            EvaluationStatusDto::Fail
        }
        SemanticComparisonDto::Unsupported => EvaluationStatusDto::Blocked,
        SemanticComparisonDto::Unknown => EvaluationStatusDto::Indeterminate,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealizationDtoError {
    Contract(ContractDocumentError),
    InvalidDto(String),
    EmptyField(&'static str),
    InvalidIdentifier {
        field: &'static str,
        value: String,
    },
    DuplicateAction(String),
    MissingDependency {
        action: String,
        dependency: String,
    },
    CycleDetected(Vec<String>),
    BackendNativeLeakage(String),
    InconsistentEvaluation {
        expected_comparison: SemanticComparisonDto,
        declared_comparison: SemanticComparisonDto,
        expected_status: EvaluationStatusDto,
        declared_status: EvaluationStatusDto,
    },
}

impl Display for RealizationDtoError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "{error}"),
            Self::InvalidDto(detail) => write!(formatter, "invalid realization DTO: {detail}"),
            Self::EmptyField(field) => write!(formatter, "{field} must not be empty"),
            Self::InvalidIdentifier { field, value } => {
                write!(formatter, "invalid {field} identifier: {value}")
            }
            Self::DuplicateAction(action) => write!(formatter, "duplicate plan action: {action}"),
            Self::MissingDependency { action, dependency } => write!(
                formatter,
                "plan action {action} depends on missing action {dependency}"
            ),
            Self::CycleDetected(actions) => write!(
                formatter,
                "mapping plan cycle detected among: {}",
                actions.join(", ")
            ),
            Self::BackendNativeLeakage(field) => write!(
                formatter,
                "backend-native object/identity field is not canonical semantic payload: {field}"
            ),
            Self::InconsistentEvaluation {
                expected_comparison,
                declared_comparison,
                expected_status,
                declared_status,
            } => write!(
                formatter,
                "inconsistent evaluation: comparison={declared_comparison:?} (expected {expected_comparison:?}), status={declared_status:?} (expected {expected_status:?})"
            ),
        }
    }
}

impl Error for RealizationDtoError {}

fn parse_contract_document<T: DeserializeOwned>(input: &str) -> Result<T, RealizationDtoError> {
    let document = CanonicalDocument::parse(input).map_err(RealizationDtoError::Contract)?;
    serde_json::from_value(document.value().clone())
        .map_err(|error| RealizationDtoError::InvalidDto(error.to_string()))
}

fn canonical_json<T: Serialize>(value: &T) -> Result<String, RealizationDtoError> {
    let json = serde_json::to_string(value)
        .map_err(|error| RealizationDtoError::InvalidDto(error.to_string()))?;
    let document = CanonicalDocument::parse(&json).map_err(RealizationDtoError::Contract)?;
    document
        .to_canonical_json()
        .map_err(|error| RealizationDtoError::InvalidDto(error.to_string()))
}

fn topological_order(actions: &[PlanActionDto]) -> Result<Vec<String>, RealizationDtoError> {
    let action_ids: BTreeSet<_> = actions.iter().map(|action| action.id.clone()).collect();
    for action in actions {
        for dependency in &action.dependencies {
            if !action_ids.contains(dependency) {
                return Err(RealizationDtoError::MissingDependency {
                    action: action.id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }

    let mut remaining: BTreeMap<String, BTreeSet<String>> = actions
        .iter()
        .map(|action| {
            (
                action.id.clone(),
                action.dependencies.iter().cloned().collect(),
            )
        })
        .collect();
    let mut ordered = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let ready: Vec<_> = remaining
            .iter()
            .filter(|(_, dependencies)| dependencies.is_empty())
            .map(|(id, _)| id.clone())
            .collect();

        if ready.is_empty() {
            return Err(RealizationDtoError::CycleDetected(
                remaining.keys().cloned().collect(),
            ));
        }

        for id in ready {
            remaining.remove(&id);
            for dependencies in remaining.values_mut() {
                dependencies.remove(&id);
            }
            ordered.push(id);
        }
    }

    Ok(ordered)
}

fn same_semantic_subject(left: &MappingSubjectDto, right: &MappingSubjectDto) -> bool {
    match (left, right) {
        (
            MappingSubjectDto::Entity { id: left_id, .. },
            MappingSubjectDto::Entity { id: right_id, .. },
        ) => left_id == right_id,
        (
            MappingSubjectDto::Relation {
                source: left_source,
                relation_kind: left_kind,
                target: left_target,
                ..
            },
            MappingSubjectDto::Relation {
                source: right_source,
                relation_kind: right_kind,
                target: right_target,
                ..
            },
        ) => left_source == right_source && left_kind == right_kind && left_target == right_target,
        _ => false,
    }
}

fn require_canonical_reference(
    field: &'static str,
    value: &str,
) -> Result<(), RealizationDtoError> {
    if is_canonical_reference(value) {
        Ok(())
    } else {
        Err(RealizationDtoError::InvalidIdentifier {
            field,
            value: value.to_owned(),
        })
    }
}

fn require_stable_symbol(field: &'static str, value: &str) -> Result<(), RealizationDtoError> {
    let valid = !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        });
    if valid {
        Ok(())
    } else {
        Err(RealizationDtoError::InvalidIdentifier {
            field,
            value: value.to_owned(),
        })
    }
}

fn reject_backend_native_fields(extensions: &Extensions) -> Result<(), RealizationDtoError> {
    const FORBIDDEN: &[&str] = &[
        "adapter_native",
        "backend_native",
        "backend_native_id",
        "backend_object",
        "native_object",
        "solver_object",
    ];
    for field in FORBIDDEN {
        if extensions.contains_key(*field) {
            return Err(RealizationDtoError::BackendNativeLeakage(
                (*field).to_owned(),
            ));
        }
    }
    Ok(())
}

fn evidence_key(evidence: &MappingEvidenceDto) -> String {
    format!(
        "{}:{}:{}",
        evidence.source,
        evidence.detail,
        stable_json(&evidence.extensions)
    )
}

fn stable_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn relation_kind_key(kind: RelationKindDto) -> &'static str {
    match kind {
        RelationKindDto::RepresentedBy => "represented_by",
        RelationKindDto::ClosedBy => "closed_by",
        RelationKindDto::ParameterizedBy => "parameterized_by",
        RelationKindDto::DefinedOn => "defined_on",
        RelationKindDto::DiscretizedBy => "discretized_by",
        RelationKindDto::AppliedTo => "applied_to",
        RelationKindDto::AnalyzedBy => "analyzed_by",
        RelationKindDto::SolvedBy => "solved_by",
        RelationKindDto::Produces => "produces",
        RelationKindDto::ObservedBy => "observed_by",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_comparison, compare_subject_and_effect, EvaluationStatusDto, MappingPlanDto,
        MappingSubjectDto, PlanActionDto, RealizationEffectDto, RealizationQualityDto,
        SemanticComparisonDto,
    };

    #[test]
    fn public_plan_orders_dependencies_deterministically() {
        let plan = MappingPlanDto::new(vec![
            PlanActionDto::new("thermal.solve").depends_on("thermal.material"),
            PlanActionDto::new("thermal.domain"),
            PlanActionDto::new("thermal.material").depends_on("thermal.domain"),
        ])
        .unwrap();

        assert_eq!(
            plan.topological_order().unwrap(),
            vec!["thermal.domain", "thermal.material", "thermal.solve"]
        );
    }

    #[test]
    fn evaluation_mapping_preserves_m01_lifecycle() {
        let subject = MappingSubjectDto::entity("thermal.energy_conservation");
        for (quality, comparison, status) in [
            (
                RealizationQualityDto::Exact,
                SemanticComparisonDto::Exact,
                EvaluationStatusDto::Pass,
            ),
            (
                RealizationQualityDto::Compatible,
                SemanticComparisonDto::Compatible,
                EvaluationStatusDto::Pass,
            ),
            (
                RealizationQualityDto::Degraded,
                SemanticComparisonDto::Degraded,
                EvaluationStatusDto::Fail,
            ),
            (
                RealizationQualityDto::Unsupported,
                SemanticComparisonDto::Unsupported,
                EvaluationStatusDto::Blocked,
            ),
            (
                RealizationQualityDto::Unknown,
                SemanticComparisonDto::Unknown,
                EvaluationStatusDto::Indeterminate,
            ),
        ] {
            let effect = RealizationEffectDto::new(subject.clone(), quality);
            assert_eq!(compare_subject_and_effect(&subject, &effect), comparison);
            assert_eq!(classify_comparison(comparison), status);
        }
    }
}
