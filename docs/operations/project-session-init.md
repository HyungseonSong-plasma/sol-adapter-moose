# SOL MOOSE Adapter Project Session Initialization

**Status:** bootstrap operating convention  
**Date:** 2026-08-22  
**Scope:** deterministic read-only bootstrap for the `sol-adaptor-moose` repository

## Purpose

Conversation history and model memory are useful hints but are not authoritative project state. A new or uncertain MOOSE-team chat must first load the reusable deterministic operating mechanics from the central `chatgpt-operation` repository, then reconstruct repository identity, accepted decisions, compatibility baseline, current milestone/Phase, PR/CI state, and the next real gate from durable repository evidence.

`moose-init` performs that skill bootstrap and project-state reconstruction without mutating project state.

It is a project-session bootstrap, not a runtime adapter operation and not a substitute for `meeting`, `resume`, or `update`.

## Trigger

Run this bootstrap when:

- the user sends `moose-init`;
- a new MOOSE Adapter Team chat begins;
- context was compacted, lost, or may be stale;
- the repository or active work is uncertain;
- the user asks to restore or verify MOOSE adapter project state.

Re-running `moose-init` is safe because it is read-only.

## Canonical entry sources

Read in this order:

1. resolve `HyungseonSong-plasma/chatgpt-operation` `main` to an exact commit SHA;
2. from that exact SHA, read central `README.md` and `skills/state-refresh/README.md`;
3. root `AGENTS.md`;
4. `docs/operations/project-session-init.md`;
5. `docs/operations/logical-agent-workflow.md`;
6. current GitHub repository evidence;
7. `docs/knowledge/sol-adapter-baseline-0.1.md` when protocol/contract behavior is relevant;
8. `docs/knowledge/moose-foundation.md` when backend behavior is relevant;
9. only the ADRs, plans, issues, PRs, tests, and validation records governing the current gate.

Do not load the entire repository or every central skill by default. Use progressive disclosure after the current gate and requested mode are identified. All central-skill reads used in one decision cycle must remain pinned to the same resolved central commit.

## Bootstrap procedure

### 0. Load central operating skills

Before interpreting project state:

- resolve `HyungseonSong-plasma/chatgpt-operation` `main` to its current exact commit SHA;
- treat that exact SHA as the central operating-skill revision for this session/decision cycle;
- load central `README.md` and `skills/state-refresh/README.md` from that exact SHA;
- use `state-refresh` to determine whether the project requires a full bootstrap, a delta refresh, or only authoritative prewrite reads later;
- record the resolved central SHA and the loaded skill set in the initialization report.

Mode-specific loading is progressive:

- before any repository file/branch write, load `skills/repository-mutation/README.md` from the same central SHA;
- before executing checked-in governed work, load `skills/governed-work/README.md`;
- before creating/resuming/evaluating a scheduled controller, load both `skills/controller-lifecycle/README.md` and `skills/controller-throughput/README.md`.

Central skills define reusable mechanics only. Repository-specific policy and scientific/semantic authority remain local. If a required skill cannot be loaded at the pinned revision, fail safe for the affected operation rather than applying remembered mechanics.

### 1. Resolve repository identity

Confirm:

- repository: `HyungseonSong-plasma/sol-adaptor-moose`;
- default branch;
- latest default-branch commit SHA;
- repository visibility when relevant;
- whether the available GitHub connection can read the repository.

Do not infer current state from an old local checkout or previous chat when fresher GitHub evidence exists.

### 2. Reload the logical operating model

Re-establish:

- `Manager` and `meeting`;
- `Planner`;
- `Researcher`;
- `Validator`;
- `Operator` and `resume` / `update`;
- the SOL/MOOSE authority boundary;
- real-gate and verification rules;
- cross-team escalation behavior.

`moose-init` itself is project-level and read-only. It is not a sixth logical agent.

### 3. Inspect current execution state

Use current GitHub evidence to identify:

- current or most recently completed adapter milestone/roadmap item;
- normative parent tracker when one exists;
- open eligible Phase/blocker issues and dependency order;
- open PRs, exact head SHA, CI/check state, mergeability, unresolved review threads, and permissions;
- latest verified `main` state;
- unresolved documentation or milestone handoff requirements.

If the repository is still in bootstrap and no milestone has been accepted, report that fact rather than inventing one.

### 4. Restore compatibility state

Identify the currently declared support for:

- SOL Public Contract version/range;
- SOL Adapter Protocol version/range;
- MOOSE Framework/application version/range when already decided;
- target/capability declarations when already implemented.

If compatibility support has not yet been accepted, classify it as a `meeting` gate.

### 5. Read minimal governing material

After the current gate is known, read only the applicable:

- accepted adapter ADRs;
- SOL contract baseline/reference material;
- current milestone/Phase acceptance criteria;
- implementation/validation records;
- MOOSE official documentation needed to validate backend facts.

Guides and README text do not override published SOL contract meanings, accepted adapter ADRs, Validator verdicts, or verified current evidence.

### 6. Classify the next mode

Recommend exactly one next mode when evidence permits:

- `meeting` — unresolved architecture, SOL compatibility, MOOSE support range, mapping semantics, roadmap, V&V criterion, or other material decision exists;
- `resume` — accepted executable work is eligible and no real gate blocks it;
- `update` — accepted implementation state needs documentation synchronization and predecessor acceptance is complete.

If evidence is insufficient, report what is missing instead of guessing.

### 7. Stop after the initialization report

`moose-init` ends after reporting restored context. Do not automatically invoke `meeting`, `resume`, or `update`.

## Required output

Keep the report concise but include:

1. **Central skill snapshot** — resolved `chatgpt-operation` exact SHA and skills loaded for this mode.
2. **Repository snapshot** — repo, default branch, latest verified SHA.
3. **Role snapshot** — logical roles and available modes.
4. **Execution snapshot** — current milestone/parent/Phase/PR/CI state, or bootstrap state if none exists.
5. **Compatibility snapshot** — declared SOL contract/protocol and MOOSE target support, or unresolved status.
6. **Real gate** — first unresolved decision/evidence/permission/CI/review/dependency gate.
7. **Recommended next mode** — `meeting`, `resume`, or `update` with one-line reason.
8. **Evidence uncertainty** — anything that could not be verified, including central-skill load failures.

## Read-only boundary

During `moose-init`, do not:

- create/update/close/comment on issues;
- create/update/review/merge/close PRs;
- create/update branches, commits, tags, releases, or milestones;
- edit repository files;
- rerun CI;
- change compatibility declarations;
- make a new mapping/architecture/semantic decision;
- silently continue into another work mode.

Discovery, inspection, and reporting are allowed.

## Counterexamples

Invalid flows include:

```text
new chat + remembered state -> resume without GitHub inspection
README says A0.1 -> assume A0.1 is accepted/current
MOOSE input validates -> assume SOL adapter conformance
MOOSE object name -> promote directly into SOL ontology
moose-init -> create issues or code
moose-init -> silently choose implementation language or MOOSE support version
```

## Suggested persistent ChatGPT Project hook

If this repository receives its own ChatGPT Project instructions, keep the persistent hook small and stable:

```text
For the SOL MOOSE Adapter project, when the user sends "moose-init", first resolve
HyungseonSong-plasma/chatgpt-operation main to an exact SHA and load the central
README plus state-refresh skill from that exact revision. Then use GitHub to open
HyungseonSong-plasma/sol-adaptor-moose and follow
docs/operations/project-session-init.md as a read-only bootstrap.
Do not rely on prior chat memory and do not mutate project state until the user
selects or confirms meeting, resume, or update. Load additional central skills
from the same pinned revision before their corresponding operation.
```

Detailed workflow rules belong in this repository rather than duplicated in Project instructions.
