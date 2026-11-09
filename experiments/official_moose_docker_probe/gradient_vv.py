#!/usr/bin/env python3
"""Experimental analytic 1D Thermal gradient checks on pinned official OCI.

This is a backend-only numerical oracle, not SOL/Physics scientific acceptance.
"""
import csv
import hashlib
import json
from pathlib import Path
import subprocess
import sys

IMAGE = "idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452"
ROOT = Path("target/official-moose-docker-candidate/gradient").resolve()
TOLERANCE_K = 1e-6


def input_text(nx, right):
    return f"""[Mesh]
  type = GeneratedMesh
  dim = 1
  nx = {nx}
  xmin = 0
  xmax = 1
[]
[Variables]
  [T]
    order = FIRST
    family = LAGRANGE
  []
[]
[Kernels]
  [heat]
    type = HeatConduction
    variable = T
    thermal_conductivity = thermal_conductivity
  []
[]
[Materials]
  [conductivity]
    type = GenericConstantMaterial
    prop_names = 'thermal_conductivity'
    prop_values = '45'
  []
[]
[BCs]
  [left]
    type = DirichletBC
    variable = T
    boundary = left
    value = 400
  []
  [right]
    type = DirichletBC
    variable = T
    boundary = right
    value = {right}
  []
[]
[Executioner]
  type = Steady
[]
[Postprocessors]
  [T25]
    type = PointValue
    variable = T
    point = '0.25 0 0'
  []
  [T75]
    type = PointValue
    variable = T
    point = '0.75 0 0'
  []
[]
[Outputs]
  exodus = false
  csv = true
[]
"""


def run(case, nx, right):
    folder = ROOT / case
    folder.mkdir(parents=True)
    script = folder / "input.i"
    script.write_text(input_text(nx, right), encoding="utf-8")
    digest = hashlib.sha256(script.read_bytes()).hexdigest()
    shared = [
        "docker", "run", "--rm", "--pull", "never", "--network", "none",
        "--volume", f"{folder}:/work:rw", "--workdir", "/work",
        "--entrypoint", "/bin/bash", IMAGE, "-lc",
        'set -euo pipefail; if [[ -f /environment ]]; then source /environment; fi; exec moose-opt "$@"',
        "bash",
    ]
    for stage, args in [("check", ["-i", "input.i", "--check-input"]), ("solve", ["-i", "input.i"])]:
        p = subprocess.run(shared + args, capture_output=True, timeout=120, check=False)
        (folder / f"{stage}.stdout.log").write_bytes(p.stdout)
        (folder / f"{stage}.stderr.log").write_bytes(p.stderr)
        if p.returncode:
            raise RuntimeError(f"{case} {stage} failed rc={p.returncode}")
    csvs = list(folder.glob("*_out.csv"))
    if len(csvs) != 1:
        raise RuntimeError(f"{case} expected exactly one output CSV, got {len(csvs)}")
    with csvs[0].open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    if not rows:
        raise RuntimeError(f"{case} solver CSV missing data")
    measured = {name: float(rows[-1][name]) for name in ("T25", "T75")}
    expected = {name: 400 + (right - 400) * x for name, x in (("T25", 0.25), ("T75", 0.75))}
    error = max(abs(measured[name] - expected[name]) for name in measured)
    if error > TOLERANCE_K:
        raise RuntimeError(f"{case}: numerical oracle failure err={error} K")
    return {
        "case": case,
        "mesh_nx": nx,
        "left_K": 400,
        "right_K": right,
        "input_sha256": digest,
        "observed_K": measured,
        "analytic_K": expected,
        "maximum_absolute_error_K": error,
        "tolerance_K": TOLERANCE_K,
    }


def main():
    ROOT.mkdir(parents=True, exist_ok=True)
    pinned = Path("target/official-moose-docker-candidate/resolved-image.txt").read_text().strip()
    if pinned != IMAGE:
        raise RuntimeError("OCI registry digest mismatch")
    report = {"source_image": IMAGE, "classification": "EXPERIMENTAL_ANALYTIC_GRADIENT_NOT_SCIENTIFIC_ACCEPTANCE"}
    try:
        report["checks"] = [run(f"gradient-n{nx}", nx, 300) for nx in (4, 8, 16)]
        report["checks"].append(run("gradient-negative-n8", 8, 299))
        report["result"] = "PASS"
        success = True
    except Exception as error:
        report["result"] = "FAIL"
        report["error"] = str(error)
        success = False
    (ROOT / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print("OCI_GRADIENT_VV", report["result"], flush=True)
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
