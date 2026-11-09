#!/usr/bin/env python3
"""Exact-source operational parity: direct Adapter vs pinned SOL Runtime over Physics OCI.

Only the existing thermal.steady_conduction AP/PC 0.2 contract is assessed.
No scientific or Conda release equivalence is implied.
"""
from __future__ import annotations

import argparse
from hashlib import sha256
import json
from pathlib import Path
import re

IMAGE = (
    "ghcr.io/hyungseonsong-plasma/physics-runtime@sha256:"
    "cda9f0d09a17aed5fa606e97d1ee5c3450f7280536cec2e5c635402bb07d972c"
)


def read_sha(path: Path) -> str:
    return sha256(path.read_bytes()).hexdigest()


def evaluate(
    external_result: Path,
    external_workspace: Path,
    direct_input: Path,
    request: Path,
    *,
    adapter_sha: str,
    runtime_sha: str,
) -> dict:
    for label, value in (("adapter", adapter_sha), ("SOL Runtime", runtime_sha)):
        if not re.fullmatch(r"[0-9a-f]{40}", value):
            raise ValueError(f"un-pinned {label} source SHA")
    request_value = json.loads(request.read_text())
    if request_value["target"] != "moose":
        raise ValueError("unsupported backend target")
    if request_value["required_capabilities"] != ["thermal.steady_conduction"]:
        raise ValueError("unsupported capability; no plasma promotion")
    plan = request_value["plan_request"]
    if plan["adapter_protocol_version"] != "0.2" or plan["target"]["public_contract_version"] != "0.2":
        raise ValueError("unexpected SOL/Adapter public protocol version")
    response = json.loads(external_result.read_text())
    expected = {
        "status": "completed",
        "adapter_id": "sol.adapter.moose",
        "adapter_protocol_version": "0.2",
        "public_contract_version": "0.2",
        "target": "moose",
        "required_capabilities": ["thermal.steady_conduction"],
        "selection": "selected",
        "validate": "accepted",
        "execute": "completed",
        "execute_response_loss_replay": "forbidden",
        "shutdown": "success",
    }
    for key, value in expected.items():
        if response.get(key) != value:
            raise ValueError(f"external consumer contract mismatch in {key}: {response.get(key)!r}")
    cases = sorted((external_workspace / "runs").glob("*/input.i"))
    if len(cases) != 1:
        raise ValueError(f"expected exactly one external OCI run, found {len(cases)}")
    external_input = cases[0]
    if direct_input.read_bytes() != external_input.read_bytes():
        raise ValueError("SOL path produced different HIT bytes from direct Adapter path")
    for dirname in [external_input.parent]:
        for filename in ("check.stdout.log", "check.stderr.log", "stdout.log", "stderr.log"):
            if not (dirname / filename).is_file():
                raise ValueError(f"missing real OCI runtime evidence: {filename}")
    if "Syntax OK" not in (external_input.parent / "check.stdout.log").read_text():
        raise ValueError("external P2 was not observed")
    if "Solve Converged!" not in (external_input.parent / "stdout.log").read_text():
        raise ValueError("external P3 was not observed")
    return {
        "schema": "adapter.physics_oci_thermal_sol_parity.v0.1",
        "status": "SOL_MEDIATED_PHYSICS_OCI_THERMAL_P2_P3_PARITY_PASS",
        "profile": "physics_oci_thermal_v1",
        "image": IMAGE,
        "adapter_sha": adapter_sha,
        "sol_runtime_sha": runtime_sha,
        "request_sha256": read_sha(request),
        "direct_input_sha256": read_sha(direct_input),
        "external_input_sha256": read_sha(external_input),
        "evidence_scope": "exact AP/PC 0.2 thermal only",
        "scientific_acceptance": "NOT_EVALUATED",
        "conda_moose_release_equivalence": "NOT_EVALUATED",
        "plasma_capability": "NOT_ADVERTISED",
    }


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--external-result", type=Path, required=True)
    p.add_argument("--external-workspace", type=Path, required=True)
    p.add_argument("--direct-input", type=Path, required=True)
    p.add_argument("--request", type=Path, required=True)
    p.add_argument("--adapter-sha", required=True)
    p.add_argument("--runtime-sha", required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    evidence = evaluate(
        args.external_result, args.external_workspace, args.direct_input,
        args.request, adapter_sha=args.adapter_sha, runtime_sha=args.runtime_sha
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
    print(evidence["status"], "science=NOT_EVALUATED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
