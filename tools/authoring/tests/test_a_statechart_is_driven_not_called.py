"""A statechart is verified by driving it, and reading what it sent.

`verify` could run one shape: a document whose generated form is one function
per output, taking this round's inputs by name. That shape is a pure
computation, and it is the minority of what SCXML is for. A statechart has no
such function to call -- it receives events, it holds states, and what it
produces for anything outside leaves as a `<send>` (W3C SCXML 6.2). So the
only command that says whether a document BEHAVES said nothing at all about
the product's main kind of document.

⚠ WHAT MAKES THIS A RUN RATHER THAN A CALL, and the thing most easily got
wrong: the machine is built ONCE and the cases are replayed through it. An
examples file is one run. What a case observes is partly the result of the
cases before it -- that is what states are -- so rebuilding between cases
would verify a machine that forgets, which is a different document.

⚠⚠ Every refusal here is a case that WOULD otherwise have been judged. Cases
that drive nothing the document listens for still produce a reading: whatever
the previous case left, attributed to this one. That reading is not a pass and
is not a failure; it is a verdict about a machine nobody drove, and the only
honest report of it is that it could not be judged.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest
from dataclasses import dataclass, field

import yaml

from sce_author.errors import AuthoringError
from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, sent_value, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

# Two states, one outward channel, and two events off ONE address -- which is
# what `becomes` is for: the same reading drives a different event depending
# on the value it took.
DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark"
       sce:kind="statechart">
  <state id="dark">
    <transition event="train.approaching" target="flashing"/>
  </state>
  <state id="flashing">
    <onentry>
      <send event="signal.flashing" type="x-sce-host"/>
    </onentry>
    <transition event="train.cleared" target="dark"/>
  </state>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "signal.scxml",
    "inputs": {
        "approaching": {"address": "plant/in/train-approach",
                        "becomes": "APPROACHING",
                        "event": "train.approaching"},
        "cleared": {"address": "plant/in/train-approach",
                    "becomes": "CLEAR",
                    "event": "train.cleared"},
    },
    "outputs": {
        "roadSignal": {
            "address": "plant/out/road-signal", "field": "value",
            "sent": {"processor": "x-sce-host"},
            # Nothing sent is the machine saying the position is unchanged
            # from its resting value, which for this document is dark.
            "when_nothing_sent": "signal.dark",
            "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"},
        },
    },
}

EXAMPLES = {
    "version": 1,
    "origin": "written for this test",
    "independent_cases": True,
    "ordered": True,
    "cases": [
        {"name": "a train approaches",
         "given": {"plant/in/train-approach": "APPROACHING"},
         "drove": ["plant/in/train-approach"],
         "expect": {"plant/out/road-signal.value": "FLASHING"}},
        {"name": "the train clears",
         "given": {"plant/in/train-approach": "CLEAR"},
         "drove": ["plant/in/train-approach"],
         "expect": {"plant/out/road-signal.value": "DARK"}},
    ],
}


@dataclass
class FakeSend:
    """What the runtime hands a host handler, reduced to what is read here."""

    processor_type: str = "x-sce-host"
    event_name: str = ""
    content: str = ""
    params: dict = field(default_factory=dict)


def codegen_is_built() -> bool:
    return _default_codegen().exists()


class ReadingWhatWasSent(unittest.TestCase):
    """`sent_value` alone -- the part that needs no machine to exercise."""

    RULE = {"sent": {}, "when_nothing_sent": "nothing"}

    def test_with_no_send_the_declared_resting_value_stands(self):
        self.assertEqual("nothing", sent_value("out", self.RULE, []))

    def test_without_a_param_the_value_is_the_event_name(self):
        got = sent_value("out", self.RULE, [FakeSend(event_name="signal.on")])
        self.assertEqual("signal.on", got)

    def test_the_last_send_of_a_case_is_what_the_position_holds(self):
        """A machine crossing two states in one case sends twice."""
        got = sent_value("out", self.RULE, [FakeSend(event_name="a"),
                                            FakeSend(event_name="b")])
        self.assertEqual("b", got, "the first is a moment inside the case that "
                                   "no record claimed to be about")

    def test_a_send_of_another_processor_does_not_answer(self):
        rule = {"sent": {"processor": "x-sce-host"},
                "when_nothing_sent": "nothing"}
        got = sent_value("out", rule,
                         [FakeSend(processor_type="x-other", event_name="a")])
        self.assertEqual("nothing", got)

    def test_a_param_carries_the_value_when_the_rule_names_one(self):
        rule = {"sent": {"param": "lamp"}, "when_nothing_sent": "nothing"}
        got = sent_value("out", rule,
                         [FakeSend(params={"lamp": ["FLASHING"]})])
        self.assertEqual("FLASHING", got)

    def test_a_param_that_repeated_is_a_refusal_not_a_choice(self):
        """`<param>` may repeat under one name, so the value is a LIST."""
        rule = {"sent": {"param": "lamp"}, "when_nothing_sent": "nothing"}
        with self.assertRaises(AuthoringError) as caught:
            sent_value("out", rule, [FakeSend(params={"lamp": ["A", "B"]})])
        self.assertIn("2 time(s)", str(caught.exception))

    def test_content_is_read_when_the_rule_asks_for_it(self):
        rule = {"sent": {"content": True}, "when_nothing_sent": "nothing"}
        got = sent_value("out", rule, [FakeSend(content="42")])
        self.assertEqual("42", got)


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class AStatechartIsDrivenNotCalled(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(DOCUMENT, encoding="utf-8")

    def run_with(self, examples=None, binding=None):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump(examples or EXAMPLES), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(binding or BINDING), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    def test_the_cases_are_replayed_through_one_machine(self):
        """The whole point: driven, and judged on what it sent."""
        result = self.run_with()
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((2, 0, 0),
                         (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_the_second_case_depends_on_what_the_first_one_left(self):
        """Rebuilding per case would hide this; replaying cannot.

        Driving `train.cleared` first leaves the machine in `dark`, where the
        second drive can reach `flashing`. Order is the subject, so the
        expectations are the ones that order produces.
        """
        examples = {**EXAMPLES, "cases": [
            {"name": "a clear reading first",
             "given": {"plant/in/train-approach": "CLEAR"},
             "drove": ["plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "DARK"}},
            {"name": "then a train",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "FLASHING"}},
            {"name": "and it is still flashing until it clears",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"],
             # ⚠ Re-driving `train.approaching` in `flashing` selects no
             # transition, so nothing is sent and the resting value stands.
             # That is the document's answer, and it is what a machine with
             # memory looks like from outside.
             "expect": {"plant/out/road-signal.value": "DARK"}},
        ]}
        result = self.run_with(examples)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(3, result.passed,
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_unordered_cases_are_refused_rather_than_replayed(self):
        examples = {**EXAMPLES, "ordered": False}
        result = self.run_with(examples)
        self.assertFalse(result.ran)
        self.assertIn("ordered", result.refusal)

    def test_a_case_that_drives_nothing_is_unjudged_not_passed(self):
        """Otherwise it reads whatever the case before it left."""
        examples = {**EXAMPLES, "cases": EXAMPLES["cases"] + [
            {"name": "a reading nothing turns into an event",
             "given": {"plant/in/obstacle": "DETECTED"},
             "drove": ["plant/in/obstacle"],
             "expect": {"plant/out/road-signal.value": "DARK"}},
        ]}
        result = self.run_with(examples)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(1, result.unjudged,
                         [(r.name, r.refusal) for r in result.results])
        self.assertEqual(2, result.passed)

    def test_an_output_not_bound_to_a_send_is_refused(self):
        """This driver reads a statechart only through what it sends."""
        binding = {**BINDING, "outputs": {"roadSignal": {
            "address": "plant/out/road-signal", "field": "value",
            "map": {"0": "DARK", "1": "FLASHING"}}}}
        result = self.run_with(binding=binding)
        self.assertTrue(result.ran, result.refusal)
        self.assertTrue(
            all("not bound to a `sent`" in r.refusal for r in result.results),
            [(r.name, r.refusal) for r in result.results])

    def test_no_input_naming_an_event_is_refused_up_front(self):
        """Nothing could drive the machine, so nothing could be judged."""
        binding = {**BINDING, "inputs": {"approaching": {
            "address": "plant/in/train-approach", "equals": "APPROACHING"}}}
        result = self.run_with(binding=binding)
        self.assertFalse(result.ran)
        self.assertIn("no input rule names an `event`", result.refusal)


if __name__ == "__main__":
    unittest.main()
