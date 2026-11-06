#!/usr/bin/env python3
"""Local acceptance runner for unreleased SOL Adapter 0.2 package candidates.

Runs only on the user's machine; CI use is an optional preliminary smoke.
Requires Python3 and the package_adapter_candidate.py from the same source.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

from package_adapter_candidate import (
    BINARY, MANIFEST, install, uninstall, verify,
)

SOL_SHA = "b6924e6556684eec9dd93677518821cc7adc84b6"


def log_command(cmd: list[str], env: dict[str, str], output: Path, timeout=600) -> subprocess.CompletedProcess:
    result = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=timeout)
    output.write_text(
        "$ " + " ".join(cmd) + "\n"
        + f"exit={result.returncode}\n"
        + "-- stdout --\n" + result.stdout + "\n"
        + "-- stderr --\n" + result.stderr + "\n"
    )
    return result


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--archive", type=Path, required=True)
    ap.add_argument("--sol-checkout", type=Path, required=True)
    ap.add_argument("--evidence-dir", type=Path, required=True)
    ap.add_argument("--moose-executable", type=Path)
    ap.add_argument("--keep-installed", action="store_true",
                    help="Preserve only the isolated installed binary; never change user PATH")
    args = ap.parse_args()

    archive = args.archive.resolve()
    sol = args.sol_checkout.resolve()
    evidence = args.evidence_dir.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    prefix = evidence / "isolated-install"
    binary = None

    manifest, _ = verify(archive)
    (evidence / "adapter-package-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    checksums = archive.with_name(archive.name + ".sha256")
    if not checksums.is_file():
        raise ValueError("missing required candidate archive checksum sidecar")
    (evidence / "adapter-archive-checksum.txt").write_text(checksums.read_text())

    git = log_command(["git", "-C", str(sol), "rev-parse", "HEAD"], os.environ.copy(), evidence / "sol-revision.log")
    if git.returncode != 0 or git.stdout.strip() != SOL_SHA:
        raise ValueError(f"the SOL Runtime checkout must be exactly {SOL_SHA}, not a floating branch")
    fixture = sol / "fixtures/adapter-protocol/0.2/thermal-realization-request.json"
    example = sol / "examples/external-runtime-v02/Cargo.toml"
    if not fixture.is_file() or not example.is_file():
        raise ValueError("SOL checkout missing pinned Runtime 0.2 fixture or Cargo manifest")

    env = os.environ.copy()
    (evidence / "local-environment.json").write_text(json.dumps({
        "host": os.uname().sysname if hasattr(os, "uname") else sys.platform,
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "cargo": subprocess.run(["cargo", "--version"], capture_output=True, text=True).stdout.strip(),
        "adapter_source": manifest["adapter_source_commit"],
        "sol_runtime_source": SOL_SHA,
        "mode": "human_workstation_not_github_ci",
    }, indent=2) + "\n")
    try:
        directory = install(archive, prefix)
        binary = directory / BINARY
        cmd = [str(binary)]
        result = log_command(cmd, env | {},
                             evidence / "describe.log",
                             timeout=20) if False else None
        # Send one newline-delimited JSON-RPC request and inspect protocol payload.
        req = {"jsonrpc": "2.0", "id": 1, "method": "describe_adapter"}
        response = subprocess.run(cmd, input=json.dumps(req) + "\n",
                                  env=env, capture_output=True, text=True, timeout=20)
        (evidence / "describe.log").write_text(
            f"exit={response.returncode}\nstdout={response.stdout}\nstderr={response.stderr}\n"
        )
        if response.returncode:
            raise ValueError("installed Adapter describe_adapter failed; see describe.log")
        data = json.loads(response.stdout.strip())
        assert data["result"]["kind"] == "protocol_success", data
        boot = data["result"]["payload"]["bootstrap"]
        assert boot["adapter_id"] == "sol.adapter.moose", boot
        assert boot["supported_adapter_protocol_versions"] == ["0.1", "0.2"], boot
        assert boot["supported_public_contract_versions"] == ["0.1", "0.2"], boot

        req["id"] = 2
        req["method"] = "validate_plan"
        req["params"] = {"adapter_protocol_version": "0.3"}
        response = subprocess.run(cmd, input=json.dumps(req) + "\n",
                                  env=env, capture_output=True, text=True, timeout=20)
        (evidence / "wrong-protocol.log").write_text(
            f"exit={response.returncode}\nstdout={response.stdout}\nstderr={response.stderr}\n"
        )
        if response.returncode != 0 or json.loads(response.stdout.strip())["result"]["kind"] != "protocol_failure":
            raise ValueError("Adapter incorrectly accepted or failed to reject Protocol 0.3")

        status = {
            "installed_adapter_describe": "PASS",
            "unsupported_protocol_03": "PASS",
            "contract_git_sha": SOL_SHA,
            "moose_backend": "NOT_RUN",
            "runtime_02": "NOT_RUN",
            "adapter_removed": "PENDING",
        }

        if args.moose_executable:
            app = args.moose_executable.resolve()
            if not app.is_file() or not os.access(app, os.X_OK):
                raise ValueError(f"MOOSE app not executable: {app}")
            env["SOL_MOOSE_EXECUTABLE"] = str(app)
            workspace = evidence / "moose-workspace"
            workspace.mkdir(exist_ok=True)
            env["SOL_MOOSE_WORKSPACE_ROOT"] = str(workspace)
            payload = json.loads(fixture.read_text())
            request = {
                "target": "moose",
                "required_capabilities": ["thermal.steady_conduction"],
                "plan_request": payload,
            }
            wrapper = evidence / "thermal-consumer-request.json"
            wrapper.write_text(json.dumps(request, indent=2) + "\n")
            result = log_command(
                ["cargo", "run", "--manifest-path", str(example),
                 "--locked", "--", "run", str(binary), str(wrapper)],
                env, evidence / "thermal-runtime-02.log", timeout=600
            )
            if result.returncode:
                status["runtime_02"] = "FAIL"
                (evidence / "local-acceptance.json").write_text(json.dumps(status, indent=2) + "\n")
                raise ValueError("Runtime 0.2 / actual MOOSE failed; see thermal-runtime-02.log")
            run = json.loads(result.stdout)
            if not (run["status"] == "completed" and run["adapter_protocol_version"] == "0.2"
                    and run["public_contract_version"] == "0.2"
                    and run["execute_response_loss_replay"] == "forbidden"):
                raise ValueError(f"unexpected actual MOOSE runtime result: {run}")
            status["runtime_02"] = "PASS"
            status["moose_backend"] = "SOLVER_EXECUTION_COMPLETED_NOT_INDEPENDENT_SCIENTIFIC_SIGNOFF"
            (evidence / "thermal-runtime-02-result.json").write_text(
                json.dumps(run, indent=2) + "\n"
            )
        if not args.keep_installed:
            uninstall(prefix, manifest["adapter_package_version"],
                      manifest["adapter_source_commit"])
            status["adapter_removed"] = "PASS"
        else:
            status["adapter_removed"] = "SKIPPED_KEEP_INSTALLED"
        (evidence / "local-acceptance.json").write_text(json.dumps(status, indent=2) + "\n")
        print(f"LOCAL_ACCEPTANCE_TECHNICAL_PASS: {evidence}")
        if not args.moose_executable:
            print("NOTE: real MOOSE execution was NOT_RUN; provide --moose-executable for E2E.")
    finally:
        # Do not silently delete a failed-run binary; evidence/isolated-install
        # survives error for inspection and explicit cleanup by the user.
        pass


if __name__ == "__main__":
    try:
        main()
    except (ValueError, AssertionError, OSError, subprocess.TimeoutExpired, KeyError, json.JSONDecodeError) as exc:
        print(f"LOCAL_ACCEPTANCE_BLOCKED: {exc}", file=sys.stderr)
        sys.exit(1)
