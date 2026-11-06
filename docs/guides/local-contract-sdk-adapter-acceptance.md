# Local workstation acceptance — SOL SDK + official Git-linked MOOSE Adapter candidate

**Status:** RUNBOOK FOR HUMAN / NOT YET EXECUTED ON USER WORKSTATION.  
**Owners:** SOL #176 / Draft PR #177 (source Contract SDK), Adapter #40 / Draft PR #42 (binary + backend), Human validator for final decision.  
**Security rule:** Never run installers as root/sudo; verify SHA before installation. The tools only write into the explicitly supplied disposable `--prefix`. Do not delete any MOOSE installation, Physics experiments or user result files.

## Scope and prerequisites

CI has verified a source archive and per-host native binary **candidate**. These are GitHub Actions artifacts, **not** signed, tagged or stable distributed packages. Artifacts expire after 30 days. Until a signed official release exists, the user must compare exact Git SHA, target architecture, artifact checksum and installed binary to this runbook.

Supported preliminary host families are Linux x86_64, macOS ARM64 and macOS Intel **only when their exact respective CI jobs passed**. A macOS 15 runner build does not guarantee macOS 10.15 compatibility. Windows native installer and generic ABI portability remain unverified.

Required: Git, Python 3, Rust/Cargo, Bash, `unzip`, and an optional externally installed compatible MOOSE executable (`moose-opt` or specified application). An internet connection is needed for Cargo dependencies.

## 1. Collect pinned sources and evidence

Choose the latest **successfully completed push** build on the following development branches. Do not download a PR merge-ref artifact if a branch-head push artifact is available:
- [SOL Contract SDK candidate GitHub Actions](https://github.com/HyungseonSong-plasma/simulation-ontology/actions/workflows/rust-core-ci.yml?query=branch%3Adesign%2Fplatform-boundary-package-local-smoke) — open the successful **push** workflow and download `sol-contract-sdk-candidate-<sha>`.
- [MOOSE Adapter native SDK-linked candidate GitHub Actions](https://github.com/HyungseonSong-plasma/sol-adapter-moose/actions/workflows/official-sdk-binary-candidate.yml?query=branch%3Apackaging%2Fofficial-sdk-consumer-draft) — open the successful **push** workflow and download an artifact matching your host: `Linux-X64`, `macOS-ARM64`, or `macOS-X64`.

The GitHub artifact itself is a ZIP that contains `.tar.gz`, `.sha256`, and (for Adapter) `smoke.json`. Record both run URL and exact `head_sha` for each. Extract those ZIPs into separate folders under your local evidence directory; never mix platform artifacts.

```bash
export SOL_QA_ROOT="$HOME/sol-acceptance"
mkdir -p "$SOL_QA_ROOT"/{sdk,adapter,results,checkout,prefix}

# Place the two downloaded GitHub Actions ZIP archives in ~/Downloads.
# Replace these placeholders with the actual ZIP filenames:
unzip -o "$HOME/Downloads/<SOL_SDK_ARTIFACT>.zip" -d "$SOL_QA_ROOT/sdk"
unzip -o "$HOME/Downloads/<ADAPTER_ARTIFACT_FOR_YOUR_PLATFORM>.zip" -d "$SOL_QA_ROOT/adapter"
```

Check the published source revision/host against the exact downloaded artifact manifest; if it does not match, **stop**. Clone the two repositories at the matching verified branch commits (use the full SHA from each successful push job, not floating `main`). You may start with:

```bash
git clone https://github.com/HyungseonSong-plasma/simulation-ontology.git "$SOL_QA_ROOT/checkout/sol"
git -C "$SOL_QA_ROOT/checkout/sol" fetch origin design/platform-boundary-package-local-smoke
# Replace with SHA of the successful SOL branch-head push run:
git -C "$SOL_QA_ROOT/checkout/sol" checkout "<SOL_P1_P2_SOURCE_SHA>"

git clone https://github.com/HyungseonSong-plasma/sol-adapter-moose.git "$SOL_QA_ROOT/checkout/adapter"
git -C "$SOL_QA_ROOT/checkout/adapter" fetch origin packaging/official-sdk-consumer-draft
# Replace with SHA of the successful Adapter branch-head push run:
git -C "$SOL_QA_ROOT/checkout/adapter" checkout "<ADAPTER_P3_P4_SOURCE_SHA>"

export SOL_ROOT="$SOL_QA_ROOT/checkout/sol"
export ADAPTER_ROOT="$SOL_QA_ROOT/checkout/adapter"
```

## 2. Verify and exercise official SOL Contract SDK source archive

```bash
python3 "$SOL_ROOT/scripts/build-contract-sdk-candidate.py" \
  --output-dir "$SOL_QA_ROOT/sdk" --verify-only

tar -xzf "$SOL_QA_ROOT/sdk/sol-contract-sdk-candidate.tar.gz" -C "$SOL_QA_ROOT/results"

cargo generate-lockfile \
  --manifest-path "$SOL_QA_ROOT/results/sol-contract-sdk-candidate/Cargo.toml"

cargo test \
  --manifest-path "$SOL_QA_ROOT/results/sol-contract-sdk-candidate/Cargo.toml" \
  --workspace --locked 2>&1 | tee "$SOL_QA_ROOT/results/sdk-crate-tests.log"

# Independently prove the immutable upstream Git consumer, not only an archive.
export SOL_EVIDENCE_DIR="$SOL_QA_ROOT/results"
bash "$SOL_ROOT/scripts/verify-git-contract-consumer.sh"
```

**PASS:** archive and every manifest SHA pass; archived crate tests pass, and upstream Git consumer runs four positive + one wrong-version rejection. The Git consumer is intentionally pinned to the previously accepted SOL `main@b6924e65...`, distinct from the non-release archive revision. This does not approve Public Contract 0.3 Dataset.

## 3. Verify/install Adapter binary in user-only prefix

```bash
export ADAPTER_ARCHIVE="$SOL_QA_ROOT/adapter/sol-adapter-moose-candidate.tar.gz"
export ADAPTER_SHA="$SOL_QA_ROOT/adapter/sol-adapter-moose-candidate.sha256"

python3 "$ADAPTER_ROOT/scripts/adapter_binary_candidate.py" verify \
  --archive "$ADAPTER_ARCHIVE" --checksum "$ADAPTER_SHA"

# Inspect source revision, target platform, claimed Protocol/Contract support:
python3 - "$ADAPTER_ARCHIVE" <<'PY'
import json,sys,tarfile
with tarfile.open(sys.argv[1]) as tar:
    manifest=json.load(tar.extractfile(
        "sol-adapter-moose-candidate/candidate-manifest.json"))
print(json.dumps(manifest,indent=2))
PY

python3 "$ADAPTER_ROOT/scripts/adapter_binary_candidate.py" install \
  --archive "$ADAPTER_ARCHIVE" --checksum "$ADAPTER_SHA" \
  --prefix "$SOL_QA_ROOT/prefix" --activate

export ADAPTER_BIN="$SOL_QA_ROOT/prefix/bin/sol-adaptor-moose"

printf '{"jsonrpc":"2.0","method":"describe_adapter","id":1}\n' \
  | "$ADAPTER_BIN" | python3 -m json.tool \
  | tee "$SOL_QA_ROOT/results/adapter-describe.json"
```

**PASS:** binary source SHA and exact `target_host` match the artifact and user host, installer reports success, `describe_adapter` reports only Protocol/Public Contract `0.1, 0.2`, target `moose` and capability `thermal.steady_conduction`. Wrong host/architecture is **STOP**, not a reason to bypass the check. An older macOS without ABI compatibility may fail even when CPU architecture matches.

## 4. Negative backend test — safe to run before MOOSE is installed

```bash
unset SOL_MOOSE_EXECUTABLE
python3 - "$ADAPTER_ROOT/tests/fixtures/sol/0.2/thermal-realization-request.json" \
  "$SOL_QA_ROOT/results/adapter-preflight-request.json" <<'PY'
import json,sys
with open(sys.argv[1]) as f:
    request=json.load(f)
with open(sys.argv[2],"w") as f:
    json.dump({"jsonrpc":"2.0","method":"validate_plan","id":2,"params":request},f)
    f.write("\n")
PY

cat "$SOL_QA_ROOT/results/adapter-preflight-request.json" \
  | "$ADAPTER_BIN" | python3 -m json.tool \
  | tee "$SOL_QA_ROOT/results/adapter-missing-backend.json"
```

**PASS:** explicit Protocol 0.2 preflight returns `unavailable` / `adapter.transient_unavailable`. The Adapter must not silently invent a result or execute MOOSE. The request JSON is written first, then passed to the Adapter, avoiding shell heredoc/pipeline ambiguity.

## 5. Actual MOOSE execution via SOL Runtime (only if installed)

Verify the external MOOSE executable first:

```bash
# Replace with your tested compatible MOOSE/Physics application absolute path.
export SOL_MOOSE_EXECUTABLE="/absolute/path/to/moose-opt"
"$SOL_MOOSE_EXECUTABLE" --version
export SOL_MOOSE_WORKSPACE_ROOT="$SOL_QA_ROOT/results/moose-workspace"
mkdir -p "$SOL_MOOSE_WORKSPACE_ROOT"
```

Construct the Runtime's request envelope **from the published same-run thermal Realization 0.2 request** (no modified ontology fields):

```bash
python3 - "$ADAPTER_ROOT/tests/fixtures/sol/0.2/thermal-realization-request.json" \
  "$SOL_QA_ROOT/results/runtime-thermal-request.json" <<'PY'
import json,sys
request=json.load(open(sys.argv[1]))
with open(sys.argv[2],"w") as out:
    json.dump({"target":"moose",
               "required_capabilities":["thermal.steady_conduction"],
               "plan_request":request},out,indent=2)
PY

cargo run --locked \
  --manifest-path "$SOL_ROOT/examples/external-runtime-v02/Cargo.toml" -- \
  run "$ADAPTER_BIN" "$SOL_QA_ROOT/results/runtime-thermal-request.json" \
  2>&1 | tee "$SOL_QA_ROOT/results/sol-runtime-moose-integration.log"
```

**PASS:** live describe→target/capability selection→Protocol 0.2 validate/execute, real MOOSE Steady Thermal input/solve and backend provenance, no unauthorized protocol upgrade or automatic re-execution after ambiguous failure. **Protocol 0.2 does not provide canonical 0.3 Temperature Field Result publication**. That requires separate accepted SOL Phase 3/4 and Physics Human review. A real MOOSE solve does not alone prove scientific validity.

## 6. Safe uninstall + evidence handoff

Get the exact installed revision from the Archive manifest (same as verified Adapter branch-head push SHA):

```bash
export ADAPTER_REV="$(python3 - "$ADAPTER_ARCHIVE" <<'PY'
import json,sys,tarfile
with tarfile.open(sys.argv[1]) as t:
    print(json.load(t.extractfile(
        "sol-adapter-moose-candidate/candidate-manifest.json"))["source_commit"])
PY
)"

python3 "$ADAPTER_ROOT/scripts/adapter_binary_candidate.py" uninstall \
  --prefix "$SOL_QA_ROOT/prefix" --revision "$ADAPTER_REV"

test ! -e "$SOL_QA_ROOT/prefix/bin/sol-adaptor-moose" &&
  echo "adapter removed; experiment data preserved"

uname -a > "$SOL_QA_ROOT/results/local-platform.txt"
rustc -V >> "$SOL_QA_ROOT/results/local-platform.txt"
cargo -V >> "$SOL_QA_ROOT/results/local-platform.txt"
```

**Human handoff:** supply the two artifact run URLs and exact SHAs, `local-platform.txt`, SDK and Git consumer logs, `adapter-describe.json`, `adapter-missing-backend.json`, `sol-runtime-moose-integration.log` if run, archive checksums, and any failed step with stderr. Record independent `PASS / FAIL / BLOCKED` judgments for SDK integrity, package installation, Protocol compatibility, real MOOSE execution, physics/numerical V&V, and Human approval. Never collapse all of them into a single 'CI green' status.

**Blocking conditions:** no downloaded artifact for your host, archive SHA failure, unsupported host/ABI, missing MOOSE for the final real run, semantic contract mismatch, or unexpected output. Stop rather than installing as root, bypassing a checksum, using a floating revision or modifying user/Physics directories.
