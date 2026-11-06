#!/usr/bin/env python3
"""Linux CI check for a candidate Adapter binary archive, without MOOSE."""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent.parent
PACKAGER = ROOT / "scripts/adapter_binary_candidate.py"
BINARY = ROOT / "target/release/sol-adaptor-moose"


def command(*args: str, accepted: tuple[int, ...] = (0,)) -> subprocess.CompletedProcess:
    run = subprocess.run(args, capture_output=True, text=True)
    if run.returncode not in accepted:
        raise RuntimeError(f"{args}: exit={run.returncode}, stdout={run.stdout}, stderr={run.stderr}")
    return run


def request(binary: Path, obj: dict) -> dict:
    env = dict(os.environ)
    env.pop("SOL_MOOSE_EXECUTABLE", None)
    env.pop("SOL_MOOSE_WORKSPACE_ROOT", None)
    completed = subprocess.run([str(binary)], input=json.dumps(obj) + "\n",
                               capture_output=True, text=True, check=True, env=env)
    assert not completed.stderr, completed.stderr
    return json.loads(completed.stdout)


def main() -> None:
    assert BINARY.is_file(), f"Release binary missing: {BINARY}"
    with tempfile.TemporaryDirectory(prefix="sol-adapter-native-package-ci-") as t:
        root = Path(t)
        output = root / "dist"
        prefix = root / "test-user-prefix"
        user_data = prefix / "data" / "my-thermal-result.txt"
        user_data.parent.mkdir(parents=True)
        user_data.write_text("KEEP - this belongs to the user\n")
        command(sys.executable, str(PACKAGER), "build", "--out", str(output),
                "--binary", str(BINARY), "--source", str(ROOT))
        archive = output / "sol-adapter-moose-candidate.tar.gz"
        checksum = output / "sol-adapter-moose-candidate.sha256"
        command(sys.executable, str(PACKAGER), "verify", "--archive", str(archive),
                "--checksum", str(checksum))
        original = archive.read_bytes()
        # Reproducible bytes on identical input/source/binary.
        command(sys.executable, str(PACKAGER), "build", "--out", str(output),
                "--binary", str(BINARY), "--source", str(ROOT))
        assert archive.read_bytes() == original, "candidate is not deterministic"
        with tarfile.open(archive) as src:
            member = src.extractfile("sol-adapter-moose-candidate/candidate-manifest.json")
            manifest = json.load(member)
        revision = manifest["source_commit"]
        command(sys.executable, str(PACKAGER), "install", "--prefix", str(prefix),
                "--archive", str(archive), "--checksum", str(checksum))
        # Repeat is idempotent.
        command(sys.executable, str(PACKAGER), "install", "--prefix", str(prefix),
                "--archive", str(archive), "--checksum", str(checksum))
        installed = prefix / "bin" / "sol-adaptor-moose"
        assert installed.exists() and installed.is_symlink()
        response = request(installed, {"jsonrpc": "2.0", "method": "describe_adapter", "id": 1})
        obj = response["result"]["payload"]
        assert response["result"]["kind"] == "protocol_success"
        assert obj["bootstrap"]["supported_adapter_protocol_versions"] == ["0.1", "0.2"]
        assert obj["bootstrap"]["supported_public_contract_versions"] == ["0.1", "0.2"]
        assert obj["targets"][0]["capabilities"][0]["capability"] == "thermal.steady_conduction"
        thermal = json.loads((ROOT / "tests/fixtures/sol/0.2/thermal-realization-request.json").read_text())
        fail = request(installed, {"jsonrpc":"2.0","method":"validate_plan","id":2,"params":thermal})
        assert fail["result"]["payload"]["preflight"] == "unavailable"
        assert fail["result"]["payload"]["diagnostics"][0]["code"] == "adapter.transient_unavailable"
        invalid = command(sys.executable, str(PACKAGER), "install", "--prefix", str(root / "wrong-host"),
                          "--expect-target", "linux-bogus-arch",
                          "--archive", str(archive), "--checksum", str(checksum), accepted=(3,))
        assert "platform mismatch" in invalid.stderr
        corrupt = root / "modified.tar.gz"
        data = bytearray(original)
        data[-1] ^= 1
        corrupt.write_bytes(data)
        bad = command(sys.executable, str(PACKAGER), "verify",
                      "--archive", str(corrupt), "--checksum", str(checksum), accepted=(3,))
        assert "archive SHA256 mismatch" in bad.stderr
        # Must never allow a non-package-owned active executable to be replaced.
        other = root / "user-bin"
        blocked = other / "bin" / "sol-adaptor-moose"
        blocked.parent.mkdir(parents=True)
        blocked.write_text("UNRELATED PROGRAM")
        not_owned = command(sys.executable, str(PACKAGER), "install", "--prefix", str(other),
                            "--archive", str(archive), "--checksum", str(checksum), accepted=(3,))
        assert "non-owned" in not_owned.stderr
        assert blocked.read_text() == "UNRELATED PROGRAM"
        command(sys.executable, str(PACKAGER), "uninstall", "--prefix", str(prefix),
                "--revision", revision)
        assert not installed.exists() and not installed.is_symlink()
        assert user_data.read_text() == "KEEP - this belongs to the user\n"
        assert not (prefix / "lib" / "sol-adapter-moose" / "versions" / revision).exists()
        # Fresh reinstall after uninstall.
        command(sys.executable, str(PACKAGER), "install", "--prefix", str(prefix),
                "--archive", str(archive), "--checksum", str(checksum))
        assert installed.exists()
        command(sys.executable, str(PACKAGER), "uninstall", "--prefix", str(prefix),
                "--revision", revision)
        assert user_data.is_file()
        evidence_dir = os.getenv("SOL_PACKAGE_CANDIDATE_EVIDENCE_DIR")
        if evidence_dir:
            dest = Path(evidence_dir)
            dest.mkdir(parents=True, exist_ok=True)
            shutil.copy2(archive, dest / archive.name)
            shutil.copy2(checksum, dest / checksum.name)
            (dest / "smoke.json").write_text(json.dumps({
                "status":"CANDIDATE_CI_PASS_NOT_HUMAN_LOCAL",
                "source_revision": revision,
                "target":manifest["target_host"],
                "binary_sha256":manifest["binary_sha256"],
                "checks":["deterministic archive","manifest hash","repeat install","describe",
                          "unsupported absent MOOSE","wrong target","tampered archive",
                          "unowned file preserved","uninstall","reinstall"],
            }, indent=2))
        print("[Adapter P4] PASS native candidate: archive, install, describe, refuse, uninstall, data preservation")


if __name__ == "__main__":
    main()
