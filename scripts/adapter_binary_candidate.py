#!/usr/bin/env python3
"""Candidate native Adapter archive and safe prefix install/uninstall on Unix.

Never writes outside the supplied --prefix during install/uninstall. This is
not an official signed/tagged release and does not bundle MOOSE.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tarfile
import tempfile

NAME = "sol-adaptor-moose"
PACKAGE = "sol-adapter-moose-candidate"
MANIFEST = "candidate-manifest.json"
VERSION = "0.1.0"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def target() -> str:
    return platform.system().lower() + "-" + platform.machine().lower()


def tar_bytes(entries: dict[str, bytes]) -> bytes:
    buf = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=buf, mtime=0, compresslevel=9) as gz:
        with tarfile.open(mode="w", fileobj=gz, format=tarfile.USTAR_FORMAT) as tf:
            for key, data in sorted(entries.items()):
                entry = tarfile.TarInfo(f"{PACKAGE}/{key}")
                entry.uid = entry.gid = entry.mtime = 0
                entry.mode = 0o755 if key == NAME else 0o644
                entry.size = len(data)
                tf.addfile(entry, io.BytesIO(data))
    return buf.getvalue()


def read_archive(archive: Path, checkfile: Path) -> tuple[dict, bytes, bytes]:
    raw = archive.read_bytes()
    checker = checkfile.read_text().strip().split()
    if len(checker) != 2 or checker[1] != archive.name or checker[0] != digest(raw):
        raise ValueError("archive SHA256 mismatch (do not install)")
    entries = {}
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as src:
        for mem in src.getmembers():
            if not mem.isfile() or mem.name not in (
                f"{PACKAGE}/{NAME}", f"{PACKAGE}/{MANIFEST}", f"{PACKAGE}/README.txt"
            ):
                raise ValueError(f"archive member not allowed: {mem.name}")
            if mem.name in entries or mem.uid or mem.gid or mem.mtime:
                raise ValueError(f"archive member duplicated or nondeterministic: {mem.name}")
            entries[mem.name] = src.extractfile(mem).read()
    if set(entries) != {f"{PACKAGE}/{NAME}", f"{PACKAGE}/{MANIFEST}", f"{PACKAGE}/README.txt"}:
        raise ValueError("archive incomplete")
    manifest = json.loads(entries[f"{PACKAGE}/{MANIFEST}"])
    binary = entries[f"{PACKAGE}/{NAME}"]
    if (manifest.get("kind") != "SOL_MOOSE_ADAPTER_BINARY_CANDIDATE_NOT_A_RELEASE"
        or len(manifest.get("source_commit", "")) != 40
        or manifest.get("binary_sha256") != digest(binary)
        or manifest.get("binary_bytes") != len(binary)
        or manifest.get("adapter_package_version") != VERSION
        or tar_bytes({name[len(PACKAGE)+1:]:value for name,value in entries.items()}) != raw):
        raise ValueError("binary/source/manifest integrity failed")
    return manifest, binary, raw


def make(out: Path, binary_file: Path, source_dir: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    revision = subprocess.check_output(
        ["git", "-C", str(source_dir), "rev-parse", "HEAD"], text=True
    ).strip()
    binary = binary_file.read_bytes()
    if not binary:
        raise ValueError("binary is empty")
    if shutil.which("ldd"):
        cmd = ["ldd", str(binary_file)]
    elif shutil.which("otool"):
        cmd = ["otool", "-L", str(binary_file)]
    else:
        cmd = None
    if cmd is not None:
        linked = subprocess.run(cmd, text=True, capture_output=True, check=False)
        # Linux ldd prints ASLR-dependent addresses: never include those in
        # reproducibility-critical archive bytes.
        report = re.sub(r"\s+\(0x[0-9a-fA-F]+\)", "", (linked.stdout + linked.stderr)).strip()
    else:
        report = "linkage-inspection-command-not-available (no ABI claim)"
    manifest = {
        "kind": "SOL_MOOSE_ADAPTER_BINARY_CANDIDATE_NOT_A_RELEASE",
        "adapter_package_version": VERSION,
        "adapter_id": "sol.adapter.moose",
        "source_commit": revision,
        "target_host": target(),
        "binary_name": NAME,
        "binary_bytes": len(binary),
        "binary_sha256": digest(binary),
        "supported_adapter_protocol_versions": ["0.1", "0.2"],
        "supported_public_contract_versions": ["0.1", "0.2"],
        "reported_capability": ["thermal.steady_conduction"],
        "moose_executable": "external:SOL_MOOSE_EXECUTABLE",
        "dynamic_linkage": report,
        "status": "CI candidate; no tagged release, scientific signoff or local user acceptance",
    }
    readme = (
        "Candidate-only native sol-adaptor-moose binary.\n"
        "No MOOSE executable or SOL Runtime is bundled.\n"
        "Install only with scripts/adapter_binary_candidate.py --prefix.\n"
    ).encode()
    entries = {NAME: binary, MANIFEST: (json.dumps(manifest, sort_keys=True, indent=2)+"\n").encode(),
               "README.txt": readme}
    raw = tar_bytes(entries)
    path = out / f"{PACKAGE}.tar.gz"
    path.write_bytes(raw)
    checksum = out / f"{PACKAGE}.sha256"
    checksum.write_text(f"{digest(raw)}  {path.name}\n")
    read_archive(path, checksum)
    print(f"[Adapter P4] PASS candidate archive SHA256={digest(raw)} binary SHA256={digest(binary)}")


def installed_version(prefix: Path, manifest: dict) -> Path:
    return prefix / "lib" / "sol-adapter-moose" / "versions" / manifest["source_commit"]


def install(prefix: Path, archive: Path, checksum: Path, expect: str | None, activate: bool) -> None:
    manifest, binary, _ = read_archive(archive, checksum)
    if manifest["target_host"] != target() or (expect and expect != target()):
        raise ValueError(f"platform mismatch: package={manifest['target_host']} host={target()} required={expect}")
    dest = installed_version(prefix, manifest)
    versions = dest.parent
    link = prefix / "bin" / NAME
    # Preflight all existing ownership paths before creating any package state.
    if (link.exists() and not link.is_symlink()) or (link.is_symlink() and
            not link.resolve(strict=False).is_relative_to(versions.resolve())):
        raise ValueError("cannot replace a non-owned active executable or symlink")
    if dest.is_symlink():
        raise ValueError("existing package version directory must not be a symlink")
    if dest.exists():
        if not dest.is_dir() or (dest / NAME).read_bytes() != binary or (
            dest / MANIFEST
        ).read_bytes() != (json.dumps(manifest, sort_keys=True, indent=2)+"\n").encode():
            raise ValueError("existing version path is not the same owned package")
    else:
        versions.mkdir(parents=True, exist_ok=True)
        stage = Path(tempfile.mkdtemp(prefix=".candidate-stage-", dir=versions))
        try:
            (stage / NAME).write_bytes(binary)
            (stage / NAME).chmod(0o755)
            (stage / MANIFEST).write_text(json.dumps(manifest, sort_keys=True, indent=2)+"\n")
            stage.rename(dest)
        finally:
            if stage.exists():
                shutil.rmtree(stage)
    link.parent.mkdir(parents=True, exist_ok=True)
    if link.exists() and not link.is_symlink():
        raise ValueError("cannot replace a non-owned active executable")
    if link.is_symlink():
        before = link.resolve(strict=False)
        if not before.is_relative_to(versions.resolve()):
            raise ValueError("existing symlink points outside managed versions")
        if not activate and before != (dest / NAME).resolve():
            print("[Adapter P4] Side-by-side installed; current active version preserved")
            return
        link.unlink()
    link.symlink_to(os.path.relpath(dest / NAME, link.parent))
    print(f"[Adapter P4] PASS installed {dest}; active={link}")


def uninstall(prefix: Path, revision: str) -> None:
    if len(revision) != 40 or any(c not in "0123456789abcdef" for c in revision):
        raise ValueError("full 40-character revision required")
    dest = prefix / "lib" / "sol-adapter-moose" / "versions" / revision
    link = prefix / "bin" / NAME
    if not dest.is_dir():
        raise ValueError("specified package version is not installed")
    manifest_file = dest / MANIFEST
    if not manifest_file.is_file():
        raise ValueError("missing ownership manifest")
    manifest = json.loads(manifest_file.read_text())
    if manifest.get("source_commit") != revision or manifest.get("kind") != "SOL_MOOSE_ADAPTER_BINARY_CANDIDATE_NOT_A_RELEASE":
        raise ValueError("not an owned adapter candidate")
    if set(p.name for p in dest.iterdir()) != {NAME, MANIFEST}:
        raise ValueError("unexpected user content in package directory; refusing removal")
    if digest((dest / NAME).read_bytes()) != manifest["binary_sha256"]:
        raise ValueError("installed binary was modified; refusing removal")
    if link.is_symlink() and link.resolve(strict=False) == (dest / NAME).resolve():
        link.unlink()
    # Remove only the two known package files and the now empty version directory.
    (dest / NAME).unlink()
    manifest_file.unlink()
    dest.rmdir()
    print(f"[Adapter P4] PASS uninstalled only owned version {revision}")


def main() -> None:
    p = argparse.ArgumentParser()
    s = p.add_subparsers(dest="cmd", required=True)
    build = s.add_parser("build")
    build.add_argument("--out", type=Path, required=True)
    build.add_argument("--binary", type=Path, required=True)
    build.add_argument("--source", type=Path, default=Path("."))
    verify = s.add_parser("verify")
    verify.add_argument("--archive", type=Path, required=True)
    verify.add_argument("--checksum", type=Path, required=True)
    inst = s.add_parser("install")
    inst.add_argument("--prefix", type=Path, required=True)
    inst.add_argument("--archive", type=Path, required=True)
    inst.add_argument("--checksum", type=Path, required=True)
    inst.add_argument("--expect-target")
    inst.add_argument("--activate", action="store_true")
    un = s.add_parser("uninstall")
    un.add_argument("--prefix", type=Path, required=True)
    un.add_argument("--revision", required=True)
    args = p.parse_args()
    try:
        if args.cmd == "build":
            make(args.out, args.binary, args.source)
        elif args.cmd == "verify":
            read_archive(args.archive, args.checksum)
            print("[Adapter P4] PASS candidate archive integrity")
        elif args.cmd == "install":
            install(args.prefix, args.archive, args.checksum, args.expect_target, args.activate)
        else:
            uninstall(args.prefix, args.revision)
    except (ValueError, OSError, json.JSONDecodeError) as exc:
        p.exit(3, f"[Adapter P4] REJECTED: {exc}\n")


if __name__ == "__main__":
    main()
