#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Regression tests for the protocol-synthesis RFC -> Mnemosyne manifest
converter (`synth` namespace, docs/spec/synth).

  * UnitTests       — heading extraction across the four section shapes
                      (h2/§-optional h3/h4/bold 5.J items), lettered-dot
                      hyphenation, appendix headings, fence skipping,
                      hierarchy emission, and a real-doc invariant pass.
  * ClosedLoopTest  — imports the real manifest into a
                      section_namespace="synth" workspace and proves
                      §synth-<id> cites resolve whole while a foreign
                      §scxml cite is skipped by namespace scope.

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

import synth_rfc_to_manifest as conv  # noqa: E402

SYNTH_RFC = REPO_ROOT / "docs" / "spec" / "synth" / "rfc-sce-protocol-synthesis.md"

FRAGMENT = """\
# RFC — SCE Forge Extensions for Wire Protocol Synthesis

## §2 Non-goals

### §2.1 Permanent non-goals

### 2.2 MVP deferrals (with migration paths)

## §5 Proposed extensions

### 5.B Codec DSL extensions

### 5.J Codegen backend coverage

**5.J.2 Statechart Rust `no_std` variant.** Scope is the statechart runtime.

1. **Stub trap symbols** — an ordered-list bold item, not a subsection.

### Shape of existing `Codec` kind

```text
## §9 not-a-heading inside a fence
### 5.Z fenced
```

## §6 Cross-cutting concerns

### 6.2 Testing requirements (new)

#### §6.2.6 Generated source drift detection

## Appendix B — Worked example: VLE ZInt u64
"""


def _by_id(manifest):
    return {e["section_id"]: e for e in manifest}


class UnitTests(unittest.TestCase):
    def setUp(self):
        self.manifest = conv.convert(FRAGMENT, "synth")
        self.by_id = _by_id(self.manifest)

    def test_all_shapes_extracted(self):
        self.assertEqual(
            sorted(self.by_id),
            [
                "synth-2",
                "synth-2.1",
                "synth-2.2",
                "synth-5",
                "synth-5-B",
                "synth-5-J",
                "synth-5-J-2",
                "synth-6",
                "synth-6.2",
                "synth-6.2.6",
                "synth-B",
            ],
        )

    def test_optional_sigil_on_h3(self):
        # "### §2.1" and "### 2.2" both extract; the sigil never enters the id.
        self.assertEqual(self.by_id["synth-2.1"]["title"], "Permanent non-goals")
        self.assertEqual(
            self.by_id["synth-2.2"]["title"], "MVP deferrals (with migration paths)"
        )

    def test_lettered_dots_hyphenated_numeric_kept(self):
        # Extractor token rule: a dot survives only between digits.
        self.assertIn("synth-5-B", self.by_id)  # 5.B  -> 5-B
        self.assertIn("synth-5-J-2", self.by_id)  # 5.J.2 -> 5-J-2
        self.assertIn("synth-6.2.6", self.by_id)  # digits keep dots

    def test_bold_item_title_strips_sentence_period(self):
        self.assertEqual(
            self.by_id["synth-5-J-2"]["title"], "Statechart Rust `no_std` variant"
        )

    def test_ordered_list_bold_item_not_claimed(self):
        for sid in self.by_id:
            self.assertNotIn("Stub", self.by_id[sid]["title"])

    def test_unnumbered_h3_and_fenced_headings_skipped(self):
        self.assertNotIn("synth-9", self.by_id)
        self.assertNotIn("synth-5-Z", self.by_id)
        for sid in self.by_id:
            self.assertNotIn("Shape of existing", self.by_id[sid]["title"])

    def test_appendix_heading(self):
        self.assertEqual(self.by_id["synth-B"]["title"], "Worked example: VLE ZInt u64")
        self.assertNotIn("parent_section", self.by_id["synth-B"])

    def test_hierarchy(self):
        self.assertEqual(self.by_id["synth-2.1"]["parent_section"], "synth-2")
        self.assertEqual(self.by_id["synth-5-B"]["parent_section"], "synth-5")
        self.assertEqual(self.by_id["synth-5-J-2"]["parent_section"], "synth-5-J")
        self.assertEqual(self.by_id["synth-6.2.6"]["parent_section"], "synth-6.2")
        self.assertNotIn("parent_section", self.by_id["synth-5"])

    def test_self_check_clean(self):
        self.assertIsNone(conv.self_check(self.manifest))

    def test_self_check_rejects_dotted_lettered_id(self):
        bad = [{"section_id": "synth-5.B", "parent_doc": "synth", "title": "x"}]
        self.assertIn("non-citation-safe", conv.self_check(bad))

    def test_self_check_rejects_orphan_parent(self):
        bad = [
            {
                "section_id": "synth-5-B",
                "parent_doc": "synth",
                "title": "x",
                "parent_section": "synth-5",
            }
        ]
        self.assertIn("not emitted", conv.self_check(bad))


class RealDocTests(unittest.TestCase):
    @unittest.skipUnless(SYNTH_RFC.exists(), "snapshot not vendored")
    def test_real_doc_invariants(self):
        manifest = conv.convert(SYNTH_RFC.read_text(encoding="utf-8"), "synth")
        by_id = _by_id(manifest)
        self.assertIsNone(conv.self_check(manifest))
        # Every label family SCE code cites must resolve.
        for letter in "ABCDEFGHIJKLMNO":
            self.assertIn(f"synth-5-{letter}", by_id)
        for n in range(1, 6):
            self.assertIn(f"synth-5-J-{n}", by_id)
        for n in range(1, 7):
            self.assertIn(f"synth-6.2.{n}", by_id)
        for sid in ("synth-A", "synth-B", "synth-C", "synth-7", "synth-3"):
            self.assertIn(sid, by_id)


MNEMOSYNE_CLI = shutil.which("mnemosyne-cli")


class ClosedLoopTest(unittest.TestCase):
    """Real-manifest import into a synth-namespaced workspace: §synth-<id>
    cites resolve whole; a foreign §scxml-<id> cite is skipped by scope."""

    @unittest.skipUnless(
        MNEMOSYNE_CLI and SYNTH_RFC.exists(), "mnemosyne-cli or snapshot unavailable"
    )
    def test_synth_cites_resolve_and_foreign_skipped(self):
        manifest = conv.convert(SYNTH_RFC.read_text(encoding="utf-8"), "synth")
        with tempfile.TemporaryDirectory() as td:
            ws = Path(td)
            (ws / "src").mkdir()
            (ws / "src" / "lib.rs").write_text(
                "// RFC §synth-5-B framed codec layout per §synth-6.2.6.\n"
                "// Statechart no_std split: §synth-5-J-2.\n"
                "// Foreign (out of scope here): §scxml-3.3.\n"
                "pub fn f() {}\n",
                encoding="utf-8",
            )
            (ws / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
            (ws / "mnemosyne.toml").write_text(
                "[workspace]\n\n"
                '[schema]\nanchor_convention = "section_number"\n'
                'medium_name = "spec_mirror"\n\n'
                "[plugins.set_equality_validator]\n"
                'paths = ["src"]\n'
                'severity_missing = "reject"\n'
                'severity_binding = "warn"\n'
                'severity_coverage = "warn"\n'
                "comment_only = true\n"
                'section_namespace = "synth"\n',
                encoding="utf-8",
            )
            run = lambda *a: subprocess.run(  # noqa: E731
                [MNEMOSYNE_CLI, *a], cwd=ws, capture_output=True, text=True
            )
            imp = run("import-sections", "--manifest", "manifest.json")
            self.assertEqual(imp.returncode, 0, imp.stderr + imp.stdout)
            chk = run("validate-code-refs")
            self.assertEqual(chk.returncode, 0, chk.stderr + chk.stdout)
            # A hallucinated synth cite must reject.
            (ws / "src" / "lib.rs").write_text(
                "// RFC §synth-5-Q does not exist.\npub fn f() {}\n", encoding="utf-8"
            )
            bad = run("validate-code-refs")
            self.assertNotEqual(bad.returncode, 0, bad.stderr + bad.stdout)


if __name__ == "__main__":
    unittest.main()
