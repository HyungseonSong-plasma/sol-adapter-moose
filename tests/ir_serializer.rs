use sol_adaptor_moose::ir::{
    ConstantMaterialProperty, DirichletBoundary, Executioner, GeneratedLineMesh, MooseInputModel,
    MooseOperator, Outputs, ScalarVariable, VariableFamily, VariableOrder,
};

fn diffusion_fixture() -> MooseInputModel {
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
        operators: vec![MooseOperator::Diffusion {
            name: "diffusion".to_owned(),
            variable: "u".to_owned(),
        }],
        materials: vec![],
        boundary_conditions: vec![
            DirichletBoundary {
                name: "right".to_owned(),
                variable: "u".to_owned(),
                boundary: "right".to_owned(),
                value: 1.0,
            },
            DirichletBoundary {
                name: "left".to_owned(),
                variable: "u".to_owned(),
                boundary: "left".to_owned(),
                value: 0.0,
            },
        ],
        executioner: Executioner::Steady,
        outputs: Outputs { exodus: false },
    }
}

#[test]
fn serializer_matches_backend_only_golden() {
    let actual = diffusion_fixture().to_moose_input().unwrap();
    let expected = include_str!("fixtures/backend/phase2_generated_diffusion.i");
    assert_eq!(actual, expected);
}

#[test]
fn serializer_is_stable_against_input_vector_order() {
    let mut first = diffusion_fixture();
    let mut second = diffusion_fixture();
    second.boundary_conditions.reverse();
    first.variables.push(ScalarVariable {
        name: "z".to_owned(),
        order: VariableOrder::Second,
        family: VariableFamily::Lagrange,
    });
    second.variables.push(ScalarVariable {
        name: "z".to_owned(),
        order: VariableOrder::Second,
        family: VariableFamily::Lagrange,
    });
    second.variables.reverse();
    assert_eq!(
        first.to_moose_input().unwrap(),
        second.to_moose_input().unwrap()
    );
}

#[test]
fn heat_conduction_requires_declared_material_property() {
    let mut model = diffusion_fixture();
    model.operators = vec![MooseOperator::HeatConduction {
        name: "heat".to_owned(),
        variable: "u".to_owned(),
        thermal_conductivity_property: "thermal_conductivity".to_owned(),
    }];
    let error = model.to_moose_input().unwrap_err();
    assert!(error.to_string().contains("missing material property"));

    model.materials.push(ConstantMaterialProperty {
        name: "conductivity".to_owned(),
        property: "thermal_conductivity".to_owned(),
        value: 45.0,
    });
    let rendered = model.to_moose_input().unwrap();
    assert!(rendered.contains("type = HeatConduction"));
    assert!(rendered.contains("type = GenericConstantMaterial"));
    assert!(rendered.contains("prop_names = 'thermal_conductivity'"));
}

#[test]
fn invalid_identifiers_and_non_finite_values_are_rejected() {
    let mut model = diffusion_fixture();
    model.variables[0].name = "u bad".to_owned();
    assert!(model.to_moose_input().is_err());

    let mut model = diffusion_fixture();
    model.boundary_conditions[0].value = f64::NAN;
    assert!(model.to_moose_input().is_err());
}
