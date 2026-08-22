use sol_adaptor_moose::backend::{check_backend_input, MooseProcessRunner};
use sol_adaptor_moose::ir::{
    ConstantMaterialProperty, DirichletBoundary, Executioner, GeneratedLineMesh, MooseInputModel,
    MooseOperator, Outputs, ScalarVariable, VariableFamily, VariableOrder,
};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn base_model(operator: MooseOperator, materials: Vec<ConstantMaterialProperty>) -> MooseInputModel {
    MooseInputModel {
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
        operators: vec![operator],
        materials,
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
    }
}

#[test]
fn generated_backend_only_ir_passes_moose_check_input() {
    let Some(executable) = std::env::var_os("SOL_MOOSE_EXECUTABLE") else {
        eprintln!("SOL_MOOSE_EXECUTABLE is unset; generated MOOSE input checks are skipped");
        return;
    };
    let runner = MooseProcessRunner::new(PathBuf::from(executable), Duration::from_secs(120));
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/phase2-generated");
    fs::create_dir_all(&target).unwrap();

    let diffusion = base_model(
        MooseOperator::Diffusion {
            name: "diffusion".to_owned(),
            variable: "u".to_owned(),
        },
        vec![],
    );
    let diffusion_path = target.join("diffusion.i");
    fs::write(&diffusion_path, diffusion.to_moose_input().unwrap()).unwrap();
    let diffusion_check = check_backend_input(&runner, &diffusion_path).unwrap();
    assert!(
        diffusion_check.success(),
        "generated diffusion input rejected:\n{}\n{}",
        diffusion_check.stdout_text(),
        diffusion_check.stderr_text()
    );

    let heat = base_model(
        MooseOperator::HeatConduction {
            name: "heat".to_owned(),
            variable: "u".to_owned(),
            thermal_conductivity_property: "thermal_conductivity".to_owned(),
        },
        vec![ConstantMaterialProperty {
            name: "conductivity".to_owned(),
            property: "thermal_conductivity".to_owned(),
            value: 45.0,
        }],
    );
    let heat_path = target.join("heat_conduction.i");
    fs::write(&heat_path, heat.to_moose_input().unwrap()).unwrap();
    let heat_check = check_backend_input(&runner, &heat_path).unwrap();
    assert!(
        heat_check.success(),
        "generated heat-conduction input rejected:\n{}\n{}",
        heat_check.stdout_text(),
        heat_check.stderr_text()
    );
}
