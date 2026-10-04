#!/usr/bin/env python3
"""Gate-first: unresolved creates cannot be certified or repair a journal."""

import json
import fcntl
import os
import tempfile
import time
import unittest
from pathlib import Path

from lifecycle_ledger import Ledger, LedgerError, reconcile_journal


class LedgerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.ledger = Ledger.create(self.root / "ledger", "a" * 32)

    def tearDown(self):
        self.temp.cleanup()

    def test_fence_is_persistent_and_monotone(self):
        self.ledger.begin("b" * 32, "sha256:" + "c" * 64)
        self.ledger.fence()
        with self.assertRaises(LedgerError):
            self.ledger.begin("d" * 32, "sha256:" + "c" * 64)
        self.assertEqual(
            Ledger(self.root / "ledger", "a" * 32).snapshot()["state"], "fenced"
        )
        self.assertEqual(self.ledger.snapshot()["entries"]["b" * 32]["phase"], "intent")

    def test_ack_and_remove_require_exact_identity(self):
        self.ledger.begin("b" * 32, "sha256:" + "c" * 64)
        with self.assertRaises(LedgerError):
            self.ledger.removed("b" * 32)
        self.ledger.acknowledge("b" * 32, "d" * 64)
        with self.assertRaises(LedgerError):
            self.ledger.acknowledge("b" * 32, "e" * 64)
        self.ledger.removed("b" * 32)
        with self.assertRaises(LedgerError):
            self.ledger.acknowledge("b" * 32, "d" * 64)

    def test_tamper_and_symlink_fail_closed(self):
        p = self.root / "ledger" / "state.json"
        m = json.loads(p.read_text())
        m["schema_version"] = True
        p.write_text(json.dumps(m))
        with self.assertRaises(LedgerError):
            self.ledger.snapshot()
        p.unlink()
        p.symlink_to(self.root / "missing")
        with self.assertRaises(LedgerError):
            self.ledger.snapshot()

    def test_stalled_ledger_lock_is_bounded(self):
        path = self.root / "ledger" / "lock"
        with path.open("wb") as writer:
            os.chmod(path, 0o600)
            fcntl.flock(writer, fcntl.LOCK_EX | fcntl.LOCK_NB)
            started = time.monotonic()
            with self.assertRaises(LedgerError):
                self.ledger.fence()
            self.assertLess(time.monotonic() - started, 2)

    def journal(self):
        store = self.root / "store"
        store.mkdir()
        data = dict(
            revision=1,
            content_id="sha256:" + "e" * 64,
            receipts={"55": {"reply": "preserve"}},
            runs=[
                dict(
                    role="ls_shell",
                    lifecycle_invocation="b" * 32,
                    created=False,
                    removed=False,
                    invocation_pending=True,
                )
            ],
        )
        (store / "journal.json").write_text(json.dumps(data))
        os.chmod(store / "journal.json", 0o600)
        return store, data

    def test_explicit_journal_repair_preserves_effects(self):
        store, prior = self.journal()
        self.ledger.begin("b" * 32, "sha256:" + "c" * 64)
        self.ledger.acknowledge("b" * 32, "d" * 64)
        self.ledger.removed("b" * 32)
        self.ledger.fence()
        proof = self.ledger.seal_reconciliation(observed_empty=True)
        result = reconcile_journal(store, self.ledger, proof)
        after = json.loads((store / "journal.json").read_text())
        self.assertEqual(
            {k: v for k, v in after.items() if k != "runs"},
            {k: v for k, v in prior.items() if k != "runs"},
        )
        self.assertTrue(after["runs"][0]["removed"])
        self.assertEqual(result["repaired"], 1)
        self.assertEqual(reconcile_journal(store, self.ledger, proof)["repaired"], 0)

    def test_active_writer_and_boolean_proof_alias_fail_closed(self):
        store, prior = self.journal()
        self.ledger.begin("b" * 32, "sha256:" + "c" * 64)
        self.ledger.acknowledge("b" * 32, "d" * 64)
        self.ledger.removed("b" * 32)
        self.ledger.fence()
        proof = self.ledger.seal_reconciliation(observed_empty=True)
        with (store / "writer.lock").open("wb") as writer:
            fcntl.flock(writer, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with self.assertRaises(LedgerError):
                reconcile_journal(store, self.ledger, proof)
        for field in ("certified", "observed_empty", "schema_version"):
            altered = dict(proof)
            altered[field] = 1 if field != "schema_version" else True
            with self.assertRaises(LedgerError):
                reconcile_journal(store, self.ledger, altered)
        self.assertEqual(json.loads((store / "journal.json").read_text()), prior)

    def test_unknown_or_changed_receipt_cannot_repair(self):
        store, prior = self.journal()
        self.ledger.begin("b" * 32, "sha256:" + "c" * 64)
        self.ledger.fence()
        with self.assertRaises(LedgerError):
            self.ledger.seal_reconciliation(observed_empty=True)
        with self.assertRaises(LedgerError):
            reconcile_journal(store, self.ledger, {"certified": True})
        self.assertEqual(json.loads((store / "journal.json").read_text()), prior)
        self.ledger.acknowledge("b" * 32, "d" * 64)
        self.ledger.removed("b" * 32)
        proof = self.ledger.seal_reconciliation(observed_empty=True)
        proof["ledger_sha256"] = "0" * 64
        with self.assertRaises(LedgerError):
            reconcile_journal(store, self.ledger, proof)


if __name__ == "__main__":
    unittest.main()
