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
