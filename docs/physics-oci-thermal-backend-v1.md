# Physics OCI Thermal backend profile v1 (candidate)

**Status:** Thermal AP/PC0.2 OCI backend implementation via Adapter PR #49; deployment is restricted to the verified Thermal slice, not an authorization for Physics scientific acceptance or SOL caller migration.
**Source acceptance:** Adapter PR HEAD, pinned SOL Runtime v0.2, existing immutable Physics runtime image.
**No new SOL API or scientific capability.** **Current decision (2026-11-09):**
the pinned Physics OCI image is the **default** for the supported Thermal 0.2 slice
in this Draft. The exact Conda release compatibility test is **deferred,
manual-only**, not passed or deleted. As long as PR #49 is unmerged,
`main` is unchanged; publication/merge requires separate approval.

## Rationale and ownership

The historical Conda backend remains a distinct, immutable package target:
`moose=2026.08.12=mpich`. Its package identity and `heat_transfer`
copyable-input probe cannot be satisfied by a `physics-opt` executable.

The **default Thermal backend execution profile** reuses the previously
validated Physics image without pretending it is a Conda package. Supported
external semantic capability stays `thermal.steady_conduction` under frozen
Adapter Protocol/Public Contract 0.2; the original SOL Runtime remains
responsible for discovery, adapter selection, lifecycle and no-replay. Physics
still owns scientific acceptance.

## Exact image contract

```text
profile: physics_oci_thermal_v1
image: ghcr.io/hyungseonsong-plasma/physics-runtime@sha256:cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c
built-from: ghcr.io/hyungseonsong-plasma/physics-build-base@sha256:2c3352da4f5c1b2ae98475fc01463ad0cdbc6ce7d221ce55055ddea152182008
MOOSE source: 9f388366ccf38b9c34542ec5561198249fde0ac9
ENTRYPOINT: /opt/physics/physics-opt
OS/arch: linux/amd64
default backend: physics_oci_thermal_v1 (Draft PR #49 only)
explicit Conda: SOL_MOOSE_BACKEND_PROFILE=conda with exact moose-opt/prefix
Conda compatibility CI: workflow_dispatch manual opt-in, not PR/push required
```

OCI readiness verifies the image is **already present locally**, its exact
repository digest, source owner, MOOSE/build-base OCI labels, platform,
content ID and exact ENTRYPOINT before any workspace creation. No implicit
remote pull, no mutable Docker tag, no inferred Conda prefix.

## Invocation (exactly scoped)

```bash
# One-time controlled input acquisition, outside the adapter process.
docker pull ghcr.io/hyungseonsong-plasma/physics-runtime@sha256:cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c

# Physics OCI is selected automatically when no profile is configured.
# No implicit fallback to Conda; image absence/mismatch is a hard error.
unset SOL_MOOSE_BACKEND_PROFILE SOL_MOOSE_EXECUTABLE SOL_MOOSE_CONDA_PREFIX
export SOL_MOOSE_WORKSPACE_ROOT=/absolute/private/workspace/root

# Then invoke the existing frozen AP/PC0.2 SOL consumer:
# sol-external-runtime-v02-consumer run /absolute/path/sol-adaptor-moose request.json

# If *explicitly* validating historical Conda compatibility instead:
# export SOL_MOOSE_BACKEND_PROFILE=conda
# export SOL_MOOSE_EXECUTABLE=/verified/prefix/bin/moose-opt
# export SOL_MOOSE_CONDA_PREFIX=/verified/prefix
```

The Adapter-generated MOOSE `input.i` remains a private backend artifact
inside an isolated fresh run directory. OCI subprocess invocation uses
`docker run --rm --pull=never --network=none --cap-drop=ALL
--security-opt=no-new-privileges`, a per-run bind mount at `/work`,
and the image's fixed entrypoint with direct argv (not a shell).
On timeout it identifies and kills the Docker container; uncertain termination
is an operational failure with no transport replay authority.

Execution evidence distinguishes `moose.oci.image` from
`moose.package` in the unchanged AP0.2 provenance envelope. The
`moose.executable` reference denotes `/opt/physics/physics-opt` for
this profile. Other references remain backend-local and noncanonical.

## Actual evidence / test contract

Routine PR/push CI requires **Physics OCI** execution and source V&V.
The original exact-Conda compatibility job remains available only via manual
`workflow_dispatch` with `run_conda_compatibility=true`. It is **SKIPPED**
(not PASS) in ordinary PR/push checks. The OCI CI proves:

1. Direct Adapter `validate_plan_v02` then
   `execute_plan_v02_with_run_key`: real pinned OCI image, real MOOSE
   `--check-input` P2, real P3, literal `Syntax OK` in MOOSE
   stdout **or stderr**, `Solve Converged!`, input and output logs,
   no workspace allocated by readiness alone.
2. Pinned `simulation-ontology@020e9a979a0b97f2af3cf54758cedf38fecb187a`
   external Runtime → same Adapter PR HEAD → same OCI image:
   live discovery/selection, accepted validate, completed execute,
   no transport replay on response loss, same generated `input.i`
   **byte-for-byte** as direct path.
3. Negative mutation of `input.i` fails the parity checker; missing
   or incompatible image/profile and ambiguous mixed Conda/OCI settings fail
   closed.

**Verified on CI #42609784700, PR candidate SHA
`febf44e92a4ac05b25a6eaec18825803842c60d3`:** scope/Rust/Clippy
PASS; Physics OCI native direct/Runtime paths PASS, same 665-byte
input SHA256
`09e6690f11dc240792b1b922f1f5aba57aa38dea6bad10f4676c06b58dd0b3cd`.
Actual evidence artifact #13670216537. Native Conda backend remained
**FAIL** due to INL `noarch/repodata.json` HTTP 502; the overall
workflow is not green.

## Non-goals and future gates

- Does **not** claim equivalence between the pinned Physics OCI image and
  INL `moose=2026.08.12=mpich`; the source revisions, application binary,
  MPI implementation and supported copyable inputs differ.
- Does **not** publish additional plasma capabilities, a new SOL result DTO,
  a generic HIT text editing API, or an alternate runtime registry.
- Does **not** replace scientific/field-value V&V, 2D plasma acceptance,
  or Physics #281 consumer cutover. These have independent authorities.
- The user explicitly **deferred Conda as a mandatory CI check** and approved
  OCI as the Thermal default on 2026-11-09. Exact Conda verification remains
  an optional, manually dispatched compatibility check, with **no PASS
  assertion** for its unresolved INL HTTP 502 and no equivalence claim.
  Do not silently remove or auto-fallback to the Conda path.
- Does **not** change the official packaged Adapter release or production
  consumers while this implementation remains Draft.

Cross-references: Adapter #34, Draft PR #49, Draft #47, SOL #172,
Physics #281, and closed read-only research PR #48.

## Historical review-only handoff (earlier 2026-11-09 decision; default/Conda policy superseded above)

- **A1**: Code review requested and recorded on Draft PR #49; this
  document and CI evidence do not constitute an independent third-party approval.
- **B1 (historical)**: OCI execution/source V&V and exact-Conda release
  compatibility were separate mandatory CI results. **Superseded by the
  explicit current user decision:** OCI is the routine default required
  check; Conda is deferred/manual-only and cannot be labeled PASS.
  No branch ruleset/protection is changed.
- **C2**: The existing bounded 400 K CSV/Exodus source V&V evidence is
  retained. Independent scientific acceptance is **deferred**, not
  granted or requested in this review cycle.
- **D1**: SOL #172 receives an Option A (frozen semantic thermal 0.2)
  contract review request; it must remain
  **DECISION_REQUIRED / NO_CONSUMER_CUTOVER** until explicitly accepted.
- **E1**: **Merged PR #38**, main merge commit
  `072951fe49c1631f0c81ffe83f1736a63342af76`, is the canonical
  owner of `scripts/verify_thermal_same_run_evidence.py`. The OCI CI
  now reads the **repository-local** integrated verifier and checks
  its exact Git blob `35029268d543e37302a52a582d3e19c6a6c7782d`;
  no cross-Draft runtime fetch is used.
  **Phase 5 same-file conflict is resolved:** original
  `tests/phase5_thermal_vv.rs` comes from merged PR #38 for historical
  Conda evidence, while `tests/physics_oci_phase5_vv.rs` covers
  OCI-only CSV+Exodus source V&V.

Local source-build backend evaluation, merging, release, Physics consumer
migration, scientific signoff and SOL contract acceptance remain outside
the approved scope. The **Draft-only OCI default switch is now explicitly
approved**, without any `main` merge or production consumer cutover.

## Container lifetime hardening (Draft only, 2026-11-09)

The OCI process runner creates an atomic, OS-random 128-bit name and a
mode-0700 private temporary CID directory before Docker launch.
It passes both `--name` and `--cidfile` and never uses a shared
predictable PID/counter CID filename. After the Docker CLI exits or
is killed at timeout, it checks `docker container ls --all --no-trunc`
for that exact name; recorded CID and listed ID must agree. If any
orphan remains, it is forcibly removed by **the private random name**
and the invocation is still **reported as failure**, including when
the CID file was not written. A failed cleanup or ambiguous CID
leaves execution status UNKNOWN, never PASS or a replay permission.
Cleanup commands are bounded by a 10-second timeout; CID symlinks and
invalid contents are rejected before use.

The separate `tests/physics_oci_lifecycle.rs` uses a fake Docker CLI,
requiring successful normal cleanup, timeout orphan removal, orphan
with missing CID, failed forced removal and forged/symlink CID rejection.
The native OCI/SOL P2/P3 and Phase 5 tests still run on the published
digest under GitHub Actions. Tests in this Draft **do not** certify
scientific or Conda release equivalence.

**Integration order:** PR #38's shared source verifier was independently
merged into main at `072951fe49c1631f0c81ffe83f1736a63342af76`. PR #49's
cross-Draft test fetch has been replaced by this integrated local source;
the Conda and OCI Phase 5 tests are in separate files. The user has
explicitly approved PR #49 integration with OCI as the default Thermal
backend and Conda as deferred/manual-only. This does not authorize
scientific acceptance, SOL #172 formal contract acceptance, Local Build,
new plasma capabilities, or Physics consumer cutover.
