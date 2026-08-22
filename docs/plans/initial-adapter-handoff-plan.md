# Initial SOL MOOSE Adapter Handoff Plan

**Status:** SOL Platform handoff baseline; MOOSE-team implementation planning still required  
**Date:** 2026-08-22

## Purpose

This document transfers the adapter-related decisions already accepted in the SOL Platform roadmap and separates them from decisions that the future MOOSE Adapter Team must still make locally.

It is not an implementation milestone by itself.

## Accepted from the SOL Platform meeting

The following architecture/product decisions are already accepted for handoff:

1. The first real SOL backend adapter targets MOOSE.
2. The adapter lives in a separate repository from `simulation-ontology`.
3. The adapter consumes the published SOL Public Contract / Adapter Protocol boundary rather than linking MOOSE into SOL Core semantics.
4. The runtime model remains an external-process adapter; a future GUI may present it with plugin-like UX.
5. Solver-neutral adapter discovery/registry/runtime management belongs to the SOL Platform, not this repository.
6. MOOSE-specific translation, process integration, regression, and physical/numerical V&V belong to this repository.
7. Backend-native object identity cannot become canonical SOL semantic identity.
8. The first real-backend vertical slice should be deliberately narrow and prove the complete SOL -> MappingPlan -> Adapter -> MOOSE -> evidence path before plasma-domain expansion.
9. A simple steady heat-conduction realization is the current bootstrap target for that first vertical slice.
10. More specialized Zapdos/CRANE/plasma mappings are later work and must not force domain-specific backend concepts into SOL Core prematurely.

## Current upstream interoperability baseline

Bootstrap source:

```text
simulation-ontology main
69779d3ab7880f56618b29af82616965776e0126
```

Published interoperability baseline at handoff:

```text
Public Contract 0.1
Adapter Protocol 0.1
JSON-RPC / stdio local transport
M0.6 external-adapter conformance tooling complete
```

The adapter implementation must explicitly declare supported compatibility rather than infer it from this source SHA.

## First vertical-slice objective

Demonstrate a real end-to-end realization:

```text
SOL thermal model/intent
 -> canonical validation and MappingPlan
 -> Adapter Protocol request
 -> sol-adaptor-moose
 -> MOOSE realization artifact/input
 -> configured MOOSE application validation/execution
 -> Protocol action/effect/provenance response
 -> separate numerical verification evidence
```

The goal is architecture proof and contract feedback, not broad MOOSE physics coverage.

## Suggested first-slice semantic coverage

The MOOSE team should investigate the smallest coherent subset covering:

- one spatial domain/mesh strategy;
- one scalar temperature field;
- steady diffusion/heat-conduction behavior;
- thermal conductivity/material property;
- simple Dirichlet and/or Neumann boundary conditions;
- steady execution configuration;
- deterministic output/provenance collection;
- an analytical or trusted expected solution.

This list is a planning baseline. Exact canonical action types and MOOSE object mappings must be verified against the actual supported SOL contract and selected MOOSE application.

## Decisions deliberately left to the first MOOSE-team `meeting`

The SOL Platform handoff does **not** decide these implementation details:

1. adapter implementation language and package/build system;
2. exact MOOSE-based executable/application used as the first target;
3. initial supported MOOSE version/version range;
4. whether initial realization is `.i` generation only or uses a deeper MOOSE integration path;
5. exact adapter-local translation-layer/IR structure;
6. target/capability discovery mechanism and whether MOOSE `--json`/registry facilities are used;
7. deterministic workspace/artifact layout;
8. process timeout/cancellation/logging strategy;
9. how MOOSE installation/build is provided in CI;
10. exact steady-thermal SOL-to-MOOSE mapping table;
11. first numerical verification benchmark and tolerances;
12. repository release/versioning convention;
13. milestone/Phase names and issue structure;
14. public packaging/distribution method;
15. license choice if/when the repository is made public.

These are real decisions and must not be silently selected during bootstrap documentation.

## Recommended first MOOSE-team meeting agenda

### Decision 1 — Backend target

Select the exact MOOSE application/executable and version policy that can support the first thermal slice.

Evidence required:

- official MOOSE documentation;
- executable/application availability;
- registered syntax/capability evidence;
- CI feasibility.

### Decision 2 — Host implementation architecture

Compare at least:

- Rust adapter host following the SOL reference skeleton;
- Python host for faster MOOSE input/process integration;
- C++ host if direct MOOSE/native linkage is materially justified;
- mixed architecture where appropriate.

Evaluation criteria:

- conformance tooling integration;
- process isolation;
- packaging;
- testability;
- MOOSE environment integration;
- long-term maintainability.

Implementation convenience alone must not redefine SOL contracts.

### Decision 3 — Thermal mapping slice

Define a concrete canonical input and expected MOOSE artifact/result.

Produce:

- mapping table;
- positive fixture;
- unsupported/negative cases;
- backend parser/input validation case;
- trusted numerical benchmark.

### Decision 4 — CI and evidence matrix

Define separate jobs/evidence for:

```text
SOL conformance
translation regression
MOOSE integration
physical/numerical V&V
```

Decide which jobs are required for each Phase and milestone exit.

### Decision 5 — Milestone decomposition

Only after Decisions 1–4, create the first repository-local milestone/parent tracker/Phase issues.

A possible name such as `A0.1 — MOOSE Adapter Bootstrap` is only a placeholder until accepted by the MOOSE team.

## Cross-team feedback objective

The first real adapter is expected to stress SOL assumptions.

When a problem appears, classify it before escalation:

```text
adapter implementation bug
MOOSE application/version limitation
unsupported adapter capability
ambiguous local mapping design
SOL Public Contract gap
SOL Adapter Protocol gap
physical/numerical validation issue
```

Only the last two require canonical SOL semantic/contract review by the SOL Platform Team.

A cross-team handoff should contain a minimal concrete case, desired solver-independent meaning, current-contract limitation, counterexample, compatibility impact, and whether work is blocked.

## Bootstrap completion gate

The SOL Platform bootstrap is complete when this repository contains enough durable information for a new MOOSE-team chat to:

- run `moose-init` without private chat/Drive knowledge;
- understand the SOL/MOOSE authority boundary;
- understand Protocol 0.1 invariants;
- verify basic MOOSE framework facts from official public sources;
- identify accepted versus undecided adapter scope;
- conduct the first independent `meeting` without reconstructing the SOL history manually.

No adapter code is required for this bootstrap gate.
