use sol_adapter_protocol::ValidatePlanRequestV02;
use sol_adaptor_moose::ir::MooseOperator;
use sol_adaptor_moose::realization_v02::{
    translate_steady_thermal_v02, ThermalRealizationError,
};
use sol_public_contract::{BackendTargetDtoV02, MappingPlanDtoV02, RealizationSpecDtoV02};

const REQUEST: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");
const TARGET: &str = include_str!("fixtures/sol/0.2/thermal-backend-target.json");
const PLAN: &str = include_str!("fixtures/sol/0.2/thermal-mapping-plan.json");
const SPEC_ALT: &str =
    include_str!("fixtures/sol/0.2/thermal-realization-spec-alternate-values.json");

#[test]
fn published_thermal_request_translates_to_deterministic_typed_moose_ir() {
    let request = ValidatePlanRequestV02::from_json(REQUEST).unwrap();
    let model = translate_steady_thermal_v02(&request).unwrap();

    assert_eq!(model.mesh.nx, 4);
    assert_eq!(model.mesh.xmin, 0.0);
    assert_eq!(model.mesh.xmax, 1.0);
    assert_eq!(model.variables.len(), 1);
    assert_eq!(model.variables[0].name, "T");
    assert_eq!(model.materials.len(), 1);
    assert_eq!(model.materials[0].property, "thermal_conductivity");
    assert_eq!(model.materials[0].value, 45.0);
    assert_eq!(model.boundary_conditions.len(), 1);
    assert_eq!(model.boundary_conditions[0].boundary, "left");
    assert_eq!(model.boundary_conditions[0].value, 400.0);
    assert!(matches!(
        model.operators.as_slice(),
        [MooseOperator::HeatConduction { .. }]
    ));

    let first = model.to_moose_input().unwrap();
    let second = translate_steady_thermal_v02(&request)
        .unwrap()
        .to_moose_input()
        .unwrap();
    assert_eq!(first, second);
    assert!(!first.contains("thermal.domain"));
    assert!(!first.contains("thermal.material"));
    assert!(!first.contains("thermal.solve"));
    assert!(!first.contains("model.thermal_reference"));
}

#[test]
fn same_plan_with_alternate_realization_values_changes_backend_realization() {
    let base = ValidatePlanRequestV02::from_json(REQUEST).unwrap();
    let alternate = ValidatePlanRequestV02::new(
        BackendTargetDtoV02::from_json(TARGET).unwrap(),
        MappingPlanDtoV02::from_json(PLAN).unwrap(),
        RealizationSpecDtoV02::from_json(SPEC_ALT).unwrap(),
    )
    .unwrap();

    assert_eq!(
        base.canonical_plan_identity().unwrap(),
        alternate.canonical_plan_identity().unwrap()
    );
    assert_ne!(
        base.canonical_realization_identity().unwrap(),
        alternate.canonical_realization_identity().unwrap()
    );

    let base_model = translate_steady_thermal_v02(&base).unwrap();
    let alternate_model = translate_steady_thermal_v02(&alternate).unwrap();
    assert_eq!(base_model.materials[0].value, 45.0);
    assert_eq!(alternate_model.materials[0].value, 200.0);
    assert_eq!(base_model.boundary_conditions[0].value, 400.0);
    assert_eq!(alternate_model.boundary_conditions[0].value, 500.0);
    assert_ne!(
        base_model.to_moose_input().unwrap(),
        alternate_model.to_moose_input().unwrap()
    );
}

#[test]
fn action_id_spelling_does_not_drive_physics_mapping() {
    let original = ValidatePlanRequestV02::from_json(REQUEST).unwrap();
    let expected = translate_steady_thermal_v02(&original)
        .unwrap()
        .to_moose_input()
        .unwrap();

    let mut value: serde_json::Value = serde_json::from_str(REQUEST).unwrap();
    let actions = value["plan"]["actions"].as_array_mut().unwrap();
    actions[0]["id"] = "step.a".into();
    actions[1]["id"] = "step.b".into();
    actions[1]["dependencies"] = serde_json::json!(["step.a"]);
    actions[2]["id"] = "step.c".into();
    actions[2]["dependencies"] = serde_json::json!(["step.b"]);
    let bindings = value["realization_spec"]["action_bindings"]
        .as_array_mut()
        .unwrap();
    bindings[0]["action_id"] = "step.a".into();
    bindings[1]["action_id"] = "step.b".into();
    bindings[2]["action_id"] = "step.c".into();

    let renamed = ValidatePlanRequestV02::from_json(&value.to_string()).unwrap();
    let actual = translate_steady_thermal_v02(&renamed)
        .unwrap()
        .to_moose_input()
        .unwrap();
    assert_eq!(expected, actual);
}

#[test]
fn unsupported_conductivity_unit_is_rejected_without_conversion_guessing() {
    let mut value: serde_json::Value = serde_json::from_str(REQUEST).unwrap();
    value["realization_spec"]["entities"][5]["parameters"][0]["quantity"]["unit"] =
        "unit.watt_per_centimeter_kelvin".into();
    let request = ValidatePlanRequestV02::from_json(&value.to_string()).unwrap();
    let error = translate_steady_thermal_v02(&request).unwrap_err();
    assert!(matches!(error, ThermalRealizationError::UnsupportedUnit { .. }));
}

#[test]
fn unsupported_boundary_position_is_rejected_instead_of_reinterpreted() {
    let mut value: serde_json::Value = serde_json::from_str(REQUEST).unwrap();
    value["realization_spec"]["entities"][2]["parameters"][0]["quantity"]["value"] =
        serde_json::json!(0.25);
    let request = ValidatePlanRequestV02::from_json(&value.to_string()).unwrap();
    let error = translate_steady_thermal_v02(&request).unwrap_err();
    assert!(matches!(error, ThermalRealizationError::UnsupportedValue { .. }));
}

#[test]
fn unsupported_thermal_semantic_type_is_rejected() {
    let mut value: serde_json::Value = serde_json::from_str(REQUEST).unwrap();
    value["realization_spec"]["entities"][9]["semantic_type"] =
        "TransientThermalTransport".into();
    let request = ValidatePlanRequestV02::from_json(&value.to_string()).unwrap();
    let error = translate_steady_thermal_v02(&request).unwrap_err();
    assert!(matches!(
        error,
        ThermalRealizationError::MissingSemanticEntity { .. }
    ));
}
