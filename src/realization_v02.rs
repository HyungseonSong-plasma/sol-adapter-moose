use crate::ir::{
    ConstantMaterialProperty, DirichletBoundary, Executioner, GeneratedLineMesh, MooseInputModel,
    MooseOperator, Outputs, ScalarVariable, VariableFamily, VariableOrder,
};
use sol_adapter_protocol::ValidatePlanRequestV02;
use sol_public_contract::{
    EntityKindDto, RealizationEntityDtoV02, RealizationSpecDtoV02, RelationKindDto,
    SemanticRelationDto, SpatialScopeDto,
};
use std::error::Error;
use std::fmt::{Display, Formatter};

const CAPABILITY: &str = "thermal.steady_conduction";
const MESH_NX: u32 = 4;

const VARIABLE_NAME: &str = "T";
const KERNEL_NAME: &str = "thermal_conduction";
const MATERIAL_NAME: &str = "thermal_material";
const CONDUCTIVITY_PROPERTY: &str = "thermal_conductivity";
const HOT_WALL_BC_NAME: &str = "hot_wall_temperature";

/// Translate the deliberately narrow SOL Realization 0.2 steady-thermal slice into
/// adapter-local MOOSE IR.
///
/// MappingPlan action-id spellings are not inspected here. Protocol 0.2 parsing has
/// already established plan/spec referential integrity; physics comes only from the
/// canonical RealizationSpec entity/relation/scope/quantity graph.
pub fn translate_steady_thermal_v02(
    request: &ValidatePlanRequestV02,
) -> Result<MooseInputModel, ThermalRealizationError> {
    if request.target.target != "moose" {
        return Err(ThermalRealizationError::UnsupportedTarget(
            request.target.target.clone(),
        ));
    }
    if request.target.required_capabilities.as_slice() != [CAPABILITY] {
        return Err(ThermalRealizationError::UnsupportedCapabilities(
            request.target.required_capabilities.clone(),
        ));
    }

    request
        .realization_spec
        .validate_against_plan(&request.plan)
        .map_err(|error| ThermalRealizationError::InvalidContract(error.to_string()))?;

    let spec = &request.realization_spec;
    let transport = unique_entity(spec, EntityKindDto::PhysicsModel, "ThermalTransport")?;
    let equation = unique_entity(
        spec,
        EntityKindDto::MathematicalModel,
        "SteadyHeatEquation",
    )?;
    let closure = unique_entity(spec, EntityKindDto::ConstitutiveModel, "FourierLaw")?;
    let conductivity = unique_entity(
        spec,
        EntityKindDto::MaterialModel,
        "ThermalConductivity",
    )?;
    let field = unique_entity(spec, EntityKindDto::MathematicalModel, "Field")?;
    let boundary_condition = unique_entity(
        spec,
        EntityKindDto::ConditionModel,
        "DirichletTemperatureBoundaryCondition",
    )?;
    let analysis = unique_entity(spec, EntityKindDto::Analysis, "StationaryAnalysis")?;
    let _observation = unique_entity(
        spec,
        EntityKindDto::ObservationModel,
        "MaximumTemperature",
    )?;
    let domain = unique_entity(spec, EntityKindDto::SpatialModel, "LineDomain1D")?;
    let boundary = unique_entity(spec, EntityKindDto::SpatialModel, "BoundaryPoint1D")?;

    require_relation(
        spec,
        RelationKindDto::RepresentedBy,
        &transport.id,
        &equation.id,
    )?;
    require_relation(
        spec,
        RelationKindDto::ClosedBy,
        &equation.id,
        &closure.id,
    )?;
    require_relation(
        spec,
        RelationKindDto::ParameterizedBy,
        &closure.id,
        &conductivity.id,
    )?;
    require_relation(
        spec,
        RelationKindDto::AppliedTo,
        &boundary_condition.id,
        &field.id,
    )?;
    require_relation(
        spec,
        RelationKindDto::AnalyzedBy,
        &spec.source_model,
        &analysis.id,
    )?;

    let domain_scope_id = relation_target(
        spec,
        RelationKindDto::DefinedOn,
        &equation.id,
        "steady heat equation domain scope",
    )?;
    let domain_scope = scope(spec, domain_scope_id)?;
    require_exact_scope_member(domain_scope, &domain.id)?;

    let hot_wall_scope_id = unique_applied_scope_target(spec, &boundary_condition.id, &field.id)?;
    let hot_wall_scope = scope(spec, hot_wall_scope_id)?;
    require_exact_scope_member(hot_wall_scope, &boundary.id)?;

    let length = quantity(domain, "geometry.length", "unit.meter")?;
    if length <= 0.0 {
        return Err(ThermalRealizationError::UnsupportedValue {
            semantic_parameter: "geometry.length".to_owned(),
            detail: "LineDomain1D length must be positive".to_owned(),
        });
    }

    let hot_wall_position = quantity(boundary, "geometry.position", "unit.meter")?;
    if hot_wall_position != 0.0 {
        return Err(ThermalRealizationError::UnsupportedValue {
            semantic_parameter: "geometry.position".to_owned(),
            detail: "A0.1 supports only the published 0 m hot-wall point mapped to the MOOSE left boundary"
                .to_owned(),
        });
    }

    let conductivity_value = quantity(
        conductivity,
        "thermal.conductivity",
        "unit.watt_per_meter_kelvin",
    )?;
    if conductivity_value <= 0.0 {
        return Err(ThermalRealizationError::UnsupportedValue {
            semantic_parameter: "thermal.conductivity".to_owned(),
            detail: "thermal conductivity must be positive".to_owned(),
        });
    }

    let boundary_temperature = quantity(
        boundary_condition,
        "thermal.temperature",
        "unit.kelvin",
    )?;

    let model = MooseInputModel {
        // A0.1 backend-local chart/discretization policy: the published LineDomain1D length
        // is placed on [0,L] with four elements. This is not canonical SOL identity.
        mesh: GeneratedLineMesh {
            nx: MESH_NX,
            xmin: 0.0,
            xmax: length,
        },
        variables: vec![ScalarVariable {
            name: VARIABLE_NAME.to_owned(),
            order: VariableOrder::First,
            family: VariableFamily::Lagrange,
        }],
        operators: vec![MooseOperator::HeatConduction {
            name: KERNEL_NAME.to_owned(),
            variable: VARIABLE_NAME.to_owned(),
            thermal_conductivity_property: CONDUCTIVITY_PROPERTY.to_owned(),
        }],
        materials: vec![ConstantMaterialProperty {
            name: MATERIAL_NAME.to_owned(),
            property: CONDUCTIVITY_PROPERTY.to_owned(),
            value: conductivity_value,
        }],
        boundary_conditions: vec![DirichletBoundary {
            name: HOT_WALL_BC_NAME.to_owned(),
            variable: VARIABLE_NAME.to_owned(),
            boundary: "left".to_owned(),
            value: boundary_temperature,
        }],
        executioner: Executioner::Steady,
        // Canonical result semantics are SOL 0.3 work and are intentionally outside A0.1.
        outputs: Outputs { exodus: false },
    };

    model
        .validate()
        .map_err(|error| ThermalRealizationError::InvalidMooseIr(error.to_string()))?;
    Ok(model)
}

fn unique_entity<'a>(
    spec: &'a RealizationSpecDtoV02,
    kind: EntityKindDto,
    semantic_type: &str,
) -> Result<&'a RealizationEntityDtoV02, ThermalRealizationError> {
    let matches = spec
        .entities
        .iter()
        .filter(|entity| entity.kind == kind && entity.semantic_type == semantic_type)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [entity] => Ok(*entity),
        [] => Err(ThermalRealizationError::MissingSemanticEntity {
            kind,
            semantic_type: semantic_type.to_owned(),
        }),
        _ => Err(ThermalRealizationError::AmbiguousSemanticEntity {
            kind,
            semantic_type: semantic_type.to_owned(),
        }),
    }
}

fn require_relation(
    spec: &RealizationSpecDtoV02,
    kind: RelationKindDto,
    source: &str,
    target: &str,
) -> Result<(), ThermalRealizationError> {
    if spec
        .relations
        .iter()
        .any(|relation| relation.kind == kind && relation.source == source && relation.target == target)
    {
        Ok(())
    } else {
        Err(ThermalRealizationError::MissingRelation {
            kind,
            source: source.to_owned(),
            target: target.to_owned(),
        })
    }
}

fn relation_target<'a>(
    spec: &'a RealizationSpecDtoV02,
    kind: RelationKindDto,
    source: &str,
    purpose: &str,
) -> Result<&'a str, ThermalRealizationError> {
    let matches = spec
        .relations
        .iter()
        .filter(|relation| relation.kind == kind && relation.source == source)
        .collect::<Vec<&SemanticRelationDto>>();
    match matches.as_slice() {
        [relation] => Ok(relation.target.as_str()),
        [] => Err(ThermalRealizationError::UnsupportedGraph(format!(
            "missing {purpose} relation"
        ))),
        _ => Err(ThermalRealizationError::UnsupportedGraph(format!(
            "ambiguous {purpose} relations"
        ))),
    }
}

fn unique_applied_scope_target<'a>(
    spec: &'a RealizationSpecDtoV02,
    bc_id: &str,
    field_id: &str,
) -> Result<&'a str, ThermalRealizationError> {
    let scope_ids = spec
        .scopes
        .iter()
        .map(|scope| scope.id.as_str())
        .collect::<Vec<_>>();
    let targets = spec
        .relations
        .iter()
        .filter(|relation| relation.kind == RelationKindDto::AppliedTo && relation.source == bc_id)
        .map(|relation| relation.target.as_str())
        .collect::<Vec<_>>();
    if !targets.contains(&field_id) {
        return Err(ThermalRealizationError::MissingRelation {
            kind: RelationKindDto::AppliedTo,
            source: bc_id.to_owned(),
            target: field_id.to_owned(),
        });
    }
    let spatial = targets
        .into_iter()
        .filter(|target| scope_ids.contains(target))
        .collect::<Vec<_>>();
    match spatial.as_slice() {
        [target] => Ok(*target),
        [] => Err(ThermalRealizationError::UnsupportedGraph(
            "Dirichlet temperature BC has no canonical spatial scope".to_owned(),
        )),
        _ => Err(ThermalRealizationError::UnsupportedGraph(
            "A0.1 supports exactly one spatial scope for the Dirichlet temperature BC".to_owned(),
        )),
    }
}

fn scope<'a>(
    spec: &'a RealizationSpecDtoV02,
    id: &str,
) -> Result<&'a SpatialScopeDto, ThermalRealizationError> {
    spec.scopes
        .iter()
        .find(|scope| scope.id == id)
        .ok_or_else(|| ThermalRealizationError::MissingScope(id.to_owned()))
}

fn require_exact_scope_member(
    scope: &SpatialScopeDto,
    entity_id: &str,
) -> Result<(), ThermalRealizationError> {
    if scope.members.as_slice() == [entity_id] {
        Ok(())
    } else {
        Err(ThermalRealizationError::UnsupportedGraph(format!(
            "scope {} must contain only {} for the A0.1 slice",
            scope.id, entity_id
        )))
    }
}

fn quantity(
    entity: &RealizationEntityDtoV02,
    semantic_parameter: &str,
    expected_unit: &str,
) -> Result<f64, ThermalRealizationError> {
    let parameter = entity
        .parameters
        .iter()
        .find(|parameter| parameter.semantic_parameter == semantic_parameter)
        .ok_or_else(|| ThermalRealizationError::MissingParameter {
            entity: entity.id.clone(),
            semantic_parameter: semantic_parameter.to_owned(),
        })?;
    if parameter.quantity.unit != expected_unit {
        return Err(ThermalRealizationError::UnsupportedUnit {
            semantic_parameter: semantic_parameter.to_owned(),
            expected: expected_unit.to_owned(),
            actual: parameter.quantity.unit.clone(),
        });
    }
    let value = parameter.quantity.value.as_f64().ok_or_else(|| {
        ThermalRealizationError::UnsupportedValue {
            semantic_parameter: semantic_parameter.to_owned(),
            detail: "quantity cannot be represented as finite f64".to_owned(),
        }
    })?;
    if !value.is_finite() {
        return Err(ThermalRealizationError::UnsupportedValue {
            semantic_parameter: semantic_parameter.to_owned(),
            detail: "quantity must be finite".to_owned(),
        });
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThermalRealizationError {
    InvalidContract(String),
    UnsupportedTarget(String),
    UnsupportedCapabilities(Vec<String>),
    MissingSemanticEntity {
        kind: EntityKindDto,
        semantic_type: String,
    },
    AmbiguousSemanticEntity {
        kind: EntityKindDto,
        semantic_type: String,
    },
    MissingRelation {
        kind: RelationKindDto,
        source: String,
        target: String,
    },
    MissingScope(String),
    MissingParameter {
        entity: String,
        semantic_parameter: String,
    },
    UnsupportedUnit {
        semantic_parameter: String,
        expected: String,
        actual: String,
    },
    UnsupportedValue {
        semantic_parameter: String,
        detail: String,
    },
    UnsupportedGraph(String),
    InvalidMooseIr(String),
}

impl Display for ThermalRealizationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidContract(detail) => write!(formatter, "invalid SOL 0.2 realization contract: {detail}"),
            Self::UnsupportedTarget(target) => write!(formatter, "unsupported backend target for A0.1 thermal mapping: {target}"),
            Self::UnsupportedCapabilities(capabilities) => write!(formatter, "unsupported required capabilities for A0.1 thermal mapping: {}", capabilities.join(", ")),
            Self::MissingSemanticEntity { kind, semantic_type } => write!(formatter, "missing canonical {:?} entity with semantic_type={semantic_type}", kind),
            Self::AmbiguousSemanticEntity { kind, semantic_type } => write!(formatter, "multiple canonical {:?} entities with semantic_type={semantic_type} are outside the A0.1 slice", kind),
            Self::MissingRelation { kind, source, target } => write!(formatter, "missing canonical relation {source} {:?} {target}", kind),
            Self::MissingScope(scope) => write!(formatter, "missing canonical spatial scope: {scope}"),
            Self::MissingParameter { entity, semantic_parameter } => write!(formatter, "canonical entity {entity} is missing parameter {semantic_parameter}"),
            Self::UnsupportedUnit { semantic_parameter, expected, actual } => write!(formatter, "unsupported unit for {semantic_parameter}: expected {expected}, got {actual}"),
            Self::UnsupportedValue { semantic_parameter, detail } => write!(formatter, "unsupported value for {semantic_parameter}: {detail}"),
            Self::UnsupportedGraph(detail) => write!(formatter, "unsupported canonical thermal graph for A0.1: {detail}"),
            Self::InvalidMooseIr(detail) => write!(formatter, "translated adapter-local MOOSE IR is invalid: {detail}"),
        }
    }
}

impl Error for ThermalRealizationError {}
