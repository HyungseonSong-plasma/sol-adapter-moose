use serde_json::json;
use sol_adaptor_moose::backend::{
    check_backend_input, probe_exact_phase1_target, MooseProcessRunner, PHASE1_MOOSE_PACKAGE_SHA256,
};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn exact_moose_backend_probe_and_check_input() {
    let Some(executable) = std::env::var_os("SOL_MOOSE_EXECUTABLE") else {
        eprintln!("SOL_MOOSE_EXECUTABLE is unset; real MOOSE probe is skipped");
        return;
    };
    let executable = PathBuf::from(executable);
    let runner = MooseProcessRunner::new(&executable, Duration::from_secs(120));
    let prefix = std::env::var_os("SOL_MOOSE_CONDA_PREFIX")
        .map(PathBuf::from)
        .or_else(|| runner.inferred_conda_prefix())
        .expect("Conda prefix must be explicit or inferable from <prefix>/bin/moose-opt");

    let evidence = probe_exact_phase1_target(&runner, &prefix).unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/backend/minimal_diffusion.i");
    let check = check_backend_input(&runner, &fixture).unwrap();
    assert!(check.success());

    let observation = json!({
        "stability": "phase1_backend_evidence_not_protocol_semantics",
        "executable": executable.to_string_lossy(),
        "conda_prefix": prefix.to_string_lossy(),
        "package": {
            "name": evidence.identity.name,
            "version": evidence.identity.version,
            "build": evidence.identity.build,
            "subdir": evidence.identity.subdir,
            "filename": evidence.identity.filename,
            "metadata_sha256": evidence.identity.sha256,
            "official_expected_sha256": PHASE1_MOOSE_PACKAGE_SHA256,
            "channel": evidence.identity.channel,
            "url": evidence.identity.url,
        },
        "interfaces": {
            "version_output": evidence.version_output.trim(),
            "application_type": evidence.application_type.trim(),
            "heat_transfer_copyable_input": true,
            "help_has_check_input": evidence.help_output.contains("--check-input"),
            "help_has_json": evidence.help_output.contains("--json"),
        },
        "backend_only_check_input": {
            "fixture": "tests/fixtures/backend/minimal_diffusion.i",
            "accepted": true,
            "status_code": check.status_code,
        }
    });

    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/phase1-moose-probe.json");
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&target, serde_json::to_vec_pretty(&observation).unwrap()).unwrap();
    println!("phase1 MOOSE evidence written to {}", target.display());
}
