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


class SlashChain(unittest.TestCase):
    def test_chain_each_member_migrated(self):
        new, migs, _ = plan("// per W3C SCXML 3.13/5.10 both apply\n")
        self.assertIn("§scxml-3.13 / §scxml-5.10", new)
        self.assertEqual([m["id"] for m in migs], ["scxml-3.13", "scxml-5.10"])

    def test_io_abbreviation_not_matched(self):
        # "I/O" must not be read as a citation chain (I is not a label).
        src = "// uses W3C SCXML I/O processor\n"
        new, migs, skipped = plan(src)
        self.assertEqual(new, src)
        self.assertEqual(migs, [])
        self.assertEqual(skipped, [])

    def test_chain_with_one_missing_member_left_whole(self):
        # 5.10 is in LEDGER, 9.99 is not -> whole chain stays prose, 9.99 reported.
        new, migs, skipped = plan("// W3C SCXML 5.10/9.99 mixed\n")
        self.assertIn("W3C SCXML 5.10/9.99", new)  # untouched
        self.assertEqual(migs, [])
        self.assertEqual([s["label"] for s in skipped], ["9.99"])


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
                "// W3C SCXML 6.2/C.2 slash chain\n"
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


class TestBareSigilForm(unittest.TestCase):
    """The bare-sigil shape: a § sigil already present after a W3C marker."""

    def test_w3c_sigil_migrated(self):
        new, migs, skips = plan("// W3C §5.10 inline\n")
        self.assertIn("§scxml-5.10", new)
        self.assertEqual(len(migs), 1)
        self.assertEqual(migs[0]["id"], "scxml-5.10")

    def test_w3c_scxml_sigil_migrated(self):
        new, migs, skips = plan("// W3C SCXML §6.2 requires\n")
        self.assertIn("§scxml-6.2", new)
        self.assertNotIn("W3C", new)
        self.assertEqual(len(migs), 1)

    def test_sigil_drops_marker(self):
        new, migs, skips = plan("// per W3C SCXML §6.2 x\n")
        self.assertEqual(new, "// per §scxml-6.2 x\n")

    def test_rfc_sigil_not_touched(self):
        # A bare § with no W3C marker is an SCE-internal design-doc ref.
        new, migs, skips = plan("// per RFC §3 design\n")
        self.assertEqual(len(migs), 0)
        self.assertEqual(len(skips), 0)
        self.assertIn("RFC §3", new)

    def test_design_doc_sigil_not_touched(self):
        new, migs, skips = plan("// see `SCE_FORGE.md` §3.1 policy\n")
        self.assertEqual(len(migs), 0)
        self.assertNotIn("scxml-3", new)

    def test_quoted_sigil_left_verbatim(self):
        # A quoted W3C citation inside a comment is a runtime string value
        # being shown, not a section citation -> leave verbatim, report it.
        new, migs, skips = plan('// emits "W3C SCXML §6.2" field\n')
        self.assertEqual(len(migs), 0)
        self.assertEqual(len(skips), 1)
        self.assertIn('"W3C SCXML §6.2"', new)

    def test_sigil_in_string_literal_untouched(self):
        new, migs, skips = plan('const char* s = "W3C SCXML §6.2";\n')
        self.assertEqual(len(migs), 0)
        self.assertIn('"W3C SCXML §6.2"', new)

    def test_sigil_ledger_miss_left_as_prose(self):
        new, migs, skips = plan("// W3C §9.99 bogus\n")
        self.assertEqual(len(migs), 0)
        self.assertEqual(len(skips), 1)

    def test_sigil_slash_chain(self):
        # 3.13 and 5.10 are both in the test LEDGER above.
        new, migs, skips = plan("// W3C SCXML §3.13/5.10 pair\n")
        self.assertIn("§scxml-3.13 / §scxml-5.10", new)
        self.assertEqual(len(migs), 2)

    def test_prose_and_sigil_coexist(self):
        new, migs, skips = plan("// W3C SCXML 6.2 and W3C §5.10\n")
        self.assertIn("§scxml-6.2", new)
        self.assertIn("§scxml-5.10", new)
        self.assertEqual(len(migs), 2)


MESH_LEDGER = {
    "mesh-16.7",
    "mesh-9.6",
    "mesh-9.6.2",
    "mesh-10.5",
    "mesh-6.4",  # Custom Transport — collides with W3C §6.4 <invoke>
    "mesh-6.3",
    "mesh-1",
    "mesh-3",
}
# Sibling scxml ledger for the cross-namespace ambiguity guard.
MESH_EXCLUDE = {"scxml-6.4", "scxml-6.3", "scxml-1", "scxml-3", "scxml-5.5"}


def mesh_plan(text, name="f.h"):
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, name)
        with open(p, "w", encoding="utf-8") as fh:
            fh.write(text)
        return mc.plan_file(
            p, MESH_LEDGER, namespace="mesh", exclude_ledger_ids=MESH_EXCLUDE
        )


class MeshNamespace(unittest.TestCase):
    def test_mesh_exclusive_bare_sigil_migrated(self):
        new, migs, _ = mesh_plan("// see §9.6.2 envelope extensions\n")
        self.assertIn("§mesh-9.6.2", new)
        self.assertEqual(migs[0]["id"], "mesh-9.6.2")

    def test_sce_mesh_md_is_not_foreign(self):
        # SCE_MESH.md is the mesh source doc; ".md" before the sigil must NOT
        # mark it foreign (the regression that whole-line matching caused).
        new, migs, _ = mesh_plan("/// `SCE_MESH.md` §16.7 rows 1-13\n")
        self.assertIn("§mesh-16.7", new)
        self.assertEqual(len(migs), 1)

    def test_w3c_marked_sigil_not_claimed_by_mesh(self):
        # mesh-6.4 exists, but the W3C marker means this is a W3C cite.
        new, migs, skips = mesh_plan("// per W3C §6.4 invoke contract\n")
        self.assertEqual(migs, [])
        self.assertIn("W3C §6.4", new)
        self.assertTrue(skips[0]["reason"].startswith("foreign"))

    def test_ietf_rfc_not_claimed(self):
        new, migs, skips = mesh_plan("// RFC 9562 §5.7 layout (big-endian)\n")
        self.assertEqual(migs, [])
        self.assertIn("RFC 9562 §5.7", new)

    def test_cross_namespace_ambiguous_reported_not_migrated(self):
        # §6.4 with no marker: in BOTH ledgers -> ambiguous, manual review.
        new, migs, skips = mesh_plan("// done.invoke contract fires in §6.4 only\n")
        self.assertEqual(migs, [])
        self.assertIn("§6.4", new)
        self.assertEqual(len(skips), 1)
        self.assertIn("ambiguous", skips[0]["reason"])

    def test_proximity_earlier_foreign_cite_does_not_poison_later_mesh(self):
        # An earlier "W3C §5.5" on the line must not disqualify the later mesh §9.6.
        new, migs, skips = mesh_plan("// W3C §5.5 donedata and §9.6 remote invoke\n")
        self.assertIn("§mesh-9.6", new)
        self.assertEqual([m["id"] for m in migs], ["mesh-9.6"])
        self.assertIn("W3C §5.5", new)  # the W3C cite stays bare for the scxml path

    def test_hyphen_glued_suffix_refused(self):
        # "§16.7-L3500" must not migrate: "§mesh-16.7-L3500" would read as one
        # bad id. The source has to space-separate the line back-reference.
        new, migs, skips = mesh_plan("// the pre-§16.7-L3500 primitive\n")
        self.assertEqual(migs, [])
        self.assertIn("§16.7-L3500", new)
        self.assertIn("glued suffix", skips[0]["reason"])

    def test_hallucinated_mesh_number_reported(self):
        new, migs, skips = mesh_plan("// §16.99 invented section\n")
        self.assertEqual(migs, [])
        self.assertIn("non-section", skips[0]["reason"])

    def test_string_literal_sigil_untouched(self):
        src = 'const char *k = "§16.7";\n'
        new, migs, _ = mesh_plan(src)
        self.assertEqual(new, src)
        self.assertEqual(migs, [])

    def test_mesh_idempotent(self):
        once, _, _ = mesh_plan("// §16.7 and §9.6\n")
        twice, migs, _ = mesh_plan(once)
        self.assertEqual(once, twice)
        self.assertEqual(migs, [])


if __name__ == "__main__":
    unittest.main()
