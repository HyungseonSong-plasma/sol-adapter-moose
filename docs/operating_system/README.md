# SOL MOOSE Adapter Operating System Binding

**Status:** Samuel consumer binding  
**Consumer:** `HyungseonSong-plasma/sol-adapter-moose`  
**Canonical OS repository:** `HyungseonSong-plasma/chatgpt-operation`  
**Pinned central revision:** `9b335e3fbb9c5e8e323e382daeec56fa25336822`  
**Expected OS:** Samuel

## Exact central authority

The authoritative version and artifact identities are in [`central_skills.json`](central_skills.json).
Use one immutable central revision for the entire decision cycle:

```text
repository          = HyungseonSong-plasma/chatgpt-operation
revision            = 9b335e3fbb9c5e8e323e382daeec56fa25336822
OS index            = docs/operating_system/README.md
essential rules     = docs/operating_system/ESSENTIAL_RULES.md
session bootstrap   = skills/session-bootstrap/README.md
state refresh       = skills/state-refresh/README.md
skill catalog       = skills/catalog.json
capability registry = skills/capability-registry.json
```

Read [`CENTRAL_SKILLS.md`](CENTRAL_SKILLS.md) for trigger routing. The pre-existing
[`docs/operations/chatgpt-operation-binding.json`](../operations/chatgpt-operation-binding.json)
is a compatibility binding, not an independent central authority. Its revision,
OS identity, and catalog/registry paths must agree with the canonical manifest;
CI fails closed on disagreement.

## Local authority retained

SOL Public Contract / Adapter Protocol semantics, supported MOOSE capabilities,
solver compatibility, physical/numerical V&V, mapping policy, adapter acceptance,
logical roles, and current work state remain adapter-owned. Samuel handles
generic execution mechanics and cannot assert SOL semantic or scientific PASS.

## Cross-chat initialization

`moose-init` routes to [`BOOTSTRAP.md`](../../BOOTSTRAP.md), then the
adapter-local [project-session-init](../operations/project-session-init.md).
It verifies the exact pinned OS, essential rules, init skills, catalog,
and capability registry, restores current GitHub work state, identifies the
first real gate, and stops read-only.

An initialized session reports:

```text
OS = Samuel
GITHUB_CAPABILITY_ROUTING = READY | BLOCKED
GITHUB_MUTATION_PREFLIGHT = TRIGGER_REQUIRED
```

Before any subsequent GitHub mutation, re-resolve the Samuel trigger, owning
skill, capability registry, provider order, current target identity and
permissions, then verify the postcondition. Missing support in one connector
does not establish global unavailability.

## Controller and compatibility discipline

Trigger-load controller, mutation, PR merge, workflow, or ownership-migration
mechanics only when applicable. Do not copy central implementations into this
repository. Do not auto-replay ambiguous non-idempotent SOL execute operations.
Protocol conformance, MOOSE integration, and numerical/scientific V&V remain
distinct evidence tracks.
