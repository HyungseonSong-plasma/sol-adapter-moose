"""Synthetic tests; no test here certifies a real MOOSE solve or cache seed."""
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from verify_moose_ci_cache import verify, build_set_hash


class CacheVerificationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.prefix = self.root / "env"
        self.cache = self.root / "pkgs"
        self.cache.mkdir()
        (self.prefix / "bin").mkdir(parents=True)
        (self.prefix / "conda-meta").mkdir()
        (self.prefix / "bin" / "moose-opt").write_bytes(b"fake")
        self.rows = [{"name": "moose", "version": "2026.08.12", "build_string": "mpich",
                      "base_url": "https://conda.software.inl.gov/public", "platform": "linux-64"}]
        self.conda_list = self.root / "list.json"
        self.conda_list.write_text(json.dumps(self.rows))
        self.archive = self.cache / "moose-2026.08.12-mpich.conda"
        self.archive.write_bytes(b"fake-archive")
        self.meta = self.prefix / "conda-meta/moose-2026.08.12-mpich.json"
        self.meta.write_text(json.dumps({"name": "moose", "version": "2026.08.12",
                                         "build": "mpich", "subdir": "linux-64",
                                         "fn": self.archive.name,
                                         "sha256": hashlib.sha256(b"fake-archive").hexdigest()}))
        self.output = self.root / "manifest.json"

    def run_verify(self, expected=None):
        return verify(self.prefix, self.cache, self.conda_list, self.output, expected_count=1,
                      expected_build_hash=expected or build_set_hash(self.rows))

    def test_verified_archives_produce_manifest(self):
        with patch("verify_moose_ci_cache.EXPECTED_MOOSE_SHA256",
                   hashlib.sha256(b"fake-archive").hexdigest()):
            self.run_verify()
        manifest = json.loads(self.output.read_text())
        self.assertEqual(manifest["package_count"], 1)
        self.assertEqual(manifest["packages"][0]["filename"], self.archive.name)

    def test_exact_official_digest_required(self):
        with self.assertRaisesRegex(ValueError, "accepted Phase 1 SHA256"):
            self.run_verify()

    def test_modified_package_archive_rejected(self):
        meta = json.loads(self.meta.read_text())
        meta["sha256"] = "0" * 64
        self.meta.write_text(json.dumps(meta))
        with self.assertRaisesRegex(ValueError, "archive SHA256 mismatch"):
            self.run_verify()

    def test_build_set_drift_rejected(self):
        with self.assertRaisesRegex(ValueError, "build-set drift"):
            self.run_verify(expected="f" * 64)

    def test_path_traversal_rejected(self):
        meta = json.loads(self.meta.read_text())
        meta["fn"] = "../moose-2026.08.12-mpich.conda"
        self.meta.write_text(json.dumps(meta))
        with self.assertRaisesRegex(ValueError, "unsafe or missing"):
            self.run_verify()

    def test_missing_executable_rejected(self):
        (self.prefix / "bin/moose-opt").unlink()
        with self.assertRaisesRegex(ValueError, "executable absent"):
            self.run_verify()


if __name__ == "__main__":
    unittest.main()
