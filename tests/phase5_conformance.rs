use serde_json::{json, Value};
use sol_adapter_protocol::{
    ExecutePlanRequestV02, ExecutePlanResponseV02, ExecutionOutcome, PreflightOutcome,
    ValidatePlanRequestV02, ValidatePlanResponseV02,
};
use sol_adaptor_moose::realization_v02::translate_steady_thermal_v02;
use sol_public_contract::{BackendTargetDtoV02, MappingPlanDtoV02, RealizationSpecDtoV02};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const PROFILE: &str = include_str!("fixtures/sol/0.2/realization-profile.json");
const REQUEST_V02: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");
const TARGET_V02: &str = include_str!("fixtures/sol/0.2/thermal-backend-target.json");
const PLAN_V02: &str = include_str!("fixtures/sol/0.2/thermal-mapping-plan.json");
const SPEC_ALT_V02: &str =
    include_str!("fixtures/sol/0.2/thermal-realization-spec-alternate-values.json");

const EXPECTED_CASE_IDS: [&str; 9] = [
    "realization-v02.required-realization-spec",
    "realization-v02.distinguishability",
    "realization-v02.hidden-semantics-rejected",
    "realization-v02.referential-integrity",
    "realization-v02.dual-axis-compatibility",
    "realization-v02.mixed-version-rejected",
    "realization-v02.validate-roundtrip",
    "realization-v02.execute-roundtrip",
    "realization-v02.no-physical-correctness-claim",
];

fn rpc(method: &str, params: Value, id: u64) -> String {
    format!(
        "{}\n",
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id,
        }))
        .unwrap()
    )
}

fn describe_rpc(id: u64) -> String {
    format!(
        "{}\n",
        serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": "describe_adapter",
            "id": id,
        }))
        .unwrap()
    )
}

fn run_adapter(frames: &[String], backend_workspace: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sol-adaptor-moose"));
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(workspace) = backend_workspace {
        command.env("SOL_MOOSE_WORKSPACE_ROOT", workspace);
    } else {
        command
            .env_remove("SOL_MOOSE_EXECUTABLE")
            .env_remove("SOL_MOOSE_CONDA_PREFIX")
            .env_remove("SOL_MOOSE_WORKSPACE_ROOT");
    }

    let mut child = command.spawn().expect("adapter binary starts");
    {
        let stdin = child.stdin.as_mut().expect("adapter stdin is piped");
        for frame in frames {
            stdin.write_all(frame.as_bytes()).expect("RPC frame writes");
        }
    }
    drop(child.stdin.take());
    child.wait_with_output().expect("adapter process exits")
}

fn responses(output: Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "adapter process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "adapter polluted stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn protocol_payload(response: &Value) -> &Value {
    assert_eq!(response["result"]["kind"], "protocol_success");
    &response["result"]["payload"]
}

fn protocol_failure_code(response: &Value) -> &str {
    assert_eq!(response["result"]["kind"], "protocol_failure");
    response["result"]["payload"]["code"]
        .as_str()
        .expect("protocol failure code")
}

#[test]
fn phase5_profile_is_the_exact_m08_realization_v02_gate() {
    let profile: Value = serde_json::from_str(PROFILE).unwrap();
    assert_eq!(profile["profile_id"], "sol.realization-conformance.0.2");
    assert_eq!(profile["adapter_protocol_version"], "0.2");
    assert_eq!(profile["public_contract_version"], "0.2");
    assert_eq!(
        profile["backend_validation_scope"],
        "not_assessed_by_conformance"
    );

    let ids = profile["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, EXPECTED_CASE_IDS);
}

#[test]
fn declared_v01_external_process_behavior_remains_non_realizing_and_side_effect_free() {
    let request = json!({
        "adapter_protocol_version": "0.1",
        "target": {
            "public_contract_version": "0.1",
            "target": "moose",
            "required_capabilities": ["thermal.solve"]
        },
        "plan": {
            "public_contract_version": "0.1",
            "actions": [{"id": "opaque.action", "dependencies": []}]
        }
    });

    let output = run_adapter(
        &[
            describe_rpc(1),
            rpc("validate_plan", request.clone(), 2),
            rpc("execute_plan", request, 3),
        ],
        None,
    );
    let observed = responses(output);
    assert_eq!(observed.len(), 3);

    let description = protocol_payload(&observed[0]);
    assert_eq!(
        description["bootstrap"]["supported_adapter_protocol_versions"],
        json!(["0.1", "0.2"])
    );
    assert_eq!(
        description["bootstrap"]["supported_public_contract_versions"],
        json!(["0.1", "0.2"])
    );

    let validate = protocol_payload(&observed[1]);
    assert_eq!(validate["adapter_protocol_version"], "0.1");
    assert_eq!(validate["preflight"], "rejected");
    assert_eq!(validate["target_compatible"], false);
    assert_eq!(validate["capabilities_satisfied"], false);

    let execute = protocol_payload(&observed[2]);
    assert_eq!(execute["adapter_protocol_version"], "0.1");
    assert_eq!(execute["execution"], "rejected");
    assert!(execute["execution_batches"].as_array().unwrap().is_empty());
    assert!(execute["effects"].as_array().unwrap().is_empty());
    assert!(execute["provenance"].is_null());
}

#[test]
fn realization_v02_contract_cases_hold_without_backend_execution() {
    let base = ValidatePlanRequestV02::from_json(REQUEST_V02).unwrap();

    // realization-v02.required-realization-spec
    let mut missing_spec: Value = serde_json::from_str(REQUEST_V02).unwrap();
    missing_spec
        .as_object_mut()
        .unwrap()
        .remove("realization_spec");
    let missing = responses(run_adapter(
        &[
            rpc("validate_plan", missing_spec.clone(), 10),
            rpc("execute_plan", missing_spec, 11),
        ],
        None,
    ));
    assert_eq!(
        protocol_failure_code(&missing[0]),
        "protocol.invalid_request"
    );
    assert_eq!(
        protocol_failure_code(&missing[1]),
        "protocol.invalid_request"
    );

    // realization-v02.distinguishability
    let alternate = ValidatePlanRequestV02::new(
        BackendTargetDtoV02::from_json(TARGET_V02).unwrap(),
        MappingPlanDtoV02::from_json(PLAN_V02).unwrap(),
        RealizationSpecDtoV02::from_json(SPEC_ALT_V02).unwrap(),
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
    assert_ne!(
        translate_steady_thermal_v02(&base)
            .unwrap()
            .to_moose_input()
            .unwrap(),
        translate_steady_thermal_v02(&alternate)
            .unwrap()
            .to_moose_input()
            .unwrap()
    );

    // realization-v02.hidden-semantics-rejected: action spelling is not realization meaning.
    let expected = translate_steady_thermal_v02(&base)
        .unwrap()
        .to_moose_input()
        .unwrap();
    let mut renamed: Value = serde_json::from_str(REQUEST_V02).unwrap();
    let actions = renamed["plan"]["actions"].as_array_mut().unwrap();
    actions[0]["id"] = "step.a".into();
    actions[1]["id"] = "step.b".into();
    actions[1]["dependencies"] = json!(["step.a"]);
    actions[2]["id"] = "step.c".into();
    actions[2]["dependencies"] = json!(["step.b"]);
    let bindings = renamed["realization_spec"]["action_bindings"]
        .as_array_mut()
        .unwrap();
    bindings[0]["action_id"] = "step.a".into();
    bindings[1]["action_id"] = "step.b".into();
    bindings[2]["action_id"] = "step.c".into();
    let renamed = ValidatePlanRequestV02::from_json(&renamed.to_string()).unwrap();
    assert_eq!(
        expected,
        translate_steady_thermal_v02(&renamed)
            .unwrap()
            .to_moose_input()
            .unwrap()
    );

    // realization-v02.referential-integrity
    let mut broken: Value = serde_json::from_str(REQUEST_V02).unwrap();
    broken["realization_spec"]["action_bindings"][0]["action_id"] = "unknown.action".into();
    match ValidatePlanRequestV02::from_json(&broken.to_string()) {
        Err(_) => {}
        Ok(request) => assert!(request
            .realization_spec
            .validate_against_plan(&request.plan)
            .is_err()),
    }

    // realization-v02.dual-axis-compatibility
    let description = responses(run_adapter(&[describe_rpc(12)], None));
    let description = protocol_payload(&description[0]);
    assert_eq!(
        description["bootstrap"]["supported_adapter_protocol_versions"],
        json!(["0.1", "0.2"])
    );
    assert_eq!(
        description["bootstrap"]["supported_public_contract_versions"],
        json!(["0.1", "0.2"])
    );

    // realization-v02.mixed-version-rejected
    let mut mixed: Value = serde_json::from_str(REQUEST_V02).unwrap();
    mixed["target"]["public_contract_version"] = "0.1".into();
    let mixed = responses(run_adapter(&[rpc("validate_plan", mixed, 13)], None));
    assert_eq!(protocol_failure_code(&mixed[0]), "protocol.invalid_request");

    // realization-v02.no-physical-correctness-claim is established by the profile itself;
    // numerical correctness is exercised separately in phase5_thermal_vv.rs.
    let profile: Value = serde_json::from_str(PROFILE).unwrap();
    assert_eq!(
        profile["backend_validation_scope"],
        "not_assessed_by_conformance"
    );
}

#[test]
fn realization_v02_external_process_roundtrips_on_exact_moose_target() {
    if std::env::var_os("SOL_MOOSE_EXECUTABLE").is_none() {
        eprintln!("SOL_MOOSE_EXECUTABLE is unset; exact-target Phase 5 conformance is skipped");
        return;
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/phase5-conformance");
    let workspace = root.join("adapter-workspace");
    let _ = fs::remove_dir_all(&workspace);
    fs::create_dir_all(&workspace).unwrap();

    let request: Value = serde_json::from_str(REQUEST_V02).unwrap();
    let output = run_adapter(
        &[
            describe_rpc(20),
            rpc("validate_plan", request.clone(), 21),
            rpc("execute_plan", request, 22),
        ],
        Some(&workspace),
    );
    let observed = responses(output);
    assert_eq!(observed.len(), 3);

    let description = protocol_payload(&observed[0]);
    assert_eq!(
        description["bootstrap"]["supported_adapter_protocol_versions"],
        json!(["0.1", "0.2"])
    );
    assert_eq!(
        description["bootstrap"]["supported_public_contract_versions"],
        json!(["0.1", "0.2"])
    );

    let validate_request = ValidatePlanRequestV02::from_json(REQUEST_V02).unwrap();
    let validate_json = serde_json::to_string(protocol_payload(&observed[1])).unwrap();
    let validate = ValidatePlanResponseV02::from_json(&validate_json).unwrap();
    assert_eq!(validate.preflight, PreflightOutcome::Accepted);
    assert!(validate.target_compatible);
    assert!(validate.capabilities_satisfied);
    assert!(validate.diagnostics.is_empty());
    validate_request
        .realization_spec
        .validate_against_plan(&validate_request.plan)
        .unwrap();

    let execute_request = ExecutePlanRequestV02::from_json(REQUEST_V02).unwrap();
    let execute_json = serde_json::to_string(protocol_payload(&observed[2])).unwrap();
    let mut execute = ExecutePlanResponseV02::from_json(&execute_json).unwrap();
    execute.validate_against(&execute_request).unwrap();
    assert_eq!(execute.execution, ExecutionOutcome::Completed);
    assert_eq!(
        execute.action_reports.len(),
        execute_request.plan.actions.len()
    );
    assert!(execute.diagnostics.is_empty());
    let provenance = execute.provenance.as_ref().expect("execution provenance");
    assert_eq!(provenance.producer, "sol.adapter.moose");
    assert!(provenance.opaque_references.iter().all(|reference| {
        !execute.effects.iter().any(|effect| match &effect.subject {
            sol_public_contract::MappingSubjectDto::Entity { id, .. } => id == &reference.reference,
            sol_public_contract::MappingSubjectDto::Relation { .. } => false,
        })
    }));

    let report = json!({
        "stability": "a0.1_phase5_ci_evidence_not_a_public_report_contract",
        "profile_id": "sol.realization-conformance.0.2",
        "adapter_protocol_version": "0.2",
        "public_contract_version": "0.2",
        "backend_validation_scope": "not_assessed_by_conformance",
        "cases": EXPECTED_CASE_IDS
            .iter()
            .map(|id| json!({"id": id, "determination": "conformant"}))
            .collect::<Vec<_>>(),
    });
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("realization-v02-report.json"),
        format!("{}\n", serde_json::to_string_pretty(&report).unwrap()),
    )
    .unwrap();
}
