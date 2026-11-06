# Local acceptance — unreleased SOL Contract SDK + MOOSE Adapter package

**State: RUNBOOK READY / ACTUAL USER WORKSTATION NOT VERIFIED.** This is a test guide, not evidence of a completed local test or official SDK release. The owner-approved Human gate and GitHub merge/review policy are separate.

## Components / exact immutable baselines

- SOL Contract SDK candidate source: [SOL Platform #176](https://github.com/HyungseonSong-plasma/simulation-ontology/issues/176) / [Draft PR #177](https://github.com/HyungseonSong-plasma/simulation-ontology/pull/177). Contract SDK pilot source is exact accepted `simulation-ontology@b6924e6556684eec9dd93677518821cc7adc84b6`; this is not a tagged official release.
- Adapter binary candidate: [Adapter #40](https://github.com/HyungseonSong-plasma/sol-adapter-moose/issues/40) / [Draft PR #43](https://github.com/HyungseonSong-plasma/sol-adapter-moose/pull/43). Use only a CI artifact from the **exact passing commit** after checking workflow summary; do not download a previous failed/cancelled run.
- Supported in this trial: explicit Adapter Protocol 0.1/0.2 and Public Contract 0.1/0.2. SOL Public Contract 0.3 scalar is separately accepted in the SDK but **this Adapter does not advertise 0.3**, and neither new Field Dataset nor Adapter Protocol 0.3 is accepted here.
- Candidate package target is per-platform/architecture, **not universal**. Check actual `uname -s`, `uname -m`, and package `target_host`. An ubuntu Linux archive cannot run on macOS. An Intel macOS archive is not an Apple Silicon-native build.
- MOOSE application is **not** included in the Adapter package. Provide your installed app's executable path for a real solve. Adapter capability currently only `thermal.steady_conduction`; this does not establish plasma physics support.

## Prerequisites on workstation

Git, Python 3.9+ standard library, Rust/Cargo with project-supported Rust >=1.80, Bash (for SOL D0/P1 shell test), sufficient disk/network to fetch Cargo packages. For actual solve, a working supported MOOSE application and external `moose-opt` executable; use its own existing installation without modifying it.

Native installer script uses only Python stdlib, does not require sudo, does not write global PATH, and installs into a user-selected test prefix. On macOS, do not install Linux binary; obtain a matching macOS candidate or build an appropriately tested candidate from the exact Adapter source. Windows-native binary and installer testing are still a separate acceptance gate; Windows users can use WSL for Linux checks but this **does not certify Windows-native compatibility**.

## Experimental macOS arm64 pre-built MOOSE profile (unreleased)

The Adapter backend now has a **candidate, exact-record** macOS arm64 Conda profile, in addition to the unchanged accepted Linux Phase 1 profile. This change concerns *backend eligibility only*, not SOL Public Contract or Adapter Protocol version support. It does not mean that a real macOS solve has succeeded.

| Conda metadata | Candidate required value |
| --- | --- |
| package name | `moose` |
| version | `2026.09.13` |
| build | `mpich` |
| subdir | `osx-arm64` |
| fn | `moose-2026.09.13-mpich.conda` |
| sha256 | `2fcdc21ef5ebb7f416df89aa7e992c2f3c77c0e1ebfe39ab3d93705829b3ad73` |

These values are from a **user-reported local Conda package record**. They are candidate identity inputs, not cryptographically independent verification of installed executable bytes or a scientific V&V verdict. A missing/incorrect macOS filename or SHA is rejected. The existing Linux accepted `2026.08.12 / mpich / linux-64` record and its historical optional-metadata behavior are preserved.

**Local execution:** obtain the exact latest passing Draft PR #43 source (do not replace an existing, modified checkout), build `sol-adaptor-moose` natively on `osx-arm64` with `cargo build --release --locked`, package it with `python3 scripts/package_adapter_candidate.py build --binary target/release/sol-adaptor-moose --output-dir <isolated-package-dir>`, and run the existing `scripts/local_adapter_acceptance.py` with `--moose-executable` set to the absolute path of the installed pre-built Conda `bin/moose` executable. Use the exact accepted SOL Runtime source revision required by the script and a fresh isolated evidence directory.

The Adapter separately probes `--version`, `--show-type`, `--show-copyable-inputs` (must include `heat_transfer`) and `--help`. The Runtime then performs `--check-input` and actual 1D Steady Thermal execution. A probe PASS is *not* a native solver PASS. Preserve full logs and report unsupported objects, MOOSE runtime failures or numeric discrepancies distinctly. Do not use the Linux x86_64 binary archive on macOS; do not publish a stable macOS release, merge a Draft, or claim Protocol 0.3 support from this candidate.

## Step 1 — obtain an exact-commit verified artifact

1. In [Adapter Draft PR #43](https://github.com/HyungseonSong-plasma/sol-adapter-moose/pull/43) open **Checks** and select the successful `Adapter Package Candidate and SOL Contract Parity` run for the exact PR head. The run provides `moose-adapter-unreleased-candidate-<runner>-<sha>`. Download the ZIP and extract it into a private working directory.
2. Check the archive, sidecar `.tar.gz.sha256` and `binary-dynamic-dependencies.txt` (on supported runner). Do not trust merely the artifact file name; verify SHA and target host.
3. Also verify the [SOL SDK Draft PR #177](https://github.com/HyungseonSong-plasma/simulation-ontology/pull/177) checksum-verified Contract SDK source candidate as an independent packaging artifact. This is SDK preparation evidence, not a required binary dependency for the adapter's test.

For a self-build fallback, check out the exact Adapter commit corresponding to the artifact, use `cargo build --release --locked`, and run `python3 scripts/package_adapter_candidate.py build --binary target/release/sol-adaptor-moose --output-dir ...`. **A self-built candidate is a local build, not a GitHub Release download.**

## Step 2 — prepare exact source revisions

```bash
git clone https://github.com/HyungseonSong-plasma/simulation-ontology.git
git -C simulation-ontology checkout b6924e6556684eec9dd93677518821cc7adc84b6

git clone https://github.com/HyungseonSong-plasma/sol-adapter-moose.git
git -C sol-adapter-moose checkout <EXACT_SUCCESSFUL_PR_43_SHA>
```

The Adapter checkout supplies the Python local acceptance/installer implementation. The SOL checkout is the accepted stable Runtime 0.2 consumer. **Do not** use floating `main` or Draft PR #173 as a substitute.

## Step 3 — run SDK standalone tests

From a separate checkout of SOL Draft PR #177, run:

```bash
export SOL_EVIDENCE_DIR="$HOME/sol-local-evidence"
bash scripts/local-contract-consumer-smoke.sh
bash scripts/verify-git-contract-consumer.sh
```

Those test D0 source-path and P1 official-Git-revision Contract SDK consumption. They are not tagged-release tests.

## Step 4 — install and verify the Adapter candidate (without MOOSE)

From the exact Adapter PR #43 checkout, with the matching, unzipped binary package `.tar.gz` and `.sha256` present:

```bash
python3 scripts/local_adapter_acceptance.py \
  --archive "/ABSOLUTE/PATH/sol-adapter-moose-candidate-<os>-<arch>.tar.gz" \
  --sol-checkout "/ABSOLUTE/PATH/simulation-ontology" \
  --evidence-dir "$HOME/sol-local-evidence/adapter-package"
```

This checks SHA, archive, host/architecture, fixed 0.1/0.2 adapter advertised versions, explicit rejection of Protocol 0.3, safe scoped installation and uninstall. It writes `local-acceptance.json`, `describe.log`, `wrong-protocol.log`, `adapter-package-manifest.json`, `local-environment.json` and related logs. `moose_backend: NOT_RUN` is expected in this stage, **not a failure**.

## Step 5 — actual SOL Runtime → Adapter → MOOSE run

Only after an MOOSE application installed on the workstation is known to work, rerun with:

```bash
python3 scripts/local_adapter_acceptance.py \
  --archive "/ABSOLUTE/PATH/sol-adapter-moose-candidate-<os>-<arch>.tar.gz" \
  --sol-checkout "/ABSOLUTE/PATH/simulation-ontology" \
  --evidence-dir "$HOME/sol-local-evidence/adapter-moose-run" \
  --moose-executable "/ABSOLUTE/PATH/TO/moose-opt"
```

The script generates a consumer request from the accepted thermal 1D 400 K RealizationSpec 0.2 fixture, sets an **isolated MOOSE workspace inside the evidence directory**, and invokes SOL's external Runtime 0.2 consumer against the installed binary. It requires completed execute/validation and `execute_response_loss_replay: forbidden`. It stores `thermal-runtime-02.log` and `thermal-runtime-02-result.json`. **This checks Protocol/Runtime integration and a completed solver invocation; it is not a scientific V&V sign-off and does not prove 0.3 Scalar+Field publication.** Native MOOSE output/log files are retained in the isolated evidence workspace for Physics inspection.

**Error handling:** if the solver's MPI/UCX transport fails, the local result must say `FAIL` (or `BLOCKED`) rather than being treated as a success. In that case capture application version, MPI implementation and `thermal-runtime-02.log`. Do not silently disable MOOSE backend tests or claim results from a different run.

## Step 6 — evidence to return for Human acceptance

From both runs, report:
- `local-acceptance.json` and `local-environment.json`;
- package manifest and checksum, exact Adapter SHA and SOL Git SHA;
- `describe.log`, `wrong-protocol.log`, `thermal-runtime-02.log`, `thermal-runtime-02-result.json` if an actual MOOSE application was supplied;
- optionally include `~/sol-local-evidence` D0/P1 logs; avoid uploading private executable paths, environment credentials, or proprietary solver data.

The independent Human Validator should then decide `APPROVE / REQUEST_CHANGES / HOLD`, with the scientific Physics evidence and actual package/runtime evidence kept separate. **No automatic merge/release/Phase #156/#175 closure results from these commands.**
