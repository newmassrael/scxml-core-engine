"""A guess a statechart makes can be tried without a script engine.

A guess is worth recording because a case can contradict it. `verify` and `gaps` reach a guess
through the value that reads it, which is a `<data>`, and a `<data>` in a statechart needs a script
engine -- which some hosts forbid. Measured 2026-10-10, five writers given such a host and one
picture put the reading on a state or a region, and `gaps` could only say `unplaced`.

A DECISION REGION is the same decision without the value: a `<state>` whose child states are the
alternatives, whose `initial` is the one chosen, and which the logic reads with `In()` (a guard the
generator accepts with no script engine). Changing `initial` to another child IS the alternative,
so the document is run again with it and every case judged.

Asserted here:

    the document reads such a mark as a region, with its chosen alternative
    a mark on a state whose candidates are not its children is not one      (the discriminator)
    before the alternatives are run it is `unexplored`, and says how to run them
    cases that tell the alternatives apart, none failing  -> held
    a wrong choice an alternative repairs                  -> refuted, with the answer
    cases that cannot tell them apart                      -> untested, and says so
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import read_document
from sce_author.counterfactual import explore, lines
from sce_author.gaps import report
from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="main" sce:kind="statechart">
  <parallel id="main">
    <state id="rule" initial="{chosen}" sce:assumed="WHEN"
           sce:assumed-reason="the text says only that the signal flashes for a train"
           sce:assumed-candidates="{first} {second}">
      <state id="onApproach"/>
      <state id="onClear"/>
      <state id="onOccupied"/>
      <state id="never"/>
    </state>
    <state id="signal" initial="dark">
      <state id="dark">
        <transition event="train.approaching" cond="In('onApproach')" target="flashing"/>
        <transition event="train.cleared" cond="In('onClear')" target="flashing"/>
        <transition event="train.occupied" cond="In('onOccupied')" target="flashing"/>
      </state>
      <state id="flashing">
        <onentry><send event="signal.flashing" type="x-sce-host"/></onentry>
      </state>
    </state>
  </parallel>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "signal.scxml",
    "inputs": {
        "approaching": {"address": "plant/in/train-approach", "becomes": "APPROACHING",
                        "event": "train.approaching"},
        "cleared": {"address": "plant/in/train-approach", "becomes": "CLEAR",
                    "event": "train.cleared"},
        "occupied": {"address": "plant/in/train-approach", "becomes": "OCCUPIED",
                     "event": "train.occupied"},
    },
    "outputs": {
        "roadSignal": {"address": "plant/out/road-signal", "field": "value",
                       "sent": {"processor": "x-sce-host"},
                       "when_nothing_sent": "signal.dark",
                       "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"}},
    },
}

APPROACH = {"name": "a train approaches",
            "given": {"plant/in/train-approach": "APPROACHING"},
            "drove": ["plant/in/train-approach"],
            "expect": {"plant/out/road-signal.value": "FLASHING"}}


def examples(*cases):
    return {"version": 1, "origin": "written for this test", "independent_cases": True,
            "ordered": True, "cases": list(cases)}


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class ADecisionRegionIsRunByItsAlternatives(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)

    def run_with(self, cases, chosen="onApproach", first="onApproach", second="onClear",
                 document=None):
        (self.tmp / "signal.scxml").write_text(
            (document or DOCUMENT).format(chosen=chosen, first=first, second=second),
            encoding="utf-8")
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump(examples(*cases)), encoding="utf-8")
        self.binding = self.tmp / "b.yaml"
        self.binding.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        self.pack = load_pack(self.tmp)
        result = verify(self.pack, self.binding)
        self.assertTrue(result.ran, result.refusal)
        return result

    def explored(self, result):
        found = explore(self.pack, self.binding, result, lambda variant: verify(self.pack, variant))
        gaps = {g.marker: g for g in report(result, self.pack, None, (), found)}
        return found, gaps["WHEN"]

    def test_the_mark_is_read_as_a_region_with_its_chosen_alternative(self):
        self.run_with([APPROACH])
        marks = read_document(self.tmp / "signal.scxml").marks
        self.assertEqual([("state", "rule", True, "onApproach")],
                         [(m.element, m.ident, m.region, m.initial) for m in marks])

    def test_candidates_that_are_not_its_children_do_not_make_a_region(self):
        """The discriminator. `initial` names a child and the candidates name something else:
        there is nothing to change `initial` to, so it is a mark on a state, as before."""
        self.run_with([APPROACH], first="onApproach", second="elsewhere")
        marks = read_document(self.tmp / "signal.scxml").marks
        self.assertEqual([False], [m.region for m in marks])

    def test_before_its_alternatives_are_run_it_says_how_to_run_them(self):
        result = self.run_with([APPROACH])
        guess = next(iter(result.assumptions.values()))
        self.assertEqual("unexplored", guess.status)
        gap = next(g for g in report(result, self.pack) if g.marker == "WHEN")
        self.assertEqual("unexplored", gap.kind)
        self.assertIn("--counterfactual", gap.fix)

    def test_cases_that_tell_the_alternatives_apart_hold_the_guess(self):
        result = self.run_with([APPROACH])
        self.assertEqual((1, 0), (result.passed, result.failed))
        found, gap = self.explored(result)
        self.assertEqual("separated", next(iter(found.values())).verdict)
        self.assertEqual("held", gap.kind)
        self.assertIn("the cases tell these alternatives apart", lines(gap))

    def test_a_wrong_choice_an_alternative_repairs_is_refuted_with_the_answer(self):
        result = self.run_with([APPROACH], chosen="onClear", first="onClear", second="onApproach")
        self.assertEqual((0, 1), (result.passed, result.failed))
        found, gap = self.explored(result)
        verdict = next(iter(found.values()))
        self.assertEqual("witness", verdict.verdict)
        self.assertEqual("refuted", gap.kind)
        self.assertEqual("onApproach", verdict.flips[0].value)
        self.assertEqual(["a train approaches"], verdict.flips[0].fixed)

    def test_cases_that_cannot_tell_the_alternatives_apart_leave_it_untested(self):
        """⚠ What a pass cannot say, said. No case drives `train.occupied` or `train.cleared`, so
        choosing `onOccupied` or `onClear` changes nothing any case sees, and all of them pass."""
        result = self.run_with([APPROACH], chosen="onOccupied", first="onOccupied", second="onClear",
                               document=DOCUMENT.replace(
                                   '<transition event="train.approaching" cond="In(\'onApproach\')"',
                                   '<transition event="train.approaching"'))
        self.assertEqual((1, 0), (result.passed, result.failed))
        found, gap = self.explored(result)
        self.assertEqual("unseparated", next(iter(found.values())).verdict)
        self.assertEqual("untested", gap.kind)
        self.assertTrue(any("no case tells these alternatives apart" in line for line in lines(gap)),
                        lines(gap))
        # A region has no position resting on it, so the fix speaks of alternatives.
        self.assertIn("tells the alternatives apart", gap.fix)
        self.assertNotIn("position", gap.fix)
        # With nothing failing there is nothing to repair, so nothing is said to be repaired.
        self.assertFalse(any("repairs every failure" in line for line in lines(gap)), lines(gap))
        self.assertTrue(any("changes nothing" in line for line in lines(gap)), lines(gap))


    def test_the_product_accepts_the_region_and_still_refuses_a_stray_orphan(self):
        """⚠ The two halves have to agree. `check` here asks a writer for a decision region, and
        the product's own reachability pass refused one for its unchosen candidates (`State is
        unreachable from the document initial configuration`) until it learned that a candidate is
        an alternative and not dead code. The control: a child that is no candidate is still one."""
        import subprocess
        codegen = str(_default_codegen())
        # Every child of the region is a candidate, as the shape requires.
        every_child = DOCUMENT.format(
            chosen="onApproach", first="onApproach", second="onClear onOccupied never")
        region = self.tmp / "region.scxml"
        region.write_text(every_child, encoding="utf-8")
        accepted = subprocess.run([codegen, "check", str(region), "--lint"],
                                  capture_output=True, text=True)
        self.assertEqual(0, accepted.returncode, accepted.stdout + accepted.stderr)
        stray = self.tmp / "stray.scxml"
        stray.write_text(every_child.replace('<state id="never"/>',
                                             '<state id="never"/><state id="orphan"/>'),
                         encoding="utf-8")
        refused = subprocess.run([codegen, "check", str(stray), "--lint"],
                                 capture_output=True, text=True)
        self.assertNotEqual(0, refused.returncode)
        self.assertIn("unreachable", refused.stdout + refused.stderr)

    def test_an_alternative_no_condition_reads_is_not_run_and_is_not_called_held(self):
        """⚠ The discriminator against a result that means nothing. `never` is a candidate that
        no condition reads: setting `initial` to it switches the rule off, and the case that
        needed the rule fails. That is a statement about missing logic, and without this it would
        read as the cases telling two readings apart (held)."""
        result = self.run_with([APPROACH], first="onApproach", second="never")
        self.assertEqual((1, 0), (result.passed, result.failed))
        found, gap = self.explored(result)
        verdict = next(iter(found.values()))
        self.assertEqual("untried", verdict.verdict)
        self.assertEqual([], verdict.flips)
        self.assertTrue(any("'never' is read by no condition" in u for u in verdict.untried),
                        verdict.untried)
        self.assertEqual("unexplored", gap.kind)

    def test_a_region_nothing_reads_decides_nothing(self):
        unread = DOCUMENT.replace("In('onApproach')", "In('onClear')")
        result = self.run_with([APPROACH], document=unread)
        found, gap = self.explored(result)
        verdict = next(iter(found.values()))
        self.assertEqual("untried", verdict.verdict)
        self.assertTrue(any("decides nothing" in u for u in verdict.untried), verdict.untried)

    def test_one_alternative_run_and_one_unread_is_not_called_untested(self):
        """Partial: what was run changed no case, and `never` was not run, so "no case tells the
        alternatives apart" is not claimed of all of them."""
        document = DOCUMENT.replace(
            "sce:assumed-candidates=\"{first} {second}\"",
            "sce:assumed-candidates=\"{first} {second} never\"").replace(
            '<transition event="train.approaching" cond="In(\'onApproach\')"',
            '<transition event="train.approaching"')
        result = self.run_with([APPROACH], chosen="onOccupied", first="onOccupied",
                               second="onClear", document=document)
        found, gap = self.explored(result)
        verdict = next(iter(found.values()))
        self.assertEqual("partial", verdict.verdict)
        self.assertEqual("unexplored", gap.kind)
        self.assertFalse(any("no case tells these alternatives apart" in l for l in lines(gap)))


if __name__ == "__main__":
    unittest.main()
