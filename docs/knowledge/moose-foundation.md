# MOOSE Foundation for SOL Adapter Development

**Status:** bootstrap backend knowledge  
**Date:** 2026-08-22  
**Primary source:** public MOOSE Framework documentation

## Purpose

This document records the minimum MOOSE framework knowledge needed to start `sol-adaptor-moose` without confusing MOOSE realization concepts with canonical SOL semantics.

Framework facts should be re-verified against the public MOOSE documentation and the actual configured MOOSE-based application/version used by this adapter.

## 1. MOOSE is an application framework

MOOSE is a multiphysics framework used to build simulation applications. A MOOSE-based application may include framework objects, MOOSE modules, and application-specific objects/syntax.

Official references:

- MOOSE home: https://mooseframework.inl.gov/
- Application development: https://mooseframework.inl.gov/application_development/index.html
- Framework system design: https://mooseframework.inl.gov/sqa/framework_sdd.html

### Adapter implication

The adapter should not assume that the abstract word `MOOSE` uniquely defines one fixed backend capability set.

A real target is closer to:

```text
configured MOOSE-based application
+ application/framework version
+ registered objects/modules/syntax
+ runtime environment
```

The exact representation of this target is a MOOSE-team design decision and must remain compatible with SOL `BackendTarget` semantics.

## 2. Normal execution model

MOOSE-based applications are commonly run as an executable with one or more input files. The standard command-line interface includes:

```text
./yourapp-opt -i input.i
```

The official command-line documentation also exposes options such as:

- `--check-input` — parse/check the supplied input and quit;
- `--json` — dump application input syntax information in JSON form;
- `--help` — display CLI usage;
- `--list-constructed-objects` — list object type names constructed by the application factory in supported versions.

Official reference:

- Command-line usage: https://mooseframework.inl.gov/application_usage/command_line_usage.html

### Adapter implication

A practical first adapter can treat the generated MOOSE input as a backend artifact and invoke a configured MOOSE executable as a child process.

Conceptually:

```text
SOL MappingPlan
 -> translation
 -> generated input.i
 -> optional backend check
 -> MOOSE execution
 -> output/log/artifact capture
 -> canonical realization/provenance evidence
```

This does not mean that an `.i` file becomes the SOL semantic model.

## 3. Input syntax is block/object oriented

MOOSE applications use customizable block-based input syntax. Applications register object families and syntax used to configure the simulation.

Common MOOSE/libMesh object families include concepts such as:

- variables;
- kernels / AD kernels;
- boundary conditions;
- materials;
- mesh;
- executioners;
- auxiliary kernels/variables;
- user objects;
- postprocessors;
- actions.

Official references:

- Application development: https://mooseframework.inl.gov/application_development/index.html
- Computational backends: https://mooseframework.inl.gov/framework_development/computational_backends.html
- BC system: https://mooseframework.inl.gov/syntax/BCs/
- Framework examples/tutorials: https://mooseframework.inl.gov/getting_started/examples_and_tutorials/index.html

### Adapter implication

These are **backend realization constructs**.

Invalid reasoning:

```text
MOOSE has Kernel
therefore SOL ontology must contain MOOSE Kernel
```

Correct reasoning:

```text
SOL operator/equation/action semantics
 -> mapping rule
 -> MOOSE realization using one or more Kernel/BC/Material/etc. objects
```

A single SOL concept may map to several MOOSE objects, and one MOOSE object may participate in realizing several canonical concerns. Mapping must therefore be semantic rather than name-based.

## 4. Boundary conditions

MOOSE has dedicated boundary-condition systems. The public documentation describes nodal/integrated forms and common Dirichlet, Neumann, and Robin-style behavior, with finite-volume BC systems handled separately where applicable.

Official reference:

- BC system: https://mooseframework.inl.gov/syntax/BCs/

### Adapter implication

The adapter should map canonical SOL condition semantics and spatial scope to MOOSE boundary realization. It should not expose a MOOSE boundary object name as canonical condition identity.

Tests should distinguish at least:

- semantic condition type/value;
- canonical spatial applicability;
- generated MOOSE block/object;
- backend parser acceptance;
- numerical effect in a trusted fixture.

## 5. Materials

MOOSE `Material` objects provide properties used by other systems such as kernels and boundary conditions. Material properties may depend on current solution variables and other material properties, and MOOSE supports automatic-differentiation-based material workflows.

Official entry points can be reached from the MOOSE syntax/application-development documentation.

### Adapter implication

SOL `MaterialModel` / constitutive parameter semantics must not be collapsed into the existence of a MOOSE `Material` object. The translation layer should explicitly document which canonical property/constitutive meaning is being realized and how.

## 6. Backend introspection is promising but not canonical

The MOOSE executable exposes syntax/introspection-related command-line capabilities such as `--json` and, in supported application versions, object listing/registry information.

Potential adapter uses include:

- discover registered application syntax;
- verify required object availability;
- derive adapter capability evidence;
- validate configured target compatibility.

However:

- the exact introspection output is a MOOSE application interface, not SOL semantic truth;
- application-specific registration means capabilities may differ across executables;
- the adapter must normalize backend discovery into SOL-compatible target/capability declarations rather than exposing the raw registry as canonical ontology.

This should be investigated in the first MOOSE-team architecture milestone.

## 7. `--check-input` is useful but limited

`--check-input` can establish that the target application accepts a generated input structurally/configurationally enough to pass its input check.

It does **not** by itself establish:

- SOL Public Contract validity;
- Adapter Protocol conformance;
- canonical mapping correctness;
- successful nonlinear/linear convergence;
- physical/numerical correctness.

Treat it as one backend validation evidence class.

## 8. Process and logging separation

The SOL local adapter transport uses JSON-RPC over the adapter process's stdin/stdout. A MOOSE application can emit normal console output.

Therefore a safe process architecture should conceptually isolate streams:

```text
SOL caller
  <-> adapter JSON-RPC stdin/stdout

adapter
  -> MOOSE child process
       stdout/stderr captured separately
```

MOOSE console output must not corrupt the adapter Protocol framing channel.

The exact subprocess/log/artifact strategy is an adapter-local architecture decision.

## 9. Testing and verification culture

MOOSE documentation includes application development, test-system, examples/tutorials, and verification-oriented material. The adapter should use MOOSE-native regression/verification practices where useful, but keep them distinct from SOL conformance evidence.

Useful official starting points:

- Application development: https://mooseframework.inl.gov/application_development/index.html
- Examples/tutorials: https://mooseframework.inl.gov/getting_started/examples_and_tutorials/index.html
- MOOSE user workshop: https://mooseframework.inl.gov/user_workshop/index.html

## 10. First vertical-slice recommendation

The bootstrap target is a simple steady heat-conduction problem because it can exercise a meaningful subset of MOOSE realization concepts without introducing plasma-domain complexity.

The MOOSE team should define a minimal trusted fixture that covers, as applicable:

- mesh/spatial domain;
- temperature field/variable;
- diffusion/heat-conduction operator realization;
- thermal conductivity material/property;
- boundary conditions;
- steady execution configuration;
- output/provenance capture;
- expected solution or benchmark for numerical verification.

The exact MOOSE objects/modules/application and supported version are not fixed by this bootstrap document. They must be selected and validated in `meeting` before implementation.

## 11. Questions for the first MOOSE-team meeting

1. Which MOOSE-based executable/application is the first supported target?
2. Which MOOSE version or version range is supported initially?
3. Is the adapter implemented as Rust, Python, C++, or another host, and why?
4. Will `.i` generation be the first realization path, or is a deeper API/library integration required?
5. Which executable introspection mechanisms are sufficiently stable for capability discovery?
6. What is the minimal steady-thermal mapping contract?
7. Which checks belong to `validate_plan` versus `execute_plan`?
8. Which backend outputs count only as provenance and which can support canonical realization evidence?
9. How will CI obtain/build/cache the target MOOSE application?
10. What analytical/trusted benchmark closes the first physical/numerical V&V gate?
