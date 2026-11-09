# Official MOOSE OCI alternative — candidate evidence and adoption gate

**Date:** 2026-11-09  
**State:** `EXPERIMENTAL_NATIVE_THERMAL_PASS / OCI_IDENTITY_NOT_ACCEPTED_AS_CONDA / SOL_PROTOCOL_PARITY_NOT_EVALUATED`.

## Immutable candidate

- Source: [MOOSE official Docker documentation](https://mooseframework.inl.gov/moose/getting_started/installation/docker.html), official Docker Hub `idaholab/moose` installed all-modules application, Rocky Linux 8.10 / MPICH 4.3.1.
- **Frozen for this test:** `idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452` (resolved registry manifest digest, not mutable `latest`).
- Container MOOSE version output: `Application Version: snapshot-20-10-27-55473-g167ee97da2`, executable `/opt/moose/bin/moose-opt`. Linux/amd64 image-config SHA: `sha256:8e2d6d65ef05e3a7deb15fe29d5475c95f67a50b9d5dd265b74fa435d382ad96`.
- Original Adapter Phase 4 Thermal input byte hash: `09e6690f11dc240792b1b922f1f5aba57aa38dea6bad10f4676c06b58dd0b3cd`, source copied byte-for-byte from [Draft PR #48](https://github.com/HyungseonSong-plasma/sol-adapter-moose/pull/48), and verified against original historical `phase4-execution-evidence/runs/thermal-realization-001/input.i`.

## Verified GitHub evidence

| Test / decision gate | Result | Evidence |
| --- | --- | --- |
| Docker Hub full Digest discovery, image pull, `moose-opt --version`, historical `--check-input`, actual Thermal solve | PASS | [Run #42609211298](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42609211298), [artifact #13669996029](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42609211298/artifacts/13669996029), artifact ZIP SHA256 `5151970233f464bcc7e1919b3cd807c8ab1a5e502b05faa2b5f7ddc8f0ca9568` |
| Pull and rerun by *explicit immutable digest*; same CLI + Thermal numeric controls | PASS | [Run #42609656169](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42609656169) |
| Real Adapter Rust backend-only generated IR, realization 0.2 check, workspace execution, Phase 5 Thermal native V&V | 4/4 PASS | [Run #42610103906](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42610103906), [artifact #13669592788](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42610103906/artifacts/13669592788); ZIP SHA256 `5d2e28cfc4bce028f550147ad40b5b08d1af58941e6ca2ff7bda3c1968963e71` |
| Frozen Conda identity `moose=2026.08.12=mpich` verified by OCI | **NOT_EVALUATED** | Docker image and Conda release are distinct packages/builds |
| Full Adapter Protocol 0.2 authoritative execution + exact backend provenance | **NOT_EVALUATED** | `ExecutionEnvironment::backend_readiness()` requires the pinned Conda package and exact inferred conda-prefix metadata; do not manufacture fake records |
| SOL 0.3 Field / Plasma V&V, multirank MPI / mesh/time convergence | **NOT_EVALUATED** | Separate scientific and compatibility gates |

### Narrow same-input comparison to actual prior Conda native execution

Authentic prior [Conda Native run #42200001106](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42200001106), [artifact #13493163467](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42200001106/artifacts/13493163467), original ZIP SHA256 `fb629aac9f74b7479a511bd3b71c9926225375fea02974afacc1277d20695e38`:

| Phase 5 steady constant-field V&V, 400 K reference | Historical Conda | Pinned official Docker |
| --- | ---: | ---: |
| Recorded minimum K | 400.0 | 400.0 |
| Recorded maximum K | 400.00000000001 | 400.00000000001 |
| Maximum absolute error K | 1.000444171950221e-11 | 1.000444171950221e-11 |
| Tolerance K | 1e-8 | 1e-8 |

The two ZIP artifacts were inspected directly and the original Phase 4 input SHA is identical. This proves **numerical agreement for this bounded benchmark only**. The OCI experiment additionally changed the boundary from 400 K to 399 K and observed 399 K, as a nonconstant-response control; it does not establish broader model or binary equivalence.

**Historical provenance caveat:** Artifact #13493163467's previously archived `phase1-moose-conda-meta.json` is actually a `moose-tools` record owing to ambiguous glob selection. Historical runtime `MooseCondaIdentity::load_from_prefix` had independently searched for `name == moose`; the artifact-label defect is separately addressed in Draft PR #46. Do not use this artifact as a full trusted 350-package closure or an archive mirror.

## Safe next decision

1. **Preserve legacy exact-identity lane.** [Issue #34](https://github.com/HyungseonSong-plasma/sol-adapter-moose/issues/34) and [Draft PR #43](https://github.com/HyungseonSong-plasma/sol-adapter-moose/pull/43) retain the required `moose=2026.08.12=mpich` Conda package identity, hash, full-closure provisioning and native gates. A Docker PASS is **not** an implicit replacement for an `INFRA_BLOCKED` Conda job.
2. **Introduce a separately declared supported OCI backend identity only after review.** Proposed adapter-local profile would pin image manifest digest, true embedded MOOSE source version, OS/ABI, MPI configuration, real exposed CLI capabilities, and explicit provenance. It must not claim `PHASE1_MOOSE_PACKAGE_FILENAME` or synthesize Conda metadata to satisfy `probe_exact_phase1_target()`.
3. **Extend compatibility and V&V.** Verify explicit supported-target compatibility decisions, real Protocol 0.2 external process execute/replay boundaries and provenance, 1D Thermal gradients and multiple mesh refinements, same-run CSV/Exodus evidence and 0.3 result obligations where applicable. Separate SOL contract owner and Physics owner approval gates remain.
4. **Make Docker a formal CI alternative only after those gates.** Keep OCI job independent and fail closed; do not let a container smoke check mark legacy Conda exact-identity CI green.

No MOOSE application source, scientific model, public SOL schema, accepted release, or existing native Conda validation is modified by this experimental PR.

## 2026-11-09 — explicit opt-in Protocol 0.2 OCI execution candidate

After the original four native Rust backend tests passed, [PR #50](https://github.com/HyungseonSong-plasma/sol-adapter-moose/pull/50) added an experimental, non-default backend identity to `src/oci_profile.rs` and `src/execution_v02.rs`:

- `ExecutionEnvironment::configured_official_oci_candidate(..., APPROVED_CANDIDATE_IMAGE)` is an explicitly called constructor. Existing `configured()`, `from_environment()` and `unavailable()` remain Conda-identity based.
- `OfficialOciProfile::candidate()` requires the **exact immutable** `idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452` and rejects mutable tags, other digests and the unrelated Physics runtime.
- The profile probes real `moose-opt --version` and `--help` for `--check-input`. The CI wrapper mounts only the repository workspace, disables the container network and invokes the exact image supplied by the trusted CI setting `MOOSE_DOCKER_PIN`. No fabricated `conda-meta` records or package hashes are introduced.
- On **completed** 0.2 execution, OCI provenance uses `moose.oci.image = idaholab/moose@sha256:...`. Legacy executions continue to use `moose.package = moose-2026.08.12-mpich.conda`; the new Protocol 0.2 test asserts the OCI result does not contain a Conda package reference.
- New `tests/moose_execution_oci_candidate.rs` executes actual `validate_plan_v02` and `execute_plan_v02_with_run_key` on the historical Thermal request and checks action/effect ordering, required side-effect artifacts and truthful OCI provenance. It is opt-in via `SOL_MOOSE_OCI_EXECUTABLE`; it cannot replace the original Conda-specific Phase 4 test.

**Latest exact-head proof:** [OCI CI #42617488600](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42617488600), [artifact #13673386767](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42617488600/artifacts/13673386767) (ZIP SHA256 `96fea5d3b57dc0fc7682b1f69f91d3a72c7cbd10bb21783b7f81e26a1946b42f`). Five real Adapter Rust tests (IR, realization, backend workspace, Phase 5 V&V, Protocol 0.2 OCI execution) all passed; the original pinned Docker Thermal/negative control passed. [Rust CI #42617488247](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42617488247) passed rustfmt/tests/Clippy/central scope.

**Remaining blocker:** the OCI profile is **a research candidate**, not a security-hardened system-wide or officially accepted backend selector. Its constructor is callable by code with a declared digest but does not independently authenticate the underlying process's OCI image; binding depends on the pinned CI wrapper and environment. A separately reviewed production transport must verify image identity inside the execution boundary and handle safe process cancellation, state/replay semantics, and provenance for all supported callers. External packaged subprocess protocol/SDK/GUI consumers, exact Conda identity parity, full scientific SOL/Physics sign-off, and native multirank MPI have not been accepted. Do not merge or change defaults on this evidence alone.

## External JSON-RPC, failure isolation and analytic gradient verification

**Evidence source:** 2026-11-09 [official OCI run #42619170458](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42619170458), [artifact #13675276135](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42619170458/artifacts/13675276135) (ZIP SHA256 `5241dbcb6c1b1e9f07ff0f616d9f24380e5de983edb813489c0c7d55c6f4c986`). The ZIP was read directly for `gradient/report.json` and `oci-external-transport-evidence/external-protocol-report.json`, not inferred from the CI checkmark.

| Independent gate | Observed result | Boundary |
| --- | --- | --- |
| Real external Adapter binary, stdio JSON-RPC `validate_plan` and two `execute_plan` requests | **PASS**, valid Protocol 0.2 responses with real OCI solve | Local experimental external binary; other consumer SDKs have not been tested |
| Two successive execute requests with the same canonical request | **PASS**, two distinct run keys and bit-identical persisted `input.i` | Proves fresh isolated workspaces, **not** exactly-once replay/idempotency or crash recovery |
| Declared OCI Digest mismatch before startup | **PASS**, adapter nonzero exit, no workspace or stdout | Environment mismatch rejection, **not** independent runtime image custody |
| Probe failure with `/usr/bin/false` | **PASS**, `ExecutionOutcome::Rejected`, no workspace | Fail-closed preflight; not a post-start failure/cancellation test |
| Exact linear 1D Thermal gradient with 400 K and 300 K Dirichlet boundaries | **PASS** on `nx=4,8,16` | Exact linear benchmark, not broad-order convergence evidence |
| Thermal negative boundary variation (right boundary 299 K, `nx=8`) | **PASS** | Shows the observed solution responds to changed boundary condition |

### Recorded gradient results

All below used the immutable OCI image digest above. The analytic temperature is `T(x) = 400 + (T_right-400) x` K over `x in [0,1]`. Tolerance was `1e-6 K`.

| Case | T(0.25) observed / reference K | T(0.75) observed / reference K | Max abs error K |
| --- | --- | --- | --- |
| nx=4, right=300 K | 374.99999999999 / 375 | 324.99999999999 / 325 | 1.000444171950221e-11 |
| nx=8, right=300 K | 375.00000000003 / 375 | 325.00000000003 / 325 | 3.001332515850663e-11 |
| nx=16, right=300 K | 375 / 375 | 325 / 325 | 0 |
| nx=8, right=299 K | 374.75 / 374.75 | 324.25 / 324.25 | 0 |

### Explicitly incomplete acceptance gates

1. **OCI executable custody: BLOCKED for production.** The new research-only `SOL_MOOSE_OCI_CANDIDATE_IMAGE` external process mode and `configured_official_oci_candidate()` check the requested digest and command responses, and CI supplies a repository-pinned wrapper. However the public constructor still accepts an arbitrary executable path: neither the method nor the external transport independently proves that the supplied executable invokes the exact OCI image. It must not be treated as a trusted production custody guarantee. Production needs a process-owned Docker/OCI executor that binds the digest at the invocation boundary and rejects untrusted paths.
2. **Replay/recovery: PARTIALLY_EVALUATED.** Different workspaces and identical input bytes prove repeatability of the evaluated Thermal request, not side-effect idempotency, deterministic replay of prior results, crash recovery or timeout/container cancellation across a real subprocess.
3. **Broader consumers: NOT_EVALUATED.** A real JSON-RPC stdio process was exercised, not every independently deployed SOL SDK, GUI or external adapter client. Its opt-in mode is not advertised or selected by default.
4. **Extended science: NOT_EVALUATED.** The bounded 1D exact linear oracle does not establish nonlinearity, multirank MPI, mesh/time convergence, source-term physics or a SOL 0.3/Physics scientific signoff.
5. **Frozen Conda backend: SEPARATE, NOT_REPLACED.** The exact Conda 2026.08.12 mpich identity remains mandatory in its original job. Passing OCI does not change the INL HTTP 502/provisioning verdict.

**Governance:** Draft PR #50 must remain non-accepting and unmerged until the explicit transport-custody and additional compatibility/approval decisions are made. Default behavior remains the legacy Conda profile; the experimental CLI mode requires deliberate environment selection.
