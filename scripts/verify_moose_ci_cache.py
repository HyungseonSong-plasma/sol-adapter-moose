#!/usr/bin/env python3
"""Fail-closed verification for the pinned linux-64 MOOSE CI environment.

Build-set fingerprint originates from 2026-11-06 CI artifact #13525430701
(run #42260284458). Per-dependency package bytes are SHA256-checked against
installed Conda records, but the hashes are not yet a reviewed immutable
external dependency-digest lock; archive supply still needs a trusted seed.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

EXPECTED_COUNT = 350
EXPECTED_BUILD_SET_SHA256 = "6de33352a3830955f32cef42a37c4e022020a7b367fcb4c0dff428e0834a3b1a"
EXPECTED_MOOSE_SHA256 = "d7b2eec6b958f11455c903b70a16ea319b7edb68b4499537313c06785abd05b3"
EXPECTED_MOOSE = ("moose", "2026.08.12", "mpich", "linux-64", "moose-2026.08.12-mpich.conda")
BUILD_FIELDS = ("name", "version", "build_string", "base_url", "platform")
SHA256_RE = re.compile(r"[0-9a-f]{64}")


def digest_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_set_hash(rows):
    values = sorted(tuple(record[name] for name in BUILD_FIELDS) for record in rows)
    payload = json.dumps(values, separators=(",", ":"), ensure_ascii=True).encode()
    return hashlib.sha256(payload).hexdigest()


def verify(prefix, pkg_cache, conda_list, output, expected_count=EXPECTED_COUNT,
           expected_build_hash=EXPECTED_BUILD_SET_SHA256):
    prefix, pkg_cache, conda_list, output = map(Path, (prefix, pkg_cache, conda_list, output))
    if not (prefix / "bin/moose-opt").is_file():
        raise ValueError("MOOSE executable absent from cache/environment")
    rows = json.loads(conda_list.read_text(encoding="utf-8"))
    if not isinstance(rows, list) or len(rows) != expected_count:
        raise ValueError(f"unexpected installed Conda dependency count: {len(rows)}")
    observed = build_set_hash(rows)
    if observed != expected_build_hash:
        raise ValueError(f"Conda dependency build-set drift: {observed}")

    records = []
    for meta_file in sorted((prefix / "conda-meta").glob("*.json")):
        obj = json.loads(meta_file.read_text(encoding="utf-8"))
        name, version, build, subdir = (obj[k] for k in ("name", "version", "build", "subdir"))
        filename, declared_sha = (obj[k] for k in ("fn", "sha256"))
        if (not isinstance(filename, str) or Path(filename).name != filename
                or not filename.endswith((".conda", ".tar.bz2"))):
            raise ValueError(f"unsafe or missing Conda archive filename for {name}")
        if not isinstance(declared_sha, str) or not SHA256_RE.fullmatch(declared_sha):
            raise ValueError(f"missing SHA256 in Conda metadata for {name}")
        archive = pkg_cache / filename
        if not archive.is_file() or archive.is_symlink():
            raise ValueError(f"required package archive missing/unsafe: {filename}")
        actual_sha = digest_file(archive)
        if actual_sha != declared_sha:
            raise ValueError(f"package archive SHA256 mismatch: {filename}")
        if name == "moose":
            if (name, version, build, subdir, filename) != EXPECTED_MOOSE:
                raise ValueError("wrong MOOSE archive identity")
            if declared_sha != EXPECTED_MOOSE_SHA256:
                raise ValueError("MOOSE archive differs from accepted Phase 1 SHA256")
        records.append({"name": name, "version": version, "build": build, "subdir": subdir,
                        "filename": filename, "sha256": actual_sha})

    if len(records) != expected_count or sum(x["name"] == "moose" for x in records) != 1:
        raise ValueError("Conda metadata does not match dependency count or MOOSE uniqueness")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps({"source": "run-42260284458-2026-11-06",
                                  "build_set_sha256": expected_build_hash,
                                  "package_count": len(records), "packages": records},
                                 indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Verified {len(records)} package archive SHA256 digests and accepted MOOSE archive")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--prefix", type=Path, required=True)
    parser.add_argument("--pkg-cache", type=Path, required=True)
    parser.add_argument("--conda-list", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    verify(args.prefix, args.pkg_cache, args.conda_list, args.output)


if __name__ == "__main__":
    main()
