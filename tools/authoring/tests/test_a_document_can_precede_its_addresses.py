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
    `verify` declines to run and says why     (rather than running on nothing)
    filling it in changes only the binding    (the document is untouched)
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
    def test_verify_declines_to_run_on_an_input_nobody_supplied(self):
        result = verify(self.pack, self.binding(self.open_input))
        self.assertFalse(result.ran)
        self.assertIn("override", result.refusal)
        self.assertIn("platform list is not available", result.refusal)

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_an_open_output_is_narrower_and_the_rest_still_runs(self):
        """⚠ Not the same as an open input. An output with nowhere to land
        stops nothing: the document still computes every other value, and only
        the positions this one would have written go unchecked.
        """
        result = verify(self.pack, self.binding(self.open_output))
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        self.assertTrue(result.results)
        unchecked = {a for case in result.results for a in case.unchecked}
        self.assertIn("plant/out/bell.value", unchecked)
        self.assertIn("plant/out/bell.value", result.unbound)
        # The other outputs were still judged.
        checked = {a for case in result.results
                   for a in case.failures} | set()
        self.assertNotIn("plant/out/road-signal.value", unchecked)

    # --------------------------------------------------- the second phase

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_filling_the_gap_changes_only_the_binding(self):
        """The whole promise of the two phases, asserted rather than claimed."""
        document = (self.tmp / "controller_resolved.scxml").read_bytes()
        before = verify(self.pack, self.binding(self.open_input))
        self.assertFalse(before.ran)

        after = verify(self.pack, self.binding(lambda d: None))
        self.assertTrue(after.ran, f"it would not run: {after.refusal}")
        self.assertEqual(
            document, (self.tmp / "controller_resolved.scxml").read_bytes(),
            "the document written in phase one must not need editing")


if __name__ == "__main__":
    unittest.main()
