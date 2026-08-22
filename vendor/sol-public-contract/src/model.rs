use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

use crate::{CanonicalDocument, ContractDocumentError, PUBLIC_CONTRACT_VERSION};

pub type Extensions = BTreeMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKindDto {
    PhysicsModel,
    MathematicalModel,
    ConstitutiveModel,
    SpatialModel,
    MaterialModel,
    ConditionModel,
    NumericalModel,
    ObservationModel,
    Analysis,
    SolverConfiguration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OntologyEntityDto {
    pub id: String,
    pub kind: EntityKindDto,
    pub semantic_type: String,
    pub label: String,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialScopeDto {
    pub id: String,
    pub members: Vec<String>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationModelDto {
    pub id: String,
    pub physics: Vec<OntologyEntityDto>,
    pub mathematical: Vec<OntologyEntityDto>,
    pub constitutive: Vec<OntologyEntityDto>,
    pub spatial: Vec<OntologyEntityDto>,
    #[serde(default)]
    pub scopes: Vec<SpatialScopeDto>,
    pub material: Vec<OntologyEntityDto>,
    pub conditions: Vec<OntologyEntityDto>,
    pub numerical: Vec<OntologyEntityDto>,
    pub observations: Vec<OntologyEntityDto>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationTaskDto {
    pub id: String,
    pub analyses: Vec<OntologyEntityDto>,
    pub solver_configurations: Vec<OntologyEntityDto>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKindDto {
    RepresentedBy,
    ClosedBy,
    ParameterizedBy,
    DefinedOn,
    DiscretizedBy,
    AppliedTo,
    AnalyzedBy,
    SolvedBy,
    Produces,
    ObservedBy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticRelationDto {
    pub kind: RelationKindDto,
    pub source: String,
    pub target: String,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationDto {
    pub public_contract_version: String,
    pub ontology_version: String,
    pub model: SimulationModelDto,
    pub tasks: Vec<SimulationTaskDto>,
    pub relations: Vec<SemanticRelationDto>,
    #[serde(flatten, default)]
    pub extensions: Extensions,
}

impl SimulationDto {
    pub fn from_json(input: &str) -> Result<Self, ModelDtoError> {
        let document = CanonicalDocument::parse(input).map_err(ModelDtoError::Contract)?;
        serde_json::from_value(document.value().clone())
            .map_err(|error| ModelDtoError::InvalidDto(error.to_string()))
    }

    pub fn to_canonical_json(&self) -> Result<String, ModelDtoError> {
        let json = serde_json::to_string(self)
            .map_err(|error| ModelDtoError::InvalidDto(error.to_string()))?;
        let document = CanonicalDocument::parse(&json).map_err(ModelDtoError::Contract)?;
        document
            .to_canonical_json()
            .map_err(|error| ModelDtoError::InvalidDto(error.to_string()))
    }

    pub fn has_supported_contract_version(&self) -> bool {
        self.public_contract_version == PUBLIC_CONTRACT_VERSION
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelDtoError {
    Contract(ContractDocumentError),
    InvalidDto(String),
}

impl Display for ModelDtoError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "{error}"),
            Self::InvalidDto(detail) => write!(formatter, "invalid public model DTO: {detail}"),
        }
    }
}

impl Error for ModelDtoError {}

pub fn is_canonical_reference(value: &str) -> bool {
    let Some((namespace, local_name)) = value.rsplit_once('.') else {
        return false;
    };

    !namespace.is_empty()
        && namespace.split('.').all(valid_reference_segment)
        && valid_reference_segment(local_name)
}

fn valid_reference_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}
