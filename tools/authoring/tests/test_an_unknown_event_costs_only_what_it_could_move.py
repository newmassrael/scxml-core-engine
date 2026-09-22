"""An event nobody can say was sent is judged until it could have mattered.

A binding may leave an input `unresolved`: nobody has said which address feeds
it. For a computation the run asks whether the answer depends on it, by trying
both values (`test_an_unknown_costs_only_what_turns_on_it`). A statechart
cannot be asked that way -- its cases share ONE run, and two answers to one
case are two machines for every case after it.

⚠ What it did instead, before this: nothing. `drive` sends no event for a rule
with no address, so the open input was dropped without a word, every case was
judged on a machine that had simply never been told, and the command exited
green on a binding that cannot ship.

What it does now is narrower and exact. W3C SCXML 3.13 selects transitions from
the active configuration only, so while no active state acts on the open event,
sending it and not sending it leave the same machine and the case is judged.
From the first configuration in which one could, the run is unknown, and every
case from there -- that one included -- is withheld. Asserted here:

  judged      an open event no active state ever acts on costs nothing, and
              the run still does not exit green
  withheld    from the first configuration that acts on it, every case is
              withheld and names the event and the state; before it, judged
  both ends   a configuration is asked before the first send AND after each
              one, because the open event could sit anywhere among them
  who acts    a state acts on an event through a transition, through forwarding
              it to an invoked child, or through an eventless condition that
              reads `_event`, which every dequeued event rebinds
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import descriptor_matches, read_document
from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, verify
from tests.test_the_commands_work_as_a_product import run

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

# The crossing signal, with one more thing it can be told: a fault in the
# gate, answered by whichever state `acting_in` names. `maintenance` is never
# entered by any case below.
STATES = ("unknown", "clear", "approaching", "maintenance")
TRANSITIONS = {
    "unknown": '<transition event="train.cleared" target="clear"/>',
    "clear": ('<transition event="train.approaching" target="approaching">'
              '<send event="signal.flashing" type="x-sce-host"/></transition>'),
    "approaching": ('<transition event="train.cleared" target="clear">'
                    '<send event="signal.dark" type="x-sce-host"/></transition>'),
    "maintenance": "",
}
FAULT = ('<transition event="gate.fault" target="maintenance">'
         '<send event="signal.flashing" type="x-sce-host"/></transition>')


def document(acting_in: str) -> str:
    states = "\n".join(
        f'  <state id="{s}">{TRANSITIONS[s]}{FAULT if s == acting_in else ""}</state>'
        for s in STATES)
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="unknown" sce:kind="statechart">
{states}
</scxml>
"""


KNOWN_INPUTS = {
    "approaching": {"address": "plant/in/train-approach",
                    "becomes": "APPROACHING", "event": "train.approaching"},
    "cleared": {"address": "plant/in/train-approach",
                "becomes": "CLEAR", "event": "train.cleared"},
}
OPEN_FAULT = {"unresolved": "the platform list naming the gate fault is not out",
              "event": "gate.fault"}


def binding(**inputs):
    return {
        "version": 1,
        "document": "signal.scxml",
        "inputs": {**KNOWN_INPUTS, **inputs},
        "outputs": {
            "roadSignal": {
                "address": "plant/out/road-signal", "field": "value",
                "sent": {"processor": "x-sce-host"},
                "when_nothing_sent": "signal.dark",
                "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"},
            },
        },
    }


def case(name, value, expect):
    return {"name": name, "given": {"plant/in/train-approach": value},
            "drove": ["plant/in/train-approach"],
            "expect": {"plant/out/road-signal.value": expect}}


# One run: unknown -> clear -> approaching -> clear.
CASES = [case("reported clear", "CLEAR", "DARK"),
         case("a train approaches", "APPROACHING", "FLASHING"),
         case("the train has gone", "CLEAR", "DARK")]


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AnOpenEventIsJudgedUntilItCouldAct(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "ordered": True, "cases": CASES}),
            encoding="utf-8")
        self.binding = self.tmp / "b.yaml"

    def judge(self, acting_in, bound):
        (self.tmp / "signal.scxml").write_text(document(acting_in), encoding="utf-8")
        self.binding.write_text(yaml.safe_dump(bound), encoding="utf-8")
        result = verify(load_pack(self.tmp), self.binding)
        self.assertTrue(result.ran, result.refusal)
        return result

    def counts(self, result):
        return (result.passed, result.failed, result.unjudged)

    def test_an_event_no_active_state_acts_on_costs_nothing(self):
        result = self.judge("maintenance", binding(fault=OPEN_FAULT))
        self.assertEqual((3, 0, 0), self.counts(result),
                         [(r.name, r.refusal) for r in result.results])
        self.assertEqual(0, result.undetermined)
        # ⚠ The half the old run lacked: the open input is REPORTED, which is
        # what keeps the status from reading an unshippable binding as done.
        self.assertEqual({"fault": OPEN_FAULT["unresolved"]}, result.unresolved)

    def test_from_the_first_state_that_acts_on_it_nothing_is_judged(self):
        """⚠ The discriminator. Before this change the three cases passed:
        the open event was never sent, so a machine that WOULD have gone to
        maintenance on a fault in `approaching` was judged as if it could not.
        """
        result = self.judge("approaching", binding(fault=OPEN_FAULT))
        self.assertEqual((1, 0, 2), self.counts(result),
                         [(r.name, r.refusal) for r in result.results])
        first, second, third = result.results
        self.assertTrue(first.passed, "before the machine could act, it is judged")
        for withheld in (second, third):
            self.assertIn("gate.fault", withheld.refusal)
            self.assertIn("'approaching'", withheld.refusal)
            self.assertIn("'a train approaches'", withheld.refusal,
                          "names where the run stopped being known")
            self.assertEqual(["plant/out/road-signal.value"], withheld.undetermined)
        self.assertEqual(2, result.undetermined)

    def test_the_configuration_is_asked_before_the_first_send(self):
        """⚠ The initial state acts on the fault and no case ever returns to
        it. Asking only after each send would judge all three."""
        result = self.judge("unknown", binding(fault=OPEN_FAULT))
        self.assertEqual((0, 0, 3), self.counts(result),
                         [(r.name, r.refusal) for r in result.results])
        self.assertIn("'reported clear'", result.results[0].refusal)

    def test_and_after_each_send(self):
        """The first case's own send enters `clear`, which acts on it. Asking
        only before each case's first send would judge that case, and the
        test above would judge the second -- the machine entered
        `approaching` by that case's own send."""
        result = self.judge("clear", binding(fault=OPEN_FAULT))
        self.assertEqual((0, 0, 3), self.counts(result),
                         [(r.name, r.refusal) for r in result.results])

    def test_the_same_document_with_the_input_named_is_judged_whole(self):
        """The other half of the discriminator: withholding is about the open
        input, not about the document. Named, the fault is an input no case
        drives, and all three cases are judged."""
        named = {"address": "plant/in/barrier-position", "becomes": "DOWN",
                 "event": "gate.fault"}
        result = self.judge("approaching", binding(fault=named))
        self.assertEqual((3, 0, 0), self.counts(result),
                         [(r.name, r.refusal) for r in result.results])
        self.assertEqual({}, result.unresolved)

    def test_the_command_does_not_exit_green_on_an_open_event(self):
        argv = ["verify", "--pack", str(self.tmp), "--binding", str(self.binding)]
        self.judge("maintenance", binding(fault=OPEN_FAULT))
        code, said, _ = run(argv)
        self.assertIn("3 passed, 0 failed", said, "the premise: nothing failed")
        self.assertEqual(1, code, said)
        self.assertIn("not a pass", said)
        # Without the open input the same run exits green, so the status above
        # is about the input and not a command that always says 1.
        self.judge("maintenance", binding())
        code, said, _ = run(argv)
        self.assertEqual(0, code, said)


def _read(text: str):
    with tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp) / "d.scxml"
        path.write_text(text, encoding="utf-8")
        return read_document(path)


def scxml(body: str, datamodel: str = "null") -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="{datamodel}" initial="a">
{body}
</scxml>
"""


class WhoActsOnAnEvent(unittest.TestCase):
    def test_a_transition_acts_on_what_its_descriptor_matches(self):
        got = _read(scxml('<state id="a"><transition event="gate" target="b"/></state>'
                          '<state id="b"/>'))
        self.assertEqual(["a"], got.states_acting_on("gate.fault"))
        self.assertEqual([], got.states_acting_on("gateway"))

    def test_a_forwarding_state_acts_on_every_event(self):
        """W3C SCXML 6.4: `autoforward` sends the child every external event
        this session receives, selected by a transition here or not."""
        got = _read(scxml(
            '<state id="a"><invoke type="scxml" autoforward="true">'
            '<content><scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">'
            '<state id="inner"><transition event="other" target="inner"/></state>'
            '</scxml></content></invoke></state>'))
        self.assertEqual(["a"], got.states_acting_on("gate.fault"))

    def test_an_inline_child_is_another_session(self):
        """⚠ Its states are not this document's. Read as this document's, the
        child's `inner` would be reported as acting here."""
        got = _read(scxml(
            '<state id="a"><invoke type="scxml">'
            '<content><scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">'
            '<state id="inner"><transition event="gate" target="inner"/></state>'
            '</scxml></content></invoke></state>'))
        self.assertEqual([], got.states_acting_on("gate.fault"))

    def test_an_eventless_condition_reading_the_event_acts_on_every_event(self):
        """W3C SCXML 5.10: `_event` is rebound by every dequeued event, and
        eventless transitions are selected again after it."""
        got = _read(scxml(
            '<state id="a"><transition cond="_event.name == \'x\'" target="b"/></state>'
            '<state id="b"><transition cond="true" target="a"/></state>',
            datamodel="ecmascript"))
        self.assertEqual(["a"], got.states_acting_on("gate.fault"))

    def test_a_script_that_reads_the_event_reaches_every_eventless_condition(self):
        got = _read(scxml(
            '<script>function seen() { return _event.name; }</script>'
            '<state id="a"><transition cond="seen() == \'x\'" target="b"/></state>'
            '<state id="b"/>', datamodel="ecmascript"))
        self.assertEqual(["a"], got.states_acting_on("gate.fault"))

    def test_a_state_nobody_can_ask_about_is_taken_to_be_active(self):
        """⚠ A state with no id cannot be named to `In()`. The question is
        whether the event COULD matter, and not being able to ask is a yes --
        answering no would judge a run the event may have moved."""
        from sce_author.verify import StatechartRun
        got = _read(scxml('<state><transition event="gate" target="b"/></state>'
                          '<state id="b"/>'))
        self.assertEqual([None], got.states_acting_on("gate.fault"))

        class NeverAsked:
            def is_state_active(self, *_):
                raise AssertionError("a state with no id cannot be asked about")

        run = StatechartRun.__new__(StatechartRun)
        run.engine = type("Engine", (), {"policy": NeverAsked()})()
        self.assertIsNotNone(run.acting_on("gate.fault", got))

    def test_the_descriptor_forms_w3c_calls_one(self):
        """W3C SCXML 3.12.1: `error`, `error.` and `error.*` are functionally
        equivalent. `error.` matched nothing before, not even `error`."""
        for descriptor in ("error", "error.", "error.*"):
            self.assertTrue(descriptor_matches(descriptor, "error"), descriptor)
            self.assertTrue(descriptor_matches(descriptor, "error.send"), descriptor)
            self.assertFalse(descriptor_matches(descriptor, "errors"), descriptor)
        self.assertTrue(descriptor_matches("*", "anything.at.all"))


if __name__ == "__main__":
    unittest.main()
