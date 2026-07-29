#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Regression tests for the A1 W3C SCXML TOC -> Mnemosyne manifest converter.

Two layers:
  * UnitTests           — converter logic against a fixed HTML fragment and
                          against the vendored snapshot. No Mnemosyne dependency.
  * ClosedLoopTest      — imports the real manifest via `mnemosyne-cli
                          import-sections`, then feeds §-cited ids through
                          `validate-code-refs`, proving the full
                          A1 -> import -> citation chain. The id grammar is
                          Mnemosyne's SSOT; this delegates to it rather than
                          re-encoding it. Skipped when mnemosyne-cli is absent.

Section ids in the manifest are BARE (no § sigil); the § is the citation form.

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
sys.path.insert(0, str(TOOL_DIR))

import scxml_toc_to_manifest as conv  # noqa: E402

SNAPSHOT = TOOL_DIR / "spec-snapshot" / "scxml-REC-20150901.html"

FRAGMENT = """
<h2><a id="abstract" name="abstract" />Abstract</h2>
<h2 id="data-module">5 Data Model and Data Manipulation</h2>
<h3 id="scxml">5.2 &lt;scxml&gt;</h3>
<h3 id="SystemVariables">5.10 System Variables</h3>
<h4 id="InternalStructureofEvents">5.10.1 The Internal Structure of Events</h4>
<h2 id="conformance">A Conformance</h2>
<h3 id="ConformingDocuments">A.1 Conforming Documents</h3>
<h2 id="AlgorithmforSCXMLInterpretation">D Algorithm for SCXML Interpretation</h2>
<h2 id="InformalSemantics">Informal Semantics</h2>
<h2 id="Algorithm">Algorithm</h2>
<h3 id="ProceduresandFunctions">Procedures and Functions</h3>
<h4 id="interpret">procedure interpret(scxml,id)</h4>
"""


def _by_id(manifest):
    return {e["section_id"]: e for e in manifest}


class UnitTests(unittest.TestCase):
    def setUp(self):
        self.manifest, self.anchors = conv.convert(
            FRAGMENT, "GENERATED.md", "REC-scxml-20150901"
        )
        self.m = _by_id(self.manifest)

    def test_ids_are_bare_no_sigil(self):
        # import-sections stores section_id literally; a § would double on render.
        for e in self.manifest:
            self.assertNotIn("§", e["section_id"])
            self.assertNotIn("§", e.get("parent_section", ""))

    def test_front_matter_without_label_is_skipped(self):
        self.assertNotIn("scxml-abstract", self.m)

    def test_numeric_labels_keep_dots_and_strip_title(self):
        self.assertEqual(self.m["scxml-5.10"]["title"], "System Variables")
        self.assertEqual(self.m["scxml-5.10"]["parent_section"], "scxml-5")
        self.assertEqual(self.m["scxml-5.10.1"]["parent_section"], "scxml-5.10")

    def test_entity_decoded_title(self):
        self.assertEqual(self.m["scxml-5.2"]["title"], "<scxml>")

    def test_lettered_appendix_uses_hyphen(self):
        self.assertIn("scxml-A-1", self.m)
        self.assertEqual(self.m["scxml-A-1"]["parent_section"], "scxml-A")
        self.assertNotIn("parent_section", self.m["scxml-A"])  # appendix root

    def test_appendix_d_root_carries_letter(self):
        self.assertEqual(self.m["scxml-D"]["title"], "Algorithm for SCXML Interpretation")

    def test_unnumbered_appendix_d_helper_uses_anchor(self):
        self.assertEqual(self.m["scxml-D-interpret"]["title"], "procedure interpret(scxml,id)")
        self.assertEqual(
            self.m["scxml-D-interpret"]["parent_section"], "scxml-D-ProceduresandFunctions"
        )

    def test_unnumbered_appendix_h2_roots_at_appendix_letter(self):
        self.assertEqual(self.m["scxml-D-InformalSemantics"]["parent_section"], "scxml-D")
        self.assertEqual(self.m["scxml-D-Algorithm"]["parent_section"], "scxml-D")

    def test_anchor_map_preserves_spec_anchor(self):
        self.assertEqual(
            self.anchors["scxml-5.10"]["anchor_url"],
            "https://www.w3.org/TR/scxml/#SystemVariables",
        )
        self.assertEqual(self.anchors["scxml-5.10"]["source_revision"], "REC-scxml-20150901")

    @unittest.skipUnless(SNAPSHOT.exists(), "vendored snapshot missing")
    def test_snapshot_naming_policy_is_self_consistent(self):
        # SCE naming-policy postcondition (NOT a copy of Mnemosyne's grammar):
        # a dotted leaf must be purely numeric; every other leaf is dot-free.
        manifest, _ = conv.convert(SNAPSHOT.read_text(encoding="utf-8"), "GENERATED.md", "rev")
        for e in manifest:
            leaf = e["section_id"][len("scxml-") :]
            if "." in leaf:
                self.assertRegex(leaf, r"^[0-9]+(\.[0-9]+)*$", e["section_id"])

    @unittest.skipUnless(SNAPSHOT.exists(), "vendored snapshot missing")
    def test_snapshot_parent_refs_resolve_and_no_duplicates(self):
        manifest, _ = conv.convert(SNAPSHOT.read_text(encoding="utf-8"), "GENERATED.md", "rev")
        ids = [e["section_id"] for e in manifest]
        self.assertEqual(len(ids), len(set(ids)), "duplicate section ids")
        idset = set(ids)
        for e in manifest:
            if "parent_section" in e:
                self.assertIn(e["parent_section"], idset, e["section_id"])


@unittest.skipUnless(MNEMOSYNE_CLI, skip_reason())
@unittest.skipUnless(SNAPSHOT.exists(), "vendored snapshot missing")
class ClosedLoopTest(unittest.TestCase):
    """A1 manifest -> real import-sections -> §-cited in code -> validate-code-refs.
    The id grammar SSOT is Mnemosyne; this proves a representative slice survives
    the actual extractor whole (no truncation)."""

    def test_full_pipeline_no_truncation(self):
        manifest, _ = conv.convert(SNAPSHOT.read_text(encoding="utf-8"), "GENERATED.md", "rev")
        ids = _by_id(manifest)
        sample = [
            "scxml-5.10", "scxml-6.2.6", "scxml-5.10.1", "scxml-A-1",
            "scxml-B-2-11", "scxml-D-interpret", "scxml-D-getEffectiveTargetStates",
        ]
        for sid in sample:
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
                'paths = ["src/"]\nseverity_missing = "warn"\nseverity_binding = "warn"\n'
                "comment_only = true\n",
                encoding="utf-8",
            )
            subprocess.run(
                [MNEMOSYNE_CLI, "import-sections", "--manifest", "manifest.json"],
                cwd=root, check=True, capture_output=True,
            )
            # Citation form carries the § sigil; the stored id is bare.
            cites = "\n".join(f"// cite §{sid}" for sid in sample)
            (root / "src" / "cite.rs").write_text(cites + "\n", encoding="utf-8")

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
            self.assertEqual(missing, set(), f"extractor truncated/missed ids: {missing}")


if __name__ == "__main__":
    unittest.main()
