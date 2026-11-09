"""Synthetic unit tests for the evidence comparator only; not MOOSE execution."""
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest

from scripts.verify_sol_v02_native_parity import verify


class SolV02NativeParityReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temp = TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.native = self.root / "runs" / "opaque" 
        self.native.mkdir(parents=True)
        self.direct = self.root / "direct.i"
        self.direct.write_bytes(b"[Mesh]\n  type = GeneratedMesh\n[]\n")
        (self.native / "input.i").write_bytes(self.direct.read_bytes())
        for filename in ("check.stdout.log", "check.stderr.log", "stderr.log"):
            (self.native / filename).write_text("", encoding="utf-8")
        (self.native / "stdout.log").write_text("Solve Converged!\n", encoding="utf-8")
        self.request = self.root / "request.json"
        self.request.write_text(json.dumps({
            "target": "moose", "required_capabilities": ["thermal.steady_conduction"],
            "plan_request": {
                "adapter_protocol_version": "0.2",
                "target": {"public_contract_version": "0.2"},
            },
        }))
        self.result = self.root / "consumer.json"
        self.result.write_text(json.dumps({
            "status": "completed", "adapter_id": "sol.adapter.moose",
            "adapter_protocol_version": "0.2", "public_contract_version": "0.2",
            "target": "moose", "required_capabilities": ["thermal.steady_conduction"],
            "selection": "selected", "validate": "accepted", "execute": "completed",
            "execute_response_loss_replay": "forbidden", "shutdown": "success",
        }))

    def run_check(self):
        return verify(
            self.result, self.root, self.direct, self.request,
            adapter_sha="a" * 40, sol_sha="b" * 40,
        )

    def test_valid_receipt_is_explicitly_not_scientific_acceptance(self):
        summary = self.run_check()
        self.assertEqual(summary["status"], "SOL_MEDIATED_THERMAL_NATIVE_PARITY_PASS")
        self.assertEqual(summary["physics_scientific_acceptance"], "NOT_EVALUATED")
        self.assertEqual(summary["direct_adapter_hit_sha256"],
                         summary["sol_adapter_hit_sha256"])

    def test_hit_drift_is_rejected(self):
        (self.native / "input.i").write_bytes(b"[Different]\n[]\n")
        with self.assertRaisesRegex(ValueError, "different HIT bytes"):
            self.run_check()

    def test_no_replay_policy_must_be_observed(self):
        result = json.loads(self.result.read_text())
        result["execute_response_loss_replay"] = "allowed"
        self.result.write_text(json.dumps(result))
        with self.assertRaisesRegex(ValueError, "execute_response_loss_replay"):
            self.run_check()

    def test_incomplete_run_is_rejected(self):
        (self.native / "check.stdout.log").unlink()
        with self.assertRaisesRegex(ValueError, "missing actual MOOSE phase evidence"):
            self.run_check()

    def test_wrong_capability_is_rejected(self):
        request = json.loads(self.request.read_text())
        request["required_capabilities"] = ["plasma.controlled_electron_energy_diffusion"]
        self.request.write_text(json.dumps(request))
        with self.assertRaisesRegex(ValueError, "not the supported thermal"):
            self.run_check()


if __name__ == "__main__":
    unittest.main()
