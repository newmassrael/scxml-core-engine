#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Regression tests for the SCE_MESH.md -> Mnemosyne manifest converter (the
markdown sibling of the A1 W3C converter; mesh design-ledger namespace).

Two layers:
  * UnitTests       — converter logic against a fixed markdown fragment and
                      against the real SCE_MESH.md. No Mnemosyne dependency.
  * ClosedLoopTest  — imports the real manifest via `mnemosyne-cli
                      import-sections` into a section_namespace="mesh"
                      workspace, then proves (a) §mesh-<n> cites resolve whole
                      and (b) a foreign §scxml-<n> cite in the same file is
                      skipped by namespace scoping, not flagged missing.
                      Skipped when mnemosyne-cli is absent.

Section ids in the manifest are BARE (no § sigil); the § is the citation form.

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
REPO_ROOT = TOOL_DIR.parent.parent
sys.path.insert(0, str(TOOL_DIR))

import sce_mesh_md_to_manifest as conv  # noqa: E402

MESH_MD = REPO_ROOT / "SCE_MESH.md"

# A numbered top-level, a numbered subsection (no trailing dot), a deep nested
# heading, an un-numbered heading (skipped), and a fenced block whose in-code
# `#### 99.` line must NOT parse as a heading.
FRAGMENT = """\
# SCE Mesh: Distributed SCXML State Machine Framework

## 1. Vision

### Problem

### 3.1 Scheduler — When to Execute

#### 9.6.1 Session establishment

## 16. Distributed W3C SCXML Conformance

### 16.7 `error.communication` raise policy

```yaml
# deploy.yaml excerpt
#### 99. Not A Heading (inside a code fence)
```

### Rationale
"""


def _by_id(manifest):
    return {e["section_id"]: e for e in manifest}


class UnitTests(unittest.TestCase):
    def setUp(self):
        self.manifest = conv.convert(FRAGMENT, "GENERATED.md")
        self.m = _by_id(self.manifest)

    def test_ids_are_bare_no_sigil(self):
        for e in self.manifest:
            self.assertNotIn("§", e["section_id"])
            self.assertNotIn("§", e.get("parent_section", ""))

    def test_top_level_label_with_trailing_dot(self):
        # "## 1. Vision" — the dot after the number is the label terminator.
        self.assertEqual(self.m["mesh-1"]["title"], "Vision")
        self.assertNotIn("parent_section", self.m["mesh-1"])

    def test_subsection_without_trailing_dot(self):
        # "### 3.1 Scheduler — ..." — no dot after the number.
        self.assertEqual(self.m["mesh-3.1"]["title"], "Scheduler — When to Execute")
        self.assertEqual(self.m["mesh-3.1"]["parent_section"], "mesh-3")

    def test_parent_is_label_derived_not_nesting_derived(self):
        # 9.6.1 nested directly under a ## 1 in the fragment, yet its parent is
        # mesh-9.6 (from the label), proving derivation does not depend on the
        # document's heading nesting.
        self.assertEqual(self.m["mesh-9.6.1"]["parent_section"], "mesh-9.6")

    def test_unnumbered_headings_skipped(self):
        # "### Problem" / "### Rationale" carry no number -> no citation target.
        titles = [e["title"] for e in self.manifest]
        self.assertNotIn("Problem", titles)
        self.assertNotIn("Rationale", titles)

    def test_fenced_code_line_not_parsed_as_heading(self):
        self.assertNotIn("mesh-99", self.m)

    def test_backtick_title_preserved(self):
        self.assertEqual(
            self.m["mesh-16.7"]["title"], "`error.communication` raise policy"
        )

    @unittest.skipUnless(MESH_MD.exists(), "SCE_MESH.md missing")
    def test_real_doc_ids_numeric_and_citation_safe(self):
        manifest = conv.convert(MESH_MD.read_text(encoding="utf-8"), "GENERATED.md")
        self.assertGreater(len(manifest), 0)
        for e in manifest:
            leaf = e["section_id"][len("mesh-") :]
            # mesh leaves are purely numeric -> every dot is digit-flanked, the
            # condition the §-citation extractor requires to keep the dot.
            self.assertRegex(leaf, r"^[0-9]+(\.[0-9]+)*$", e["section_id"])

    @unittest.skipUnless(MESH_MD.exists(), "SCE_MESH.md missing")
    def test_real_doc_parent_refs_resolve_no_dups(self):
        manifest = conv.convert(MESH_MD.read_text(encoding="utf-8"), "GENERATED.md")
        ids = [e["section_id"] for e in manifest]
        self.assertEqual(len(ids), len(set(ids)), "duplicate section ids")
        idset = set(ids)
        for e in manifest:
            if "parent_section" in e:
                self.assertIn(e["parent_section"], idset, e["section_id"])


@unittest.skipUnless(shutil.which("mnemosyne-cli"), "mnemosyne-cli not on PATH")
@unittest.skipUnless(MESH_MD.exists(), "SCE_MESH.md missing")
class ClosedLoopTest(unittest.TestCase):
    """mesh manifest -> real import-sections (namespace=mesh) -> §mesh cited in
    code -> validate-code-refs. Proves the mesh ids survive the extractor whole
    AND that a foreign §scxml cite is skipped (not flagged) by namespace scope."""

    def test_mesh_cites_resolve_and_foreign_namespace_skipped(self):
        manifest = conv.convert(MESH_MD.read_text(encoding="utf-8"), "GENERATED.md")
        ids = _by_id(manifest)
        sample = ["mesh-16.7", "mesh-9.6.1", "mesh-9.6.2", "mesh-10.7.1", "mesh-14.4"]
        for sid in sample:
            self.assertIn(sid, ids, "fixture id missing from real manifest")

        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "src").mkdir()
            (root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
            (root / "mnemosyne.toml").write_text(
                '[workspace]\ndocs = ["GENERATED.md"]\ndefault_doc = "GENERATED.md"\n\n'
                "[atomic]\n"
                'sidecar_path = ".atomic/workspace.atomic.json"\noutput_path = "GENERATED.md"\n\n'
                "[plugins.set_equality_validator]\n"
                'paths = ["src/"]\nseverity_missing = "reject"\nseverity_binding = "warn"\n'
                'comment_only = true\nsection_namespace = "mesh"\n',
                encoding="utf-8",
            )
            subprocess.run(
                ["mnemosyne-cli", "import-sections", "--manifest", "manifest.json"],
                cwd=root, check=True, capture_output=True,
            )
            cites = [f"// cite §{sid}" for sid in sample]
            # A foreign-namespace cite: no mesh section, must be SKIPPED by the
            # section_namespace="mesh" scope rather than reported missing.
            cites.append("// foreign §scxml-5.10 (skipped by namespace scope)")
            (root / "src" / "cite.rs").write_text("\n".join(cites) + "\n", encoding="utf-8")

            out = subprocess.run(
                ["mnemosyne-cli", "validate-code-refs", "--json"],
                cwd=root, check=True, capture_output=True, text=True,
            )
            report = json.loads(out.stdout)
            missing = {
                v["entry_id"]
                for v in report.get("violations", [])
                if v.get("kind") == "section_missing"
            }
            self.assertEqual(missing, set(), f"unexpected missing ids: {missing}")

    def test_hallucinated_mesh_cite_is_rejected(self):
        manifest = conv.convert(MESH_MD.read_text(encoding="utf-8"), "GENERATED.md")
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "src").mkdir()
            (root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
            (root / "mnemosyne.toml").write_text(
                '[workspace]\ndocs = ["GENERATED.md"]\ndefault_doc = "GENERATED.md"\n\n'
                "[atomic]\n"
                'sidecar_path = ".atomic/workspace.atomic.json"\noutput_path = "GENERATED.md"\n\n'
                "[plugins.set_equality_validator]\n"
                'paths = ["src/"]\nseverity_missing = "reject"\nseverity_binding = "warn"\n'
                'comment_only = true\nsection_namespace = "mesh"\n',
                encoding="utf-8",
            )
            subprocess.run(
                ["mnemosyne-cli", "import-sections", "--manifest", "manifest.json"],
                cwd=root, check=True, capture_output=True,
            )
            # mesh-999.999 is in the mesh namespace but not a real section.
            (root / "src" / "cite.rs").write_text(
                "// cite §mesh-999.999 (hallucinated)\n", encoding="utf-8"
            )
            proc = subprocess.run(
                ["mnemosyne-cli", "validate-code-refs"],
                cwd=root, capture_output=True, text=True,
            )
            self.assertNotEqual(proc.returncode, 0, "hallucinated mesh cite should reject")


if __name__ == "__main__":
    unittest.main()
