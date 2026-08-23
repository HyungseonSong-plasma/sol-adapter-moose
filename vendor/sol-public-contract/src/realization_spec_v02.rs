use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{
    canonicalize_value, is_canonical_reference, ContractDocumentError, ContractVersion,
    EntityKindDto, Extensions, MappingSubjectDto, PlanActionDto, RelationKindDto,
    SemanticRelationDto, SpatialScopeDto, PUBLIC_CONTRACT_VERSION_0_2,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingPlanDtoV02 {
    pub public_contract_version: String,
    pub actions: Vec<PlanActionDto>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl MappingPlanDtoV02 {
    pub fn new(actions: Vec<PlanActionDto>) -> Result<Self, RealizationSpecError> {
        let mut plan = Self {
            public_contract_version: PUBLIC_CONTRACT_VERSION_0_2.to_owned(),
            actions,
            extensions: BTreeMap::new(),
        };
        plan.normalize()?;
        Ok(plan)
    }

    pub fn from_json(input: &str) -> Result<Self, RealizationSpecError> {
        let value = parse_v02_document(input)?;
        let mut plan: Self = serde_json::from_value(value)
            .map_err(|error| RealizationSpecError::InvalidDto(error.to_string()))?;
        plan.normalize()?;
        Ok(plan)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationSpecError> {
        let mut plan = self.clone();
        plan.normalize()?;
        canonical_json_v02(&plan)
    }

    pub fn topological_order(&self) -> Result<Vec<String>, RealizationSpecError> {
        let mut plan = self.clone();
        plan.normalize()?;
        topological_order(&plan.actions)
    }

    fn normalize(&mut self) -> Result<(), RealizationSpecError> {
        require_v02(&self.public_contract_version)?;
        reject_backend_native_extensions(&self.extensions)?;
        for action in &mut self.actions {
            require_stable_symbol("plan action id", &action.id)?;
            reject_backend_native_extensions(&action.extensions)?;
            reject_hidden_realization_extensions(&action.extensions)?;
            for dependency in &action.dependencies {
                require_stable_symbol("plan dependency", dependency)?;
            }
            action.dependencies.sort();
            action.dependencies.dedup();
        }
        self.actions.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in self.actions.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(RealizationSpecError::DuplicateAction(pair[0].id.clone()));
            }
        }
        topological_order(&self.actions)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendTargetDtoV02 {
    pub public_contract_version: String,
    pub target: String,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl BackendTargetDtoV02 {
    pub fn new(
        target: impl Into<String>,
        required_capabilities: Vec<String>,
    ) -> Result<Self, RealizationSpecError> {
        let mut target = Self {
            public_contract_version: PUBLIC_CONTRACT_VERSION_0_2.to_owned(),
            target: target.into(),
            required_capabilities,
            extensions: BTreeMap::new(),
        };
        target.normalize()?;
        Ok(target)
    }

    pub fn from_json(input: &str) -> Result<Self, RealizationSpecError> {
        let value = parse_v02_document(input)?;
        let mut target: Self = serde_json::from_value(value)
            .map_err(|error| RealizationSpecError::InvalidDto(error.to_string()))?;
        target.normalize()?;
        Ok(target)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationSpecError> {
        let mut target = self.clone();
        target.normalize()?;
        canonical_json_v02(&target)
    }

    fn normalize(&mut self) -> Result<(), RealizationSpecError> {
        require_v02(&self.public_contract_version)?;
        require_stable_symbol("backend target", &self.target)?;
        reject_backend_native_extensions(&self.extensions)?;
        for capability in &self.required_capabilities {
            require_stable_symbol("backend capability", capability)?;
        }
        self.required_capabilities.sort();
        self.required_capabilities.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantityDtoV02 {
    pub value: Number,
    pub unit: String,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl QuantityDtoV02 {
    fn validate(&self) -> Result<(), RealizationSpecError> {
        require_canonical_reference("quantity unit", &self.unit)?;
        reject_backend_native_extensions(&self.extensions)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationParameterDtoV02 {
    pub semantic_parameter: String,
    pub quantity: QuantityDtoV02,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl RealizationParameterDtoV02 {
    fn normalize(&mut self) -> Result<(), RealizationSpecError> {
        require_canonical_reference("semantic parameter", &self.semantic_parameter)?;
        self.quantity.validate()?;
        reject_backend_native_extensions(&self.extensions)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationEntityDtoV02 {
    pub id: String,
    pub kind: EntityKindDto,
    pub semantic_type: String,
    #[serde(default)]
    pub parameters: Vec<RealizationParameterDtoV02>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl RealizationEntityDtoV02 {
    fn normalize(&mut self) -> Result<(), RealizationSpecError> {
        require_canonical_reference("realization entity id", &self.id)?;
        require_stable_symbol("semantic type", &self.semantic_type)?;
        reject_backend_native_extensions(&self.extensions)?;
        for parameter in &mut self.parameters {
            parameter.normalize()?;
        }
        self.parameters
            .sort_by(|left, right| left.semantic_parameter.cmp(&right.semantic_parameter));
        for pair in self.parameters.windows(2) {
            if pair[0].semantic_parameter == pair[1].semantic_parameter {
                return Err(RealizationSpecError::DuplicateParameter {
                    entity: self.id.clone(),
                    parameter: pair[0].semantic_parameter.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRealizationBindingDtoV02 {
    pub action_id: String,
    #[serde(default)]
    pub subjects: Vec<MappingSubjectDto>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl ActionRealizationBindingDtoV02 {
    fn normalize(&mut self) -> Result<(), RealizationSpecError> {
        require_stable_symbol("action binding id", &self.action_id)?;
        reject_backend_native_extensions(&self.extensions)?;
        if self.subjects.is_empty() && self.scopes.is_empty() {
            return Err(RealizationSpecError::EmptyActionBinding(
                self.action_id.clone(),
            ));
        }
        for subject in &self.subjects {
            validate_subject_shape(subject)?;
        }
        self.subjects.sort_by_key(subject_key);
        self.subjects.dedup();
        for scope in &self.scopes {
            require_canonical_reference("action binding scope", scope)?;
        }
        self.scopes.sort();
        self.scopes.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizationSpecDtoV02 {
    pub public_contract_version: String,
    pub ontology_version: String,
    pub source_model: String,
    #[serde(default)]
    pub entities: Vec<RealizationEntityDtoV02>,
    #[serde(default)]
    pub scopes: Vec<SpatialScopeDto>,
    #[serde(default)]
    pub relations: Vec<SemanticRelationDto>,
    #[serde(default)]
    pub action_bindings: Vec<ActionRealizationBindingDtoV02>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl RealizationSpecDtoV02 {
    pub fn from_json(input: &str) -> Result<Self, RealizationSpecError> {
        let value = parse_v02_document(input)?;
        let mut spec: Self = serde_json::from_value(value)
            .map_err(|error| RealizationSpecError::InvalidDto(error.to_string()))?;
        spec.normalize()?;
        Ok(spec)
    }

    pub fn to_canonical_json(&self) -> Result<String, RealizationSpecError> {
        let mut spec = self.clone();
        spec.normalize()?;
        canonical_json_v02(&spec)
    }

    pub fn validate_against_plan(
        &self,
        plan: &MappingPlanDtoV02,
    ) -> Result<(), RealizationSpecError> {
        let mut spec = self.clone();
        spec.normalize()?;
        let mut plan = plan.clone();
        plan.normalize()?;

        let plan_ids: BTreeSet<_> = plan
            .actions
            .iter()
            .map(|action| action.id.as_str())
            .collect();
        let binding_ids: BTreeSet<_> = spec
            .action_bindings
            .iter()
            .map(|binding| binding.action_id.as_str())
            .collect();

        for action in &plan.actions {
            if !binding_ids.contains(action.id.as_str()) {
                return Err(RealizationSpecError::MissingActionBinding(
                    action.id.clone(),
                ));
            }
        }
        for binding in &spec.action_bindings {
            if !plan_ids.contains(binding.action_id.as_str()) {
                return Err(RealizationSpecError::UnknownActionBinding(
                    binding.action_id.clone(),
                ));
            }
        }
        Ok(())
    }

    fn normalize(&mut self) -> Result<(), RealizationSpecError> {
        require_v02(&self.public_contract_version)?;
        if self.ontology_version.is_empty() {
            return Err(RealizationSpecError::EmptyField("ontology version"));
        }
        require_canonical_reference("source model", &self.source_model)?;
        reject_backend_native_extensions(&self.extensions)?;

        for entity in &mut self.entities {
            entity.normalize()?;
        }
        self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in self.entities.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(RealizationSpecError::DuplicateReference(pair[0].id.clone()));
            }
        }

        let entity_kinds: BTreeMap<_, _> = self
            .entities
            .iter()
            .map(|entity| (entity.id.clone(), entity.kind))
            .collect();

        for scope in &mut self.scopes {
            require_canonical_reference("spatial scope id", &scope.id)?;
            reject_backend_native_extensions(&scope.extensions)?;
            if scope.members.is_empty() {
                return Err(RealizationSpecError::EmptyScope(scope.id.clone()));
            }
            for member in &scope.members {
                require_canonical_reference("spatial scope member", member)?;
                match entity_kinds.get(member) {
                    Some(EntityKindDto::SpatialModel) => {}
                    Some(_) => {
                        return Err(RealizationSpecError::NonSpatialScopeMember {
                            scope: scope.id.clone(),
                            member: member.clone(),
                        })
                    }
                    None => return Err(RealizationSpecError::UnresolvedReference(member.clone())),
                }
            }
            scope.members.sort();
            scope.members.dedup();
        }
        self.scopes.sort_by(|left, right| left.id.cmp(&right.id));
        for pair in self.scopes.windows(2) {
            if pair[0].id == pair[1].id {
                return Err(RealizationSpecError::DuplicateReference(pair[0].id.clone()));
            }
        }

        let entity_ids: BTreeSet<_> = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect();
        let scope_ids: BTreeSet<_> = self.scopes.iter().map(|scope| scope.id.clone()).collect();
        if entity_ids.contains(&self.source_model) || scope_ids.contains(&self.source_model) {
            return Err(RealizationSpecError::DuplicateReference(
                self.source_model.clone(),
            ));
        }
        for scope_id in &scope_ids {
            if entity_ids.contains(scope_id) {
                return Err(RealizationSpecError::DuplicateReference(scope_id.clone()));
            }
        }

        let mut known_refs = entity_ids.clone();
        known_refs.extend(scope_ids.iter().cloned());
        known_refs.insert(self.source_model.clone());

        for relation in &self.relations {
            reject_backend_native_extensions(&relation.extensions)?;
            require_canonical_reference("relation source", &relation.source)?;
            require_canonical_reference("relation target", &relation.target)?;
            if !known_refs.contains(&relation.source) {
                return Err(RealizationSpecError::UnresolvedReference(
                    relation.source.clone(),
                ));
            }
            if !known_refs.contains(&relation.target) {
                return Err(RealizationSpecError::UnresolvedReference(
                    relation.target.clone(),
                ));
            }
        }
        self.relations.sort_by(|left, right| {
            relation_key(left)
                .cmp(&relation_key(right))
                .then_with(|| stable_json(left).cmp(&stable_json(right)))
        });
        self.relations.dedup();

        for binding in &mut self.action_bindings {
            binding.normalize()?;
            for subject in &binding.subjects {
                match subject {
                    MappingSubjectDto::Entity { id, .. } => {
                        if !known_refs.contains(id) {
                            return Err(RealizationSpecError::UnresolvedReference(id.clone()));
                        }
                    }
                    MappingSubjectDto::Relation {
                        source,
                        relation_kind,
                        target,
                        ..
                    } => {
                        if !known_refs.contains(source) {
                            return Err(RealizationSpecError::UnresolvedReference(source.clone()));
                        }
                        if !known_refs.contains(target) {
                            return Err(RealizationSpecError::UnresolvedReference(target.clone()));
                        }
                        if !self.relations.iter().any(|relation| {
                            relation.source == *source
                                && relation.kind == *relation_kind
                                && relation.target == *target
                        }) {
                            return Err(RealizationSpecError::UnresolvedRelationSubject {
                                source: source.clone(),
                                relation_kind: *relation_kind,
                                target: target.clone(),
                            });
                        }
                    }
                }
            }
            for scope in &binding.scopes {
                if !scope_ids.contains(scope) {
                    return Err(RealizationSpecError::UnresolvedScope(scope.clone()));
                }
            }
        }
        self.action_bindings
            .sort_by(|left, right| left.action_id.cmp(&right.action_id));
        for pair in self.action_bindings.windows(2) {
            if pair[0].action_id == pair[1].action_id {
                return Err(RealizationSpecError::DuplicateActionBinding(
                    pair[0].action_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealizationSpecError {
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
    DuplicateReference(String),
    DuplicateParameter {
        entity: String,
        parameter: String,
    },
    EmptyScope(String),
    NonSpatialScopeMember {
        scope: String,
        member: String,
    },
    UnresolvedReference(String),
    UnresolvedScope(String),
    UnresolvedRelationSubject {
        source: String,
        relation_kind: RelationKindDto,
        target: String,
    },
    EmptyActionBinding(String),
    DuplicateActionBinding(String),
    MissingActionBinding(String),
    UnknownActionBinding(String),
    BackendNativeLeakage(String),
    HiddenRealizationExtension(String),
}

impl Display for RealizationSpecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "{error}"),
            Self::InvalidDto(detail) => {
                write!(formatter, "invalid Public Contract 0.2 DTO: {detail}")
            }
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
            Self::DuplicateReference(reference) => {
                write!(formatter, "duplicate realization reference: {reference}")
            }
            Self::DuplicateParameter { entity, parameter } => write!(
                formatter,
                "realization entity {entity} has duplicate parameter {parameter}"
            ),
            Self::EmptyScope(scope) => write!(formatter, "spatial scope {scope} must not be empty"),
            Self::NonSpatialScopeMember { scope, member } => write!(
                formatter,
                "spatial scope {scope} contains non-spatial member {member}"
            ),
            Self::UnresolvedReference(reference) => {
                write!(formatter, "unresolved realization reference: {reference}")
            }
            Self::UnresolvedScope(scope) => {
                write!(formatter, "unresolved realization scope: {scope}")
            }
            Self::UnresolvedRelationSubject {
                source,
                relation_kind,
                target,
            } => write!(
                formatter,
                "unresolved realization relation subject: {source} {relation_kind:?} {target}"
            ),
            Self::EmptyActionBinding(action) => write!(
                formatter,
                "action binding {action} contains no subjects or scopes"
            ),
            Self::DuplicateActionBinding(action) => {
                write!(formatter, "duplicate action realization binding: {action}")
            }
            Self::MissingActionBinding(action) => {
                write!(formatter, "missing action realization binding: {action}")
            }
            Self::UnknownActionBinding(action) => write!(
                formatter,
                "realization binding references unknown action: {action}"
            ),
            Self::BackendNativeLeakage(field) => write!(
                formatter,
                "backend-native object/identity field is not canonical realization data: {field}"
            ),
            Self::HiddenRealizationExtension(field) => write!(
                formatter,
                "PlanAction extension cannot carry canonical realization meaning: {field}"
            ),
        }
    }
}

impl Error for RealizationSpecError {}

fn parse_v02_document(input: &str) -> Result<Value, RealizationSpecError> {
    let document = crate::CanonicalDocument::parse_for(input, ContractVersion::realization_v02())
        .map_err(RealizationSpecError::Contract)?;
    Ok(document.value().clone())
}

fn canonical_json_v02<T: Serialize>(value: &T) -> Result<String, RealizationSpecError> {
    let value = serde_json::to_value(value)
        .map_err(|error| RealizationSpecError::InvalidDto(error.to_string()))?;
    let raw = serde_json::to_string(&canonicalize_value(value))
        .map_err(|error| RealizationSpecError::InvalidDto(error.to_string()))?;
    crate::CanonicalDocument::parse_for(&raw, ContractVersion::realization_v02())
        .map_err(RealizationSpecError::Contract)?;
    Ok(raw)
}

fn require_v02(version: &str) -> Result<(), RealizationSpecError> {
    if version == PUBLIC_CONTRACT_VERSION_0_2 {
        Ok(())
    } else {
        Err(RealizationSpecError::Contract(
            ContractDocumentError::UnsupportedVersion(version.to_owned()),
        ))
    }
}

fn require_canonical_reference(
    field: &'static str,
    value: &str,
) -> Result<(), RealizationSpecError> {
    if is_canonical_reference(value) {
        Ok(())
    } else {
        Err(RealizationSpecError::InvalidIdentifier {
            field,
            value: value.to_owned(),
        })
    }
}

fn require_stable_symbol(field: &'static str, value: &str) -> Result<(), RealizationSpecError> {
    let valid = !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        });
    if valid {
        Ok(())
    } else {
        Err(RealizationSpecError::InvalidIdentifier {
            field,
            value: value.to_owned(),
        })
    }
}

fn topological_order(actions: &[PlanActionDto]) -> Result<Vec<String>, RealizationSpecError> {
    let action_ids: BTreeSet<_> = actions.iter().map(|action| action.id.clone()).collect();
    for action in actions {
        for dependency in &action.dependencies {
            if !action_ids.contains(dependency) {
                return Err(RealizationSpecError::MissingDependency {
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
            return Err(RealizationSpecError::CycleDetected(
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

fn validate_subject_shape(subject: &MappingSubjectDto) -> Result<(), RealizationSpecError> {
    match subject {
        MappingSubjectDto::Entity { id, extensions } => {
            require_canonical_reference("mapping subject entity", id)?;
            reject_backend_native_extensions(extensions)
        }
        MappingSubjectDto::Relation {
            source,
            target,
            extensions,
            ..
        } => {
            require_canonical_reference("mapping subject relation source", source)?;
            require_canonical_reference("mapping subject relation target", target)?;
            reject_backend_native_extensions(extensions)
        }
    }
}

fn reject_backend_native_extensions(extensions: &Extensions) -> Result<(), RealizationSpecError> {
    for (key, value) in extensions {
        if is_backend_native_key(key) {
            return Err(RealizationSpecError::BackendNativeLeakage(key.clone()));
        }
        reject_backend_native_value(value)?;
    }
    Ok(())
}

fn reject_backend_native_value(value: &Value) -> Result<(), RealizationSpecError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if is_backend_native_key(key) {
                    return Err(RealizationSpecError::BackendNativeLeakage(key.clone()));
                }
                reject_backend_native_value(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_backend_native_value(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_backend_native_key(key: &str) -> bool {
    matches!(
        key,
        "adapter_native"
            | "backend_native"
            | "backend_native_id"
            | "backend_object"
            | "native_object"
            | "solver_object"
            | "mesh_selection_handle"
    )
}

fn reject_hidden_realization_extensions(
    extensions: &Extensions,
) -> Result<(), RealizationSpecError> {
    const FORBIDDEN: &[&str] = &[
        "realization",
        "realization_spec",
        "physics",
        "semantic_type",
        "parameters",
        "quantity",
        "unit",
        "subjects",
        "scopes",
        "material",
        "boundary_condition",
        "equation",
        "field",
        "analysis",
        "observation",
    ];
    for key in extensions.keys() {
        if FORBIDDEN.contains(&key.as_str()) {
            return Err(RealizationSpecError::HiddenRealizationExtension(
                key.clone(),
            ));
        }
    }
    Ok(())
}

fn relation_key(relation: &SemanticRelationDto) -> String {
    format!(
        "{}:{}:{}",
        relation.source,
        relation_kind_key(relation.kind),
        relation.target
    )
}

fn subject_key(subject: &MappingSubjectDto) -> String {
    match subject {
        MappingSubjectDto::Entity { id, .. } => format!("entity:{id}"),
        MappingSubjectDto::Relation {
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

fn stable_json<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .map(canonicalize_value)
        .and_then(|value| serde_json::to_string(&value))
        .unwrap_or_default()
}
