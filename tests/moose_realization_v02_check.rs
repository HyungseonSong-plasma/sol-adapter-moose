use sol_adapter_protocol::ValidatePlanRequestV02;
use sol_adaptor_moose::backend::{check_backend_input, MooseProcessRunner};
use sol_adaptor_moose::realization_v02::translate_steady_thermal_v02;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const REQUEST: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");

#[test]
fn canonical_thermal_v02_translation_passes_exact_moose_check_input() {
    let Some(executable) = std::env::var_os("SOL_MOOSE_EXECUTABLE") else {
        eprintln!("SOL_MOOSE_EXECUTABLE is unset; Phase 3 MOOSE check is skipped");
        return;
    };

    let request = ValidatePlanRequestV02::from_json(REQUEST).unwrap();
    let model = translate_steady_thermal_v02(&request).unwrap();
    let input = model.to_moose_input().unwrap();

    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/phase3-realization-v02");
    fs::create_dir_all(&target).unwrap();
    let input_path = target.join("thermal-realization-v02.i");
    fs::write(&input_path, input).unwrap();

    let runner = MooseProcessRunner::new(PathBuf::from(executable), Duration::from_secs(120));
    let check = check_backend_input(&runner, &input_path).unwrap();
    assert!(
        check.success(),
        "translated canonical thermal input rejected by exact MOOSE:\n{}\n{}",
        check.stdout_text(),
        check.stderr_text()
    );
}
