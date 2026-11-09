#!/usr/bin/env python3
"""Fail-closed, bounded provisioning of the exact Phase-1 MOOSE Conda package.

This script does not validate numerical results or supply a mirror.  It captures
package-channel failures separately from solver failures and retries only
transient transport faults.  Package identity and run evidence are checked by
the existing subsequent CI steps.
"""
from __future__ import annotations

import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

CHANNEL = "https://conda.software.inl.gov/public"
PACKAGE = "moose=2026.08.12=mpich"
PACKAGE_ARCHIVE_SHA256 = (
    "d7b2eec6b958f11455c903b70a16ea319b7edb68b4499537313c06785abd05b3"
)
TRANSIENT_HTTP = re.compile(r"\b(?:HTTP |HTTPError: HTTP )(?:500|502|503|504)\b")
TRANSIENT_NET = re.compile(
    r"(?:ReadTimeoutError|ConnectTimeoutError|ConnectionResetError|"
    r"Temporary failure in name resolution|Connection refused)"
)
NONRETRYABLE = re.compile(
    r"(?:PackagesNotFoundError|UnsatisfiableError|ChecksumMismatch|"
    r"CondaVerificationError|CondaMultiError.*checksum|InvalidArchiveError)"
)


def classify(exit_code: int, output: str) -> str:
    if exit_code == 0:
        return "DEPENDENCIES_INSTALLED_IDENTITY_UNVERIFIED"
    if NONRETRYABLE.search(output):
        return "DEPENDENCY_OR_INTEGRITY_FAILURE"
    if TRANSIENT_HTTP.search(output):
        return "TRANSIENT_UPSTREAM_HTTP_5XX"
    if TRANSIENT_NET.search(output):
        return "TRANSIENT_UPSTREAM_TRANSPORT"
    return "PROVISIONING_FAILURE_UNCLASSIFIED"


def provision(
    conda: Path,
    prefix: Path,
    receipt: Path,
    *,
    attempts: int = 2,
    delay_seconds: float = 4.0,
    runner=subprocess.run,
    sleeper=time.sleep,
) -> int:
    if not (1 <= attempts <= 2):
        raise ValueError("max attempts must be 1 or 2")
    if not (0 <= delay_seconds <= 15):
        raise ValueError("delay outside bounded range")
    receipt.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(conda), "create", "-y", "-p", str(prefix), "--override-channels",
        "-c", CHANNEL, "-c", "conda-forge", PACKAGE,
    ]
    records: list[dict] = []
    status = "INFRA_BLOCKED"
    for attempt in range(1, attempts + 1):
        try:
            proc = runner(cmd, capture_output=True, text=True, timeout=240, check=False)
            rc = proc.returncode
            output = proc.stdout + "\n" + proc.stderr
        except subprocess.TimeoutExpired as exc:
            rc = 124
            output = f"ReadTimeoutError: conda create exceeded timeout: {exc}"
        except OSError as exc:
            rc = 127
            output = f"Conda launcher failed: {type(exc).__name__}: {exc}"

        failure = classify(rc, output)
        log_name = f"conda-attempt-{attempt}.log"
        log_path = receipt.parent / log_name
        log_path.write_text(output, encoding="utf-8")
        records.append({
            "attempt": attempt,
            "exit_code": rc,
            "classification": failure,
            "log_file": log_name,
            "log_sha256": sha256(output.encode("utf-8")).hexdigest(),
        })
        if rc == 0:
            status = "PROVISIONING_COMPLETED_IDENTITY_NOT_YET_ATTESTED"
            break
        if failure not in {
            "TRANSIENT_UPSTREAM_HTTP_5XX", "TRANSIENT_UPSTREAM_TRANSPORT"
        }:
            status = "NONTRANSIENT_PROVISIONING_FAILURE"
            break
        if prefix.exists():
            status = "PARTIAL_PREFIX_RETRY_UNSAFE"
            break
        if attempt < attempts:
            sleeper(delay_seconds)
        else:
            status = "INFRA_BLOCKED_TRANSIENT_EXHAUSTED"
    evidence = {
        "schema": "adapter.moose-pinned-provisioning.v0.1",
        "status": status,
        "native_backend_result": "NOT_EVALUATED",
        "science_acceptance": "NOT_EVALUATED",
        "package": PACKAGE,
        "channel": CHANNEL,
        "top_level_package_expected_archive_sha256": PACKAGE_ARCHIVE_SHA256,
        "dependency_closure_sha256": "NOT_ESTABLISHED",
        "backend_identity": "NOT_VERIFIED_BY_THIS_SCRIPT",
        "max_attempts": attempts,
        "attempts": records,
    }
    receipt.write_text(json.dumps(evidence, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(
        f"PINNED_MOOSE_PROVISIONING={status} attempts={len(records)} "
        f"classification={records[-1]['classification']}",
        flush=True,
    )
    return 0 if status == "PROVISIONING_COMPLETED_IDENTITY_NOT_YET_ATTESTED" else 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--conda", required=True, type=Path)
    ap.add_argument("--prefix", required=True, type=Path)
    ap.add_argument("--receipt", required=True, type=Path)
    ap.add_argument("--attempts", type=int, default=2)
    args = ap.parse_args()
    if args.prefix.exists():
        ap.error("target prefix already exists: refusing to modify it")
    return provision(args.conda, args.prefix, args.receipt, attempts=args.attempts)


if __name__ == "__main__":
    sys.exit(main())
