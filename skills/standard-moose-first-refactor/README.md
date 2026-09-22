# standard-moose-first-refactor

**Status:** adapter-local reusable skill contract  
**Scope:** MOOSE realization refactors  
**Activation:** trigger when a custom MOOSE object/helper is being added, replaced, migrated, or retired.

## Purpose

Prefer standard MOOSE capabilities and composition before maintaining custom C++ ownership.

This skill provides a repeatable capability census and migration procedure. It does not choose scientific equations, coefficients, signs, units, or model closures.

## Ownership boundary

This skill owns:

- MOOSE capability census;
- classification of standard-object/composition options;
- custom-code justification gate;
- Unity-build compatibility as a required C++ validation surface;
- `--check-input` and bounded-runtime validation ordering;
- custom-owner retirement procedure.

It does not own:

- plasma/scientific semantics;
- SOL Public Contract meaning;
- numerical acceptance thresholds;
- selection of physical wall/sheath/RF models;
- compatibility commitments not already accepted;
- scientific promotion.

When ownership moves between components/repositories, compose this skill with the central `characterized-ownership-migration` contract.

## Required inputs

```text
accepted semantic contract
current MOOSE owner(s)
current consumer/caller set
target MOOSE application/version identity
required backend behavior
required failure behavior
validation fixtures/evidence
retirement target
```

The semantic contract must already be accepted. If it is not, return to the appropriate design/meeting path.

## Capability census

For each required behavior, classify exactly one primary route:

```text
STANDARD_OBJECT
STANDARD_COMPOSITION
PARSED_FUNCTOR_COMPOSITION
THIN_ADAPTER
REPRESENTATION_MISMATCH
TRUE_CAPABILITY_GAP
```

### STANDARD_OBJECT

A standard MOOSE object directly represents the required backend semantics.

Prefer this route when available.

### STANDARD_COMPOSITION

Multiple standard MOOSE objects compose to represent the behavior without custom C++.

Examples include standard Materials/Functors/BCs/Kernels/Postprocessors wired together declaratively.

### PARSED_FUNCTOR_COMPOSITION

The behavior can be expressed with standard parsed/functor/material objects plus standard solver objects.

Typical examples:

```text
ADParsedFunctorMaterial
ParsedFunctorMaterial
ADGenericFunctorMaterial
FVFunctorNeumannBC
FVCoupledForce
standard Postprocessors
```

This route is preferred over custom C++ when it preserves required state evaluation, AD behavior, units, and sign semantics.

### THIN_ADAPTER

A small adapter is required for representation/translation mechanics, but scientific meaning remains outside the adapter.

The adapter must not become a second semantic authority.

### REPRESENTATION_MISMATCH

A MOOSE capability exists but represents a materially different mathematical/discretization object.

A representation mismatch is not a true capability gap and does not justify silently changing the model.

### TRUE_CAPABILITY_GAP

No standard object, standard composition, parsed/functor composition, or acceptable thin adapter can preserve the required contract.

Custom C++ is permitted only after this classification is demonstrated.

## Decision hierarchy

```text
semantic/physics fidelity
> accepted discretization and state semantics
> validation tractability
> standard MOOSE object
> standard composition
> parsed/functor composition
> thin adapter
> custom C++
```

Implementation convenience must not override semantic fidelity.

## Algorithm

### MF-01 — Freeze the required contract

Record the behavior being preserved:

```text
equation / flux / source meaning
normalization
sign convention
units
state side / face / element evaluation
AD/Jacobian expectations
boundary scope
failure domain
observables
```

Do not begin implementation while these are ambiguous.

### MF-02 — Census MOOSE capability

Search current configured MOOSE/application capability rather than relying only on historical assumptions.

Classify each required term using the capability census above.

### MF-03 — Select the least-custom valid realization

Choose the highest valid route in this order:

```text
STANDARD_OBJECT
STANDARD_COMPOSITION
PARSED_FUNCTOR_COMPOSITION
THIN_ADAPTER
TRUE_CAPABILITY_GAP -> CUSTOM_CPP
```

Document why rejected higher-priority routes cannot preserve the contract.

### MF-04 — Characterize current custom behavior

Before replacing an existing custom owner, preserve the externally relevant behavior with tests or evidence.

At minimum capture material dimensions from MF-01 plus known positive/negative controls.

### MF-05 — Implement a bounded replacement

Replace only the targeted owner/behavior.

Do not mix physics changes, coefficient changes, timestep changes, unrelated cleanup, or broader ownership movement into the same refactor unless separately accepted.

### MF-06 — Validate construction

Required construction checks, when applicable:

```text
input/render generation
reference resolution
registered-object availability
standard-object parameters
sign/unit wiring
state/functor ownership
```

### MF-07 — Validate C++/Unity compatibility

If any custom C++ remains or was touched:

- build with the repository's actual MOOSE build mode;
- treat Unity-build symbol collisions as first-class failures;
- avoid translation-unit assumptions that disappear under Unity aggregation;
- do not reintroduce shared custom helpers merely to hide a local namespace collision.

A normal separate-compilation mental model is insufficient evidence for MOOSE applications that use Unity sources.

### MF-08 — Run `--check-input`

After a successful application build, validate generated representative inputs with the configured application executable.

`--check-input` establishes backend syntax/registration compatibility only. It does not establish physical correctness.

### MF-09 — Run bounded runtime characterization

Execute the smallest runtime needed to prove the replaced owner is active and preserves the required contract.

Use exact configured application/runtime provenance.

### MF-10 — Establish parity

Compare replacement behavior with the characterized contract.

Relevant dimensions may include:

```text
construction
sign
units
state dependence
flux/source magnitude
failure behavior
integrated balance
runtime observables
```

Scientific acceptance criteria remain consumer-owned.

### MF-11 — Cut over consumers and retire custom owner

When composed with `characterized-ownership-migration`:

```text
parity established
 -> canonical consumers cut over
 -> zero canonical callers
 -> custom C++ owner/header/helper removed
 -> exact-head validation
```

Do not delete the old owner while canonical or required compatibility consumers still depend on it.

## Custom-code authorization gate

Custom C++ should be introduced or retained only when all are true:

```text
SEMANTIC_CONTRACT_FROZEN = true
STANDARD_CAPABILITY_CENSUS = complete
STANDARD_OBJECT_EQUIVALENT = none
STANDARD_COMPOSITION_EQUIVALENT = none
PARSED_FUNCTOR_EQUIVALENT = none
THIN_ADAPTER_SUFFICIENT = false
TRUE_CAPABILITY_GAP = explicit
INDEPENDENT_VALIDATION_PLAN = ready
POSITIVE_CONTROL = defined
NEGATIVE_OR_COUNTERFACTUAL_CONTROL = defined
```

## Validation ladder

Default ordering:

```text
static contract / characterization
 -> application build
 -> Unity compatibility
 -> --check-input
 -> bounded runtime
 -> parity evidence
 -> consumer cutover
 -> zero-caller scan
 -> custom-owner deletion
 -> exact-head CI
```

A later stage must not be used to excuse a failure at an earlier semantic gate.

## Core invariants

```text
STANDARD_MOOSE_FIRST = true
CUSTOM_CPP_BY_DEFAULT = false
REPRESENTATION_MISMATCH_IS_CAPABILITY_GAP = false
PHYSICS_CHANGE_HIDDEN_IN_REFACTOR = false
CHECK_INPUT_IMPLIES_PHYSICS_PASS = false
RUNTIME_SUCCESS_IMPLIES_SCIENTIFIC_PASS = false
UNITY_BUILD_IS_VALIDATION_SURFACE = true
RETIRE_WITH_CANONICAL_CALLERS = false
```

## Typical example

A custom electron wall BC whose algebra can be represented as:

```text
ADParsedFunctorMaterial
  -> wall flux functor

FVFunctorNeumannBC
  -> applies outward/inward flux
```

should normally be migrated to that composition rather than keeping a bespoke C++ BC, provided state evaluation, signs, units, and Jacobian behavior satisfy the accepted contract.

The choice of particle prefactor, sheath law, energy-per-particle law, or other scientific closure is outside this skill.

## Failure classifications

- `BLOCKED_SEMANTIC_CONTRACT`
- `CAPABILITY_CENSUS_INCOMPLETE`
- `REPRESENTATION_MISMATCH`
- `TRUE_CAPABILITY_GAP`
- `CONSTRUCTION_FAILED`
- `UNITY_BUILD_FAILED`
- `CHECK_INPUT_FAILED`
- `RUNTIME_CHARACTERIZATION_FAILED`
- `PARITY_NOT_ESTABLISHED`
- `RETIREMENT_BLOCKED`
- `STANDARD_MOOSE_REFACTOR_VALIDATED`

## Durable evidence

A completed refactor should record:

```text
accepted semantic contract
MOOSE/application identity
capability classification
selected realization
rejected higher-priority routes and rationale
build/Unity result
--check-input result
runtime characterization
parity result
remaining callers
retired custom files
exact validated revision
claims explicitly not established
```
