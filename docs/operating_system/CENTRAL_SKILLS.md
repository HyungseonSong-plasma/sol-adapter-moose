# Samuel Central Operating Contracts — SOL MOOSE Adapter

**Status:** canonical consumer manifest guide  
**OS:** Samuel  
**Central revision:** `9b335e3fbb9c5e8e323e382daeec56fa25336822`

`central_skills.json` is the machine-readable source of exact revision,
paths, blob SHAs, and adopted skill triggers. All central artifacts must match
that one revision. No floating `main` or mixed-source skills are permitted.

## Init load — `moose-init`

Verify Git blob SHA and read these artifacts from the exact central revision:

```text
docs/operating_system/README.md
docs/operating_system/ESSENTIAL_RULES.md
skills/session-bootstrap/README.md
skills/state-refresh/README.md
skills/catalog.json
skills/capability-registry.json
```

Index the catalog and registry without granting write authority.
`GITHUB_CAPABILITY_ROUTING=READY` means both were read and verified,
not that a provider is accessible. When either cannot be verified, report
`BLOCKED`. Always report
`GITHUB_MUTATION_PREFLIGHT=TRIGGER_REQUIRED`.

## Trigger-loaded central paths

```text
MUTATE / GITHUB_BRANCH_DELETE / GITHUB_MUTATION_CAPABILITY_VERDICT
  -> repository-mutation
GOVERNED_WORK -> governed-work
SCHEDULED_CONTROLLER -> state-refresh + controller-throughput + controller-lifecycle
CHARACTERIZED_OWNERSHIP_MIGRATION -> characterized-ownership-migration
GOVERNED_MATRIX -> governed-matrix
GITHUB_ACTIONS_EXECUTION -> github-actions-execution
GITHUB_ACTIONS_OBSERVATION -> github-actions-observation
GITHUB_PR_MERGE -> pull-request-merge
RESEARCH_CONTROLLER -> research-controller
ARTIFACT_STAGING -> artifact-staging
SCIENTIFIC_DISCRIMINATOR_CONTROLLER -> scientific-discriminator-controller
```

For GitHub state changes, resolve owning trigger and central skill before
selecting the ordered providers in the pinned capability registry. Verify
fresh target identity, permissions, and the mutation read-back. For an
unavailable-capability claim, exhaust all registered providers and satisfy
the prescribed machine guard; an absent connector method is not sufficient.

## Adapter ownership boundary

This manifest does not transfer SOL semantics, backend realization policy,
solver version support, Physics scientific claims, or acceptance authority to
Samuel. It also does not deploy central controller workflows into this repo;
native execution providers must be evaluated against their actual target
repository prerequisites when triggered.

## Evidence and guards

The Rust CI `scope` job checks that the compatibility binding matches the
manifest, that the OS identifies Samuel, and that every declared central blob
SHA matches the fetched immutable Git object. Runtime/conformance claims
remain governed by separate technical evidence.
