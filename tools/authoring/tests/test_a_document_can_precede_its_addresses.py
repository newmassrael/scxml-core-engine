"""The logic can be written before anybody knows the addresses.

A specification usually arrives before a platform's list of addresses does,
and the document does not need that list: it uses its OWN identifiers and the
binding is the dictionary. So the two can be written months apart, and the
tool's job in between is to hold the gap open rather than let it be filled.

⚠ THE POINT IS NOT THE FEATURE, IT IS WHAT HAPPENS WITHOUT IT. Asked for a
complete binding with no list to hand, an author -- human or model -- produces
a complete-LOOKING one. A plausible wrong address is invisible: every command
here accepts it, the generator emits from it, and nothing says a word. A
declared missing one is a question with an owner. Absence has to be writable or
it does not get written, which the document's `sce:unresolved` has been
demonstrating for VALUES since before this package existed.

These cases assert the whole route:

    the binding may say it does not know      (the schema accepts it)
    `check` names the gap, not a bad name     (and still fails, honestly)
    `verify` never judges on the unknown      (and says why, and what it cost)
    filling it in changes only the binding    (the document is untouched)

⚠ The third line used to read "`verify` declines to run". That was right
about the premise -- no verdict may rest on a value nobody supplied -- and too
wide in the conclusion. It made declaring an unknown cost every case of the
component while a plausible wrong address cost nothing, and measured across
thirty documents written from prose, the key was used zero times. `verify` now
runs each case under every value the unknown could take and withholds only
the positions that come out different. What the old assertion protected -- no
verdict about an invented value -- is asserted directly below instead.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import check
from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
CLOSED = CROSSING / "controller_resolved.binding.yaml"

REASON = "the platform list is not available yet; the source calls it the override"


class ADocumentCanPrecedeItsAddresses(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml",
                     "examples.yaml", "controller_resolved.scxml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        self.pack = load_pack(self.tmp)

    def binding(self, mutate):
        doc = yaml.safe_load(CLOSED.read_text(encoding="utf-8"))
        mutate(doc)
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(doc), encoding="utf-8")
        return path

    def open_input(self, doc):
        doc["inputs"]["override"] = {"unresolved": REASON}

    def open_output(self, doc):
        doc["outputs"]["bell"] = {"unresolved": REASON}

    # -------------------------------------------------------------- check

    def test_check_names_the_gap_rather_than_a_name_that_does_not_exist(self):
        findings = check(self.pack, self.binding(self.open_input))
        said = "\n".join(str(f) for f in findings)
        self.assertIn("no address yet", said)
        self.assertIn("platform list is not available", said,
                      "the author's reason is the message; it has to survive")
        self.assertNotIn("is not in the interface model", said)

    def test_an_incomplete_binding_still_fails_check(self):
        """It cannot reach the platform, which is what `check` answers."""
        self.assertTrue(check(self.pack, self.binding(self.open_input)))
        # ⚠ The discriminator: the SAME binding with the address filled in has
        # nothing to report. Without this, a check that always failed would
        # pass the case above.
        self.assertEqual([], check(self.pack, self.binding(lambda d: None)))

    # ------------------------------------------------------------- verify

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_verify_never_judges_what_depends_on_an_input_nobody_supplied(self):
        """The property the old refusal protected, asserted directly.

        No position whose value turns on the unknown may be counted either
        way, and no case resting on one may count as passed. The author's
        reason has to reach the reader, because the next step is to take it to
        whoever knows the address.
        """
        result = verify(self.pack, self.binding(self.open_input))
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        self.assertIn("override", result.unresolved)
        self.assertIn("platform list is not available",
                      result.unresolved["override"])
        # ⚠ Arity floor. Without it the assertions below hold vacuously for a
        # run that withheld nothing -- which is the old all-or-nothing run
        # reached by a different road.
        self.assertGreater(result.undetermined, 0,
                           "the fixture's outputs depend on the override; "
                           "some position must have been withheld")
        for case in result.results:
            if case.undetermined:
                self.assertFalse(case.passed,
                                 f"{case.name}: passed on part of what it asserts")

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_an_open_output_is_narrower_and_the_rest_still_runs(self):
        """⚠ Not the same as an open input. An output with nowhere to land
        stops nothing: the document still computes every other value, and only
        the positions this one could have written go unjudged.
        """
        result = verify(self.pack, self.binding(self.open_output))
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        self.assertTrue(result.results)
        self.assertEqual({"bell": REASON}, result.unaddressed_outputs)
        withheld = {a for case in result.results for a in case.undetermined}
        self.assertEqual({"plant/out/bell.value"}, withheld)
        self.assertIn("plant/out/bell.value", result.unbound)
        # The other outputs were still judged.
        unchecked = {a for case in result.results for a in case.unchecked}
        self.assertNotIn("plant/out/road-signal.value", withheld | unchecked)

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_an_open_output_cannot_turn_a_failure_into_a_pass(self):
        """⚠ The loophole, measured 2026-09-22 on this very fixture. Its
        binding fails one case, on the bell. Written `unresolved` on the bell,
        the position was counted unchecked -- a position no rule claims, which
        does not stop a pass -- and the run read 5 passed, 0 failed, exit 0.
        """
        from tests.test_the_commands_work_as_a_product import run
        closed = verify(self.pack, self.binding(lambda d: None))
        self.assertEqual(1, closed.failed, "the premise: one case fails")
        self.assertEqual(
            ["plant/out/bell.value"],
            [a for c in closed.results for a, _, _ in c.failures],
            "the premise: it fails on the output about to be opened")

        path = self.binding(self.open_output)
        opened = verify(self.pack, path)
        failing = next(c.name for c in closed.results if c.failures)
        self.assertFalse(
            next(c for c in opened.results if c.name == failing).passed,
            "opening the failing output must not pass its case")
        # ⚠ Not equal: every case expecting the bell is withheld, including
        # those that passed with it placed. What matters is that the count
        # can only fall -- never rise -- when a value is opened.
        self.assertLessEqual(opened.passed, closed.passed)
        for case in opened.results:
            if "plant/out/bell.value" in case.undetermined:
                self.assertFalse(case.passed, case.name)
        code, said, _ = run(["verify", "--pack", str(self.tmp),
                             "--binding", str(path)])
        self.assertEqual(1, code, said)
        self.assertIn("not a pass", said)
        self.assertIn("has no address for yet", said)

    def test_an_open_output_only_withholds_positions_ending_in_its_fields(self):
        """An open output writes its unknown address followed by its own
        fields, so a position ending in another field cannot be its."""
        from sce_author.verify import CaseJudge, Verification
        verification = Verification(unaddressed_outputs={"bell": REASON})
        outputs = {"bell": {"unresolved": REASON, "field": "value",
                            "also": {"id": "X"}}}
        judge = CaseJudge(verification, self.pack, {}, None, outputs,
                          {"plant/out/road-signal.value"}, {})
        self.assertEqual(["bell"], judge.landing_here("plant/out/other.value"))
        self.assertEqual(["bell"], judge.landing_here("plant/out/other.id"))
        self.assertEqual([], judge.landing_here("plant/out/other.status"))
        self.assertEqual([], judge.landing_here("plant/out/road-signal.value"),
                         "a position a named rule writes is that rule's")
        bare = CaseJudge(verification, self.pack, {}, None,
                         {"bell": {"unresolved": REASON}}, set(), {})
        self.assertEqual(["bell"], bare.landing_here("plant/out/anything"),
                         "a rule writing the bare address could be anywhere")

    # --------------------------------------------------- the second phase

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_filling_the_gap_changes_only_the_binding(self):
        """The whole promise of the two phases, asserted rather than claimed."""
        document = (self.tmp / "controller_resolved.scxml").read_bytes()
        before = verify(self.pack, self.binding(self.open_input))
        self.assertGreater(before.undetermined, 0,
                           "with the address open, something must be withheld")

        after = verify(self.pack, self.binding(lambda d: None))
        self.assertTrue(after.ran, f"it would not run: {after.refusal}")
        self.assertEqual(0, after.undetermined,
                         "filled in, nothing is left waiting on the unknown")
        self.assertEqual({}, after.unresolved)
        self.assertEqual(
            document, (self.tmp / "controller_resolved.scxml").read_bytes(),
            "the document written in phase one must not need editing")


if __name__ == "__main__":
    unittest.main()
