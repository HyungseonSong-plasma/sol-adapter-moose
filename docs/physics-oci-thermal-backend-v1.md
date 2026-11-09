# Physics OCI Thermal backend profile v1 (candidate)

**Status:** Draft implementation under Adapter #34 / PR #49; bounded actual Native verification, *not merged into production*.
**Source acceptance:** Adapter PR HEAD, pinned SOL Runtime v0.2, existing immutable Physics runtime image.
**No new SOL API or scientific capability.**

## Rationale and ownership

The historical Conda backend remains a distinct, immutable package target:
\`moose=2026.08.12=mpich\`. Its package identity and \`heat_transfer\`
copyable-input probe cannot be satisfied by a \`physics-opt\` executable.

This opt-in **independent backend execution profile** reuses the previously
validated Physics image without pretending it is a Conda package. Supported
external semantic capability stays \`thermal.steady_conduction\` under frozen
Adapter Protocol/Public Contract 0.2; the original SOL Runtime remains
responsible for discovery, adapter selection, lifecycle and no-replay. Physics
still owns scientific acceptance.

## Exact image contract

\`\`\`text
profile: physics_oci_thermal_v1
image: ghcr.io/hyungseonsong-plasma/physics-runtime@sha256:cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c
built-from: ghcr.io/hyungseonsong-plasma/physics-build-base@sha256:2c3352da4f5c1b2ae98475fc01463ad0cdbc6ce7d221ce55055ddea152182008
MOOSE source: 9f388366ccf38b9c34542ec5561198249fde0ac9
ENTRYPOINT: /opt/physics/physics-opt
OS/arch: linux/amd64
default backend: unchanged Conda moose-opt
\`\`\`

OCI readiness verifies the image is **already present locally**, its exact
repository digest, source owner, MOOSE/build-base OCI labels, platform,
content ID and exact ENTRYPOINT before any workspace creation. No implicit
remote pull, no mutable Docker tag, no inferred Conda prefix.

## Invocation (exactly scoped)

\`\`\`bash
# One-time controlled input acquisition, outside the adapter process.
docker pull ghcr.io/hyungseonsong-plasma/physics-runtime@sha256:cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c

# The external SOL Runtime caller launches this adapter binary.
# No SOL_MOOSE_EXECUTABLE or SOL_MOOSE_CONDA_PREFIX in this profile.
export SOL_MOOSE_BACKEND_PROFILE=physics_oci_thermal_v1
unset SOL_MOOSE_EXECUTABLE SOL_MOOSE_CONDA_PREFIX
export SOL_MOOSE_WORKSPACE_ROOT=/absolute/private/workspace/root

# Then run the existing frozen AP/PC0.2 consumer:
# sol-external-runtime-v02-consumer run /absolute/path/sol-adaptor-moose request.json
\`\`\`

The Adapter-generated MOOSE \`input.i\` remains a private backend artifact
inside an isolated fresh run directory. OCI subprocess invocation uses
\`docker run --rm --pull=never --network=none --cap-drop=ALL
--security-opt=no-new-privileges\`, a per-run bind mount at \`/work\`,
and the image's fixed entrypoint with direct argv (not a shell).
On timeout it identifies and kills the Docker container; uncertain termination
is an operational failure with no transport replay authority.

Execution evidence distinguishes \`moose.oci.image\` from
\`moose.package\` in the unchanged AP0.2 provenance envelope. The
\`moose.executable\` reference denotes \`/opt/physics/physics-opt\` for
this profile. Other references remain backend-local and noncanonical.

## Actual evidence / test contract

CI invokes both independently:

1. Direct Adapter \`validate_plan_v02\` then
   \`execute_plan_v02_with_run_key\`: real pinned OCI image, real MOOSE
   \`--check-input\` P2, real P3, literal \`Syntax OK\` in MOOSE
   stdout **or stderr**, \`Solve Converged!\`, input and output logs,
   no workspace allocated by readiness alone.
2. Pinned \`simulation-ontology@020e9a979a0b97f2af3cf54758cedf38fecb187a\`
   external Runtime → same Adapter PR HEAD → same OCI image:
   live discovery/selection, accepted validate, completed execute,
   no transport replay on response loss, same generated \`input.i\`
   **byte-for-byte** as direct path.
3. Negative mutation of \`input.i\` fails the parity checker; missing
   or incompatible image/profile and ambiguous mixed Conda/OCI settings fail
   closed.

**Verified on CI #42609784700, PR candidate SHA
\`febf44e92a4ac05b25a6eaec18825803842c60d3\`:** scope/Rust/Clippy
PASS; Physics OCI native direct/Runtime paths PASS, same 665-byte
input SHA256
\`09e6690f11dc240792b1b922f1f5aba57aa38dea6bad10f4676c06b58dd0b3cd\`.
Actual evidence artifact #13670216537. Native Conda backend remained
**FAIL** due to INL \`noarch/repodata.json\` HTTP 502; the overall
workflow is not green.

## Non-goals and future gates

- Does **not** claim equivalence between the pinned Physics OCI image and
  INL \`moose=2026.08.12=mpich\`; the source revisions, application binary,
  MPI implementation and supported copyable inputs differ.
- Does **not** publish additional plasma capabilities, a new SOL result DTO,
  a generic HIT text editing API, or an alternate runtime registry.
- Does **not** replace scientific/field-value V&V, 2D plasma acceptance,
  or Physics #281 consumer cutover. These have independent authorities.
- Does **not** bypass the previous required Conda CI gate or mark its
  failure PASS. Transition of the required CI backend profile is a separately
  reviewed **CI/contract-policy** decision after numerical and compatibility
  evidence is accepted.
- Does **not** change the official packaged Adapter release or production
  consumers while this implementation remains Draft.

Cross-references: Adapter #34, Draft PR #49, Draft #47, SOL #172,
Physics #281, and closed read-only research PR #48.
