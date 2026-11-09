#!/usr/bin/env python3
"""Official idaholab/moose OCI candidate: digest discovery, native thermal probe.

Non-accepting experiment. Never asserts equivalence to the frozen
moose=2026.08.12=mpich Conda closure, SOL Contract, or Physics science V&V.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys

SOURCE = Path(__file__).with_name("thermal_phase4.i")
SOURCE_SHA256 = "09e6690f11dc240792b1b922f1f5aba57aa38dea6bad10f4676c06b58dd0b3cd"
IMAGE_TAG = "idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452"
DIGEST_PATTERN = re.compile(r"idaholab/moose@sha256:[0-9a-f]{64}")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Probe:
    def __init__(self, root: Path):
        self.root = root.resolve()
        self.root.mkdir(parents=True, exist_ok=True)
        self.report = {
            "schema": "adapter.official-moose-oci-candidate.v0.2",
            "classification": "NON_ACCEPTING_ALTERNATIVE_CANDIDATE",
            "requested_immutable_image": IMAGE_TAG,
            "frozen_conda_target": "moose=2026.08.12=mpich (linux-64)",
            "conda_identity_parity": "NOT_EVALUATED",
            "sol_protocol_parity": "NOT_EVALUATED",
            "physics_scientific_acceptance": "NOT_EVALUATED",
            "commands": {},
        }

    def command(self, name: str, args: list[str], timeout: int = 180) -> bytes:
        try:
            proc = subprocess.run(args, capture_output=True, timeout=timeout, check=False)
            out, err, status = proc.stdout, proc.stderr, proc.returncode
        except subprocess.TimeoutExpired as error:
            out, err, status = error.stdout or b"", error.stderr or b"", 124
        (self.root / (name + ".stdout.log")).write_bytes(out)
        (self.root / (name + ".stderr.log")).write_bytes(err)
        self.report["commands"][name] = {
            "exit_status": status,
            "stdout_sha256": hashlib.sha256(out).hexdigest(),
            "stderr_sha256": hashlib.sha256(err).hexdigest(),
        }
        if status:
            raise RuntimeError(f"{name} failed (exit={status}); see artifact logs")
        return out

    def moose(self, name: str, ref: str, work: Path, args: list[str]) -> None:
        command = ("set -euo pipefail; "
                   "if test -f /environment; then source /environment; fi; "
                   "command -v moose-opt; exec moose-opt " + shlex.join(args))
        self.command(name, [
            "docker", "run", "--rm", "--pull", "never", "--network", "none",
            "--volume", str(work.resolve()) + ":/work:rw",
            "--workdir", "/work", "--entrypoint", "/bin/bash",
            ref, "-lc", command,
        ], timeout=180)

    def measurement(self, work: Path, stem: str, expected: float) -> float:
        candidates = list(work.glob(stem + "_out.csv"))
        if len(candidates) != 1:
            raise RuntimeError(f"{stem}: expected one CSV, found {len(candidates)}")
        with candidates[0].open(encoding="utf-8", newline="") as handle:
            rows = list(csv.DictReader(handle))
        if not rows or "T_mid" not in rows[-1]:
            raise RuntimeError(f"{stem}: missing T_mid measurement")
        actual = float(rows[-1]["T_mid"])
        if abs(actual - expected) > 1e-5:
            raise RuntimeError(f"{stem}: numerical mismatch expected {expected}, got {actual}")
        return actual

    def run(self):
        actual = sha256(SOURCE)
        if actual != SOURCE_SHA256:
            raise RuntimeError("Historical Adapter Phase 4 thermal source SHA256 mismatch")
        self.report["source_input_sha256"] = actual
        self.command("docker-pull", ["docker", "pull", IMAGE_TAG], timeout=900)
        inspection = json.loads(self.command("docker-inspect", [
            "docker", "image", "inspect", IMAGE_TAG,
        ]))[0]
        refs = [v for v in inspection.get("RepoDigests", []) if DIGEST_PATTERN.fullmatch(v)]
        if len(refs) != 1:
            raise RuntimeError("Cannot prove exactly one Docker Hub official image digest")
        if inspection.get("Architecture") != "amd64" or inspection.get("Os") != "linux":
            raise RuntimeError("Unexpected official MOOSE image platform")
        ref = refs[0]
        self.report["resolved_immutable_image"] = ref
        self.report["image_config_id"] = inspection["Id"]
        self.report["image_platform"] = "linux/amd64"
        (self.root / "resolved-image.txt").write_text(ref + "\n", encoding="utf-8")

        work = self.root / "case"
        work.mkdir(exist_ok=True)
        shutil.copyfile(SOURCE, work / "thermal_phase4.i")
        self.moose("native-version", ref, work, ["--version"])
        self.moose("historical-thermal-check", ref, work, ["--check-input", "-i", "thermal_phase4.i"])
        self.moose("historical-thermal-execution", ref, work, ["-i", "thermal_phase4.i"])

        # Only output/probe changes: measured constant-field solution T(x)=400 K.
        original = SOURCE.read_text(encoding="utf-8")
        target = "  exodus = false\n"
        if original.count(target) != 1:
            raise RuntimeError("Source Outputs block changed unexpectedly")
        enhanced = original.replace(target, target + "  csv = true\n")
        enhanced += ("\n[Postprocessors]\n  [T_mid]\n"
                     "    type = PointValue\n    variable = T\n"
                     "    point = '0.375 0 0'\n  []\n[]\n")
        (work / "thermal_measurement.i").write_text(enhanced, encoding="utf-8")
        self.moose("thermal-measurement-check", ref, work, ["--check-input", "-i", "thermal_measurement.i"])
        self.moose("thermal-measurement-execution", ref, work, ["-i", "thermal_measurement.i"])
        self.report["thermal_T_mid_K"] = self.measurement(work, "thermal_measurement", 400.0)

        perturb = "    value = 400\n"
        if enhanced.count(perturb) != 1:
            raise RuntimeError("Thermal boundary condition changed unexpectedly")
        control = enhanced.replace(perturb, "    value = 399\n")
        (work / "thermal_negative.i").write_text(control, encoding="utf-8")
        self.moose("thermal-negative-execution", ref, work, ["-i", "thermal_negative.i"])
        self.report["negative_T_mid_K"] = self.measurement(work, "thermal_negative", 399.0)
        self.report["verified"] = "OFFICIAL_OCI_NATIVE_THERMAL_AND_NUMERIC_CANDIDATE_PASS"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    experiment = Probe(args.output)
    try:
        experiment.run()
        success = True
    except Exception as e:
        experiment.report["verified"] = "NOT_PROVEN"
        experiment.report["failure"] = str(e)
        success = False
    (experiment.root / "report.json").write_text(
        json.dumps(experiment.report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print("OFFICIAL_MOOSE_OCI_CANDIDATE", experiment.report["verified"])
    if "resolved_immutable_image" in experiment.report:
        print("IMAGE_DIGEST", experiment.report["resolved_immutable_image"])
    if "failure" in experiment.report:
        print("FAILURE", experiment.report["failure"], file=sys.stderr)
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main())
