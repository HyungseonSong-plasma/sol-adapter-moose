# SOL MOOSE Adapter Team — Agent Entry Point

This repository uses logical project roles and explicit work modes. They are project operating concepts, not runtime SOL or MOOSE agents.

This file is intentionally self-contained enough for a future public repository. Do not depend on private Google Drive operating guides or chat memory to operate the project.

## Repository identity

- Repository: `HyungseonSong-plasma/sol-adaptor-moose`
- Team context: SOL MOOSE Adapter Team
- Product role: backend adapter that realizes SOL intent through a configured MOOSE-based application
- Upstream semantic authority: SOL Public Contract and Adapter Protocol versions explicitly declared as supported by this adapter
- Backend factual authority: public MOOSE Framework documentation and verified behavior of the configured MOOSE application/version

This is an independent SOL ecosystem project and is not an official MOOSE Framework or Idaho National Laboratory component.

## New or uncertain session

When the user sends `moose-init`, use the exact consumer binding in `docs/operations/chatgpt-operation-binding.json`.

Initialization must:

1. verify the pinned central revision and `OS = Paul`;
2. read Paul `docs/operating_system/ESSENTIAL_RULES.md`;
3. load the bound `session-bootstrap` and `state-refresh` skills;
4. apply the adapter-local additions in `docs/operations/project-session-init.md`;
5. restore current SOL/MOOSE repository and compatibility state;
6. report the first real gate and recommended next mode;
7. stop read-only.

Generic pinning, memory/current-evidence, evidence-boundary, real-gate, documentation-truth, and interruption rules are owned centrally by Paul and are not duplicated here.

## Canonical local modes

- `moose-init` — read-only MOOSE-team session bootstrap;
- `meeting` — Manager-led decision process;
- `resume` — Operator execution of accepted work until a real gate;
- `update` — Operator synchronization of repository documentation with accepted state.

Planner, Researcher, Validator, and Operator responsibilities are defined in `docs/operations/logical-agent-workflow.md`.

## Logical roles

### Manager
Owns agenda, decision-state orchestration, escalation, and acceptance flow. Manager does not create semantic truth merely because discussion converged.

### Planner
Owns objectives, constraints, sequencing, dependency structure, milestone/Phase decomposition, and acceptance criteria.

### Researcher
Owns MOOSE realization architecture, backend capability investigation, SOL-to-MOOSE mapping proposals, and identification of cross-team semantic gaps. Implementation convenience alone is not sufficient reason to alter SOL semantics.

### Validator
Independently challenges proposals and implementation evidence for compatibility, backend leakage, determinism, replay/side-effect safety, regression coverage, and physical/numerical evidence boundaries.

### Operator
Executes accepted repository work, CI/fix loops, issue/PR state transitions, merges when authorized by accepted scope and gates, main verification, and documentation synchronization. Operator must not invent new SOL semantics or compatibility commitments during execution.

## Authority hierarchy

When sources disagree, use this hierarchy:

1. published SOL Public Contract / Adapter Protocol meanings for the versions this adapter declares compatible with;
2. accepted adapter-repository ADRs and explicit compatibility decisions;
3. accepted plans, milestone/Phase acceptance criteria, and Validator verdicts;
4. current verified repository evidence: `main`, issues, PRs, exact-head CI, tests, and backend regression/V&V evidence;
5. repository operating conventions and guides;
6. README/examples;
7. chat history or model memory.

A lower-authority source must not silently override a higher-authority contract or accepted decision.

## SOL / MOOSE authority boundary

The SOL Platform Team defines canonical SOL meaning. This repository realizes that meaning in MOOSE.

```text
SOL Platform
  -> canonical ontology
  -> Public Contract
  -> Adapter Protocol
  -> MappingPlan semantics

sol-adaptor-moose
  -> MOOSE mapping
  -> input/backend artifact generation
  -> configured MOOSE application invocation
  -> solver-specific regression and V&V
```

If MOOSE implementation work exposes a missing or contradictory SOL semantic/contract requirement:

1. do not silently reinterpret the Public Contract or Adapter Protocol;
2. record the concrete MOOSE case, expected behavior, counterexample, and compatibility impact;
3. stop the affected decision path;
4. escalate the semantic question to the SOL Platform Team through a durable cross-repository handoff/issue;
5. continue only after the relevant SOL decision is accepted or the adapter-local workaround is explicitly validated as contract-preserving.

MOOSE is an important real-system feedback source, not the authority for canonical SOL semantics.

## Mandatory Adapter Protocol invariants

Unless a later explicit compatible/versioned SOL decision changes them, preserve these Protocol 0.1 rules:

- interoperability requires Adapter Protocol compatibility **and** Public Contract compatibility;
- adapter implementation version proves neither compatibility axis;
- `describe_adapter` reports implementation identity, supported contract ranges, targets, and capabilities;
- `validate_plan` is advisory and side-effect free;
- `execute_plan` is authoritative and must re-check the complete current request and relevant backend state before new side effects;
- `execute_plan` is non-idempotent by default;
- ambiguous/lost execute responses do not authorize automatic replay;
- valid negative preflight/execution states are distinct from `ProtocolFailure`;
- transport/process failures are distinct from logical Protocol failures;
- MappingPlan dependency edges must be preserved even if MOOSE execution reorders independent work;
- backend-native identifiers are opaque provenance/evidence only and cannot define canonical SOL semantic identity;
- Protocol conformance is interoperability evidence, not solver-native physical/numerical correctness.

Read `docs/knowledge/sol-adapter-baseline-0.1.md` before changing protocol-facing behavior.

## MOOSE-specific discipline

- Treat the configured MOOSE-based application and version as backend reality; do not assume every MOOSE application exposes identical registered objects or modules.
- Prefer public MOOSE Framework documentation for framework facts and verify application-specific behavior against the actual executable/test environment when available.
- MOOSE input blocks, `Kernel`, `BC`, `Material`, `Executioner`, `Mesh`, `AuxKernel`, `UserObject`, `Action`, and other backend constructs are realization concepts, not automatically SOL ontology classes.
- A generated `.i` input file is a backend artifact, not canonical SOL semantic truth.
- `--check-input` or successful MOOSE execution is useful backend evidence but does not replace SOL contract validation or adapter conformance.
- Keep MOOSE subprocess stdout/stderr and artifacts isolated from the adapter's JSON-RPC/stdin-stdout Protocol framing.

## Adapter-local evidence and gates

Apply Paul ER-05 through ER-09 for current evidence, evidence-class boundaries, real gates, interruption recovery, and documentation truth.

This repository adds the following adapter-specific requirements:

- keep framework documentation, configured executable/application evidence, adapter regression evidence, and physical/numerical V&V distinct;
- unresolved SOL Public Contract / Adapter Protocol meaning, compatibility support, mapping semantics, backend capability semantics, replay/side-effect behavior, physical/numerical validity, licensing, Validator rejection/revision, milestone exit, permission/merge conflict, or required non-green CI is a local real gate;
- completion requires the applicable current conformance, MOOSE regression, artifact/input, V&V, merge-on-main, and accepted Phase evidence.

## Public-repository hygiene

Do not commit or instruct future agents to depend on:

- private Google Drive documents;
- private chat transcripts;
- secrets, credentials, tokens, or private endpoints;
- personal/private filesystem paths;
- proprietary/licensed solver content without redistribution permission;
- language implying MOOSE/INL endorsement or official ownership of this adapter.

Everything required for a public contributor or agent to understand repository operation should be present in this repository or in publicly accessible normative upstream documentation.

## Current bootstrap source

The initial SOL Platform handoff was prepared from `simulation-ontology` main commit:

```text
69779d3ab7880f56618b29af82616965776e0126
```

This SHA is provenance for the bootstrap snapshot, not a permanent compatibility lock. Supported SOL contract versions must be declared explicitly by the adapter and updated through normal compatibility review.
