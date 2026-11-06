#!/usr/bin/env python3
"""Read-only-to-working-tree parity experiment for officially owned SOL crates.

Compiles baseline vendored adapter and independent temporary copy replacing
the three vendored dependencies with the exact upstream SOL Git revision.
Does not remove vendor from Git history, merge code, or publish binaries.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent
SOL = "https://github.com/HyungseonSong-plasma/simulation-ontology.git"
SOL_SHA = "b6924e6556684eec9dd93677518821cc7adc84b6"
PACKAGES = ["sol-public-contract", "sol-adapter-protocol", "sol-adapter-transport"]


def command(args: list[str], *, cwd: Path, env: dict[str,str] | None = None) -> str:
    completed = subprocess.run(args, cwd=cwd, env=env, text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if completed.returncode:
        raise RuntimeError(
            f"Command failed: {args}: stdout={completed.stdout[-1500:]} stderr={completed.stderr[-3500:]}"
        )
    print(f"[Adapter P3] {' '.join(args[:3])} ... PASS")
    return completed.stdout


def source_copy(location: Path) -> None:
    shutil.copytree(REPO, location, ignore=shutil.ignore_patterns(".git", "target", "__pycache__", "*.pyc"))


def patch_candidate(directory: Path) -> None:
    cargo = directory / "Cargo.toml"
    content = cargo.read_text(encoding="utf-8")
    for name in PACKAGES:
        original = f'{name} = {{ path = "vendor/{name}" }}'
        upstream = f'{name} = {{ git = "{SOL}", rev = "{SOL_SHA}", version = "=0.1.0" }}'
        if content.count(original) != 1:
            raise RuntimeError(f"Unexpected upstream dependency declaration for {name}")
        content = content.replace(original, upstream)
    pattern = r'(\[workspace\]\s*resolver\s*=\s*"2"\s*members\s*=\s*)\[.*?\]'
    content, count = re.subn(pattern, r'\g<1>["."]', content, flags=re.S)
    if count != 1:
        raise RuntimeError("Unsupported Cargo workspace layout")
    cargo.write_text(content, encoding="utf-8")
    # Physically remove the original vendor from the TEST COPY only. The
    # real branch retains all vendor sources until reviewed acceptance.
    shutil.rmtree(directory / "vendor")
    (directory / "Cargo.lock").unlink(missing_ok=True)


def metadata_matches(text: str) -> None:
    obj = json.loads(text)
    packs = {p["name"]: p for p in obj["packages"] if p["name"] in PACKAGES}
    if set(packs) != set(PACKAGES):
        raise RuntimeError(f"Missing SOL dependencies: {set(PACKAGES) - set(packs)}")
    for name, item in packs.items():
        source = item.get("source") or ""
        if not source.startswith(f"git+{SOL}") or f"rev={SOL_SHA}" not in source or not source.endswith("#" + SOL_SHA):
            raise RuntimeError(f"Not a pinned official SOL crate: {name} -> {source}")
        if item["version"] != "0.1.0":
            raise RuntimeError(f"Unexpected {name} version: {item['version']}")
    print("[Adapter P3] All 3 SDK crates: official SOL Git SHA + version verified")


def invoke_adapter(binary: Path, request: dict | str) -> dict:
    payload = request if isinstance(request, str) else json.dumps(request, sort_keys=True)
    env = os.environ.copy()
    env.pop("SOL_MOOSE_EXECUTABLE", None)
    env.pop("SOL_MOOSE_WORKSPACE_ROOT", None)
    result = subprocess.run([str(binary)], input=payload + "\n", text=True,
                            capture_output=True, env=env, check=True, timeout=40)
    if result.stderr:
        raise RuntimeError(f"Unexpected stderr from {binary}: {result.stderr[:500]}")
    lines = result.stdout.strip().splitlines()
    if len(lines) != 1:
        raise RuntimeError(f"Wrong response count from {binary}: {len(lines)}")
    return json.loads(lines[0])


def requests(thermal: dict) -> list[dict | str]:
    def rpc(i: int, method: str, params: dict | None = None) -> dict:
        req = {"jsonrpc": "2.0", "id": i, "method": method}
        if params is not None:
            req["params"] = params
        return req
    return [
        rpc(1, "describe_adapter"),
        "{not-json}",
        rpc(2, "validate_plan", {"adapter_protocol_version": "0.1"}),
        rpc(3, "validate_plan", thermal),
        rpc(4, "execute_plan", thermal),
        rpc(5, "validate_plan", {"adapter_protocol_version": "9.9"}),
    ]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-dir", type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="sol-adapter-upstream-parity-") as root:
        base, candidate = Path(root) / "vendor-baseline", Path(root) / "official-sdk-candidate"
        source_copy(base)
        source_copy(candidate)
        patch_candidate(candidate)
        command(["cargo", "generate-lockfile"], cwd=candidate)
        metadata = command(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=candidate)
        metadata_matches(metadata)
        command(["cargo", "test", "--locked"], cwd=base)
        command(["cargo", "test", "--locked"], cwd=candidate)
        command(["cargo", "build", "--locked", "--bin", "sol-adaptor-moose"], cwd=base)
        command(["cargo", "build", "--locked", "--bin", "sol-adaptor-moose"], cwd=candidate)

        thermal = json.loads((REPO / "tests/fixtures/sol/0.2/thermal-realization-request.json").read_text())
        a, b = base / "target/debug/sol-adaptor-moose", candidate / "target/debug/sol-adaptor-moose"
        cases = requests(thermal)
        observations = []
        for idx, req in enumerate(cases):
            original = invoke_adapter(a, req)
            new = invoke_adapter(b, req)
            if original != new:
                raise RuntimeError(f"Contract parity changed for case {idx}: {original!r} -> {new!r}")
            observations.append({"index": idx, "equal": True, "kind": original.get("result", {}).get("kind") or "transport_error"})
        assert observations[0]["kind"] == "protocol_success", "missing positive describe"
        assert observations[3]["kind"] == "protocol_success", "missing protocol v0.2 preflight"
        assert observations[4]["kind"] == "protocol_success", "missing protocol v0.2 execution"
        assert observations[5]["kind"] == "protocol_failure", "unsupported version was not rejected"
        if args.evidence_dir:
            args.evidence_dir.mkdir(parents=True, exist_ok=True)
            (args.evidence_dir / "resolved-git-metadata.json").write_text(json.dumps(json.loads(metadata), indent=2))
            (args.evidence_dir / "parity-observations.json").write_text(json.dumps(observations, indent=2))
        print(f"[Adapter P3] PASS {len(cases)} exact JSON-RPC response comparisons; no MOOSE backend")
        print("[Adapter P3] PASS vendor baseline + pinned SOL Git compilation and tests")
        print("[Adapter P3] No actual adapter vendor source or SOL meaning was changed")


if __name__ == "__main__":
    main()
