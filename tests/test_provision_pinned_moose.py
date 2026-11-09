"""Local, network-free characterization for pinned MOOSE provisioning decisions."""
from pathlib import Path
import json
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest

from scripts.provision_pinned_moose import (
    CHANNEL, PACKAGE, PACKAGE_ARCHIVE_SHA256, classify, provision,
)


class ProvisioningContractTests(unittest.TestCase):
    def test_classification_does_not_turn_backend_into_pass(self):
        self.assertEqual(classify(0, ""), "DEPENDENCIES_INSTALLED_IDENTITY_UNVERIFIED")
        self.assertEqual(
            classify(1, "CondaHTTPError: HTTP 502 Bad Gateway for url "
                    "<https://conda.software.inl.gov/public/noarch/repodata.json>"),
            "TRANSIENT_UPSTREAM_HTTP_5XX",
        )
        self.assertEqual(classify(1, "ReadTimeoutError: request timed out"),
                         "TRANSIENT_UPSTREAM_TRANSPORT")
        self.assertEqual(classify(1, "PackagesNotFoundError: moose"),
                         "DEPENDENCY_OR_INTEGRITY_FAILURE")
        self.assertEqual(classify(1, "UnsatisfiableError"),
                         "DEPENDENCY_OR_INTEGRITY_FAILURE")
        self.assertEqual(classify(1, "HTTP 404 Not Found"),
                         "PROVISIONING_FAILURE_UNCLASSIFIED")
        self.assertEqual(classify(1, "SIGSEGV 139"), "PROVISIONING_FAILURE_UNCLASSIFIED")
        self.assertEqual(
            classify(1, "PackagesNotFoundError\nCondaHTTPError: HTTP 502"),
            "DEPENDENCY_OR_INTEGRITY_FAILURE",
        )

    def run_case(self, answers, create_prefix_on_error=False):
        with TemporaryDirectory() as temp:
            root = Path(temp)
            prefix = root / "moose-env"
            receipt = root / "target" / "provisioning.json"
            counter = []
            slept = []
            def runner(cmd, **kwargs):
                self.assertEqual(cmd[-1], PACKAGE)
                self.assertEqual(cmd[cmd.index("-c") + 1], CHANNEL)
                self.assertEqual(kwargs["timeout"], 240)
                i = len(counter)
                counter.append(cmd)
                if create_prefix_on_error and answers[i][0] != 0:
                    prefix.mkdir()
                return SimpleNamespace(
                    returncode=answers[i][0], stdout="", stderr=answers[i][1]
                )
            code = provision(
                Path("fake-conda"), prefix, receipt,
                attempts=2, delay_seconds=0, runner=runner, sleeper=slept.append
            )
            report = json.loads(receipt.read_text())
            self.assertEqual(report["dependency_closure_sha256"], "NOT_ESTABLISHED")
            self.assertEqual(
                report["top_level_package_expected_archive_sha256"], PACKAGE_ARCHIVE_SHA256
            )
            self.assertEqual(report["native_backend_result"], "NOT_EVALUATED")
            self.assertEqual(len(report["attempts"]), len(counter))
            return code, report, len(counter), len(slept)

    def test_transient_one_retry_and_no_more(self):
        first = (1, "CondaHTTPError: HTTP 502 Bad Gateway")
        success = (0, "environment solved")
        code, report, count, waits = self.run_case([first, success])
        self.assertEqual((code, count, waits), (0, 2, 1))
        self.assertEqual(report["status"], "PROVISIONING_COMPLETED_IDENTITY_NOT_YET_ATTESTED")
        code, report, count, waits = self.run_case([first, first])
        self.assertEqual((code, count, waits), (1, 2, 1))
        self.assertEqual(report["status"], "INFRA_BLOCKED_TRANSIENT_EXHAUSTED")

    def test_nontransient_does_not_retry(self):
        code, report, count, waits = self.run_case(
            [(1, "PackagesNotFoundError: package missing")]
        )
        self.assertEqual((code, count, waits), (1, 1, 0))
        self.assertEqual(report["status"], "NONTRANSIENT_PROVISIONING_FAILURE")

    def test_partial_prefix_fails_closed(self):
        code, report, count, waits = self.run_case(
            [(1, "CondaHTTPError: HTTP 503 Service Unavailable")],
            create_prefix_on_error=True,
        )
        self.assertEqual((code, count, waits), (1, 1, 0))
        self.assertEqual(report["status"], "PARTIAL_PREFIX_RETRY_UNSAFE")


if __name__ == "__main__":
    unittest.main()
