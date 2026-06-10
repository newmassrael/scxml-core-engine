#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Regression tests for the EventSchema bytes-guard RFC -> Mnemosyne manifest
converter (`bytesguard` namespace, docs/sce-ledger/bytesguard).

Run:  python3 -m unittest discover -s tools/mnemosyne-adoption/tests
"""

import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL_DIR = HERE.parent
REPO_ROOT = TOOL_DIR.parent.parent
sys.path.insert(0, str(TOOL_DIR))

import bytesguard_rfc_to_manifest as conv  # noqa: E402

BG_RFC = (
    REPO_ROOT / "docs" / "sce-ledger" / "bytesguard" / "rfc-eventschema-bytes-guard.md"
)

FRAGMENT = """\
# RFC — EventSchema `bytes`-field native transition guards

## §1 Background

### §1.3 The SSOT-honesty problem

## §3 Lock-in decisions

### §3.1 Honest interim gate (before B1–B6 land)

### 9.9 no sigil, not a section in this document's convention

```text
## §9 fenced, not a heading
```

## §6 Acceptance gates
"""


def _by_id(manifest):
    return {e["section_id"]: e for e in manifest}


class UnitTests(unittest.TestCase):
    def setUp(self):
        self.manifest = conv.convert(FRAGMENT, "bytesguard")
        self.by_id = _by_id(self.manifest)

    def test_sections_extracted(self):
        self.assertEqual(
            sorted(self.by_id),
            [
                "bytesguard-1",
                "bytesguard-1.3",
                "bytesguard-3",
                "bytesguard-3.1",
                "bytesguard-6",
            ],
        )

    def test_sigil_mandatory(self):
        # This document always writes the sigil in headings; an unsigiled
        # numbered heading is prose, never a section.
        self.assertNotIn("bytesguard-9.9", self.by_id)

    def test_fenced_heading_skipped(self):
        self.assertNotIn("bytesguard-9", self.by_id)

    def test_hierarchy(self):
        self.assertEqual(self.by_id["bytesguard-1.3"]["parent_section"], "bytesguard-1")
        self.assertEqual(self.by_id["bytesguard-3.1"]["parent_section"], "bytesguard-3")
        self.assertNotIn("parent_section", self.by_id["bytesguard-3"])

    def test_self_check_clean(self):
        self.assertIsNone(conv.self_check(self.manifest))

    def test_self_check_rejects_orphan_parent(self):
        bad = [
            {
                "section_id": "bytesguard-3.1",
                "parent_doc": "bytesguard",
                "title": "x",
                "parent_section": "bytesguard-3",
            }
        ]
        self.assertIn("not emitted", conv.self_check(bad))


class RealDocTests(unittest.TestCase):
    @unittest.skipUnless(BG_RFC.exists(), "RFC not tracked")
    def test_real_doc_invariants(self):
        manifest = conv.convert(BG_RFC.read_text(encoding="utf-8"), "bytesguard")
        by_id = _by_id(manifest)
        self.assertIsNone(conv.self_check(manifest))
        # Every section family SCE code cites must resolve.
        for sid in ("bytesguard-1.3", "bytesguard-3", "bytesguard-3.1", "bytesguard-6"):
            self.assertIn(sid, by_id)
        # The document has no §8 — the historical "RFC §8 commit 3c" cites
        # belong to a different document and must never resolve here.
        self.assertNotIn("bytesguard-8", by_id)


if __name__ == "__main__":
    unittest.main()
