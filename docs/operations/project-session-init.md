# SOL MOOSE Adapter Project Session Initialization

**Status:** consumer-local Paul bootstrap configuration  
**Command:** `moose-init`  
**Scope:** adapter-specific additions to the central Paul `session-bootstrap` skill

## Central authority

Use the exact binding in:

```text
docs/operations/chatgpt-operation-binding.json
```

A valid initialization requires:

```text
OS = Paul
Paul essential rules loaded
session-bootstrap loaded
state-refresh loaded
```

The generic bootstrap algorithm, exact-pin rule, fail-closed behavior, read-only boundary, current-evidence rules, and interruption recovery are owned by the pinned Paul OS and central skills.

Do not reimplement those algorithms locally.

## Adapter-local authority sources

After the central bootstrap layer is established, restore only the minimum local material needed for the current gate:

1. root `AGENTS.md`;
2. this file;
3. `docs/operations/logical-agent-workflow.md`;
4. current GitHub repository evidence;
5. `docs/knowledge/sol-adapter-baseline-0.1.md` when protocol/contract behavior is material;
6. `docs/knowledge/moose-foundation.md` when backend facts are material;
7. only the ADRs, plans, tests, issues, PRs, and validation records governing the current gate.

## Consumer inputs to session-bootstrap

```text
consumer repository:
  HyungseonSong-plasma/sol-adapter-moose

local semantic authority:
  supported SOL Public Contract / Adapter Protocol
  accepted adapter ADRs
  compatibility decisions
  configured MOOSE application/backend evidence

durable work state:
  current milestone / parent tracker / phase issue / PR / main evidence

local modes:
  meeting
  resume
  update
```

## Compatibility restoration

Identify the currently accepted support for:

- SOL Public Contract version/range;
- SOL Adapter Protocol version/range;
- configured MOOSE Framework/application version/range when decided;
- implemented targets/capabilities.

Unresolved compatibility support is a `meeting` gate.

## Current execution state

Inspect enough current GitHub evidence to identify:

- latest verified `main` SHA;
- current or most recently completed adapter milestone/roadmap item;
- parent tracker when one exists;
- eligible Phase/blocker issue and dependency order;
- open PR, exact-head CI/check state, mergeability, review/permission state;
- unresolved documentation or milestone handoff.

## Adapter-local trigger skill

When accepted work adds, replaces, migrates, or retires a custom MOOSE object/helper, load:

```text
skills/standard-moose-first-refactor/README.md
```

This local skill owns the MOOSE capability census and standard-object/composition preference. If the work also transfers canonical ownership, compose it with the pinned central `characterized-ownership-migration` skill when that skill is available in the active central revision.

Scientific semantics and compatibility commitments remain local authority and must be resolved before the refactor skill executes.

## Next-mode selection

Recommend exactly one when evidence permits:

- `meeting` — unresolved architecture, SOL compatibility, MOOSE support range, mapping semantics, roadmap, V&V criterion, or other material decision;
- `resume` — accepted executable work is eligible and no real gate blocks it;
- `update` — accepted implementation state needs documentation synchronization.

The recommendation does not execute the mode.

## Initialization report

Report:

1. Paul central revision and loaded init skills;
2. repository/ref;
3. current execution state;
4. compatibility state;
5. first real gate;
6. recommended next mode;
7. unresolved evidence uncertainty.

No operating-metrics context is required.

## Persistent Project hook

A minimal stable hook is sufficient:

```text
For the SOL MOOSE Adapter project, when the user sends "moose-init", use GitHub to
open HyungseonSong-plasma/sol-adapter-moose, read the exact Paul binding in
docs/operations/chatgpt-operation-binding.json, follow the pinned central
session-bootstrap skill, then apply docs/operations/project-session-init.md for
adapter-specific additions. Stop read-only after the initialization report.
```

Generic operating rules belong in `chatgpt-operation`; adapter semantics and scientific/compatibility policy remain here.
