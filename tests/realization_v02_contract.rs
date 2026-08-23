use sol_adapter_protocol::{RealizationRequestError, ValidatePlanRequestV02};
use sol_public_contract::{
    CanonicalDocument, ContractDocumentError, MappingPlanDtoV02, RealizationSpecDtoV02,
};

const THERMAL_REQUEST: &str =
    include_str!("fixtures/sol/0.2/thermal-realization-request.json");
const THERMAL_PLAN: &str = include_str!("fixtures/sol/0.2/thermal-mapping-plan.json");
const THERMAL_SPEC: &str = include_str!("fixtures/sol/0.2/thermal-realization-spec.json");
const THERMAL_SPEC_ALT: &str =
    include_str!("fixtures/sol/0.2/thermal-realization-spec-alternate-values.json");

#[test]
fn published_thermal_realization_request_parses_as_explicit_v02() {
    let request = ValidatePlanRequestV02::from_json(THERMAL_REQUEST).unwrap();
    assert_eq!(request.adapter_protocol_version, "0.2");
    assert_eq!(request.target.public_contract_version, "0.2");
    assert_eq!(request.plan.public_contract_version, "0.2");
    assert_eq!(request.realization_spec.public_contract_version, "0.2");
}

#[test]
fn v01_default_parser_does_not_silently_accept_v02() {
    assert_eq!(
        CanonicalDocument::parse(THERMAL_PLAN),
        Err(ContractDocumentError::UnsupportedVersion("0.2".to_owned()))
    );
}

#[test]
fn same_mapping_plan_can_carry_distinct_realization_identity() {
    let plan = MappingPlanDtoV02::from_json(THERMAL_PLAN).unwrap();
    let spec = RealizationSpecDtoV02::from_json(THERMAL_SPEC).unwrap();
    let alternate = RealizationSpecDtoV02::from_json(THERMAL_SPEC_ALT).unwrap();

    spec.validate_against_plan(&plan).unwrap();
    alternate.validate_against_plan(&plan).unwrap();
    assert_eq!(plan.to_canonical_json().unwrap(), plan.to_canonical_json().unwrap());
    assert_ne!(
        spec.to_canonical_json().unwrap(),
        alternate.to_canonical_json().unwrap()
    );
}

#[test]
fn missing_realization_spec_is_rejected() {
    let mut value: serde_json::Value = serde_json::from_str(THERMAL_REQUEST).unwrap();
    value.as_object_mut().unwrap().remove("realization_spec");
    let error = ValidatePlanRequestV02::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(error, RealizationRequestError::InvalidRequest(_)));
}

#[test]
fn mixed_public_contract_versions_are_rejected() {
    let mut value: serde_json::Value = serde_json::from_str(THERMAL_REQUEST).unwrap();
    value["target"]["public_contract_version"] = serde_json::Value::String("0.1".to_owned());
    let error = ValidatePlanRequestV02::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error,
        RealizationRequestError::IncoherentPublicContractVersions { .. }
    ));
}

#[test]
fn hidden_plan_action_physics_is_rejected() {
    let mut value: serde_json::Value = serde_json::from_str(THERMAL_REQUEST).unwrap();
    value["plan"]["actions"][0]["physics"] =
        serde_json::Value::String("ThermalTransport".to_owned());
    let error = ValidatePlanRequestV02::from_json(&value.to_string()).unwrap_err();
    assert!(matches!(
        error,
        RealizationRequestError::InvalidPublicPayload { field: "plan", .. }
    ));
}
