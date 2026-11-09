# JSON-RPC Adapter backend choice (PR #50 candidate)

**Scope:** the `sol-adaptor-moose` stdin/stdout JSON-RPC process and its existing Adapter Protocol 0.1/0.2. This is **not** a change to SOL Public Contract meaning, a new physics capability, or acceptance of Conda binary parity.

## Consumer selection

The process reads the following environment when it starts. Backend selection is **process-scoped**, never inferred from the requested SOL plan.

| Variable | Meaning |
| --- | --- |
| `SOL_MOOSE_BACKEND` | `oci` (default) or `conda`. Anything else fails before JSON-RPC dispatch. |
| `SOL_MOOSE_OCI_CANDIDATE_IMAGE` | Optional, but if present it **must equal** the candidate Digest below. No floating tags. |
| `SOL_MOOSE_OCI_EXECUTABLE` | OCI launcher path. If absent, the adapter remains available for `describe_adapter`, but 0.2 native validation/execution responds unavailable; it does **not** fallback to Conda. |
| `MOOSE_DOCKER_PIN` | Must equal the exact OCI image Digest if the external OCI launcher is configured. |
| `SOL_MOOSE_WORKSPACE_ROOT` | Runtime workspace for input/results. Defaults to a temporary `sol-adaptor-moose-runtime` directory. |
| `SOL_MOOSE_OCI_MOUNT_ROOT` | Optional host mount root used by the experimental pinned wrapper. Defaults to `GITHUB_WORKSPACE` in Actions; otherwise the process working directory. Run directories must be within this root. |
| `SOL_MOOSE_EXECUTABLE` | Only for the **explicit** `conda` selection. It remains subject to the original exact Conda identity/metadata and capability probes. |

Frozen experimental OCI image (Linux/amd64):

```text
idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452
```

### Example: explicit OCI local consumer

The first package download must come through a trusted Docker registry; this does not depend on the INL Conda repodata channel.

```bash
export MOOSE_DOCKER_PIN='idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452'
docker pull "$MOOSE_DOCKER_PIN"

install -m 0755 experiments/official_moose_docker_probe/moose_wrapper.sh ./target/moose-oci-wrapper
export SOL_MOOSE_BACKEND=oci                 # optional: this is the default
export SOL_MOOSE_OCI_EXECUTABLE="$(pwd)/target/moose-oci-wrapper"
export SOL_MOOSE_WORKSPACE_ROOT="$(pwd)/target/moose-oci-workspace"
export SOL_MOOSE_OCI_MOUNT_ROOT="$(pwd)"

cargo run --locked --bin sol-adaptor-moose
# Send line-delimited JSON-RPC requests on stdin, one request per line.
```

The provided wrapper launches only the pinned Digest via `docker run --pull never --network none --rm`, with the current working directory inside the selected mount root, and records the actual OCI Digest in completed Protocol 0.2 provenance (`moose.oci.image`). Do not set `SOL_MOOSE_EXECUTABLE` to impersonate a Conda install.

### Example: explicit legacy Conda consumer

```bash
export SOL_MOOSE_BACKEND=conda
export SOL_MOOSE_EXECUTABLE=/trusted/moose-env/bin/moose-opt
export SOL_MOOSE_WORKSPACE_ROOT="$(pwd)/target/legacy-conda-workspace"
cargo run --locked --bin sol-adaptor-moose
```

The Conda package identity is still `moose=2026.08.12=mpich` for `linux-64`. Selecting `conda` does not waive any identity/probe gate or solve the INL channel outage.

## Explicit CI status

The original `moose-backend` installation Job in `.github/workflows/rust-ci.yml` is **DEFERRED**, **not PASS**, unless repository GitHub Actions variable `MOOSE_CONDA_CI_ENABLED=true` is deliberately enabled. The `conda-provisioning-deferred` job writes an explicit `NOT_EVALUATED` notice. The original Conda job source remains available and must be restored as a required acceptance gate before exact Conda parity or release acceptance. OCI numerical and Protocol CI tests remain separately required.

## Unresolved before production rollout

1. **Executable custody:** the experimental API still accepts an external `SOL_MOOSE_OCI_EXECUTABLE` and cannot itself prove the binary is the pinned Docker runtime. Digest/pin environment consistency and a CI-verified wrapper do not provide trustworthy attestation of arbitrary consumer launchers. Implement and verify a process-owned OCI executor (or approved signed/attested launcher with immutable binding) before promoting this default to production.
2. **Container cleanup and cancellation:** the current timeout in `MooseProcessRunner` kills the wrapper client process, not necessarily the Docker daemon's container. A supervised container ID, guaranteed stop/reap and timeout evidence are necessary.
3. **Recovery policy:** `execute_plan` is non-idempotent by default. A lost response after solver execution must not trigger an automatic replay. The two-run experiment proves input-repeatability and workspace isolation only.
4. **Consumer compatibility:** certify packaged JSON-RPC deployments and the distinct SOL SDK/GUI consumers; the current CI covers a real local subprocess but not all supported deployments.
5. **Physics scope:** the 1D constant/linear Thermal oracles and Phase 5 V&V are not multirank MPI, nonlinear, time-dependent, or SOL 0.3 scientific acceptance.

This PR records the **requested default selection in candidate code**; it is still Draft and does not change `main`, merge, global public contract acceptance, or deployment configuration elsewhere.
