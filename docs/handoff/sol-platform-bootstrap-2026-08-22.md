# SOL Platform -> MOOSE Adapter Team Bootstrap Handoff

**Date:** 2026-08-22  
**Prepared by:** SOL Platform Team  
**Receiving team:** SOL MOOSE Adapter Team (to be initialized)  
**Repository:** `HyungseonSong-plasma/sol-adaptor-moose`

## Handoff purpose

The MOOSE Adapter Team is a separate repository/team context from the SOL Platform Team. Because the receiving team did not yet exist when this repository was created, the SOL Platform Team seeded the durable minimum context needed for independent operation.

The receiving team should not need private chat history or an external private operating guide to understand the repository's purpose, authority boundaries, initialization flow, or starting technical questions.

## SOL source state

The bootstrap was prepared from the verified SOL Platform `main` state:

```text
HyungseonSong-plasma/simulation-ontology
main = 69779d3ab7880f56618b29af82616965776e0126
```

Relevant accepted upstream state at handoff:

- M0.1 Semantic Core Bootstrap complete;
- M0.2 Public Contract 0.1 complete;
- M0.3 Adapter Protocol 0.1 complete;
- M0.4 MockAdapter reference conformance complete;
- M0.5 JSON-RPC/stdin-stdout transport complete;
- M0.6 reusable external-adapter conformance tooling complete.

The adapter authoring baseline already establishes:

- dual Public Contract / Adapter Protocol compatibility;
- advisory side-effect-free `validate_plan`;
- authoritative current-state `execute_plan`;
- non-idempotent-by-default execution and no automatic replay after ambiguous response loss;
- dependency-preserving backend scheduling;
- opaque backend provenance separate from canonical identity;
- separation of protocol conformance from backend physical/numerical correctness.

## Documents transferred into this repository

### Operating entry

- `AGENTS.md`
- `docs/operations/project-session-init.md`
- `docs/operations/logical-agent-workflow.md`

These documents contain the MOOSE-team operating rules locally. They do not require a private external operating guide.

### SOL knowledge

- `docs/knowledge/sol-adapter-baseline-0.1.md`

This records the minimum Public Contract / Adapter Protocol / MappingPlan / conformance boundary needed by adapter developers. It is a bootstrap snapshot and does not replace upstream SOL semantic authority.

### MOOSE knowledge

- `docs/knowledge/moose-foundation.md`

This records framework facts and adapter implications backed by public MOOSE documentation. Application-specific facts must still be verified against the selected executable/version.

### Architecture

- `docs/architecture/sol-moose-boundary.md`

This fixes the Core-versus-adapter-versus-MOOSE responsibility boundary and cross-team escalation rule.

### Plan

- `docs/plans/initial-adapter-handoff-plan.md`

This separates SOL-approved direction from implementation decisions deliberately left for the first MOOSE-team meeting.

## Public-source MOOSE references used during bootstrap

Primary public source:

- https://mooseframework.inl.gov/

Key pages used to establish the initial backend foundation:

- command-line usage and input execution;
- application-development documentation;
- framework system design;
- computational backend/object-family documentation;
- boundary-condition syntax;
- examples/tutorials and workshop material.

These sources support the initial facts that MOOSE-based applications are executable/input driven, expose input checking and syntax/introspection facilities, and are extensible through framework/application object systems.

## Accepted direction transferred from SOL planning

The following is the current accepted strategic direction:

```text
SOL Platform track
  -> solver-neutral Adapter Runtime / Registry / SDK/GUI boundary

Real-adapter track
  -> sol-adaptor-moose
  -> external process
  -> first narrow thermal vertical slice
```

The GUI may eventually make adapters feel plugin-like, but this repository should remain an external adapter and should not own the canonical solver-neutral GUI registry.

## Receiving-team first action

A new MOOSE-team chat should start with:

```text
moose-init
```

That bootstrap is read-only. Because this repository is currently only a seeded foundation, the expected next mode after the first initialization is likely:

```text
meeting
```

The first meeting should resolve the implementation language, exact MOOSE target/version, thermal mapping scope, CI environment, conformance consumption model, and numerical V&V benchmark before implementation issues are created.

## Cross-team authority reminder

The receiving MOOSE team may discover that real MOOSE behavior stresses SOL assumptions. That is expected and valuable.

However:

```text
MOOSE Team discovers and evidences contract pressure
SOL Platform Team decides canonical semantic/contract evolution
MOOSE Team consumes the accepted versioned result
```

Do not silently modify Public Contract 0.1 or Adapter Protocol 0.1 meaning inside this repository.

## Bootstrap completion statement

This handoff establishes repository-local operating context and technical starting knowledge only. It does not claim that the MOOSE adapter is implemented, protocol-conformant, MOOSE-integrated, physically validated, or production-ready.
