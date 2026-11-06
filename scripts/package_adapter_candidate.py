#!/usr/bin/env python3
"""Create, verify and safely install an *unreleased* sol-adaptor-moose binary candidate.

Standard library only. No MOOSE solver executable or Physics data are bundled.
Distribution is a platform-specific DRAFT candidate, not a supported release.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
import platform
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile

NAME = "sol-adapter-moose-candidate"
BINARY = "sol-adaptor-moose"
MANIFEST = "package-manifest.json"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def host_platform() -> str:
    return f"{platform.system().lower()}-{platform.machine().lower()}"


def git_sha(root: Path) -> str:
    sha = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()
    if len(sha) != 40 or any(c not in "0123456789abcdef" for c in sha):
        raise ValueError("missing exact Git SHA")
    return sha


def package_bytes(binary: bytes, manifest: bytes) -> bytes:
    out = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=out, mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.USTAR_FORMAT) as tar:
            for path, value, mode in (
                (f"{NAME}/{BINARY}", binary, 0o755),
                (f"{NAME}/{MANIFEST}", manifest, 0o644),
            ):
                entry = tarfile.TarInfo(path)
                entry.size = len(value)
                entry.mode = mode
                entry.mtime = 0
                entry.uid = entry.gid = 0
                entry.uname = entry.gname = ""
                tar.addfile(entry, io.BytesIO(value))
    return out.getvalue()


def build(binary: Path, root: Path, output: Path) -> None:
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError(f"missing executable: {binary}")
    sha = git_sha(root)
    data = binary.read_bytes()
    if not data:
        raise ValueError("empty executable")
    manifest = {
        "kind": "SOL_ADAPTER_MOOSE_BINARY_CANDIDATE_NOT_A_RELEASE",
        "schema_version": 1,
        "adapter_binary": BINARY,
        "adapter_package_version": "0.1.0",
        "adapter_source_commit": sha,
        "sol_contract_source_commit": "b6924e6556684eec9dd93677518821cc7adc84b6",
        "supported_adapter_protocol_versions": ["0.1", "0.2"],
        "supported_public_contract_versions": ["0.1", "0.2"],
        "advertised_capabilities": ["thermal.steady_conduction"],
        "target_host": host_platform(),
        "binary_sha256": digest(data),
        "binary_bytes": len(data),
        "notes": [
            "Not an official release, not signed or ABI-universal.",
            "External MOOSE application executable is NOT bundled.",
            "Installation and protocol conformance are not scientific V&V.",
        ],
    }
    mbytes = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode()
    output.mkdir(parents=True, exist_ok=True)
    archive = output / f"{NAME}-{host_platform()}.tar.gz"
    archive.write_bytes(package_bytes(data, mbytes))
    (output / (archive.name + ".sha256")).write_text(
        f"{digest(archive.read_bytes())}  {archive.name}\n"
    )
    verify(archive)
    print(f"[ADAPTER P4] Built candidate: {archive}")


def verify(archive: Path) -> tuple[dict, bytes]:
    raw = archive.read_bytes()
    checksum = archive.with_name(archive.name + ".sha256")
    if checksum.exists():
        expected = checksum.read_text().strip().split()
        if (len(expected) != 2 or expected[1] != archive.name
                or expected[0] != digest(raw)):
            raise ValueError("release-candidate archive checksum mismatch")
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as tar:
        members = tar.getmembers()
        names = {x.name for x in members}
        required = {f"{NAME}/{BINARY}", f"{NAME}/{MANIFEST}"}
        if names != required or len(members) != 2:
            raise ValueError("unexpected or missing package members")
        if any(not x.isfile() or x.uid != 0 or x.gid != 0 or x.mtime != 0
               for x in members):
            raise ValueError("unexpected archive metadata/member type")
        binary = tar.extractfile(f"{NAME}/{BINARY}").read()
        manifest_bytes = tar.extractfile(f"{NAME}/{MANIFEST}").read()
        manifest = json.loads(manifest_bytes)
    if manifest.get("kind") != "SOL_ADAPTER_MOOSE_BINARY_CANDIDATE_NOT_A_RELEASE":
        raise ValueError("not an adapter candidate")
    if (manifest.get("binary_sha256") != digest(binary)
            or manifest.get("binary_bytes") != len(binary)):
        raise ValueError("binary checksum/size mismatch")
    if not manifest.get("target_host") or len(manifest.get("adapter_source_commit", "")) != 40:
        raise ValueError("missing provenance/target")
    print(f"[ADAPTER P4] VERIFIED: {archive.name}, host={manifest['target_host']}")
    return manifest, binary


def selected_path(prefix: Path, manifest: dict) -> Path:
    version = manifest["adapter_package_version"]
    commit = manifest["adapter_source_commit"]
    if not isinstance(version, str) or not version.replace(".", "").isdigit():
        raise ValueError("unsafe version string")
    if len(commit) != 40 or any(c not in "0123456789abcdef" for c in commit):
        raise ValueError("unsafe revision")
    return prefix / "lib" / "sol-adapter-moose" / "versions" / f"{version}-{commit[:12]}"


def install(archive: Path, prefix: Path) -> Path:
    manifest, binary = verify(archive)
    if manifest["target_host"] != host_platform():
        raise ValueError(f"incompatible host: expected {manifest['target_host']}, actual {host_platform()}")
    destination = selected_path(prefix, manifest)
    if destination.is_symlink():
        raise ValueError("destination is symlink")
    if destination.exists():
        current = destination / MANIFEST
        existing_binary = destination / BINARY
        if (not current.is_file() or not existing_binary.is_file()
                or json.loads(current.read_text()) != manifest
                or digest(existing_binary.read_bytes()) != manifest["binary_sha256"]):
            raise ValueError("existing installation differs: refusing overwrite")
        print(f"[ADAPTER P4] Already installed: {destination}")
        return destination
    parent = destination.parent
    parent.mkdir(parents=True, exist_ok=True)
    if parent.is_symlink():
        raise ValueError("installation parent is symlink")
    stage = Path(tempfile.mkdtemp(prefix=".sol-adapter-stage-", dir=parent))
    try:
        path = stage / BINARY
        path.write_bytes(binary)
        path.chmod(0o755)
        (stage / MANIFEST).write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
        stage.rename(destination)  # atomic same-filesystem staged installation
    finally:
        if stage.exists():
            shutil.rmtree(stage)
    print(f"[ADAPTER P4] INSTALLED: {destination / BINARY}")
    return destination


def uninstall(prefix: Path, version: str, commit: str) -> None:
    manifest_hint = {"adapter_package_version": version, "adapter_source_commit": commit}
    target = selected_path(prefix, manifest_hint)
    if not target.exists():
        raise ValueError("version was not installed")
    if target.is_symlink():
        raise ValueError("refuse symlinked install")
    listing = {item.name for item in target.iterdir()}
    if listing != {BINARY, MANIFEST} or any(item.is_symlink() for item in target.iterdir()):
        raise ValueError("unrecognized installation contents; refusing deletion")
    manifest = json.loads((target / MANIFEST).read_text())
    if (manifest.get("kind") != "SOL_ADAPTER_MOOSE_BINARY_CANDIDATE_NOT_A_RELEASE"
            or manifest.get("adapter_package_version") != version
            or manifest.get("adapter_source_commit") != commit
            or digest((target / BINARY).read_bytes()) != manifest["binary_sha256"]):
        raise ValueError("installed package integrity mismatch; refusing deletion")
    (target / BINARY).unlink()
    (target / MANIFEST).unlink()
    target.rmdir()
    print(f"[ADAPTER P4] UNINSTALLED: {target}")
    # Do not remove prefix/lib/bin, other versions, MOOSE executable, or user data.


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="mode", required=True)
    b = sub.add_parser("build")
    b.add_argument("--binary", required=True, type=Path)
    b.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    b.add_argument("--output-dir", required=True, type=Path)
    v = sub.add_parser("verify")
    v.add_argument("--archive", required=True, type=Path)
    i = sub.add_parser("install")
    i.add_argument("--archive", required=True, type=Path)
    i.add_argument("--prefix", required=True, type=Path)
    u = sub.add_parser("uninstall")
    u.add_argument("--prefix", required=True, type=Path)
    u.add_argument("--version", required=True)
    u.add_argument("--commit", required=True)
    args = parser.parse_args()
    if args.mode == "build":
        build(args.binary, args.root, args.output_dir)
    elif args.mode == "verify":
        verify(args.archive)
    elif args.mode == "install":
        install(args.archive, args.prefix)
    else:
        uninstall(args.prefix, args.version, args.commit)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, json.JSONDecodeError, tarfile.TarError) as exc:
        print(f"[ADAPTER P4] BLOCKED: {exc}", file=sys.stderr)
        sys.exit(1)
