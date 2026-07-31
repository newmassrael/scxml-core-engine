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
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL_DIR = HERE.parent
sys.path.insert(0, str(TOOL_DIR))

import migrate_citations as mc  # noqa: E402

from _mnemosyne_bin import MNEMOSYNE_CLI, skip_reason  # noqa: E402

MIGRATE = TOOL_DIR / "migrate_citations.py"
HAVE_CLI = MNEMOSYNE_CLI is not None

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


SYNTH_LEDGER = {"synth-5-B", "synth-7", "synth-6.2.6"}


def plan_synth(text, name="f.h"):
    """Plan `text` against the synth ledger (bare-sigil path, synth rules)."""
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, name)
        with open(p, "w", encoding="utf-8") as fh:
            fh.write(text)
        return mc.plan_file(p, SYNTH_LEDGER, "W3C SCXML", "synth")


class SynthDocMarker(unittest.TestCase):
    """A numeric synth label is claimable only under the marker that names the
    document. The marker is a bare string constant with no other reader, so a
    rename silently unclaims every numeric cite unless it is pinned here."""

    def test_numeric_claimed_under_document_marker(self):
        new, migs, _ = plan_synth("// SCE Protocol-Synthesis RFC §7 rollout\n")
        self.assertIn("§synth-7", new)
        self.assertEqual([m["id"] for m in migs], ["synth-7"])

    def test_numeric_without_marker_is_reported_not_claimed(self):
        src = "// RFC §7 rollout\n"
        new, migs, skipped = plan_synth(src)
        self.assertEqual(new, src)
        self.assertEqual(migs, [])
        self.assertIn("document-naming marker", skipped[0]["reason"])

    def test_other_document_marker_does_not_claim(self):
        # Only the SCE-owned document name claims a numeric label. Any other
        # document-naming prose in that position — including a marker this
        # constant used to carry — must leave the label for manual review,
        # since "RFC §<n>" names at least six documents in this tree.
        for marker in ("Legacy Upstream", "C11 Backend", "EventSchema"):
            src = f"// {marker} RFC §7 rollout\n"
            new, migs, skipped = plan_synth(src)
            self.assertEqual(new, src, marker)
            self.assertEqual(migs, [], marker)
            self.assertIn("document-naming marker", skipped[0]["reason"], marker)

    def test_lettered_label_claimable_without_marker(self):
        # Lettered labels are structurally unique to this RFC, so they never
        # needed the marker — pinned so the two paths stay distinguishable.
        new, migs, _ = plan_synth("// RFC §5.B codec DSL\n")
        self.assertIn("§synth-5-B", new)
        self.assertEqual([m["id"] for m in migs], ["synth-5-B"])


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

    def test_other_connectives_before_label_are_seen(self):
        # `Appendix` was only the first of these. `Section` hid 22 sites and
        # `specification` 14 — several inside directories a ledger enrolls, so the
        # form gate was blind there too. The connective set is closed; anything
        # else in that position is reported by the hidden-citation detector.
        for word in ("Section", "section", "specification"):
            new, migs, _ = plan(f"// per W3C SCXML {word} 5.10 note\n")
            self.assertEqual([m["id"] for m in migs], ["scxml-5.10"], word)
            self.assertIn("§scxml-5.10", new, word)

    def test_unknown_word_before_label_is_reported_not_guessed(self):
        # "W3C SCXML Algorithm C.1" — a real cite the main pattern cannot see.
        # Reported rather than migrated: the word may mean the author aimed at a
        # different section entirely (this exact shape was aimed at Appendix D).
        new, migs, skipped = plan("// W3C SCXML Algorithm C.2 conflict\n")
        self.assertEqual(migs, [])
        self.assertEqual(new, "// W3C SCXML Algorithm C.2 conflict\n")
        self.assertEqual(
            [s["label"] for s in skipped if s["reason"].startswith("citation hidden")],
            ["C.2"],
        )

    def test_test_number_after_word_is_not_a_hidden_citation(self):
        # "W3C SCXML test 530" is a W3C IRP test id, not a section. A bare
        # integer must never be reported as a hidden citation.
        _, migs, skipped = plan("// W3C SCXML test 530: content expr\n")
        self.assertEqual(migs, [])
        self.assertEqual(
            [s for s in skipped if s["reason"].startswith("citation hidden")], []
        )

    def test_appendix_word_before_label_is_seen(self):
        # "W3C SCXML Appendix C.2" put the word in the label position, so the
        # citation matched NOTHING — neither migrated nor reported — and a
        # fabricated appendix subsection passed every gate. Measured: 376 sites
        # cited "Appendix D.2" while D.2 is not a ledger section.
        new, migs, _ = plan("// per W3C SCXML Appendix C.2 processor\n")
        self.assertIn("§scxml-C-2", new)
        self.assertEqual([m["id"] for m in migs], ["scxml-C-2"])
        # The word is consumed with the label — the §id already names the appendix.
        self.assertNotIn("Appendix", new)

    def test_appendix_label_absent_from_ledger_is_reported(self):
        new, migs, skipped = plan("// W3C SCXML Appendix D.2 algorithm\n")
        self.assertEqual(migs, [])
        self.assertEqual([s["label"] for s in skipped], ["D.2"])
        self.assertIn("Appendix D.2", new)  # left for a human to resolve

    def test_trailing_slash_prose_refused(self):
        # "3.13/Appendix D" is a label followed by PROSE, not a chain member.
        # Migrating the head would emit "§scxml-3.13/Appendix D", which the
        # validator reads as one id ("scxml-3.13/Appendix") and reports as a
        # section nobody cited. Refuse instead: the prose stays, so the
        # citation-form gate surfaces it for a human to separate.
        src = "// W3C SCXML 3.13/Appendix D: transition processing\n"
        new, migs, _ = plan(src)
        self.assertEqual(new, src)
        self.assertEqual(migs, [])
        self.assertNotIn("§scxml-3.13/", new)


class Idempotency(unittest.TestCase):
    def test_second_pass_is_noop(self):
        once, _, _ = plan("// W3C SCXML 6.2 and W3C SCXML C.2\n")
        twice, migs, _ = plan(once)
        self.assertEqual(once, twice)
        self.assertEqual(migs, [])


@unittest.skipUnless(MNEMOSYNE_CLI, skip_reason())
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
            '[workspace]\n\n'
            "[atomic]\n"
            'sidecar_path = ".atomic/workspace.atomic.json"\n\n'
            "[plugins.set_equality_validator]\n"
            f'paths = ["{src_rel}"]\n'
            'severity_missing = "reject"\nseverity_binding = "warn"\ncomment_only = true\n',
            encoding="utf-8",
        )
        subprocess.run(
            [MNEMOSYNE_CLI, "import-sections", "--manifest", "manifest.json"],
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
                [MNEMOSYNE_CLI, "validate-code-refs", "--json"],
                cwd=root, capture_output=True, text=True,
            )
            self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
            report = json.loads(out.stdout.splitlines()[0])
            self.assertEqual(report["section_missing_count"], 0)

            # a hallucinated citation fails the reject gate
            with f.open("a", encoding="utf-8") as fh:
                fh.write("// invented §scxml-9.99 reference\n")
            bad = subprocess.run(
                [MNEMOSYNE_CLI, "validate-code-refs", "--json"],
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


WIRE_LEDGER = {"wire-W0", "wire-W1", "wire-W2", "wire-W3", "wire-W4", "wire-W4.5", "wire-W5"}


def wire_plan(text, name="f.h"):
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, name)
        with open(p, "w", encoding="utf-8") as fh:
            fh.write(text)
        # wire needs no cross-namespace exclude (W<n> labels are unique).
        return mc.plan_file(p, WIRE_LEDGER, namespace="wire")


class WireNamespace(unittest.TestCase):
    def test_wave_bare_sigil_migrated(self):
        new, migs, _ = wire_plan("// §W4 typed-throw surface\n")
        self.assertIn("§wire-W4", new)
        self.assertEqual(migs[0]["id"], "wire-W4")

    def test_rfc_marker_is_not_foreign_for_wire(self):
        # "RFC §W4" is the Wire RFC, not the IETF "RFC <digits>" form.
        new, migs, _ = wire_plan("// RFC §W4 D1-C typed surface\n")
        self.assertIn("RFC §wire-W4 D1-C", new)
        self.assertEqual(len(migs), 1)

    def test_dotted_half_wave_kept_verbatim(self):
        new, migs, _ = wire_plan("// §W4.5 debt repayment\n")
        self.assertIn("§wire-W4.5", new)
        self.assertEqual(migs[0]["id"], "wire-W4.5")

    def test_space_suffix_preserved(self):
        new, _, _ = wire_plan("// §W5 D5 typed-throw\n")
        self.assertEqual(new, "// §wire-W5 D5 typed-throw\n")

    def test_hyphen_range_refused(self):
        # "§W3-5" (waves W3 through W5) would corrupt to §wire-W3-5; refuse it.
        new, migs, skips = wire_plan("// RFC §W3-5: re-thrown downstream\n")
        self.assertEqual(migs, [])
        self.assertIn("§W3-5", new)
        self.assertIn("glued suffix", skips[0]["reason"])

    def test_hallucinated_wave_reported(self):
        new, migs, skips = wire_plan("// §W9 invented wave\n")
        self.assertEqual(migs, [])
        self.assertIn("non-section", skips[0]["reason"])

    def test_string_literal_untouched(self):
        src = 'const char *k = "§W4";\n'
        new, migs, _ = wire_plan(src)
        self.assertEqual(new, src)
        self.assertEqual(migs, [])


class FormGateCheck(unittest.TestCase):
    """--check forbids free-text 'W3C SCXML <n>' section cites (forcing the
    §scxml- form so validate-code-refs can gate them) while leaving the
    spec version and W3C test numbers as prose."""

    def _ledger(self, d):
        store = Path(d) / "store.json"
        store.write_text(
            json.dumps({"sections": {"scxml-5.10": {}, "scxml-6.2": {}}}),
            encoding="utf-8",
        )
        return store

    def _check(self, d, srcdir):
        return subprocess.run(
            [sys.executable, str(MIGRATE), "--check",
             "--ledger", str(self._ledger(d)), str(srcdir)],
            capture_output=True, text=True,
        )

    def test_version_and_test_number_pass(self):
        with tempfile.TemporaryDirectory() as d:
            src = Path(d) / "src"
            src.mkdir()
            (src / "a.rs").write_text(
                "// already migrated §scxml-5.10\n"
                "// W3C SCXML 403 test-suite reference\n"
                "// W3C SCXML 1.0 spec version\n",
                encoding="utf-8",
            )
            r = self._check(d, src)
            self.assertEqual(r.returncode, 0, r.stderr)

    def test_free_text_section_and_hallucination_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            src = Path(d) / "src"
            src.mkdir()
            (src / "a.rs").write_text(
                "// valid free-text W3C SCXML 5.10\n"
                "// hallucinated W3C SCXML 99.99\n"
                "// test W3C SCXML 403\n"
                "// version W3C SCXML 1.0\n",
                encoding="utf-8",
            )
            r = self._check(d, src)
            self.assertEqual(r.returncode, 1, r.stdout)
            self.assertIn("5.10", r.stderr)  # real section in free-text
            self.assertIn("99.99", r.stderr)  # dotted hallucination
            self.assertNotIn(" 403 ", r.stderr)  # bare-int test number stays
            self.assertNotIn("§scxml-1.0", r.stderr)  # version allowlisted

    def test_string_literal_not_flagged(self):
        with tempfile.TemporaryDirectory() as d:
            src = Path(d) / "src"
            src.mkdir()
            (src / "a.rs").write_text(
                'let s = "W3C SCXML 5.10";\n', encoding="utf-8"
            )
            r = self._check(d, src)
            self.assertEqual(r.returncode, 0, r.stderr)


class LedgerExistenceGate(unittest.TestCase):
    """--check-ledger keeps only the hallucination half of --check.

    Trees whose comments are emitted verbatim into generated code (the codegen
    templates) must stay in prose, so the form half cannot apply to them — but a
    section number that does not exist is a false claim about the spec either
    way. This gate is what covers those trees.
    """

    def _ledger(self, d):
        store = Path(d) / "store.json"
        store.write_text(
            json.dumps({"sections": {"scxml-5.10": {}, "scxml-6.2": {}}}),
            encoding="utf-8",
        )
        return store

    def _check_ledger(self, d, srcdir):
        return subprocess.run(
            [sys.executable, str(MIGRATE), "--check-ledger",
             "--ledger", str(self._ledger(d)), str(srcdir)],
            capture_output=True, text=True,
        )

    def _run(self, body, name="a.rs"):
        with tempfile.TemporaryDirectory() as d:
            src = Path(d) / "src"
            src.mkdir()
            (src / name).write_text(body, encoding="utf-8")
            return self._check_ledger(d, src)

    def test_prose_cite_to_real_section_passes(self):
        # The whole point: prose is allowed here, unlike --check.
        r = self._run("// W3C SCXML 5.10 system variables\n")
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_fabricated_section_rejected(self):
        r = self._run("// W3C SCXML 6.4.6 autoforward\n")
        self.assertEqual(r.returncode, 1)
        self.assertIn("6.4.6", r.stderr)
        self.assertIn("not in ledger", r.stderr)

    def test_spec_version_and_test_number_pass(self):
        # "1.0" is the spec version; "403" is a W3C IRP test number.
        r = self._run("// W3C SCXML 1.0 conformance, test W3C SCXML 403\n")
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_string_literal_not_flagged(self):
        r = self._run('let s = "W3C SCXML 6.4.6";\n')
        self.assertEqual(r.returncode, 0, r.stderr)


class PathsFromToml(unittest.TestCase):
    def test_extracts_array_and_skips_commented_header(self):
        with tempfile.TemporaryDirectory() as d:
            toml = Path(d) / "mnemosyne.toml"
            toml.write_text(
                "# [plugins.set_equality_validator] named in a comment only\n"
                "[workspace]\n"
                "root = '.'\n\n"
                "[plugins.set_equality_validator]\n"
                "paths = [\n"
                '    "sce/include/core",\n'
                "    # a comment inside the array\n"
                '    "sce-build/src",\n'
                "]\n"
                'severity_missing = "reject"\n',
                encoding="utf-8",
            )
            got = [
                os.path.relpath(p, mc.REPO_ROOT)
                for p in mc.paths_from_toml(str(toml))
            ]
            self.assertEqual(got, ["sce/include/core", "sce-build/src"])


if __name__ == "__main__":
    unittest.main()
