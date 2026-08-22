use sol_adaptor_moose::backend::{execute_backend_input, MooseProcessRunner, WorkspaceLayout};
use sol_adaptor_moose::ir::{
    DirichletBoundary, Executioner, GeneratedLineMesh, MooseInputModel, MooseOperator, Outputs,
    ScalarVariable, VariableFamily, VariableOrder,
};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn backend_only_generated_input_executes_in_workspace_on_exact_moose_target() {
    let Some(executable) = std::env::var_os("SOL_MOOSE_EXECUTABLE") else {
        eprintln!("SOL_MOOSE_EXECUTABLE is unset; backend workspace execution is skipped");
        return;
    };

    let model = MooseInputModel {
        mesh: GeneratedLineMesh {
            nx: 4,
            xmin: 0.0,
            xmax: 1.0,
        },
        variables: vec![ScalarVariable {
            name: "u".to_owned(),
            order: VariableOrder::First,
            family: VariableFamily::Lagrange,
        }],
        operators: vec![MooseOperator::Diffusion {
            name: "diffusion".to_owned(),
            variable: "u".to_owned(),
        }],
        materials: vec![],
        boundary_conditions: vec![
            DirichletBoundary {
                name: "left".to_owned(),
                variable: "u".to_owned(),
                boundary: "left".to_owned(),
                value: 0.0,
            },
            DirichletBoundary {
                name: "right".to_owned(),
                variable: "u".to_owned(),
                boundary: "right".to_owned(),
                value: 1.0,
            },
        ],
        executioner: Executioner::Steady,
        outputs: Outputs { exodus: false },
    };

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/backend-workspace-evidence");
    let layout = WorkspaceLayout::new(&root, "backend-only-diffusion").unwrap();
    let runner = MooseProcessRunner::new(PathBuf::from(executable), Duration::from_secs(120));
    let output = execute_backend_input(&runner, &layout, &model.to_moose_input().unwrap()).unwrap();

    assert!(
        output.success(),
        "backend-only MOOSE execution failed: status={:?}, timed_out={}, stdout={}, stderr={}",
        output.status_code,
        output.timed_out,
        output.stdout_text(),
        output.stderr_text()
    );
    assert!(layout.input_path().is_file());
    assert!(layout.stdout_path().is_file());
    assert!(layout.stderr_path().is_file());

    let persisted_stdout = fs::read(layout.stdout_path()).unwrap();
    let persisted_stderr = fs::read(layout.stderr_path()).unwrap();
    assert_eq!(persisted_stdout, output.stdout);
    assert_eq!(persisted_stderr, output.stderr);
}
