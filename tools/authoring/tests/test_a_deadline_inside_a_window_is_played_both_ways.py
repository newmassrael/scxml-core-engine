"""A delayed act that fell inside a time a record knows only as a window is played both ways.

A setup step whose `elapsed_ms` is a window leaves the machine somewhere before or after an act due
inside it, and the cases share one run, so there are two machines for every case after. `verify`
used to give the whole run up at the first such deadline: every case from there on was withheld,
whether or not the choice could change what it reads. Measured 2026-10-16: a component whose setup
step lasted 0..40 ms before a 30 ms delay had all seven of its cases not judged.

What is asserted, against a signal that starts flashing 500 ms after a train is detected, with a
setup step that lasts 0..600 ms (the act may or may not have fired by the time it ended):

    a case that reads the same on both machines is judged          (the discriminator)
    a case that reads differently is not, and says why
    the case that reads differently does not take the later ones with it
    a case no machine passes is failed, not withheld
    the number of plays is bounded, and a deadline past it gives the run up as before
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import load_pack
from sce_author.verify import verify
from tests.test_a_statechart_is_driven_not_called import (
    BINDING, CROSSING, DELAYED, EXAMPLES, codegen_is_built)

APPROACH = "plant/in/train-approach"
SIGNAL = "plant/out/road-signal.value"

# The act that falls inside the setup step's window changes nothing a case reads: it only moves the
# machine from one waiting state to another, and an occupied crossing flashes from either.
TICKED = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark" sce:kind="statechart">
  <state id="dark"><transition event="train.approaching" target="arming"/></state>
  <state id="arming">
    <onentry><send event="tick" delay="500ms"/></onentry>
    <transition event="tick" target="ticked"/>
    <transition event="train.occupied" target="flashing"/>
  </state>
  <state id="ticked"><transition event="train.occupied" target="flashing"/></state>
  <state id="flashing">
    <onentry><send event="signal.flashing" type="x-sce-host"/></onentry>
    <transition event="train.cleared" target="dark"/>
  </state>
</scxml>
"""

# The setup step: the train is detected, and the record says only that the step lasted 0..600 ms.
DETECTED = {"given": {APPROACH: "APPROACHING"}, "drove": [APPROACH],
            "elapsed_ms": {"min": 0, "max": 600}}


def counts(result):
    return (result.passed, result.failed, result.unjudged)


def details(result):
    return [(r.name, r.refusal, r.failures) for r in result.results]


@unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
class ADeadlineInsideAWindowIsPlayedBothWays(unittest.TestCase):
    OCCUPIED = {"address": APPROACH, "becomes": "OCCUPIED", "event": "train.occupied"}

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(DELAYED, encoding="utf-8")
        self.binding = {**BINDING, "activation": "on-change",
                        "inputs": {**BINDING["inputs"], "occupied": self.OCCUPIED}}

    def run_cases(self, cases):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump({**EXAMPLES, "cases": cases}), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(self.binding), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    def use(self, document: str) -> None:
        (self.tmp / "signal.scxml").write_text(document, encoding="utf-8")

    @staticmethod
    def occupied(expect, name="the train is on the crossing", before=None):
        return {"name": name, "given": {APPROACH: "OCCUPIED"}, "drove": [APPROACH],
                "elapsed_ms": 1000, "expect": {SIGNAL: expect},
                "before": [DETECTED] if before is None else before}

    def test_a_case_that_reads_the_same_either_way_is_judged(self):
        """⚠ The discriminator. The act that falls inside the setup step only moves the machine to
        a waiting state of the same behaviour, so the occupied crossing flashes whether it had
        fired or not. Given up at the deadline, this was not judged."""
        self.use(TICKED)
        result = self.run_cases([self.occupied("FLASHING")])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), counts(result), details(result))

    def test_a_case_that_reads_differently_is_not_judged_and_says_why(self):
        """Here the act is the flashing itself: when it had fired inside the setup step the
        occupied round sends nothing and reads dark, and when it had not it fires inside the
        reading's window and reads flashing. Neither answer is the record's to give, so neither
        is claimed."""
        result = self.run_cases([self.occupied("FLASHING")])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 0, 1), counts(result), details(result))
        (one,) = result.results
        self.assertIn("not known", one.refusal)
        self.assertIn("passed", one.refusal)
        self.assertIn("failed", one.refusal)
        self.assertEqual([SIGNAL], one.undetermined)

    def test_the_case_that_reads_differently_does_not_take_the_later_ones_with_it(self):
        """The run goes on from the choice, so a case after the doubtful one that reads the same on
        both machines is still judged."""
        clears = {"name": "the train clears", "given": {APPROACH: "CLEAR"}, "drove": [APPROACH],
                  "elapsed_ms": 100, "expect": {SIGNAL: "DARK"}}
        result = self.run_cases([self.occupied("FLASHING"), clears])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 1), counts(result), details(result))
        self.assertEqual(["the train is on the crossing"],
                         [r.name for r in result.results if not r.judged])

    def test_a_case_no_machine_passes_is_failed_not_withheld(self):
        """Both machines read FLASHING for the occupied crossing, and the case expects DARK."""
        self.use(TICKED)
        result = self.run_cases([self.occupied("DARK")])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 1, 0), counts(result), details(result))

    def test_a_failed_case_blames_the_guess_it_failed_at_and_a_withheld_one_does_not(self):
        """`refuted` is rebuilt from the verdicts that stand: a case the plays did not agree on is
        withheld, and a guess it would have failed at is not marked as refuted by it."""
        result = self.run_cases([self.occupied("FLASHING")])
        self.assertEqual({}, result.refuted)

    def test_the_number_of_plays_is_bounded(self):
        """With no forks allowed the run is given up at the deadline, as it was before."""
        import importlib
        from unittest import mock

        self.use(TICKED)
        with mock.patch.object(importlib.import_module("sce_author.verify"),
                               "MAX_CLOCK_FORKS", 0):
            result = self.run_cases([self.occupied("FLASHING")])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 0, 1), counts(result), details(result))
        self.assertIn("whether it fired is not known", result.results[0].refusal)


if __name__ == "__main__":
    unittest.main()
