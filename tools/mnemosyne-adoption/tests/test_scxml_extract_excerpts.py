#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Regression tests for the R2 excerpt extractor and apply driver.

  * UnitTests       — body-extraction logic against a fixed HTML fragment.
  * SnapshotTests   — invariants over the vendored snapshot (every excerpt id
                      is a real A1 section, anchors absolute, text non-blank).
  * ApplyClosedLoop — extract -> import -> apply_excerpts -> query returns the
                      excerpt. Skipped when mnemosyne-cli is absent.

No Mnemosyne dependency for the first two layers.

Run:  python3 -m unittest discover -s tools/mnemosyne-adoption/tests
"""

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL_DIR = HERE.parent
sys.path.insert(0, str(TOOL_DIR))

import scxml_extract_excerpts as ex  # noqa: E402
import scxml_toc_to_manifest as a1  # noqa: E402

SNAPSHOT = TOOL_DIR / "spec-snapshot" / "scxml-REC-20150901.html"
APPLY = TOOL_DIR / "apply_excerpts.py"

# Headings with body prose between them; one container heading (A) with no
# direct body, only a subsection (A.1).
FRAGMENT = """
<h2 id="data-module">5 Data Model</h2>
<p>Intro prose for section 5.</p>
<h3 id="SystemVariables">5.10 System Variables</h3>
<p>The Processor MUST maintain &lt;_event&gt;.</p>
<h2 id="conformance">A Conformance</h2>
<h3 id="ConformingDocuments">A.1 Docs</h3>
<p>A doc is conformant if it matches the schema.</p>
"""


class UnitTests(unittest.TestCase):
    def setUp(self):
        self.e = ex.extract(FRAGMENT, "REC-scxml-20150901")

    def test_body_between_headings(self):
        self.assertEqual(self.e["scxml-5"]["text"], "Intro prose for section 5.")
        self.assertEqual(self.e["scxml-A-1"]["text"], "A doc is conformant if it matches the schema.")

    def test_entity_decoded_and_tags_stripped(self):
        self.assertEqual(self.e["scxml-5.10"]["text"], "The Processor MUST maintain <_event>.")

    def test_container_without_direct_body_is_omitted(self):
        # "A Conformance" has only a subsection between it and A.1 -> no excerpt.
        self.assertNotIn("scxml-A", self.e)

    def test_anchor_url_and_revision(self):
        self.assertEqual(
            self.e["scxml-5.10"]["anchor_url"], "https://www.w3.org/TR/scxml/#SystemVariables"
        )
        self.assertEqual(self.e["scxml-5.10"]["source_revision"], "REC-scxml-20150901")


@unittest.skipUnless(SNAPSHOT.exists(), "vendored snapshot missing")
class SnapshotTests(unittest.TestCase):
    def setUp(self):
        html = SNAPSHOT.read_text(encoding="utf-8")
        self.excerpts = ex.extract(html, "REC-scxml-20150901")
        manifest, _ = a1.convert(html, "GENERATED.md", "REC-scxml-20150901")
        self.section_ids = {e["section_id"] for e in manifest}

    def test_every_excerpt_is_a_real_section(self):
        # Extractor reuses A1's id SSOT -> every key must be a known section.
        for sid in self.excerpts:
            self.assertIn(sid, self.section_ids, sid)

    def test_excerpts_well_formed(self):
        for sid, e in self.excerpts.items():
            self.assertTrue(e["text"].strip(), f"{sid} blank text")
            self.assertTrue(e["anchor_url"].startswith("https://www.w3.org/TR/scxml/#"), sid)
            self.assertTrue(e["source_revision"], sid)

    def test_known_section_text_prefix(self):
        self.assertTrue(self.excerpts["scxml-5.10"]["text"].startswith("[This section is normative.]"))


@unittest.skipUnless(shutil.which("mnemosyne-cli"), "mnemosyne-cli not on PATH")
@unittest.skipUnless(SNAPSHOT.exists(), "vendored snapshot missing")
class ApplyClosedLoop(unittest.TestCase):
    def test_apply_then_query_returns_excerpt(self):
        html = SNAPSHOT.read_text(encoding="utf-8")
        manifest, _ = a1.convert(html, "GENERATED.md", "REC-scxml-20150901")
        all_excerpts = ex.extract(html, "REC-scxml-20150901")
        subset = {k: all_excerpts[k] for k in ("scxml-5.10", "scxml-D-interpret")}

        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
            (root / "excerpts.json").write_text(json.dumps(subset), encoding="utf-8")
            (root / "mnemosyne.toml").write_text(
                '[workspace]\ndocs = ["GENERATED.md"]\ndefault_doc = "GENERATED.md"\n\n'
                "[atomic]\n"
                'sidecar_path = ".atomic/workspace.atomic.json"\noutput_path = "GENERATED.md"\n',
                encoding="utf-8",
            )
            subprocess.run(
                ["mnemosyne-cli", "import-sections", "--manifest", "manifest.json"],
                cwd=root, check=True, capture_output=True,
            )
            r = subprocess.run(
                [sys.executable, str(APPLY), "--excerpts", "excerpts.json"],
                cwd=root, capture_output=True, text=True,
            )
            self.assertEqual(r.returncode, 0, r.stderr)
            out = subprocess.run(
                ["mnemosyne-cli", "query", "§scxml-5.10", "--json"],
                cwd=root, check=True, capture_output=True, text=True,
            )
            ne = json.loads(out.stdout).get("normative_excerpt") or {}
            self.assertTrue(ne.get("text", "").startswith("[This section is normative.]"), ne)


if __name__ == "__main__":
    unittest.main()
