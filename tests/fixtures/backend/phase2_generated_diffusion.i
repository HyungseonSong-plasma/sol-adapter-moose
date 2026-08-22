# Adapter-local backend-only MOOSE IR. Not canonical SOL semantics.
[Mesh]
  type = GeneratedMesh
  dim = 1
  nx = 4
  xmin = 0
  xmax = 1
[]

[Variables]
  [u]
    order = FIRST
    family = LAGRANGE
  []
[]

[Kernels]
  [diffusion]
    type = Diffusion
    variable = u
  []
[]

[BCs]
  [left]
    type = DirichletBC
    variable = u
    boundary = 'left'
    value = 0
  []
  [right]
    type = DirichletBC
    variable = u
    boundary = 'right'
    value = 1
  []
[]

[Executioner]
  type = Steady
[]

[Outputs]
  exodus = false
[]
