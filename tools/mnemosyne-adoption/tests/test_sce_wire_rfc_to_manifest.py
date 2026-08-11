#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Regression tests for the Wire RFC -> Mnemosyne manifest converter (wire
design-ledger namespace).

  * UnitTests       — wave extraction, legacy-duplicate dedup, the W4.5 dotted
                      label, fence skipping, and a real-doc invariant pass.
  * ClosedLoopTest  — imports the real manifest into a section_namespace="wire"
                      workspace and proves `§wire-W<n>` cites resolve whole while a
                      foreign §scxml cite is skipped by namespace scope.

Run:  python3 -m unittest discover -s tools/mnemosyne-adoption/tests
"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent

from _mnemosyne_bin import MNEMOSYNE_CLI, skip_reason  # noqa: E402
TOOL_DIR = HERE.parent
REPO_ROOT = TOOL_DIR.parent.parent
sys.path.insert(0, str(TOOL_DIR))

import sce_wire_rfc_to_manifest as conv  # noqa: E402

WIRE_RFC = REPO_ROOT / "docs" / "sce-ledger" / "wire" / "rfc-sce-diagnostic-wire-unification.md"

FRAGMENT = """\
# RFC: Wire Contract

## §3 Milestone roadmap

### W0 contract (prerequisite for W1)

### W1 contract (this RFC)

### W4 LANDED 2026-04-26 (α-strict)

### W4 RFC (legacy section header, retained below)

### W4.5 LANDED 2026-04-26 (debt repayment)

```text
### W9 not-a-heading inside a fence
```

#### Q1 not a wave (level 4)
"""


def _by_id(manifest):
    return {e["section_id"]: e for e in manifest}


class UnitTests(unittest.TestCase):
    def setUp(self):
        self.manifest = conv.convert(FRAGMENT, "GENERATED.md")
        self.m = _by_id(self.manifest)

    def test_waves_extracted(self):
        self.assertIn("wire-W0", self.m)
        self.assertIn("wire-W1", self.m)
        self.assertIn("wire-W4", self.m)

    def test_legacy_duplicate_dropped_first_wins(self):
        # "W4 LANDED ..." (first) wins over "W4 RFC (legacy ...)".
        self.assertEqual(len([e for e in self.manifest if e["section_id"] == "wire-W4"]), 1)
        self.assertEqual(self.m["wire-W4"]["title"], "LANDED 2026-04-26 (α-strict)")

    def test_dotted_half_wave_label_kept_verbatim(self):
        self.assertIn("wire-W4.5", self.m)
        self.assertEqual(self.m["wire-W4.5"]["title"], "LANDED 2026-04-26 (debt repayment)")

    def test_fenced_and_sub_level_headings_skipped(self):
        self.assertNotIn("wire-W9", self.m)  # inside a code fence
        # Q1 is a #### level non-wave heading -> never a wave id.
        self.assertNotIn("wire-Q1", self.m)

    def test_ids_are_bare_no_sigil(self):
        for e in self.manifest:
            self.assertNotIn("§", e["section_id"])

    @unittest.skipUnless(WIRE_RFC.exists(), "Wire RFC missing")
    def test_real_doc_waves_unique_and_citation_safe(self):
        manifest = conv.convert(WIRE_RFC.read_text(encoding="utf-8"), "GENERATED.md")
        ids = [e["section_id"] for e in manifest]
        self.assertEqual(len(ids), len(set(ids)), "duplicate wave ids")
        self.assertIn("wire-W4.5", set(ids))
        for e in manifest:
            leaf = e["section_id"][len("wire-") :]
            for i, ch in enumerate(leaf):
                if ch == ".":
                    self.assertTrue(
                        leaf[i - 1].isdigit() and leaf[i + 1].isdigit(), e["section_id"]
                    )


@unittest.skipUnless(MNEMOSYNE_CLI, skip_reason())
@unittest.skipUnless(WIRE_RFC.exists(), "Wire RFC missing")
class ClosedLoopTest(unittest.TestCase):
    def test_wire_cites_resolve_and_foreign_skipped(self):
        manifest = conv.convert(WIRE_RFC.read_text(encoding="utf-8"), "GENERATED.md")
        ids = _by_id(manifest)
        for sid in ("wire-W4", "wire-W4.5", "wire-W5"):
            self.assertIn(sid, ids, "fixture id missing from real manifest")

        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "src").mkdir()
            (root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
            (root / "mnemosyne.toml").write_text(
                '[workspace]\n\n'
                "[atomic]\n"
                'sidecar_path = ".atomic/workspace.atomic.json"\n\n'
                "[plugins.set_equality_validator]\n"
                'paths = ["src/"]\nseverity_missing = "reject"\nseverity_binding = "warn"\n'
                'comment_only = true\nsection_namespace = "wire"\n',
                encoding="utf-8",
            )
            subprocess.run(
                [MNEMOSYNE_CLI, "import-sections", "--manifest", "manifest.json"],
                cwd=root, check=True, capture_output=True,
            )
            (root / "src" / "cite.rs").write_text(
                "// RFC §wire-W4 D1-C typed-throw\n"
                "// §wire-W4.5 debt repayment\n"
                "// foreign §scxml-5.10 (skipped by namespace scope)\n",
                encoding="utf-8",
            )
            out = subprocess.run(
                [MNEMOSYNE_CLI, "validate-code-refs", "--json"],
                cwd=root, check=True, capture_output=True, text=True,
            )
            report = json.loads(out.stdout)
            missing = {
                v["entry_id"]
                for v in report.get("violations", [])
                if v.get("kind") == "section_missing"
            }
            self.assertEqual(missing, set(), f"unexpected missing ids: {missing}")


if __name__ == "__main__":
    unittest.main()
