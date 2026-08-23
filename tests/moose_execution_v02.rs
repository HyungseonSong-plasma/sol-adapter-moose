use sol_adapter_protocol::{
    ActionExecutionState, ExecutePlanRequestV02, ExecutionOutcome, PreflightOutcome,
    ValidatePlanRequestV02,
};
use sol_adaptor_moose::execution_v02::ExecutionEnvironment;
use sol_public_contract::MappingSubjectDto;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const REQUEST: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");
const RUN_KEY: &str = "thermal-realization-001";

#[test]
fn canonical_thermal_v02_executes_authoritatively_on_exact_moose_target() {
    let Some(executable) = std::env::var_os("SOL_MOOSE_EXECUTABLE") else {
        eprintln!("SOL_MOOSE_EXECUTABLE is unset; Phase 4 authoritative execution is skipped");
        return;
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/phase4-execution-evidence");
    let run_dir = root.join("runs").join(RUN_KEY);
    let _ = fs::remove_dir_all(&run_dir);

    let environment = ExecutionEnvironment::configured(
        PathBuf::from(executable),
        &root,
        Duration::from_secs(120),
    );
    let execute_request = ExecutePlanRequestV02::from_json(REQUEST).unwrap();
    let validate_request = ValidatePlanRequestV02::new(
        execute_request.target.clone(),
        execute_request.plan.clone(),
        execute_request.realization_spec.clone(),
    )
    .unwrap();

    let preflight = environment.validate_plan_v02(&validate_request).unwrap();
    assert_eq!(preflight.preflight, PreflightOutcome::Accepted);
    assert!(preflight.target_compatible);
    assert!(preflight.capabilities_satisfied);
    assert!(preflight.diagnostics.is_empty());

    let mut response = environment
        .execute_plan_v02_with_run_key(&execute_request, RUN_KEY)
        .unwrap();
    response.validate_against(&execute_request).unwrap();

    assert_eq!(response.execution, ExecutionOutcome::Completed);
    assert_eq!(
        response.execution_batches,
        vec![
            vec!["thermal.domain".to_owned()],
            vec!["thermal.material".to_owned()],
            vec!["thermal.solve".to_owned()],
        ]
    );
    assert_eq!(response.action_reports.len(), 3);
    assert!(response
        .action_reports
        .iter()
        .all(|report| report.state == ActionExecutionState::Completed));
    assert!(response.diagnostics.is_empty());

    let effect_ids = response
        .effects
        .iter()
        .map(|effect| match &effect.subject {
            MappingSubjectDto::Entity { id, .. } => id.as_str(),
            MappingSubjectDto::Relation { .. } => panic!("A0.1 Phase 4 reports entity effects"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        effect_ids,
        vec![
            "domain.main",
            "property.thermal_conductivity",
            "thermal.temperature_field",
        ]
    );

    let provenance = response.provenance.as_ref().expect("execution provenance");
    assert_eq!(provenance.producer, "sol.adapter.moose");
    assert!(provenance
        .opaque_references
        .iter()
        .any(|reference| reference.namespace == "moose.workspace" && reference.reference == RUN_KEY));
    assert!(provenance
        .opaque_references
        .iter()
        .all(|reference| !effect_ids.contains(&reference.reference.as_str())));

    for path in [
        run_dir.join("input.i"),
        run_dir.join("check.stdout.log"),
        run_dir.join("check.stderr.log"),
        run_dir.join("stdout.log"),
        run_dir.join("stderr.log"),
    ] {
        assert!(path.is_file(), "missing Phase 4 artifact {}", path.display());
    }

    let input = fs::read_to_string(run_dir.join("input.i")).unwrap();
    assert!(input.contains("type = HeatConduction"));
    assert!(input.contains("value = 45"));
    assert!(input.contains("value = 400"));
}
