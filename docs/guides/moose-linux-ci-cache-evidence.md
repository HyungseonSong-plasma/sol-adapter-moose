# Linux MOOSE CI cache: pinned build set and archive evidence

**Scope:** Adapter PR #43's Ubuntu 24.04 `moose-backend` CI, for `moose=2026.08.12=mpich` on `linux-64`. This is **not** a change to SOL Public Contract, Adapter Protocol, or Physics scientific acceptance.

## Prior durable installation evidence

- [CI run #42260284458](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42260284458) (2026-11-06) installed the exact package and recorded [artifact #13525430701](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/runs/42260284458/artifacts/13525430701). Although the subsequent MPI/UCX `--version` probe failed, package acquisition and the original MOOSE archive SHA-256 check succeeded.
- That artifact includes `phase1-conda-list.json`, containing **350** Conda dependency records. Canonicalize them as sorted arrays of `[name,version,build_string,base_url,platform]`, JSON-encoded with `separators=(',',':')` and ASCII escaping. Their SHA-256 build-set fingerprint is `6de33352a3830955f32cef42a37c4e022020a7b367fcb4c0dff428e0834a3b1a`. The archived build manifest is identity evidence, **not** a per-dependency artifact checksum list.
- Accepted MOOSE archive identity: `moose-2026.08.12-mpich.conda`, SHA-256 `d7b2eec6b958f11455c903b70a16ea319b7edb68b4499537313c06785abd05b3`.
- The old workflow used `find -name 'moose-*.json' -print -quit`, which could copy `moose-tools` metadata instead of the intended `moose` record. The cache workflow selects `conda-meta/moose-2026.08.12-mpich.json` exactly.

## Cache contract

1. On an exact-key cache hit, restore both the installed prefix and **all cached Conda package archives** at runner-owned, fixed paths. Skip network access to the INL Conda channel for package installation.
2. On a cache miss, attempt the unmodified exact Conda installation with bounded transient HTTP retries. There is **no trusted seeded cache yet**. A cold run can still fail while the INL channel responds with HTTP 502.
3. Before any cache save or MOOSE execution, verify the package count, canonical 350-package build-set fingerprint, exact MOOSE archive identity/hash, and each archived dependency's SHA-256 against Conda's installed metadata. Reject absent, inconsistent, or unsafe package archives. This checks archived bytes against their Conda metadata; it does **not** independently pin each dependency checksum to an already-reviewed upstream immutable lock.
4. Export `conda list --explicit --sha256` as `target/phase1-explicit-sha256.lock`, plus its digest and `phase1-dependency-sha256-manifest.json` as CI artifacts. This produces the full per-package hash-bearing lock candidate when a complete installation becomes available. Before claiming a **separately approved immutable transitive dependency lock**, its digest and original artifact provenance must be independently reviewed and adopted as a pinned repository input.
5. Save a successful *package verification* before MPI probing: the package cache is an acquisition result, not a claim that MOOSE startup, Thermal numerical V&V, or Physics validation passed. All prior native execution and scientific tests remain mandatory and fail closed.

The cache is identified by Ubuntu runner family, exact MOOSE version/build, accepted MOOSE archive identity, and the historical 350-dependency build-set fingerprint. Cache restoration is not authoritative without fresh SHA and build-set validation.

## Failure classes and next gate

- `CACHE_MISS + INL_HTTP_502`: upstream distribution unavailable; no solver verdict. A mirror or reviewed, independently retained artifact is needed to bootstrap without that channel.
- `CACHE_HIT + INTEGRITY_FAILURE`: reject the candidate cache, no fallback-to-pass; investigate and rotate the cache key only through a reviewed change.
- `PACKAGE_PASS + MOOSE_MPI_FAIL`: retain the verified cache as acquisition evidence and separately diagnose native MPICH/UCX. The diagnostic `UCX_TLS` invocation may not replace the baseline or pass the gate.
- `PACKAGE_PASS + MPI_PASS + NATIVE_TEST_PASS`: backend CI evidence only; not 0.3 Field or Plasma scientific sign-off.

Never republish a third-party package as an official INL release, silently substitute another MOOSE version, disable native MOOSE tests, or interpret cache restoration as scientific V&V.
