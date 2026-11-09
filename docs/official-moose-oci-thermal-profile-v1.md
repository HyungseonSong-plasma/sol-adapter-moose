# Official MOOSE OCI Thermal backend v1 — candidate profile

**Status:** Draft implementation stacked on [Adapter PR #49](https://github.com/HyungseonSong-plasma/sol-adapter-moose/pull/49), building on independent [official Docker evidence PR #50](https://github.com/HyungseonSong-plasma/sol-adapter-moose/pull/50). **Not a production-approved backend, accepted SOL contract change, Conda substitution, or scientific sign-off.**

## Exact backend identities

The Adapter has three distinct profiles. They may not be conflated:

| Explicit profile | Immutable backend identity | Runtime executable | Status |
| --- | --- | --- | --- |
| default / `conda` | `moose=2026.08.12=mpich`, archive SHA256 `d7b2eec6b958f11455c903b70a16ea319b7edb68b4499537313c06785abd05b3` | local Conda `moose-opt` | Existing accepted Phase 1 target; INL provisioning blocked |
| `physics_oci_thermal_v1` | Physics GHCR `physics-runtime@sha256:cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c` | `/opt/physics/physics-opt` | Separate PR #49 candidate |
| `official_moose_oci_thermal_v1` | `idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452` | `/opt/moose/bin/moose-opt` | This candidate |

**Authoritative image observation** from [PR #50 real OCI Run #42610103906, artifact #13669592788](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42610103906/artifacts/13669592788):
- image platform: `linux/amd64`;
- Docker image configuration SHA256: `sha256:8e2d6d65ef05e3a7deb15fe29d5475c95f67a50b9d5dd265b74fa435d382ad96`;
- image-supplied source label: `moose-openmpi-x86_64.moose.sha=167ee97da204df5e8643695e18b9b28910f0014b`;
- module flavor: **OpenMPI** (the actual image labels are the source of truth for this exact digest); `moose-dev-openmpi-x86_64.version=2026.07.30`;
- Docker entrypoint: `/bin/bash -l -c` with `/environment` initialization required for runtime paths;
- observed `moose-opt --version`: `snapshot-20-10-27-55473-g167ee97da2`.

Official documentation: [MOOSE Docker installation](https://mooseframework.inl.gov/moose/getting_started/installation/docker.html) describes the installed `idaholab/moose` image as containing all MOOSE modules. Its generalized descriptions must not override the **observed exact-digest metadata**, including MPI flavor.

## Execution contract and fail-closed behavior

- Explicit opt-in: `SOL_MOOSE_BACKEND_PROFILE=official_moose_oci_thermal_v1`. No change to default Conda behavior.
- Pull the **exact immutable digest** as a separate provisioning step. Readiness checks `docker image inspect` only; it verifies the RepoDigest, image config ID, source/version labels, Entrypoint and platform. Unknown or mixed OCI+Conda configuration is rejected before workspace allocation.
- The real `MooseProcessRunner` creates the same Adapter-generated HIT and invokes the exact image with `--pull=never`, `--network=none`, `--cap-drop=ALL`, `no-new-privileges`, bounded pids, and a bounded bind-mounted run directory. The approved literal Bash bootstrap sources `/environment` and then `exec moose-opt` with positional arguments. No user-controlled command substitution or network-enabled container.
- The existing PR #49 container-ID based timeout shutdown is retained; no automatic replay when outcome is uncertain.
- The Protocol 0.2 execution provenance uses `moose.oci.image` with the exact official digest and `moose.executable=/opt/moose/bin/moose-opt`. It never emits a synthetic `moose.package` claim for OCI.
- Same `thermal.steady_conduction` capability, SOL Runtime v0.2 handshake, realization and bounded external execution semantics. No new canonical SOL schema or backend-native semantics in the Ontology.

## Evidence and remaining gates

1. **Already verified in isolated PR #50:** exact historical Phase 4 Thermal input `--check-input` and execution, direct Adapter IR/workspace and Phase 5 constant-field V&V with 1e-8 K threshold; 400 K and 399 K control results. Full digest reused on subsequent runs. Source input SHA256 `09e6690f11dc240792b1b922f1f5aba57aa38dea6bad10f4676c06b58dd0b3cd`.
2. **This stacked PR must prove:** official-digest profile's actual Protocol v0.2 preflight/execute and direct Adapter vs pinned SOL Runtime generated HIT byte parity, negative mutation rejection, correct provenance and no-replay semantics at exact HEAD. A passing unrelated Docker smoke check is insufficient.
3. **Still separate:** historical Conda 350-package checksum-attested supply and `moose=2026.08.12=mpich` native CI; explicit profile-owner acceptance; independent reviewer; SOL #172 official contract gate; Physics #282 plasma scientific verification; more thermal gradients/mesh refinement and multirank MPI checks. Do not relabel or skip any failed/missing gate.

**Merge ordering:** this PR depends on PR #49 for OCI transport and Protocol integration. It should only be considered for final merge after PR #49 is accepted and the base branch is reconciled; standalone PR #50 remains a non-accepting evidence source. No approved release or official backend substitution is inferred from test success.
