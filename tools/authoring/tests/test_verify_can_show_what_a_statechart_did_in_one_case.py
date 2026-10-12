"""`verify --explain` shows what a STATECHART did in a case, not only a computation.

`--explain TEXT` was asked of every document and answered for one kind of it. A statechart is driven
by events and answers with `<send>`s, so the values between the inputs and a failing position are the
events it was handed, the sends it made and the states it was in -- and `verify` read the request, took
the statechart path, and said nothing: no lines, no refusal, a judgement as if nothing had been asked.
The one case that most needed explaining was a statechart failing at 0 ms where 10 s was expected, and
there was no way to ask why. Measured 2026-10-12 on a document that sent its delay as `10000`.

Asserted here, with documents that are really run:

  asked        a case named in `explain` carries the events it was driven with, the sends the machine
               made and the positions landed from them, and the states active afterwards
  setup        a case that takes steps before its own round shows every step, setup first
  not asked    every other case carries nothing, so the report does not grow with the suite
  unchanged    asking changes no verdict
  by text      a name matches by substring, and one that matches nothing shows nothing
  time         a send made on the way to a reading is shown with the instant it was read at
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, verify

from tests.test_a_statechart_is_driven_not_called import (
    BINDING, CROSSING, DELAYED, DOCUMENT, EXAMPLES)

APPROACH = "a train approaches"
CLEAR = "the train clears"
ADDRESS = "plant/in/train-approach"
POSITION = "plant/out/road-signal.value"
OCCUPIED = {"address": ADDRESS, "becomes": "OCCUPIED", "event": "train.occupied"}


def codegen_is_built() -> bool:
    return _default_codegen().exists()


class _Drives:
    """A pack, a document and a binding in a folder of their own, and `verify` run over them."""

    document = DOCUMENT
    binding = BINDING

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(self.document, encoding="utf-8")

    def run_with(self, *explain, cases=None):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump({**EXAMPLES, "cases": cases or EXAMPLES["cases"]}), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(self.binding), encoding="utf-8")
        result = verify(load_pack(self.tmp), path, None, "python", tuple(explain))
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        return result

    def cases(self, *explain, cases=None):
        return {case.name: case for case in self.run_with(*explain, cases=cases).results}

    @staticmethod
    def sends(case) -> dict:
        return {key: value for key, value in case.shown["returned"].items() if key.startswith("send")}


@unittest.skipUnless(codegen_is_built(), "the product's generator is not built; nothing can be run")
class AStatechartCaseCanBeShown(_Drives, unittest.TestCase):
    # ---------------------------------------------------------------- asked

    def test_a_case_that_was_asked_for_carries_its_four_parts(self):
        shown = self.cases(APPROACH)[APPROACH].shown
        self.assertEqual({"inputs", "happened", "returned", "kept"}, set(shown))

    def test_the_inputs_are_the_events_the_machine_was_handed(self):
        shown = self.cases(APPROACH)[APPROACH].shown
        self.assertEqual({"01 event": "train.approaching"}, shown["inputs"])

    def test_the_returned_part_is_the_sends_and_the_positions_landed_from_them(self):
        returned = self.cases(APPROACH)[APPROACH].shown["returned"]
        self.assertEqual("x-sce-host signal.flashing", returned["send 01 read at 0 ms"])
        self.assertEqual("FLASHING", returned[f"wrote {POSITION}"])

    def test_the_kept_part_is_the_states_active_afterwards(self):
        shown = self.cases(APPROACH)[APPROACH].shown
        self.assertEqual({"active flashing": True}, shown["kept"])

    def test_the_happened_part_is_the_event_the_transition_and_the_move_in_order(self):
        happened = self.cases(APPROACH)[APPROACH].shown["happened"]
        self.assertEqual({"001 event": "train.approaching",
                          "002 take": "in dark, on train.approaching -> flashing",
                          "003 exit": "dark",
                          "004 enter": "flashing"}, happened)

    def test_an_event_nothing_takes_is_told_as_discarded(self):
        """Not handled and not silent: the event came in and no active state answered it."""
        stray = [{"name": "a clear track at the start", "given": {ADDRESS: "CLEAR"},
                  "drove": [ADDRESS], "expect": {POSITION: "DARK"}}]
        happened = self.cases("clear track", cases=stray)["a clear track at the start"].shown["happened"]
        self.assertEqual({"001 event": "train.cleared",
                          "002 discarded": "train.cleared (no active state has a transition "
                                           "that takes it)"}, happened)

    def test_a_case_that_sends_nothing_has_no_send_line(self):
        """Nothing sent is an answer too -- the position keeps what it held -- and the report does
        not invent a line for it."""
        quiet = [{"name": "a clear track at the start", "given": {ADDRESS: "CLEAR"},
                  "drove": [ADDRESS], "expect": {POSITION: "DARK"}}]
        case = self.cases("clear track", cases=quiet)["a clear track at the start"]
        self.assertEqual({}, self.sends(case))

    # ---------------------------------------------------------------- setup

    SETUP = [{"name": "it clears after an approach",
              "given": {ADDRESS: "CLEAR"}, "drove": [ADDRESS],
              "expect": {POSITION: "DARK"},
              "before": [{"given": {ADDRESS: "APPROACHING"}, "drove": [ADDRESS]}]}]

    def test_a_case_with_setup_shows_every_step_setup_first(self):
        shown = self.cases("after an approach", cases=self.SETUP)["it clears after an approach"].shown
        self.assertEqual({"step 01 (setup) 01 event": "train.approaching",
                          "step 02 (the case) 01 event": "train.cleared"}, shown["inputs"])

    def test_the_states_after_each_step_are_shown_so_the_path_can_be_read(self):
        shown = self.cases("after an approach", cases=self.SETUP)["it clears after an approach"].shown
        self.assertEqual({"step 01 (setup) active flashing": True,
                          "step 02 (the case) active dark": True}, shown["kept"])

    def test_what_happened_in_each_step_is_told_beside_its_step(self):
        shown = self.cases("after an approach", cases=self.SETUP)["it clears after an approach"].shown
        self.assertEqual({
            "step 01 (setup) 001 event": "train.approaching",
            "step 01 (setup) 002 take": "in dark, on train.approaching -> flashing",
            "step 01 (setup) 003 exit": "dark",
            "step 01 (setup) 004 enter": "flashing",
            "step 02 (the case) 001 event": "train.cleared",
            "step 02 (the case) 002 take": "in flashing, on train.cleared -> dark",
            "step 02 (the case) 003 exit": "flashing",
            "step 02 (the case) 004 enter": "dark"}, shown["happened"])

    # ------------------------------------------------------------ not asked

    def test_a_case_nobody_asked_about_carries_nothing(self):
        self.assertEqual({}, self.cases(APPROACH)[CLEAR].shown)

    def test_without_explain_nothing_is_shown_anywhere(self):
        for case in self.cases().values():
            self.assertEqual({}, case.shown)

    # ------------------------------------------------------------ unchanged

    def test_asking_changes_no_verdict(self):
        plain, asked = self.run_with(), self.run_with(APPROACH, CLEAR)
        self.assertEqual((plain.passed, plain.failed, plain.unjudged),
                         (asked.passed, asked.failed, asked.unjudged))
        self.assertEqual([c.failures for c in plain.results], [c.failures for c in asked.results])

    def test_a_failing_case_is_shown_beside_its_failure(self):
        wrong = [{"name": "a train approaches and the lamp is dark", "given": {ADDRESS: "APPROACHING"},
                  "drove": [ADDRESS], "expect": {POSITION: "DARK"}}]
        case = self.cases("lamp is dark", cases=wrong)["a train approaches and the lamp is dark"]
        self.assertTrue(case.failures, "it still fails")
        self.assertEqual("FLASHING", case.shown["returned"][f"wrote {POSITION}"])

    # -------------------------------------------------------------- by text

    def test_a_name_matches_by_substring(self):
        shown = [name for name, case in self.cases("approach").items() if case.shown]
        self.assertEqual([APPROACH], shown)

    def test_a_name_that_matches_nothing_shows_nothing_and_is_not_an_error(self):
        cases = self.cases("no such case anywhere")
        self.assertEqual(2, len(cases))
        self.assertTrue(all(not case.shown for case in cases.values()))


@unittest.skipUnless(codegen_is_built(), "the product's generator is not built; nothing can be run")
class TheInstantAMachineWasReadAtIsShown(_Drives, unittest.TestCase):
    """A send made while time moved is shown with the instant the machine was read at."""

    document = DELAYED
    binding = {**BINDING, "inputs": {**BINDING["inputs"], "occupied": OCCUPIED}}

    TIMED = [
        {"name": "a train is detected", "given": {ADDRESS: "APPROACHING"}, "drove": [ADDRESS],
         "elapsed_ms": 100, "expect": {POSITION: "DARK"}},
        {"name": "it reaches the crossing", "given": {ADDRESS: "OCCUPIED"}, "drove": [ADDRESS],
         "elapsed_ms": 600, "expect": {POSITION: "FLASHING"}},
    ]

    def test_the_delayed_send_is_shown_with_the_instant_it_was_read_at(self):
        case = self.cases("reaches the crossing", cases=self.TIMED)["it reaches the crossing"]
        self.assertEqual("x-sce-host signal.flashing", case.shown["returned"]["send 01 read at 600 ms"])

    def test_the_arming_round_shows_no_send(self):
        """The delay has not run out: the round that arms it sent nothing, which is what a reader
        asking why a position is late needs to see."""
        case = self.cases("is detected", cases=self.TIMED)["a train is detected"]
        self.assertEqual({}, self.sends(case))
        self.assertEqual({"active arming": True}, case.shown["kept"])


# A wait armed with a number and no unit, and a catch-all that answers whatever comes: the
# document measured on 2026-10-12. The argument error of the send is an event like any other,
# and the catch-all took it -- which no value of the document and no send shows.
UNITLESS = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark"
       sce:kind="statechart">
  <state id="dark">
    <transition event="train.approaching" target="arming"/>
  </state>
  <state id="arming">
    <onentry><send event="signal.flashing" type="x-sce-host" delay="500"/></onentry>
    <transition event="*"><send event="signal.dark" type="x-sce-host"/></transition>
  </state>
</scxml>
"""


@unittest.skipUnless(codegen_is_built(), "the product's generator is not built; nothing can be run")
class AnErrorTheMachineRaisedIsShownAsTheEventItIs(_Drives, unittest.TestCase):
    document = UNITLESS

    DETECTED = [{"name": "a train is detected", "given": {ADDRESS: "APPROACHING"}, "drove": [ADDRESS],
                 "elapsed_ms": 100, "expect": {POSITION: "DARK"}}]

    def happened(self) -> dict:
        return self.cases("is detected", cases=self.DETECTED)["a train is detected"].shown["happened"]

    def test_the_argument_error_is_an_event_in_the_order_the_round_took_it(self):
        self.assertEqual({
            "001 event": "train.approaching",
            "002 take": "in dark, on train.approaching -> arming",
            "003 exit": "dark",
            "004 enter": "arming",
            "005 event": "error.execution",
            "006 take": "in arming, on * -> (stays; runs its content)"}, self.happened())

    def test_the_catch_all_that_took_it_is_named_by_what_it_listens_for(self):
        self.assertIn("on *", self.happened()["006 take"])

    def test_a_wait_with_a_unit_raises_no_error_for_the_catch_all_to_take(self):
        """The nearest document that must read differently: the same machine, the unit written."""
        (self.tmp / "signal.scxml").write_text(UNITLESS.replace('delay="500"', 'delay="500ms"'),
                                               encoding="utf-8")
        events = [value for key, value in self.happened().items() if key.endswith("event")]
        self.assertEqual(["train.approaching"], events)


if __name__ == "__main__":
    unittest.main()
