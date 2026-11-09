#!/usr/bin/env python3
"""Reuse actual SOL Runtime -> Adapter parity checks for pinned official MOOSE OCI."""
import argparse
import json
from pathlib import Path

from verify_physics_oci_sol_thermal_parity import evaluate

IMAGE = "idaholab/moose@sha256:47fc6c5bff5e86869e7c467f999e9585ece13af9ffd1f580fef5369728a93452"
PROFILE = "official_moose_oci_thermal_v1"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--external-result", type=Path, required=True)
    parser.add_argument("--external-workspace", type=Path, required=True)
    parser.add_argument("--direct-input", type=Path, required=True)
    parser.add_argument("--direct-receipt", type=Path, required=True)
    parser.add_argument("--request", type=Path, required=True)
    parser.add_argument("--adapter-sha", required=True)
    parser.add_argument("--runtime-sha", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = evaluate(
        args.external_result, args.external_workspace,
        args.direct_input, args.request,
        adapter_sha=args.adapter_sha, runtime_sha=args.runtime_sha,
    )
    receipt = json.loads(args.direct_receipt.read_text(encoding="utf-8"))
    if receipt.get("profile") != PROFILE or receipt.get("image") != IMAGE:
        raise ValueError("direct Adapter profile/image differs from official immutable OCI")
    if receipt.get("status") != "OFFICIAL_MOOSE_OCI_DIRECT_THERMAL_P2_P3_PASS":
        raise ValueError("direct Adapter native Thermal P2/P3 not passed")
    if receipt.get("scientific_acceptance") != "NOT_EVALUATED":
        raise ValueError("unexpected promotion of scientific acceptance")
    result.update({
        "schema": "adapter.official_moose_oci_thermal_sol_parity.v0.1",
        "status": "SOL_MEDIATED_OFFICIAL_MOOSE_OCI_THERMAL_P2_P3_PARITY_PASS",
        "profile": PROFILE,
        "image": IMAGE,
        "mpi_build": "openmpi",
        "moose_source_sha": "167ee97da204df5e8643695e18b9b28910f0014b",
        "conda_moose_release_equivalence": "NOT_EVALUATED",
    })
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(result["status"], "science=NOT_EVALUATED")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
