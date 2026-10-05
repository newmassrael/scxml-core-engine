"""A cyclic signal is the same drive said again at a fixed rate.

A platform's test can send a signal every few milliseconds for a stretch and
read the result at the end. A component that averages, counts or integrates
over what it receives answers differently after one delivery than after the
twenty the cycle made. Folding the cycle into one step shows the document a
single event, and it passes or fails on a world the record does not describe.

So a `before` step may say `repeat: {every_ms, for_ms}`, and it is driven that
many times, each round observed one period after its drive. Asserted here:

  reading   the step becomes `for_ms / every_ms` rounds (rounded down, at least
            one), each observed exactly one period after its drive; a step
            without `repeat` is untouched; the format refuses a malformed
            `repeat`, and a step that says both `repeat` and `elapsed_ms`
  a machine a document that needs three deliveries is shown three when the
            cycle ran them, and is shown fewer when the cycle ran fewer
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.errors import PackError
from sce_author.pack import load_examples, load_pack
from sce_author.verify import _default_codegen, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

ADDRESS = "plant/in/train-approach"
STEP = {"given": {ADDRESS: "APPROACHING"}, "drove": [ADDRESS], "delivered": [ADDRESS]}

# The crossing flashes on the third report of an approaching train and not
# before: the third delivery is the one that decides.
THIRD = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="idle" sce:kind="statechart">
  <state id="idle">
    <transition event="train.approaching" target="heard1"/>
  </state>
  <state id="heard1">
    <transition event="train.approaching" target="heard2"/>
  </state>
  <state id="heard2">
    <transition event="train.approaching" target="flashing">
      <send event="signal.flashing" type="x-sce-host"/>
    </transition>
  </state>
  <state id="flashing"/>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "signal.scxml",
    "inputs": {
        "approaching": {"address": ADDRESS, "becomes": "APPROACHING",
                        "event": "train.approaching"},
    },
    "outputs": {
        "roadSignal": {
            "address": "plant/out/road-signal", "field": "value",
            "sent": {"processor": "x-sce-host"},
            "when_nothing_sent": "signal.dark",
            "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"},
        },
    },
}


def examples(*cases):
    return {"version": 1, "origin": "written for this test",
            "independent_cases": True, "ordered": True, "cases": list(cases)}


def case_after(repeat):
    """The third report arrives as the case itself, after `repeat` made the rest."""
    return {"name": "the third report", "before": [{**STEP, "repeat": repeat}],
            "given": {ADDRESS: "APPROACHING"}, "drove": [ADDRESS], "delivered": [ADDRESS],
            "expect": {"plant/out/road-signal.value": "FLASHING"}}


class TheCycleIsRead(unittest.TestCase):
    def load(self, step):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "examples.yaml"
            path.write_text(yaml.safe_dump(examples(
                {"name": "c", "before": [step], "given": {ADDRESS: "CLEAR"},
                 "expect": {}})), encoding="utf-8")
            return load_examples([path])

    def test_a_step_is_driven_as_often_as_the_cycle_ran(self):
        got = self.load({**STEP, "repeat": {"every_ms": 10, "for_ms": 200}})
        steps = got.cases[0].before
        self.assertEqual(20, len(steps))
        self.assertEqual({(10.0, 10.0)}, {s.elapsed_window for s in steps})
        self.assertEqual({(ADDRESS,)}, {s.drove for s in steps})
        self.assertEqual({(ADDRESS,)}, {s.delivered for s in steps})
        self.assertIn("repeat 1 of 20", steps[0].name)
        self.assertIn("repeat 20 of 20", steps[-1].name)
        self.assertEqual(1, got.case_count, "a round of the cycle is not a case")

    def test_a_cycle_shorter_than_its_period_is_still_driven_once(self):
        self.assertEqual(1, len(self.load(
            {**STEP, "repeat": {"every_ms": 100, "for_ms": 50}}).cases[0].before))

    def test_the_count_is_rounded_down(self):
        self.assertEqual(2, len(self.load(
            {**STEP, "repeat": {"every_ms": 100, "for_ms": 250}}).cases[0].before))

    def test_a_step_without_repeat_is_one_round(self):
        steps = self.load(dict(STEP)).cases[0].before
        self.assertEqual(1, len(steps))
        self.assertIsNone(steps[0].elapsed_window)

    def test_a_cycle_that_does_not_say_how_long_is_refused(self):
        with self.assertRaises(PackError):
            self.load({**STEP, "repeat": {"every_ms": 10}})

    def test_a_period_that_is_not_positive_is_refused(self):
        with self.assertRaises(PackError):
            self.load({**STEP, "repeat": {"every_ms": 0, "for_ms": 100}})

    def test_a_step_that_says_both_when_it_was_observed_is_refused(self):
        with self.assertRaises(PackError) as caught:
            self.load({**STEP, "elapsed_ms": 5, "repeat": {"every_ms": 10, "for_ms": 100}})
        self.assertIn("`repeat` and `elapsed_ms`", str(caught.exception))


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AMachineIsShownEveryRoundOfTheCycle(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(THIRD, encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        self.binding = path

    def judge(self, case):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump(examples(case)), encoding="utf-8")
        result = verify(load_pack(self.tmp), self.binding)
        self.assertTrue(result.ran, result.refusal)
        return (result.passed, result.failed, result.unjudged)

    def test_two_reports_before_the_case_make_the_third(self):
        self.assertEqual((1, 0, 0), self.judge(case_after({"every_ms": 10, "for_ms": 20})))

    def test_and_one_report_before_it_does_not(self):
        """The discriminator. Without it the case above proves nothing: a document
        that flashed on any report would pass it too."""
        self.assertEqual((0, 1, 0), self.judge(case_after({"every_ms": 10, "for_ms": 10})))


if __name__ == "__main__":
    unittest.main()
