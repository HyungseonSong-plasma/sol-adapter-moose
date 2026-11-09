use sol_adapter_protocol::{
    ExecutePlanRequestV02, ExecutionOutcome, PreflightOutcome, ValidatePlanRequestV02,
};
use sol_adaptor_moose::execution_v02::ExecutionEnvironment;
use sol_adaptor_moose::official_moose_oci::{IMAGE, PROFILE};
use std::fs;
use std::path::PathBuf;

const REQUEST: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");

#[test]
fn actual_pinned_official_moose_oci_thermal_runs_with_adapter_owned_realization() {
    if std::env::var("SOL_MOOSE_BACKEND_PROFILE").ok().as_deref() != Some(PROFILE) {
        eprintln!(
            "Official MOOSE OCI native test requires explicitly selected profile; not asserting PASS"
        );
        return;
    }
    assert!(std::env::var_os("SOL_MOOSE_EXECUTABLE").is_none());
    assert!(std::env::var_os("SOL_MOOSE_CONDA_PREFIX").is_none());
    let environment = ExecutionEnvironment::from_environment();
    let workspace = environment.workspace_root();
    assert!(workspace.is_absolute());
    let run_key = "direct-thermal-official-oci-001";
    let dir = workspace.join("runs").join(run_key);
    assert!(!dir.exists(), "fresh native workspace is required");

    let request = ExecutePlanRequestV02::from_json(REQUEST).unwrap();
    let validate_request = ValidatePlanRequestV02::new(
        request.target.clone(),
        request.plan.clone(),
        request.realization_spec.clone(),
    )
    .unwrap();

    let preflight = environment.validate_plan_v02(&validate_request).unwrap();
    assert_eq!(preflight.preflight, PreflightOutcome::Accepted);
    assert!(
        !dir.exists(),
        "backend readiness must not allocate a run workspace"
    );

    let mut response = environment
        .execute_plan_v02_with_run_key(&request, run_key)
        .expect("OCI MOOSE P2 and P3 must execute");
    response.validate_against(&request).unwrap();
    assert_eq!(response.execution, ExecutionOutcome::Completed);
    let references = &response.provenance.as_ref().unwrap().opaque_references;
    assert!(references
        .iter()
        .any(|r| r.namespace == "moose.oci.image" && r.reference == IMAGE));
    assert!(!references.iter().any(|r| r.namespace == "moose.package"));
    assert!(references
        .iter()
        .any(|r| r.namespace == "moose.executable" && r.reference == "/opt/moose/bin/moose-opt"));
    for name in [
        "input.i",
        "check.stdout.log",
        "check.stderr.log",
        "stdout.log",
        "stderr.log",
    ] {
        assert!(
            dir.join(name).is_file(),
            "missing actual backend evidence: {name}"
        );
    }
    let input = fs::read_to_string(dir.join("input.i")).unwrap();
    assert!(input.contains("type = HeatConduction"));
    assert!(input.contains("prop_values = '45'"));
    assert!(input.contains("value = 400"));
    let check = format!(
        "{}\n{}",
        fs::read_to_string(dir.join("check.stdout.log")).unwrap(),
        fs::read_to_string(dir.join("check.stderr.log")).unwrap()
    );
    assert!(
        check.contains("Syntax OK"),
        "native P2 must report Syntax OK on stdout or stderr: {check}"
    );
    let stdout = fs::read_to_string(dir.join("stdout.log")).unwrap();
    assert!(
        stdout.contains("Solve Converged!"),
        "native P3 must converge"
    );
    let receipt = serde_json::json!({
        "status": "OFFICIAL_MOOSE_OCI_DIRECT_THERMAL_P2_P3_PASS",
        "image": IMAGE,
        "profile": PROFILE,
        "native_input": dir.join("input.i").display().to_string(),
        "scientific_acceptance": "NOT_EVALUATED",
        "conda_release_identity": "NOT_EQUIVALENT",
        "sol_consumer_parity": "NOT_EVALUATED_BY_DIRECT_TEST",
    });
    let evidence_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/physics-oci-evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    fs::write(
        evidence_dir.join("direct-result.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
}
