"""A design that sends to its parent is played only against a caller the owner named.

A machine written from prose sends its notices to `#_parent` because a send needs
a target, while the specification never says who the caller is. Started on its
own, every such send raises `error.communication` (W3C SCXML C.1), and the driver
used to class the run with every other unanswered error: a defect of the design.
It is not one. The design is waiting for a fact only the owner has, and a verdict
that says "failed" about a design that did nothing wrong, or one that plays it
against a parent the driver made up, are the two ways to be dishonest here.

So the run says what it is: with a question open on the send, BLOCKED by that
decision; with none recorded, a design that sends to a parent nobody named and
says how to record it; and with the owner's route in the interface, played
against a parent that records what it was sent and answers nothing.
"""

from __future__ import annotations

import json
import pathlib
import tempfile
import unittest

from sce_author.scenario_driver import run
from sce_author.verify import _default_codegen

SCXML_PROCESSOR = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"

NOTIFIER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       initial="idle">
  <state id="idle">
    <transition event="go" target="done">
      {send}
    </transition>
    <transition event="stray" target="done">
      {stray}
    </transition>
  </state>
  {done}
</scxml>
"""

ASKED = ('<send event="notice" target="#_parent" sce:unresolved="caller-target" '
         'sce:unresolved-reason="the specification does not say who the caller is"/>')
PLAIN = '<send event="notice" target="#_parent"/>'
ASSUMED = ('<send event="notice" target="#_parent" sce:assumed="parent-by-default" '
           'sce:assumed-reason="the standing rule says a notice goes to the parent"/>')
NOWHERE = '<send event="lost" target="#_nobody"/>'


def notifier(send: str, stray: str = "", final: bool = False) -> str:
    """The machine that notifies on `go`. It ends in a final state only when
    asked: a machine that has finished never handles the error a send to a
    missing parent raises, which is the case the driver used to get wrong."""
    done = '<final id="done"/>' if final else '<state id="done"/>'
    return NOTIFIER.format(send=send, stray=stray, done=done)


def codegen_is_built() -> bool:
    return _default_codegen().exists()


# The domain-free job of the authoring-core lane builds nothing on purpose, so a
# case that spawns the product's generator is skipped there and judged where the
# generator is built.
@unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
class TestACallerNobodyNamed(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.work = pathlib.Path(self.directory.name)
        self.codegen = _default_codegen()

    def answer(self, send: str, stray: str = "", via: dict | None = None,
               scenario: str = "go", final: bool = False) -> dict:
        design = self.work / "notifier.scxml"
        design.write_text(notifier(send, stray, final), encoding="utf-8")
        output = {"name": "notice"}
        if via is not None:
            output["via"] = via
        spec = {
            "record": "sce-scenario-set", "v": 1,
            "specification": {"doc_id": "notifier", "rev": "1"},
            "origin": "ai-proposed",
            "interface": {
                "why": "The specification names the signals and does not say who the caller is.",
                "inputs": [{"name": "go"}, {"name": "stray"}],
                "outputs": [output],
            },
            "scenarios": [{
                "id": "N1",
                "quote": "When go arrives, the machine sends notice to its caller",
                "steps": [{"send": scenario,
                           "expect": {"outbound": [{"event": "notice"}], "finished": final}}],
            }],
        }
        path = self.work / "notifier.scenarios.json"
        path.write_text(json.dumps(spec), encoding="utf-8")
        return run(path, design, self.codegen)

    @staticmethod
    def verdict(answer: dict) -> dict:
        return next(r for r in answer["judgement"] if r["kind"] == "verdict")

    def test_a_send_that_carries_an_open_question_blocks_the_example_by_it(self):
        """The verdict is BLOCKED and the cause is the decision, which is neither
        the design's defect nor a fact about the machine. The reason names the
        decision, so the owner knows what to answer, and the run counts no gap
        and no failure."""
        answer = self.answer(ASKED)
        verdict = self.verdict(answer)
        self.assertEqual("blocked", verdict["verdict"], answer["judgement"])
        self.assertEqual("decision", verdict.get("cause"), verdict)
        self.assertIn("open decision `caller-target`", verdict["reason"])
        self.assertIn("does not invent one", verdict["reason"])
        summary = answer["judgement"][0]
        self.assertEqual(0, summary["fail"], summary)
        self.assertEqual(0, summary["pass"], summary)

    def test_a_machine_that_finishes_on_the_send_is_blocked_too(self):
        """A machine that reaches a final state in the macrostep that sends never
        handles the `error.communication` the send raises, so no error is left
        unanswered to read. The run used to go on, and the example failed on an
        output that was never sent, which is a defect claimed about a design that
        was only waiting for a caller. The send is what is detected, not the error."""
        for send, cause, verdict_word in ((ASKED, "decision", "blocked"),
                                          (PLAIN, "design", None)):
            with self.subTest(send=send):
                answer = self.answer(send, final=True)
                verdict = self.verdict(answer)
                self.assertEqual(cause, verdict.get("cause"), answer["judgement"])
                self.assertEqual(0, answer["judgement"][0]["fail"], answer["judgement"][0])
                if verdict_word:
                    self.assertEqual(verdict_word, verdict["verdict"], verdict)
                else:
                    self.assertNotEqual("fail", verdict["verdict"], verdict)

    def test_a_send_with_no_recorded_question_is_the_designs_and_says_how_to_record_one(self):
        answer = self.answer(PLAIN)
        verdict = self.verdict(answer)
        self.assertNotEqual("pass", verdict["verdict"], verdict)
        self.assertEqual("design", verdict.get("cause"), verdict)
        self.assertIn("neither the interface nor the specification's decisions", verdict["reason"])
        self.assertIn("sce:unresolved", verdict["reason"])
        self.assertNotEqual("blocked", verdict["verdict"], verdict)

    def test_an_assumed_value_is_not_a_question(self):
        """`sce:assumed` is a value chosen without an answer and applied. It is not
        a route nobody decided, so it does not turn the run into a blocked one."""
        verdict = self.verdict(self.answer(ASSUMED))
        self.assertEqual("design", verdict.get("cause"), verdict)
        self.assertNotEqual("blocked", verdict["verdict"], verdict)
        self.assertNotIn("parent-by-default", verdict["reason"])

    def test_the_owners_route_through_the_parent_plays_the_example(self):
        """With the interface routing the output through `#_parent`, the machine is
        given a parent that records what it is sent and answers nothing, and the
        example is judged like any other: the event was observed leaving."""
        via = {"type": SCXML_PROCESSOR, "target": "#_parent"}
        answer = self.answer(PLAIN, via=via)
        self.assertEqual("pass", self.verdict(answer)["verdict"], answer["judgement"])

    def test_the_route_is_the_engines_not_the_interfaces(self):
        """The engine delivers to `#_parent` through the SCXML Event I/O Processor
        and no other. An interface naming another type for its parent is not what
        the machine does, and is refused with both routes, not agreed with."""
        via = {"type": "x-caller", "target": "#_parent"}
        verdict = self.verdict(self.answer(PLAIN, via=via))
        self.assertNotEqual("pass", verdict["verdict"], verdict)
        self.assertIn("x-caller", verdict["reason"])
        self.assertIn(SCXML_PROCESSOR, verdict["reason"])

    def test_an_error_the_missing_parent_did_not_raise_is_not_laid_to_it(self):
        """The design also sends to a target nothing answers at. The example plays
        that one, the engine raises `error.communication`, and the open parent
        send is never reached. Giving the machine a parent does not change the
        run, so the open question is not what stopped it, and the refusal stays
        the design's with its own words."""
        answer = self.answer(ASKED, stray=NOWHERE, scenario="stray")
        verdict = self.verdict(answer)
        self.assertEqual("design", verdict.get("cause"), verdict)
        self.assertNotEqual("blocked", verdict["verdict"], verdict)
        self.assertIn("error.communication", verdict["reason"])
        self.assertNotIn("to its parent", verdict["reason"])
        self.assertNotIn("does not invent one", verdict["reason"])


if __name__ == "__main__":
    unittest.main()
