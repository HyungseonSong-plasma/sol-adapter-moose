use serde_json::json;
use sol_adapter_protocol::ValidatePlanRequestV02;
use sol_adaptor_moose::backend::{execute_backend_input, MooseProcessRunner, WorkspaceLayout};
use sol_adaptor_moose::realization_v02::translate_steady_thermal_v02;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const REQUEST: &str = include_str!("fixtures/sol/0.2/thermal-realization-request.json");
const RUN_KEY: &str = "thermal-constant-field-vv";
const EXPECTED_TEMPERATURE_K: f64 = 400.0;
const MAX_ABS_ERROR_TOLERANCE_K: f64 = 1.0e-8;

fn with_vv_only_instrumentation(production_input: &str) -> String {
    let postprocessors = r#"[Postprocessors]
  [temperature_max_vv]
    type = NodalExtremeValue
    variable = T
    value_type = max
  []
  [temperature_min_vv]
    type = NodalExtremeValue
    variable = T
    value_type = min
  []
[]

"#;

    assert!(production_input.contains("[Outputs]\n"));
    assert!(production_input.contains("  exodus = false\n"));

    let instrumented = production_input.replacen(
        "[Outputs]\n",
        &format!("{postprocessors}[Outputs]\n"),
        1,
    );
    instrumented.replacen(
        "  exodus = false\n",
        "  exodus = false\n  csv = true\n",
        1,
    )
}

fn find_single_csv(run_dir: &Path) -> PathBuf {
    let csv = fs::read_dir(run_dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("csv"))
        .collect::<Vec<_>>();
    assert_eq!(
        csv.len(),
        1,
        "expected exactly one V&V CSV output in {}, got {csv:?}",
        run_dir.display()
    );
    csv.into_iter().next().unwrap()
}

fn parse_extremes(csv_path: &Path) -> (f64, f64) {
    let input = fs::read_to_string(csv_path).unwrap();
    let rows = input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert!(rows.len() >= 2, "CSV must contain a header and data row");

    let header = rows[0]
        .split(',')
        .map(|field| field.trim().trim_matches('"'))
        .collect::<Vec<_>>();
    let data = rows
        .last()
        .unwrap()
        .split(',')
        .map(|field| field.trim().trim_matches('"'))
        .collect::<Vec<_>>();
    assert_eq!(header.len(), data.len());

    let value = |name: &str| {
        let index = header
            .iter()
            .position(|field| *field == name)
            .unwrap_or_else(|| panic!("missing `{name}` column in {header:?}"));
        data[index]
            .parse::<f64>()
            .unwrap_or_else(|error| panic!("invalid `{name}` value `{}`: {error}", data[index]))
    };

    (value("temperature_min_vv"), value("temperature_max_vv"))
}

#[test]
fn steady_thermal_slice_matches_predeclared_constant_field_benchmark() {
    let Some(executable) = std::env::var_os("SOL_MOOSE_EXECUTABLE") else {
        eprintln!("SOL_MOOSE_EXECUTABLE is unset; Phase 5 numerical V&V is skipped");
        return;
    };

    let request = ValidatePlanRequestV02::from_json(REQUEST).unwrap();
    let model = translate_steady_thermal_v02(&request).unwrap();
    let production_input = model.to_moose_input().unwrap();

    // V&V instrumentation is deliberately appended only in this test. It is not part of the
    // canonical SOL contract, RealizationSpec mapping, adapter-local production IR, or Protocol
    // result semantics.
    let input = with_vv_only_instrumentation(&production_input);
    assert!(!production_input.contains("NodalExtremeValue"));
    assert!(!production_input.contains("csv = true"));
    assert!(input.contains("type = NodalExtremeValue"));
    assert!(input.contains("value_type = max"));
    assert!(input.contains("value_type = min"));
    assert!(input.contains("csv = true"));

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/phase5-vv-evidence");
    let layout = WorkspaceLayout::new(&root, RUN_KEY).unwrap();
    let _ = fs::remove_dir_all(layout.run_dir());

    let runner = MooseProcessRunner::new(PathBuf::from(executable), Duration::from_secs(120));
    let output = execute_backend_input(&runner, &layout, &input).unwrap();
    assert!(
        output.success(),
        "MOOSE V&V solve failed: status={:?}, timed_out={}, stderr={}",
        output.status_code,
        output.timed_out,
        output.stderr_text()
    );

    let csv_path = find_single_csv(&layout.run_dir());
    let (observed_min_k, observed_max_k) = parse_extremes(&csv_path);
    let min_error_k = (observed_min_k - EXPECTED_TEMPERATURE_K).abs();
    let max_error_k = (observed_max_k - EXPECTED_TEMPERATURE_K).abs();
    let max_abs_error_k = min_error_k.max(max_error_k);

    assert!(
        max_abs_error_k <= MAX_ABS_ERROR_TOLERANCE_K,
        "steady thermal V&V failed: Tmin={observed_min_k}, Tmax={observed_max_k}, max_abs_error={max_abs_error_k:e} K, tolerance={MAX_ABS_ERROR_TOLERANCE_K:e} K"
    );

    let evidence = json!({
        "stability": "a0.1_phase5_backend_vv_evidence_not_sol_result_semantics",
        "benchmark": "steady_1d_constant_temperature_natural_zero_flux",
        "expected_temperature_k": EXPECTED_TEMPERATURE_K,
        "observed_min_temperature_k": observed_min_k,
        "observed_max_temperature_k": observed_max_k,
        "max_abs_error_k": max_abs_error_k,
        "tolerance_k": MAX_ABS_ERROR_TOLERANCE_K,
        "acceptance": "max(abs(Tmin-400 K), abs(Tmax-400 K)) <= 1e-8 K",
        "instrumentation": "backend_owned_vv_only_nodal_extreme_value_csv",
        "canonical_result_semantics_claimed": false,
    });
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("thermal-vv.json"),
        format!("{}\n", serde_json::to_string_pretty(&evidence).unwrap()),
    )
    .unwrap();
}
