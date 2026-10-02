"""A verdict is only as true as the run it is read off.

The engine stops a macrostep that will not end (W3C SCXML 3.13) and goes on; an
error raised at a send that nothing answers leaves the entry block ended where it
failed (W3C SCXML 4.9). Either way what the machine does from there is not what
its document says, and every ordinary reading of it (it runs, it names a state,
the call returned) says it is fine. The scenario driver refused such a run from
the start. `compare` was taught the same on 2026-10-02 and `verify` was not:
measured the same day against `da8e53c9ea` by an outside review, a design that
loops passed the case "a train is detected" exactly as a design that settles did.

Asserted here, for `verify`, through the rules `lowering` holds for every consumer:

    a design that cuts its macrostep at the start refuses the whole run, and
    one that cuts it after an input withholds that case and every one after it
    an error nothing answers is held to the same, at the start and after an input
    the same machine that settles is still judged, both ways      (the control)
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
    BINDING, CROSSING, EXAMPLES, codegen_is_built)

APPROACH = "plant/in/train-approach"
SIGNAL = "plant/out/road-signal.value"

_HEAD = ('<?xml version="1.0" encoding="UTF-8"?>\n'
         '<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" '
         'version="1.0" datamodel="ecmascript" initial="dark" sce:kind="statechart">\n')

FLASH = '<send event="signal.flashing" type="x-sce-host"/>'
# Sent to a target nobody is at: error.communication (W3C SCXML 6.2.4), unanswered.
NOWHERE = '<send event="ping" target="#_nobody"/>'

SETTLES = _HEAD + f"""
  <state id="dark"><transition event="train.approaching" target="flashing"/></state>
  <state id="flashing"><onentry>{FLASH}</onentry>
    <transition event="train.cleared" target="dark"/></state>
</scxml>
"""

# The eventless self-transition is enabled again every time it is taken.
LOOPS_AT_START = _HEAD + f"""
  <state id="dark"><transition target="dark"/>
    <transition event="train.approaching" target="flashing"/></state>
  <state id="flashing"><onentry>{FLASH}</onentry></state>
</scxml>
"""

LOOPS_AFTER_AN_INPUT = _HEAD + f"""
  <state id="dark"><transition event="train.approaching" target="busy"/></state>
  <state id="busy"><onentry>{FLASH}</onentry><transition target="busy"/>
    <transition event="train.cleared" target="dark"/></state>
</scxml>
"""

RAISES_AT_START = _HEAD + f"""
  <state id="dark"><onentry>{NOWHERE}</onentry>
    <transition event="train.approaching" target="flashing"/></state>
  <state id="flashing"><onentry>{FLASH}</onentry></state>
</scxml>
"""

# The same failure, from a send whose route is read from data. Open, the specification
# never says who the caller is; plain, nothing is open and the draft is at fault.
_ROUTE_DATA = ('<data id="callerTarget" expr="\'#_nobody\'"{open}/>')
_OPEN = (' sce:unresolved="caller-target" '
         'sce:unresolved-reason="the specification does not say who the caller is"')
_SEND_BY_ROUTE = '<send event="ping" targetexpr="callerTarget"/>'

RAISES_ON_AN_OPEN_ROUTE = _HEAD + f"""
  <datamodel>{_ROUTE_DATA.format(open=_OPEN)}</datamodel>
  <state id="dark"><onentry>{_SEND_BY_ROUTE}</onentry>
    <transition event="train.approaching" target="flashing"/></state>
  <state id="flashing"><onentry>{FLASH}</onentry></state>
</scxml>
"""

RAISES_ON_A_PLAIN_ROUTE = RAISES_ON_AN_OPEN_ROUTE.replace(_OPEN, "")

RAISES_AFTER_AN_INPUT = _HEAD + f"""
  <state id="dark"><transition event="train.approaching" target="flashing"/></state>
  <state id="flashing"><onentry>{FLASH}{NOWHERE}</onentry>
    <transition event="train.cleared" target="dark"/></state>
</scxml>
"""


def case(name: str, becomes: str, expect: str) -> dict:
    return {"name": name, "given": {APPROACH: becomes}, "drove": [APPROACH],
            "expect": {SIGNAL: expect}}


@unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
class AVerdictIsOnlyAsTrueAsTheRun(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)

    def run_cases(self, document: str, *cases: dict):
        (self.tmp / "signal.scxml").write_text(document, encoding="utf-8")
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump({**EXAMPLES, "cases": list(cases)}), encoding="utf-8")
        binding = {**BINDING, "activation": "on-change"}
        (self.tmp / "b.yaml").write_text(yaml.safe_dump(binding), encoding="utf-8")
        return verify(load_pack(self.tmp), self.tmp / "b.yaml")

    @staticmethod
    def counts(result):
        return (result.passed, result.failed, result.unjudged)

    # -- the control: a machine that settles is judged ---------------------

    def test_a_design_that_settles_is_judged_both_ways(self):
        right = self.run_cases(SETTLES, case("a train is detected", "APPROACHING", "FLASHING"))
        self.assertTrue(right.ran, right.refusal)
        self.assertEqual((1, 0, 0), self.counts(right))
        wrong = self.run_cases(SETTLES, case("a train is detected", "APPROACHING", "DARK"))
        self.assertEqual((0, 1, 0), self.counts(wrong))

    # -- the run is cut at the start: nothing can be judged ----------------

    def test_a_design_whose_macrostep_never_ends_refuses_the_whole_run(self):
        for expect in ("FLASHING", "DARK"):
            with self.subTest(expect=expect):
                result = self.run_cases(
                    LOOPS_AT_START, case("a train is detected", "APPROACHING", expect))
                self.assertFalse(result.ran, "a design that loops was judged")
                self.assertIn("stable configuration", result.refusal)

    def test_a_design_that_raises_an_error_nothing_answers_refuses_the_whole_run(self):
        result = self.run_cases(
            RAISES_AT_START, case("a train is detected", "APPROACHING", "FLASHING"))
        self.assertFalse(result.ran, "a design that raised was judged")
        self.assertIn("no state answered", result.refusal)
        self.assertIn("error.", result.refusal)

    # -- the run is cut after an input: that case and every later one ------

    def withheld_from_the_first_case(self, document: str, why: str):
        result = self.run_cases(
            document,
            case("a train is detected", "APPROACHING", "FLASHING"),
            case("the train clears", "CLEAR", "DARK"))
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 0, 2), self.counts(result), [
            (r.name, r.refusal, r.failures) for r in result.results])
        for one in result.results:
            self.assertIn(why, one.refusal)

    # -- the failure is a route nobody has decided: whose question it is ---

    def test_a_route_that_rests_on_an_open_decision_says_so_when_it_fails(self):
        """The send's route is read from a data item the specification leaves
        open. The run is refused as any unanswered error is, and the sentence
        also says it is the owner's question: a verification that only said "no
        state answered it" sent the draft back to be mended for a fault it does not
        have (the scenario driver already said it was blocked by the decision)."""
        result = self.run_cases(
            RAISES_ON_AN_OPEN_ROUTE, case("a train is detected", "APPROACHING", "FLASHING"))
        self.assertFalse(result.ran, "a design that raised was judged")
        self.assertIn("no state answered", result.refusal)
        self.assertIn("open decision `caller-target`", result.refusal)

    def test_the_same_failure_without_an_open_decision_is_not_called_one(self):
        result = self.run_cases(
            RAISES_ON_A_PLAIN_ROUTE, case("a train is detected", "APPROACHING", "FLASHING"))
        self.assertFalse(result.ran)
        self.assertIn("no state answered", result.refusal)
        self.assertNotIn("open decision", result.refusal)

    def test_a_macrostep_cut_after_an_input_withholds_that_case_and_the_rest(self):
        self.withheld_from_the_first_case(LOOPS_AFTER_AN_INPUT, "stable configuration")

    def test_an_error_nothing_answers_after_an_input_withholds_that_case_and_the_rest(self):
        self.withheld_from_the_first_case(RAISES_AFTER_AN_INPUT, "no state answered")


if __name__ == "__main__":
    unittest.main()
