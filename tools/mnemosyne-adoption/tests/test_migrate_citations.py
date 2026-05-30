#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Regression tests for the R3 citation migrator (migrate_citations.py).

Covers: numeric/lettered normalization, prose left untouched, comment-only
scope (string/char literals never edited), non-ledger labels reported not
migrated, nested Rust block comments, and idempotency.

Run:  python3 -m unittest discover -s tools/mnemosyne-adoption/tests
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL_DIR = HERE.parent
sys.path.insert(0, str(TOOL_DIR))

import migrate_citations as mc  # noqa: E402

MIGRATE = TOOL_DIR / "migrate_citations.py"
HAVE_CLI = shutil.which("mnemosyne-cli") is not None

# A representative ledger id set (the real ledger is larger; these suffice).
LEDGER = {
    "scxml-3.13",
    "scxml-5.10",
    "scxml-6.2",
    "scxml-6.3",
    "scxml-6.4",
    "scxml-5.3",
    "scxml-C-2",
    "scxml-C-1",
    "scxml-B-2",
    "scxml-5.10.1",
}


def plan(text, name="f.h"):
    """Write text to a temp file with the given extension and return
    (new_text, migrations, skipped)."""
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, name)
        with open(p, "w", encoding="utf-8") as fh:
            fh.write(text)
        return mc.plan_file(p, LEDGER, "W3C SCXML")


class LabelPolicy(unittest.TestCase):
    def test_numeric_keeps_dots(self):
        self.assertEqual(mc.label_to_id("6.2"), "scxml-6.2")
        self.assertEqual(mc.label_to_id("5.10.1"), "scxml-5.10.1")

    def test_lettered_dots_to_hyphens(self):
        self.assertEqual(mc.label_to_id("C.2"), "scxml-C-2")
        self.assertEqual(mc.label_to_id("B.2"), "scxml-B-2")


class CommentScope(unittest.TestCase):
    def test_line_comment_migrated(self):
        new, migs, _ = plan("// W3C SCXML 6.2: delayed send\n")
        self.assertIn("§scxml-6.2", new)
        self.assertEqual(migs[0]["id"], "scxml-6.2")

    def test_block_doc_comment_migrated(self):
        src = " * W3C SCXML C.2 BasicHTTP Event I/O Processor:\n"
        src = "/*\n" + src + " */\n"
        new, migs, _ = plan(src)
        self.assertIn("§scxml-C-2", new)
        # the prose word after the label is untouched
        self.assertIn("BasicHTTP Event I/O Processor", new)

    def test_string_literal_never_touched(self):
        # Same text inside a string literal must survive verbatim.
        src = 'const char *e = "W3C SCXML 6.2 compliance";\n'
        new, migs, _ = plan(src)
        self.assertEqual(new, src)
        self.assertEqual(migs, [])

    def test_char_literal_apostrophe_does_not_desync(self):
        # An apostrophe in a comment must not open a char-literal state.
        src = "// it's per W3C SCXML 5.10 here\n"
        new, _, _ = plan(src)
        self.assertIn("§scxml-5.10", new)


class ProseUntouched(unittest.TestCase):
    def test_word_led_mentions_left(self):
        for word in ("BasicHTTPEventProcessor", "Platform", "specification", "compliant"):
            src = f"// W3C SCXML {word} note\n"
            new, migs, skipped = plan(src)
            self.assertEqual(new, src, word)
            self.assertEqual(migs, [], word)
            self.assertEqual(skipped, [], word)

    def test_trailing_period_not_consumed(self):
        new, migs, _ = plan("// per W3C SCXML 6.2.\n")
        self.assertIn("§scxml-6.2.", new)
        self.assertEqual(migs[0]["label"], "6.2")


class LedgerGate(unittest.TestCase):
    def test_version_like_left_and_reported(self):
        # 1.0 is the spec *version*, not a section -> not in ledger.
        new, migs, skipped = plan("// SCXML version per W3C SCXML 1.0 spec\n")
        self.assertEqual(migs, [])
        self.assertEqual(len(skipped), 1)
        self.assertEqual(skipped[0]["label"], "1.0")
        self.assertEqual(new[: len("// SCXML")], "// SCXML")  # unchanged text

    def test_hallucinated_section_left_and_reported(self):
        new, migs, skipped = plan("// W3C SCXML 6.99 invented\n")
        self.assertEqual(migs, [])
        self.assertEqual(skipped[0]["id"], "scxml-6.99")


class RustNesting(unittest.TestCase):
    def test_nested_block_comment(self):
        src = "/* outer /* inner W3C SCXML 5.3 */ still W3C SCXML 6.2 */\n"
        new, migs, _ = plan(src, name="f.rs")
        ids = sorted(m["id"] for m in migs)
        self.assertEqual(ids, ["scxml-5.3", "scxml-6.2"])


class Idempotency(unittest.TestCase):
    def test_second_pass_is_noop(self):
        once, _, _ = plan("// W3C SCXML 6.2 and W3C SCXML C.2\n")
        twice, migs, _ = plan(once)
        self.assertEqual(once, twice)
        self.assertEqual(migs, [])


@unittest.skipUnless(HAVE_CLI, "mnemosyne-cli not on PATH")
class ValidateClosedLoop(unittest.TestCase):
    """migrate -> wire set_equality_validator -> validate-code-refs is green;
    a hallucinated citation makes the reject-severity gate fail."""

    def _workspace(self, root, src_rel):
        manifest = [
            {"section_id": "scxml-6.2", "parent_doc": "GENERATED.md", "title": "Send"},
            {"section_id": "scxml-C-2", "parent_doc": "GENERATED.md", "title": "BasicHTTP"},
        ]
        (root / "manifest.json").write_text(json.dumps(manifest), encoding="utf-8")
        (root / "mnemosyne.toml").write_text(
            '[workspace]\ndocs = ["GENERATED.md"]\ndefault_doc = "GENERATED.md"\n\n'
            "[atomic]\n"
            'sidecar_path = ".atomic/workspace.atomic.json"\noutput_path = "GENERATED.md"\n\n'
            "[plugins.set_equality_validator]\n"
            f'paths = ["{src_rel}"]\n'
            'severity_missing = "reject"\nseverity_binding = "warn"\ncomment_only = true\n',
            encoding="utf-8",
        )
        subprocess.run(
            ["mnemosyne-cli", "import-sections", "--manifest", "manifest.json"],
            cwd=root, check=True, capture_output=True,
        )

    def test_green_then_reject(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            src = root / "src"
            src.mkdir()
            f = src / "Sender.cpp"
            f.write_text(
                "// W3C SCXML 6.2: delayed send\n"
                "// W3C SCXML C.2 BasicHTTP processor\n"
                "// W3C SCXML specification compliance note\n",
                encoding="utf-8",
            )
            self._workspace(root, "src")
            store = root / ".atomic" / "workspace.atomic.json"

            # migrate against the temp ledger
            r = subprocess.run(
                [sys.executable, str(MIGRATE), str(f), "--ledger", str(store), "--apply"],
                capture_output=True, text=True,
            )
            self.assertEqual(r.returncode, 0, r.stderr)
            text = f.read_text(encoding="utf-8")
            self.assertIn("§scxml-6.2", text)
            self.assertIn("§scxml-C-2", text)
            self.assertIn("W3C SCXML specification", text)  # prose untouched

            # gate is green: no section_missing, exit 0
            out = subprocess.run(
                ["mnemosyne-cli", "validate-code-refs", "--json"],
                cwd=root, capture_output=True, text=True,
            )
            self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
            report = json.loads(out.stdout.splitlines()[0])
            self.assertEqual(report["section_missing_count"], 0)

            # a hallucinated citation fails the reject gate
            with f.open("a", encoding="utf-8") as fh:
                fh.write("// invented §scxml-9.99 reference\n")
            bad = subprocess.run(
                ["mnemosyne-cli", "validate-code-refs", "--json"],
                cwd=root, capture_output=True, text=True,
            )
            self.assertNotEqual(bad.returncode, 0)
            bad_report = json.loads(bad.stdout.splitlines()[0])
            self.assertEqual(bad_report["section_missing_count"], 1)


if __name__ == "__main__":
    unittest.main()
