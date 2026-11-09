# Adapter-local backend-only MOOSE IR. Not canonical SOL semantics.
[Mesh]
  type = GeneratedMesh
  dim = 1
  nx = 4
  xmin = 0
  xmax = 1
[]

[Variables]
  [T]
    order = FIRST
    family = LAGRANGE
  []
[]

[Kernels]
  [thermal_conduction]
    type = HeatConduction
    variable = T
    thermal_conductivity = thermal_conductivity
  []
[]

[Materials]
  [thermal_material]
    type = GenericConstantMaterial
    prop_names = 'thermal_conductivity'
    prop_values = '45'
  []
[]

[BCs]
  [hot_wall_temperature]
    type = DirichletBC
    variable = T
    boundary = 'left'
    value = 400
  []
[]

[Executioner]
  type = Steady
[]

[Outputs]
  exodus = false
[]
