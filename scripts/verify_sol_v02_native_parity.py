#!/usr/bin/env python3
"""Compare direct Adapter-v02 and SOL-runtime-v02 paths after real MOOSE runs.

This is an operational, same-request parity check. It never establishes Physics
scientific validity, a SOL 0.3 result DTO, or generic HIT API compatibility.
"""
from __future__ import annotations

import argparse
from hashlib import sha256
import json
from pathlib import Path
import re


def file_sha(path: Path) -> str:
    return sha256(path.read_bytes()).hexdigest()


def verify(
    external_result: Path,
    external_root: Path,
    direct_input: Path,
    request_path: Path,
    *,
    adapter_sha: str,
    sol_sha: str,
) -> dict:
    for label, value in (("adapter", adapter_sha), ("SOL", sol_sha)):
        if not re.fullmatch(r"[0-9a-f]{40}", value):
            raise ValueError(f"{label} source revision must be an immutable 40-char SHA")
    request = json.loads(request_path.read_text(encoding="utf-8"))
    if request["target"] != "moose" or request["required_capabilities"] != ["thermal.steady_conduction"]:
        raise ValueError("not the supported thermal consumer envelope")
    if (request["plan_request"]["adapter_protocol_version"] != "0.2"
            or request["plan_request"]["target"]["public_contract_version"] != "0.2"):
        raise ValueError("consumer envelope is not the frozen AP/PC0.2 profile")

    outcome = json.loads(external_result.read_text(encoding="utf-8"))
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
        if outcome.get(key) != value:
            raise ValueError(f"invalid SOL consumer outcome field {key}: {outcome.get(key)!r}")

    # fresh_run_key() is intentionally opaque. Exactly one run is permitted in
    # this isolated directory, and no path value becomes a canonical SOL ID.
    candidates = sorted((external_root / "runs").glob("*/input.i"))
    if len(candidates) != 1:
        raise ValueError(f"expected exactly one external native run, observed {len(candidates)}")
    actual_input = candidates[0]
    run_dir = actual_input.parent
    if actual_input.read_bytes() != direct_input.read_bytes():
        raise ValueError("direct and SOL-mediated Adapter generated different HIT bytes")
    for name in ("check.stdout.log", "check.stderr.log", "stdout.log", "stderr.log"):
        if not (run_dir / name).is_file():
            raise ValueError(f"missing actual MOOSE phase evidence: {name}")
    if "Solve Converged!" not in (run_dir / "stdout.log").read_text(
        encoding="utf-8", errors="replace"
    ):
        raise ValueError("external native MOOSE solver convergence not observed")
    return {
        "schema": "adapter.sol-runtime-thermal-v02-operational-parity.v0.1",
        "status": "SOL_MEDIATED_THERMAL_NATIVE_PARITY_PASS",
        "scope": "SUPPORTED_THERMAL_AP0_2_ONLY",
        "adapter_sha": adapter_sha,
        "sol_runtime_sha": sol_sha,
        "request_sha256": file_sha(request_path),
        "direct_adapter_hit_sha256": file_sha(direct_input),
        "sol_adapter_hit_sha256": file_sha(actual_input),
        "external_native_phases": {
            "selection": "PASS", "validate": "PASS", "check_input": "OBSERVED",
            "execution": "COMPLETED", "no_replay_policy": "VERIFIED"
        },
        "physics_scientific_acceptance": "NOT_EVALUATED",
        "plasma_consumer_cutover": "NOT_AUTHORIZED",
        "sol_0_3_result_producer": "NOT_IMPLEMENTED",
        "source_artifact_note": "Backend input bytes are evidence, not a SOL semantic identifier",
    }


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--external-result", required=True, type=Path)
    p.add_argument("--external-root", required=True, type=Path)
    p.add_argument("--direct-input", required=True, type=Path)
    p.add_argument("--request", required=True, type=Path)
    p.add_argument("--adapter-sha", required=True)
    p.add_argument("--sol-sha", required=True)
    p.add_argument("--output", required=True, type=Path)
    args = p.parse_args()
    result = verify(
        args.external_result, args.external_root, args.direct_input, args.request,
        adapter_sha=args.adapter_sha, sol_sha=args.sol_sha,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(result["status"], "scientific_acceptance=NOT_EVALUATED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
