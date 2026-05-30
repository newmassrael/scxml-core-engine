#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Regression tests for the B1 spec-snapshot drift checker.

Offline only — the live `upstream` fetch is exercised manually / by the
scheduled CI job, not here, so the suite stays deterministic and network-free.
The real-snapshot integrity test doubles as a guard that the committed snapshot
and its PROVENANCE stay in sync (the license-SSOT pattern).

Run:  python3 -m unittest discover -s tools/mnemosyne-adoption/tests
"""

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL_DIR = HERE.parent
sys.path.insert(0, str(TOOL_DIR))

import check_spec_drift as drift  # noqa: E402

REAL_PROVENANCE = TOOL_DIR / "spec-snapshot" / "PROVENANCE.json"


def _write_workspace(tmp, content_bytes, sha256):
    """Build a throwaway provenance + snapshot pair and return the prov path."""
    snap = Path(tmp) / "snap.html"
    snap.write_bytes(content_bytes)
    prov = Path(tmp) / "PROVENANCE.json"
    prov.write_text(
        json.dumps(
            {
                "url": "https://www.w3.org/TR/scxml/",
                "file": "snap.html",
                "fetched_sha256": sha256,
            }
        ),
        encoding="utf-8",
    )
    return str(prov)


class CompareTests(unittest.TestCase):
    def test_match(self):
        data = b"hello"
        in_sync, actual = drift.compare(hashlib.sha256(data).hexdigest(), data)
        self.assertTrue(in_sync)
        self.assertEqual(actual, hashlib.sha256(data).hexdigest())

    def test_mismatch(self):
        in_sync, actual = drift.compare("0" * 64, b"hello")
        self.assertFalse(in_sync)
        self.assertEqual(actual, hashlib.sha256(b"hello").hexdigest())


class ProvenanceValidationTests(unittest.TestCase):
    def test_malformed_sha_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            prov = _write_workspace(tmp, b"x", "NOTHEX")
            with self.assertRaises(drift.ProvenanceError):
                drift.load_provenance(prov)

    def test_uppercase_sha_rejected(self):
        # Format is locked lowercase; uppercase must fail (B2 contract).
        with tempfile.TemporaryDirectory() as tmp:
            prov = _write_workspace(tmp, b"x", "A" * 64)
            with self.assertRaises(drift.ProvenanceError):
                drift.load_provenance(prov)

    def test_missing_file_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            prov = Path(tmp) / "PROVENANCE.json"
            prov.write_text(
                json.dumps(
                    {"url": "https://x/", "fetched_sha256": "a" * 64}
                ),
                encoding="utf-8",
            )
            with self.assertRaises(drift.ProvenanceError):
                drift.load_provenance(str(prov))


class IntegrityTests(unittest.TestCase):
    def test_in_sync(self):
        with tempfile.TemporaryDirectory() as tmp:
            data = b"<html>spec</html>"
            prov = _write_workspace(tmp, data, hashlib.sha256(data).hexdigest())
            code, msg = drift.verify_integrity(prov)
            self.assertEqual(code, drift.EXIT_OK, msg)

    def test_drift_detected(self):
        with tempfile.TemporaryDirectory() as tmp:
            # provenance claims a different sha than the file actually has
            prov = _write_workspace(tmp, b"<html>changed</html>", "b" * 64)
            code, msg = drift.verify_integrity(prov)
            self.assertEqual(code, drift.EXIT_DRIFT)
            self.assertIn("DRIFT", msg)

    @unittest.skipUnless(REAL_PROVENANCE.exists(), "vendored provenance missing")
    def test_committed_snapshot_matches_provenance(self):
        code, msg = drift.verify_integrity(str(REAL_PROVENANCE))
        self.assertEqual(code, drift.EXIT_OK, msg)


if __name__ == "__main__":
    unittest.main()
