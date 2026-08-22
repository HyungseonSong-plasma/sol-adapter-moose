# SOL-to-MOOSE Adapter Architecture Boundary

**Status:** accepted bootstrap boundary / implementation details pending MOOSE-team meeting  
**Date:** 2026-08-22

## Purpose

This document fixes the responsibility boundary that the SOL Platform Team hands to the future MOOSE Adapter Team. It intentionally does not choose the adapter implementation language, MOOSE version, exact target application, or detailed mapping table.

## System boundary

```text
SOL Platform
  canonical ontology
  Public Contract
  MappingPlan
  Adapter Protocol
  conformance tooling
        |
        | versioned language-neutral contract
        v
sol-adaptor-moose
  protocol endpoint
  target/capability adapter logic
  SOL -> MOOSE translation layer
  backend artifact generation
  MOOSE process integration
  backend regression / V&V
        |
        v
configured MOOSE-based application
```

The adapter is an external SOL ecosystem component. The Core repository must not acquire a MOOSE runtime dependency merely because this adapter exists.

## Canonical versus runtime concepts

Keep three concept classes distinct:

### Canonical SOL

Examples:

- simulation/model semantics;
- fields/operators/conditions/material/space semantics;
- `MappingPlan` and action dependencies;
- `BackendTarget` payload meaning;
- canonical realization effects and subjects.

### Adapter runtime

Examples:

- adapter executable path;
- adapter process/session;
- configured MOOSE executable;
- working directory;
- subprocess environment;
- timeout/log/artifact handling;
- local cache.

### MOOSE realization

Examples:

- input blocks;
- variables;
- kernels;
- BC objects;
- materials;
- executioners;
- MOOSE output files;
- application-specific objects and syntax.

Runtime and MOOSE realization objects are not canonical ontology identities.

## Translation layer

The translation layer should consume canonical plan semantics and produce an explicit backend realization model/artifact.

A useful conceptual split is:

```text
Protocol DTOs
   |
   v
Adapter semantic checks
   |
   v
MOOSE realization IR / builder  # adapter-local if useful
   |
   v
MOOSE input/backend artifacts
   |
   v
MOOSE executable
```

An adapter-local MOOSE realization IR may be introduced if it reduces coupling or improves testability, but it must be clearly non-canonical and must not escape as SOL semantic truth.

## Backend target and capability boundary

`describe_adapter` must declare supported targets/capabilities using accepted SOL Protocol semantics.

Because MOOSE is an extensible application framework, the adapter must not assume every MOOSE-based executable has the same capabilities. Target/capability evidence should ultimately be grounded in the configured application/version and adapter-supported mapping set.

Potential backend introspection is implementation evidence only. Raw MOOSE registry/syntax output must be normalized before becoming a Protocol capability declaration.

## Validation boundary

### SOL / Protocol checks

Examples:

- target compatibility;
- declared capability support;
- supported plan action types;
- dependency/contract invariants.

### Backend preflight evidence

Examples:

- configured MOOSE executable exists/is runnable;
- required registered objects/syntax are available;
- generated candidate input passes applicable backend checks;
- required files/resources are available.

Backend preflight may support a `validate_plan` decision, but it cannot alter the side-effect-free/advisory meaning of `validate_plan`.

## Execution boundary

`execute_plan` must re-check the current request and relevant backend state before side effects.

A first implementation may conceptually:

1. normalize/check target and plan;
2. build MOOSE realization artifacts;
3. write a deterministic backend workspace/input;
4. optionally run backend input validation;
5. invoke the configured MOOSE application;
6. capture exit/process/output evidence;
7. translate backend results into Protocol action/effect/provenance evidence;
8. return a Protocol response while keeping backend-native identity opaque.

The exact transaction/workspace strategy is a MOOSE-team decision.

## Side effects and replay

Creating files, spawning MOOSE, modifying a backend workspace, or launching a solver job may be side effects.

The implementation must preserve the Protocol rule:

```text
ambiguous execute response loss
 -> effects may have occurred
 -> automatic replay forbidden
```

Do not infer idempotency from deterministic input generation.

## Provenance

Backend provenance may include namespaced opaque evidence such as:

- MOOSE application/version;
- execution reference;
- generated artifact reference;
- output reference;
- backend run metadata.

It must not replace canonical semantic subject IDs or define equality.

## Conformance versus V&V

The repository should eventually expose separate CI/evidence jobs for:

```text
1. SOL Protocol/Public Contract conformance
2. translation/unit regression
3. MOOSE executable integration regression
4. physical/numerical V&V
```

A Phase may require one or several of these, but acceptance criteria must state which evidence is claimed.

## GUI/plugin relationship

The intended SOL GUI experience may treat installed adapters like selectable plugins, but plugin discovery/registry/runtime selection is a solver-neutral SOL Platform responsibility.

This repository should expose a clean external adapter executable/contract boundary. It should not implement the canonical GUI Adapter Registry or make the GUI understand MOOSE object types.

## Cross-team change rule

If a required MOOSE realization cannot be expressed with the supported SOL contract without changing meaning, the adapter team must raise a durable cross-team issue/handoff. The SOL Platform Team owns any canonical semantic/Public Contract/Adapter Protocol decision.

The adapter may continue only with a clearly contract-preserving local solution or after consuming the accepted versioned upstream decision.
