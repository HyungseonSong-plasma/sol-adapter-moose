#!/usr/bin/env python3
"""Bounded, read-only Physics OCI reuse probe for Adapter #34.

Runs exact published Physics images; does not modify MOOSE, SOL, the Adapter
backend, the physics runtime, the frozen Conda target, or release configuration.
Observations are not equivalent to an exact Conda package identity or to V&V.
"""
from __future__ import annotations

import argparse
from hashlib import sha256
import json
from pathlib import Path
import shutil
import subprocess
import sys

RUNTIME = (
    "ghcr.io/hyungseonsong-plasma/physics-runtime@sha256:"
    "cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c"
)
BASE = (
    "ghcr.io/hyungseonsong-plasma/physics-build-base@sha256:"
    "2c3352da4f5c1b2ae98475fc01463ad0cdbc6ce7d221ce55055ddea152182008"
)
MOOSE_SHA = "9f388366ccf38b9c34542ec5561198249fde0ac9"
INPUT_SHA = "09e6690f11dc240792b1b922f1f5aba57aa38dea6bad10f4676c06b58dd0b3cd"
FIXTURE = Path("experiments/physics_oci_reuse_probe/thermal_phase4.i")


def sha(data: bytes) -> str:
    return sha256(data).hexdigest()


def call(args: list[str], name: str, root: Path, timeout: int = 120) -> dict:
    try:
        result = subprocess.run(
            args, capture_output=True, timeout=timeout, check=False
        )
        stdout, stderr = result.stdout, result.stderr
        rc = result.returncode
        timed_out = False
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b"", error.stderr or b""
        rc = 124
        timed_out = True
    except OSError as error:
        stdout, stderr = b"", str(error).encode()
        rc = 127
        timed_out = False
    if isinstance(stdout, str):
        stdout = stdout.encode()
    if isinstance(stderr, str):
        stderr = stderr.encode()
    (root / f"{name}.stdout.log").write_bytes(stdout)
    (root / f"{name}.stderr.log").write_bytes(stderr)
    return {
        "rc": rc,
        "timed_out": timed_out,
        "stdout_sha256": sha(stdout),
        "stderr_sha256": sha(stderr),
        "stdout_log": f"{name}.stdout.log",
        "stderr_log": f"{name}.stderr.log",
    }


def inspect(ref: str, root: Path) -> dict:
    record = call(["docker", "image", "inspect", ref], "image-inspect", root)
    if record["rc"] != 0:
        raise RuntimeError("Docker image inspect failed")
    body = json.loads((root / record["stdout_log"]).read_text())[0]
    labels = body.get("Config", {}).get("Labels") or {}
    if labels.get("io.physics.moose.sha") != MOOSE_SHA:
        raise RuntimeError("pinned MOOSE SHA label mismatch")
    if ref == RUNTIME and labels.get("io.physics.build-base.ref") != BASE:
        raise RuntimeError("runtime image's build-base provenance mismatch")
    if ref == BASE and labels.get("io.physics.moose-dev.tag") != "2026.06.16":
        raise RuntimeError("unexpected physics build-base MOOSE developer image tag")
    return {
        "image_id": body["Id"],
        "repo_digests": body.get("RepoDigests", []),
        "moose_sha": labels["io.physics.moose.sha"],
        "build_base_ref": labels.get("io.physics.build-base.ref"),
        "moose_dev_tag": labels.get("io.physics.moose-dev.tag"),
        "image_provenance": "PINNED_PHYSICS_IMAGE_NOT_CONDA_CLOSURE",
    }


def run_runtime(root: Path, report: dict) -> None:
    bytes_ = FIXTURE.read_bytes()
    if sha(bytes_) != INPUT_SHA:
        raise RuntimeError("Adapter historical Phase4 thermal input source SHA mismatch")
    case = root / "thermal-case"
    case.mkdir()
    shutil.copyfile(FIXTURE, case / "input.i")
    report["fixture_sha256"] = sha(bytes_)
    report["original_artifact"] = (
        "adapter run 42200001106 artifact 13493163467 "
        "phase4-execution-evidence/runs/thermal-realization-001/input.i"
    )
    report["baseline_smoke_p2"] = call(
        ["docker", "run", "--rm", "--network", "none", RUNTIME,
         "--check-input", "-i", "/opt/physics/ci/smoke.i"],
        "physics-smoke-p2", root,
    )
    if report["baseline_smoke_p2"]["rc"] != 0:
        report["finding"] = "PHYSICS_IMAGE_BASELINE_SMOKE_FAILED"
        return
    report["show_copyable_inputs"] = call(
        ["docker", "run", "--rm", "--network", "none", RUNTIME,
         "--show-copyable-inputs"], "physics-copyable", root,
    )
    mount = f"{case.resolve()}:/work"
    args = [
        "docker", "run", "--rm", "--network", "none", "-v", mount,
        "-w", "/work", RUNTIME,
    ]
    report["thermal_p2"] = call(
        args + ["--check-input", "-i", "input.i"], "thermal-p2", root,
    )
    if report["thermal_p2"]["rc"] != 0:
        report["finding"] = "PHYSICS_RUNTIME_NOT_PROVEN_COMPATIBLE_WITH_THERMAL"
        report["thermal_p3"] = "NOT_RUN_AFTER_P2_FAILURE"
        return
    report["thermal_p3"] = call(
        args + ["-i", "input.i"], "thermal-p3", root, timeout=180,
    )
    report["finding"] = (
        "PHYSICS_RUNTIME_THERMAL_P2_P3_PASS_DIFFERENT_IDENTITY"
        if report["thermal_p3"]["rc"] == 0
        else "PHYSICS_RUNTIME_THERMAL_P3_FAILURE"
    )


def run_base(root: Path, report: dict) -> None:
    # Inspection only. The build base does not advertise an unambiguous
    # replacement for moose-opt; never use physics-opt as a transparent stand-in.
    command = """
set -euo pipefail
source /environment
printf 'COMPILER='; command -v c++ || true
printf 'MOOSE_SOURCE='; test -d /opt/physics_vendor/moose/framework && echo PRESENT || echo ABSENT
printf 'HEAT_TRANSFER_SOURCE='; test -d /opt/physics_vendor/moose/modules/heat_transfer && echo PRESENT || echo ABSENT
printf 'HEAT_TRANSFER_LIBRARY='; find /opt/physics_vendor/moose/modules/heat_transfer -maxdepth 3 -name '*-opt.la' -print -quit 2>/dev/null || true
printf 'AVAILABLE_OPT_BINARIES:\\n'
find /opt/physics_vendor/moose -maxdepth 5 -type f -name '*-opt' -print 2>/dev/null | head -40 || true
printf 'PHYSICS_EXE='; test -x /opt/physics/physics-opt && echo PRESENT || echo ABSENT
"""
    report["build_base_introspection"] = call(
        ["docker", "run", "--rm", "--network", "none",
         "--entrypoint", "/bin/bash", BASE, "-lc", command],
        "buildbase-introspection", root,
    )
    if report["build_base_introspection"]["rc"] != 0:
        report["finding"] = "BUILD_BASE_INTROSPECTION_FAILED"
    else:
        report["finding"] = (
            "BUILD_BASE_AVAILABLE_REBUILD_OR_COMPATIBILITY_PENDING"
        )
    report["thermal_p2"] = "NOT_EVALUATED_ON_BUILD_BASE"
    report["thermal_p3"] = "NOT_EVALUATED_ON_BUILD_BASE"


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--image", choices=("runtime", "base"), required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    root = args.output.resolve()
    root.mkdir(parents=True, exist_ok=True)
    image = RUNTIME if args.image == "runtime" else BASE
    report = {
        "schema": "adapter.physics-existing-oci-reuse-probe.v0.1",
        "image": image,
        "mode": args.image,
        "physics_scientific_acceptance": "NOT_EVALUATED",
        "adapter_conda_identity_parity": "NOT_EVALUATED",
        "adapter_sol_runtime_parity": "NOT_EVALUATED",
        "adapted_backend": "NONE_RESEARCH_ONLY",
    }
    success = False
    try:
        report["image_pull"] = call(
            ["docker", "pull", image], "image-pull", root, timeout=600
        )
        if report["image_pull"]["rc"] != 0:
            raise RuntimeError("immutable Physics image pull failed")
        report["image_identity"] = inspect(image, root)
        if args.image == "runtime":
            run_runtime(root, report)
        else:
            run_base(root, report)
        success = True
    except Exception as error:
        report["error"] = repr(error)
        report["finding"] = "DIAGNOSTIC_INFRA_OR_SOURCE_FAILED"
    report["observation_collection"] = "PASS" if success else "FAIL"
    (root / "report.json").write_text(
        json.dumps(report, sort_keys=True, indent=2) + "\n", encoding="utf-8"
    )
    print(
        "PHYSICS_OCI_REUSE_PROBE",
        args.image,
        report["observation_collection"],
        report["finding"],
        flush=True,
    )
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
