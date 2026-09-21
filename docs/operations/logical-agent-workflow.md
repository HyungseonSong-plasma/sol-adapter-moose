# SOL MOOSE Adapter Logical Agent Workflow

**Status:** bootstrap operating convention  
**Date:** 2026-08-22  
**Scope:** decision, planning, validation, implementation, and documentation workflow for `sol-adaptor-moose`

## Purpose

This repository uses logical roles to preserve distinct reasoning objectives and authority boundaries. These roles are project-operating concepts, not runtime SOL or MOOSE concepts.

The local roles are:

- `Manager`
- `Planner`
- `Researcher`
- `Validator`
- `Operator`

The primary local modes are:

- `moose-init` — read-only session bootstrap;
- `meeting` — structured decision flow;
- `resume` — execute accepted work until a real gate;
- `update` — synchronize docs with accepted state.

## Central Paul operating layer

`moose-init` and later operating actions use the exact central revision declared in `docs/operations/chatgpt-operation-binding.json`.

```text
moose-init
  -> Paul essential rules
  -> session-bootstrap + state-refresh
  -> adapter-local authority/current state
  -> first real gate
  -> meeting | resume | update recommendation
  -> stop read-only
```

Later deterministic mechanics are trigger-loaded from the same central pin:

- repository writes -> `repository-mutation`;
- governed checked-in manifests -> `governed-work`;
- scheduled-controller work -> `controller-throughput` + `controller-lifecycle`.

The central layer does not decide SOL meaning, MOOSE realization semantics, compatibility support, scientific validity, or adapter acceptance.

## 1. Manager

Manager owns decision orchestration and state transitions, not technical truth by fiat.

### `meeting`

Use `meeting` for material choices such as:

- adapter architecture;
- implementation language/runtime packaging;
- supported MOOSE application/version strategy;
- SOL Public Contract / Adapter Protocol compatibility commitment;
- mapping semantics;
- backend capability model;
- retry/replay/side-effect behavior;
- milestone and Phase decomposition;
- V&V acceptance criteria;
- cross-team contradictions with the SOL Platform.

Manager should:

1. state the decision question;
2. ask Planner for objectives, constraints, alternatives, sequencing, and acceptance criteria;
3. ask Researcher for architecture/backend investigation and cross-team semantic implications;
4. ask Validator to challenge the proposal independently;
5. return findings for revision rather than treating Validator suggestions as automatic truth;
6. escalate unresolved compatibility/semantic questions to the user or SOL Platform Team as appropriate;
7. make acceptance state explicit;
8. hand accepted work to Operator.

Useful decision states:

```text
PROPOSED
  -> CHALLENGED
  -> REVISED
  -> VALIDATED
  -> USER_APPROVED when required
  -> ACCEPTED
  -> HANDOFF_TO_OPERATOR
```

Other outcomes include `REJECTED` and `DEFERRED`.

## 2. Planner

Planner owns work structure, not implementation truth.

Responsibilities:

- define objectives and constraints;
- compare alternatives and trade-offs;
- identify dependencies and sequencing;
- shape milestone/Phase structure and acceptance gates;
- separate SOL contract decisions from adapter-local implementation choices;
- identify required MOOSE environment, fixture, CI, and V&V prerequisites;
- distinguish current capability from deferred scope.

Primary outputs:

- plans;
- milestone/Phase decomposition;
- option comparisons;
- dependency graph;
- acceptance criteria.

## 3. Researcher

Researcher owns MOOSE realization and architecture investigation.

Responsibilities:

- investigate how canonical SOL concepts can be realized in MOOSE without backend leakage;
- validate MOOSE framework facts against public official documentation and configured executable evidence;
- define translation-layer boundaries;
- identify candidate mappings from SOL actions/effects to MOOSE artifacts/objects;
- distinguish framework-level facts from application-specific registered syntax;
- define provenance boundaries;
- identify when a MOOSE need exposes a real SOL Public Contract / Adapter Protocol gap;
- prepare adapter ADR proposals when local architecture is fixed or changed.

Researcher must not treat MOOSE object structure as canonical SOL ontology structure merely because the implementation is convenient.

## 4. Validator

Validator challenges proposals and implementation evidence independently.

Evaluation dimensions include:

- SOL contract compatibility;
- backend leakage;
- target/capability correctness;
- dependency preservation;
- deterministic contract behavior;
- transport/process separation;
- side-effect and replay safety;
- MOOSE-version/application compatibility;
- solver regression coverage;
- distinction between conformance and physical/numerical correctness;
- counterexample coverage.

Validator verdicts should be explicit:

```text
APPROVE
APPROVE WITH GATES
REJECT / REVISE
NOT ESTABLISHED  # evidence is insufficient, not equivalent to rejection
```

Validator findings inform revision and acceptance; they do not automatically create new SOL meaning.

## 5. Operator

Operator executes accepted work against current repository state.

Responsibilities:

- inspect current issue/Phase/PR/CI/main evidence before acting;
- implement accepted work in dependency order;
- add fixtures, tests, translation logic, process integration, docs, and CI as required;
- run SOL conformance evidence separately from MOOSE regression/V&V evidence;
- apply the smallest root-cause fix after failures;
- record current evidence before closing work;
- verify merged state on `main`;
- stop and escalate newly discovered architecture/compatibility/semantic questions rather than deciding them silently.

### `resume`

`resume` means continue accepted executable work until a real gate.

Expected loop:

```text
inspect current state
 -> identify eligible accepted task
 -> implement
 -> tests / conformance / MOOSE regression as applicable
 -> exact/current evidence
 -> fix loop if needed
 -> review/merge when authorized
 -> verify main
 -> update durable checkpoint
 -> next eligible task
 -> stop at real gate
```

A real gate includes:

- unresolved SOL Public Contract / Adapter Protocol meaning;
- unresolved MOOSE support-range or capability commitment;
- new architecture choice with material alternatives;
- Validator `REJECT/REVISE`;
- milestone exit audit;
- missing required MOOSE environment/license/access;
- non-green required CI;
- merge conflict, permission, branch-protection, or review block;
- explicit user stop.

### Merge discipline

Operator may merge accepted implementation work without repeated user confirmation only when all of the following are true:

- scope and acceptance criteria were already accepted;
- no new unresolved SOL semantic/contract/compatibility decision is introduced;
- required protocol/conformance and backend-specific tests for that scope are complete;
- current exact-head CI is green where CI exists;
- mergeability/review/permission gates are clear;
- the merge is not itself a milestone exit/closure decision requiring Validator or Manager/user review.

After merge, verify `main` before closing dependent work.

## 6. `update`

`update` synchronizes user/developer-facing docs with accepted implementation state.

Typical targets:

- README;
- adapter authoring/integration guides;
- compatibility matrix;
- architecture docs;
- MOOSE mapping documentation;
- examples;
- build/run instructions;
- test/conformance/V&V guidance;
- roadmap/status;
- operations docs when an accepted workflow change occurs.

Rules:

- documentation reflects accepted state; it does not invent it;
- link to public normative upstream SOL/MOOSE material where appropriate, but keep repository-critical operating rules locally available;
- distinguish implemented, experimental, and planned behavior;
- if docs expose an unresolved semantic/compatibility contradiction, stop that path and return to `meeting`.

## 7. SOL Platform cross-team handoff

This repository is a consumer and real-backend feedback source for the SOL Platform.

Use cross-team escalation when MOOSE evidence suggests the current SOL contract cannot express required solver-independent meaning.

The MOOSE team should provide a durable decision-ready artifact containing:

- concrete SOL input/MappingPlan or minimal semantic case;
- required MOOSE realization behavior;
- why the current Public Contract/Adapter Protocol cannot represent it without reinterpretation;
- positive example;
- counterexample;
- compatibility impact;
- whether an adapter-local contract-preserving workaround exists;
- urgency/blocking status.

The SOL Platform Team decides canonical semantic/contract evolution. This repository must then consume the accepted versioned result.

## 8. Evidence model

Keep these evidence classes separate:

```text
SOL protocol/schema conformance
MOOSE application integration regression
backend artifact/input validation
physical/numerical V&V
performance/scalability evidence
```

No one class implies the others.

Examples:

- Protocol-conformant but physically wrong mapping -> adapter bug despite interoperability.
- MOOSE solve succeeds but Protocol response violates aggregate-effect semantics -> SOL non-conformance.
- `--check-input` succeeds but generated physics is semantically wrong -> backend syntax valid, realization invalid.
- lost `execute_plan` response -> possible side effect remains; do not auto-replay.

## 9. Durable checkpoint and interruption

Long-running work must leave durable checkpoints in GitHub artifacts.

A useful checkpoint records:

- current state;
- completed artifact/work;
- current evidence/verdict;
- unresolved finding;
- next state;
- input references needed to resume.

Operational interruption is not a technical verdict:

```text
ACTIVE -> RESOURCE_INTERRUPTED -> READ SOURCE OF TRUTH -> RESTORE -> NEXT STATE
```

Tool timeout, connector failure, rate limit, or context exhaustion must not be converted into `REJECT` or a backend failure claim.

## 10. Normal flow

For a new or uncertain session:

```text
moose-init
 -> exact Paul binding
 -> essential rules + session-bootstrap/state-refresh
 -> restore adapter-local state
 -> meeting | resume | update
```

For decision-heavy work:

```text
Manager meeting
 -> Planner
 -> Researcher
 -> Validator
 -> revision/escalation as needed
 -> ACCEPTED
 -> Operator resume
```

For documentation after accepted implementation:

```text
accepted state
 -> Operator update
 -> consistency verification
```

Generic current-evidence, real-gate, interruption, and documentation-truth rules come from Paul; this document defines adapter-local role and semantic behavior.
