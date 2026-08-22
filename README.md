# sol-adaptor-moose

MOOSE backend adapter for the Simulation Ontology (SOL) ecosystem.

> This is an independent SOL ecosystem project. It is **not** an official component of, endorsed by, or maintained by the MOOSE Framework or Idaho National Laboratory.

## Status

**Bootstrap / pre-A0.1.**

The SOL Platform Team seeded this repository on 2026-08-22 so that a dedicated MOOSE Adapter Team can begin from an explicit interoperability, architecture, and operating baseline rather than reconstructing SOL knowledge from chat history.

The source SOL Platform baseline for this handoff is:

```text
HyungseonSong-plasma/simulation-ontology
main: 69779d3ab7880f56618b29af82616965776e0126
Public Contract: 0.1
Adapter Protocol: 0.1
M0.6 Adapter Conformance Tooling: complete
```

## Purpose

This repository realizes solver-independent SOL `MappingPlan` intent through a MOOSE-based backend while preserving the canonical SOL contract boundary.

Conceptually:

```text
SOL canonical model
      |
      v
MappingPlan + BackendTarget
      |
      v
Adapter Protocol 0.1
      |
      v
sol-adaptor-moose
      |
      +--> MOOSE input/backend artifacts
      +--> configured MOOSE-based application
      +--> backend execution/provenance
      +--> realization evidence
```

The adapter owns MOOSE-specific realization. It does **not** redefine canonical SOL semantics.

## Non-goals

This repository does not:

- move MOOSE-native object identity into the SOL ontology;
- redefine Public Contract or Adapter Protocol meanings;
- make protocol conformance equivalent to physical/numerical correctness;
- make the SOL Core depend on a MOOSE runtime;
- claim official MOOSE Framework project status;
- treat a successful MOOSE input check or solve as sufficient SOL semantic validation.

## Core responsibility split

### SOL Platform repository owns

- canonical ontology and semantic identity;
- Public Contract and schemas;
- Adapter Protocol;
- `MappingPlan` semantics and dependency rules;
- reusable adapter conformance tooling;
- solver-neutral Adapter Runtime / Registry and SDK surfaces.

### This repository owns

- SOL-to-MOOSE translation;
- MOOSE-target capability reporting;
- backend artifact/input generation;
- configured MOOSE application invocation;
- MOOSE-version/application compatibility evidence;
- solver-specific regression testing;
- backend physical/numerical verification and validation where applicable;
- opaque backend provenance that does not define SOL semantic identity.

## Adapter Protocol baseline

The published logical operation surface is:

```text
describe_adapter -> AdapterDescription | ProtocolFailure
validate_plan    -> ValidatePlanResponse | ProtocolFailure
execute_plan     -> ExecutePlanResponse | ProtocolFailure
```

Key invariants carried into this repository:

- interoperability requires **Adapter Protocol compatibility AND Public Contract compatibility**;
- `validate_plan` is advisory and side-effect free;
- `execute_plan` is authoritative and re-checks current request/backend state;
- `execute_plan` is non-idempotent by default;
- lost execute responses do not authorize automatic replay;
- dependency edges must be preserved even if the backend reorders independent actions;
- backend-native IDs may be opaque provenance, never canonical SOL semantic identity;
- protocol conformance and backend physical/numerical V&V are separate evidence tracks.

See [`docs/knowledge/sol-adapter-baseline-0.1.md`](docs/knowledge/sol-adapter-baseline-0.1.md).

## MOOSE foundation

MOOSE-based applications are normally driven by a block-structured input file and executable. The framework exposes command-line facilities including `-i <input_file>`, `--check-input`, and syntax/introspection output such as `--json`. MOOSE applications can register framework and application-specific object types, so adapter capability discovery must be based on the configured target application rather than assuming every MOOSE application has identical capabilities.

See [`docs/knowledge/moose-foundation.md`](docs/knowledge/moose-foundation.md) for the initial official-reference-backed foundation and adapter implications.

## Initial vertical slice

The current handoff baseline proposes a deliberately narrow first real-backend path:

```text
SOL thermal intent
   -> MappingPlan
   -> Adapter Protocol
   -> sol-adaptor-moose
   -> simple steady heat-conduction MOOSE realization
   -> backend artifact / execution evidence
```

The first MOOSE-team `meeting` must turn this into a repository-local milestone with explicit implementation language, supported MOOSE application/version, mapping scope, CI strategy, and V&V gates before code implementation begins.

## Repository operating model

Logical project roles are:

- `Manager` — decision orchestration (`meeting`)
- `Planner` — scope, sequencing, milestones, acceptance criteria
- `Researcher` — MOOSE realization architecture and SOL-boundary investigation
- `Validator` — adversarial architecture/compatibility/solver-boundary validation
- `Operator` — accepted implementation (`resume`) and documentation synchronization (`update`)

A new or uncertain MOOSE-team chat should use `moose-init` and follow [`docs/operations/project-session-init.md`](docs/operations/project-session-init.md) as a read-only bootstrap.

See [`AGENTS.md`](AGENTS.md) and [`docs/operations/logical-agent-workflow.md`](docs/operations/logical-agent-workflow.md).

## Public-repository hygiene

This repository is intended to be publishable. Do not commit:

- private Google Drive references or operating-guide dependencies;
- secrets, tokens, credentials, private filesystem paths, or private CI endpoints;
- proprietary solver artifacts or licensed content without redistribution rights;
- internal-only conversation transcripts as project authority;
- wording that implies MOOSE/INL sponsorship or endorsement.

Durable project rules required to operate this repository must live in the repository itself.

## Primary external reference

MOOSE Framework documentation: https://mooseframework.inl.gov/

Relevant starting points are recorded in [`docs/knowledge/moose-foundation.md`](docs/knowledge/moose-foundation.md).
