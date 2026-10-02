"""A send whose route comes from data the specification left open blocks the example.

A machine written from prose chooses where a notice goes from a data item
(`targetexpr="callerTarget"`) because a send needs a target, while the
specification never says who the caller is, and the draft marks that item as an
open question. Played, the send raises `error.communication` (W3C SCXML C.1) and
the driver used to class the run with every other unanswered error: a defect of
the design, `not-judged` with the design as the cause. It is not one. The
question is on the data item, not on the send, and it is the owner's to answer.

So the generated machine raises a failure OF THE ROUTE with the questions the route
rests on; the engine keeps them with the error and gives them back for the one
nothing answered; and the driver calls the run BLOCKED by that decision. Only the
route's own failure carries them: the same send can fail in its event name, its
delay or its namelist, and those are faults of the draft that no answer to the
route's question would mend. The literal `#_parent` route has its own test file
(`test_a_send_to_a_caller_nobody_named_blocks_the_example.py`); this is the
sibling for the routes a build-time walk cannot name the value of.
"""

from __future__ import annotations

import json
import pathlib
import tempfile
import unittest

from sce_author.lowering import load
from sce_author.scenario_driver import run
from sce_author.verify import _default_codegen, generate

NOTIFIER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       initial="idle" datamodel="ecmascript">
  <datamodel>
    {data}
  </datamodel>
  <state id="idle">
    <transition event="go" target="done">
      {send}
    </transition>
    <transition event="stray" target="done">
      {stray}
    </transition>
  </state>
  <state id="done"/>
</scxml>
"""

#: Where the notice goes, left open: the draft's value is a placeholder that
#: names nothing a machine can reach.
WHERE_OPEN = ('<data id="callerTarget" expr="\'#_nobody\'" sce:unresolved="caller-target" '
              'sce:unresolved-reason="the specification does not say who the caller is"/>')
WHERE_ASSUMED = ('<data id="callerTarget" expr="\'#_nobody\'" sce:assumed="caller-by-default" '
                 'sce:assumed-reason="the standing rule says a notice goes to the caller"/>')
WHERE_PLAIN = '<data id="callerTarget" expr="\'#_nobody\'"/>'
#: A route that WORKS and still rests on the question: the send reaches itself, so
#: the only fault in a case that uses it is the one the case names.
WHERE_OPEN_AND_REACHABLE = (
    '<data id="callerTarget" expr="\'#_internal\'" sce:unresolved="caller-target" '
    'sce:unresolved-reason="the specification does not say who the caller is"/>')
UNRELATED_OPEN =('<data id="retention" expr="0" sce:unresolved="retention-period" '
                  'sce:unresolved-reason="the specification does not say how long"/>')

#: The words a refusal uses only when it lays the failure to a question. The
#: design's own refusal also lists the questions the design holds, so a
#: question's id appearing in a reason does not say it was blamed.
LAID_TO_A_QUESTION = "the send that failed chose where it goes"

NOTICE = '<send event="notice" targetexpr="callerTarget"/>'
NOTICE_ASKING = ('<send event="notice" targetexpr="callerTarget" sce:unresolved="send-shape" '
                 'sce:unresolved-reason="the specification does not say a notice is sent"/>')
NOWHERE = '<send event="lost" target="#_nobody"/>'

#: Faults of the document that sit in the same send as an open route and that no
#: answer to the route's question would mend.
FAULTS_BESIDE_AN_OPEN_ROUTE = {
    "the event name reads a variable nobody declared":
        '<send eventexpr="missingEvent" targetexpr="callerTarget"/>',
    "the delay reads a variable nobody declared":
        '<send event="notice" delayexpr="missingDelay" targetexpr="callerTarget"/>',
    "the namelist names a variable nobody declared":
        '<send event="notice" targetexpr="callerTarget" namelist="missingName"/>',
}


def codegen_is_built() -> bool:
    return _default_codegen().exists()


# The domain-free job of the authoring-core lane builds nothing on purpose, so a
# case that spawns the product's generator is skipped there and judged where the
# generator is built.
@unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
class TestARouteRestingOnAnOpenDecision(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.work = pathlib.Path(self.directory.name)
        self.codegen = _default_codegen()

    def design(self, data: str, send: str, stray: str = "") -> pathlib.Path:
        path = self.work / "notifier.scxml"
        path.write_text(NOTIFIER.format(data=data, send=send, stray=stray), encoding="utf-8")
        return path

    def answer(self, data: str, send: str, stray: str = "", scenario: str = "go") -> dict:
        design = self.design(data, send, stray)
        spec = {
            "record": "sce-scenario-set", "v": 1,
            "specification": {"doc_id": "notifier", "rev": "1"},
            "origin": "ai-proposed",
            "interface": {
                "why": "The specification names the signals and does not say who the caller is.",
                "inputs": [{"name": "go"}, {"name": "stray"}],
                "outputs": [{"name": "notice"}],
            },
            "scenarios": [{
                "id": "N1",
                "quote": "When go arrives, the machine sends notice to its caller",
                "steps": [{"send": scenario,
                           "expect": {"outbound": [{"event": "notice"}], "finished": False}}],
            }],
        }
        path = self.work / "notifier.scenarios.json"
        path.write_text(json.dumps(spec), encoding="utf-8")
        return run(path, design, self.codegen)

    @staticmethod
    def verdict(answer: dict) -> dict:
        return next(r for r in answer["judgement"] if r["kind"] == "verdict")

    def engine_after(self, data: str, send: str, stray: str, event: str):
        """The generated machine, started and sent `event`, as the driver runs it."""
        design = self.design(data, send, stray)
        into = self.work / "built"
        built = generate(design, self.codegen, into, serves=())
        self.assertFalse(built.refusal, built.refusal)
        module = load(into, design)
        engine = module.create_engine()
        engine.initialize()
        engine.send_event(engine.policy.get_event_from_name(event))
        return engine, engine.policy

    def test_a_route_read_from_an_open_data_item_blocks_the_example_by_it(self):
        """The verdict is BLOCKED and the cause is the decision, which is neither
        the design's defect nor a fact about the machine. The reason names the
        decision, so the owner knows what to answer, and the run counts no gap
        and no failure."""
        answer = self.answer(WHERE_OPEN, NOTICE)
        verdict = self.verdict(answer)
        self.assertEqual("blocked", verdict["verdict"], answer["judgement"])
        self.assertEqual("decision", verdict.get("cause"), verdict)
        self.assertIn("open decision `caller-target`", verdict["reason"])
        self.assertIn("error.communication", verdict["reason"])
        self.assertIn("does not invent one", verdict["reason"])
        summary = answer["judgement"][0]
        self.assertEqual(0, summary["fail"], summary)
        self.assertEqual(0, summary["pass"], summary)
        self.assertEqual([], [g for g in answer["judgement"] if g["kind"] == "gap"])

    def test_every_question_the_route_rests_on_is_named(self):
        """The send is marked with one question and the data it reads with
        another: the owner has two things to answer, and is told both."""
        verdict = self.verdict(self.answer(WHERE_OPEN, NOTICE_ASKING))
        self.assertEqual("blocked", verdict["verdict"], verdict)
        self.assertIn("`send-shape`, `caller-target`", verdict["reason"])

    def test_the_engine_gives_back_the_questions_the_failed_send_rested_on(self):
        engine, _ = self.engine_after(WHERE_OPEN, NOTICE_ASKING, "", "go")
        self.assertEqual(1, engine.unhandled_error_events())
        self.assertEqual(("send-shape", "caller-target"),
                         engine.last_unhandled_error_rests_on())

    def test_an_assumed_value_is_not_a_question(self):
        """`sce:assumed` is a value chosen without an answer and applied. The
        route it feeds is not one nobody decided, so the failure is the design's."""
        answer = self.answer(WHERE_ASSUMED, NOTICE)
        verdict = self.verdict(answer)
        self.assertEqual("design", verdict.get("cause"), verdict)
        self.assertNotEqual("blocked", verdict["verdict"], verdict)
        self.assertNotIn(LAID_TO_A_QUESTION, verdict["reason"])
        engine, _ = self.engine_after(WHERE_ASSUMED, NOTICE, "", "go")
        self.assertEqual(1, engine.unhandled_error_events())
        self.assertEqual((), engine.last_unhandled_error_rests_on())

    def test_a_question_the_route_does_not_read_is_not_laid_to_it(self):
        """The design holds an open question, but the failing send's route reads
        a data item that carries none. The failure is not that question showing
        through, and calling the example blocked by it would hide a design that
        sends to a place nothing answers."""
        answer = self.answer(WHERE_PLAIN + UNRELATED_OPEN, NOTICE)
        verdict = self.verdict(answer)
        self.assertEqual("design", verdict.get("cause"), verdict)
        self.assertNotEqual("blocked", verdict["verdict"], verdict)
        self.assertNotIn("does not invent one", verdict["reason"])

    def test_an_error_another_send_raised_is_not_laid_to_the_open_route(self):
        """The machine also sends to a target nothing answers at, with a literal
        route, and the example plays that one. The open route is never reached,
        so the failure is the design's with its own words."""
        answer = self.answer(WHERE_OPEN, NOTICE, stray=NOWHERE, scenario="stray")
        verdict = self.verdict(answer)
        self.assertEqual("design", verdict.get("cause"), verdict)
        self.assertNotEqual("blocked", verdict["verdict"], verdict)
        self.assertIn("error.communication", verdict["reason"])
        self.assertNotIn(LAID_TO_A_QUESTION, verdict["reason"])
        engine, _ = self.engine_after(WHERE_OPEN, NOTICE, NOWHERE, "stray")
        self.assertEqual(1, engine.unhandled_error_events())
        self.assertEqual((), engine.last_unhandled_error_rests_on())

    def test_a_fault_beside_an_open_route_is_the_documents_and_not_the_questions(self):
        """The route is sound and rests on an open question; another argument of the
        same send fails. Measured 2026-10-02, the earlier attribution (by the send's
        id) blamed the question for the event name, the delay and the namelist
        alike, and told the owner to name a caller when the draft had to be mended.
        Only a failure of the ROUTE carries the question."""
        for what, send in FAULTS_BESIDE_AN_OPEN_ROUTE.items():
            with self.subTest(what):
                verdict = self.verdict(self.answer(WHERE_OPEN_AND_REACHABLE, send))
                self.assertEqual("design", verdict.get("cause"), verdict)
                self.assertNotEqual("blocked", verdict["verdict"], verdict)
                self.assertNotIn(LAID_TO_A_QUESTION, verdict["reason"])
                self.assertIn("error.execution", verdict["reason"])
                engine, _ = self.engine_after(WHERE_OPEN_AND_REACHABLE, send, "", "go")
                self.assertEqual(1, engine.unhandled_error_events())
                self.assertEqual((), engine.last_unhandled_error_rests_on())

    def test_a_fault_has_the_same_cause_with_or_without_the_open_question(self):
        """A question marked on the route's data must not change what an unrelated
        fault is called."""
        plain = ('<data id="callerTarget" expr="\'#_internal\'"/>')
        for what, send in FAULTS_BESIDE_AN_OPEN_ROUTE.items():
            with self.subTest(what):
                asked = self.verdict(self.answer(WHERE_OPEN_AND_REACHABLE, send))
                unasked = self.verdict(self.answer(plain, send))
                self.assertEqual(unasked["verdict"], asked["verdict"])
                self.assertEqual(unasked.get("cause"), asked.get("cause"))

    def test_a_type_read_from_an_open_question_blocks_the_example_by_it(self):
        """The route is its type as much as its target."""
        data = ('<data id="callerType" expr="\'x-unsupported-processor\'" '
                'sce:unresolved="caller-type" '
                'sce:unresolved-reason="the specification does not say how the caller is reached"/>')
        answer = self.answer(data, '<send event="notice" typeexpr="callerType"/>')
        verdict = self.verdict(answer)
        self.assertEqual("blocked", verdict["verdict"], answer["judgement"])
        self.assertEqual("decision", verdict.get("cause"), verdict)
        self.assertIn("open decision `caller-type`", verdict["reason"])

    def test_a_later_send_is_not_given_the_questions_of_an_earlier_one(self):
        """An open question on a route that works fails nothing. The next send
        fails with a literal route, which says nothing about questions, and its
        error is not given the earlier send's: the engine keeps the questions of
        the send it was told of last, and gives them only to an error that
        carries that send's id."""
        reachable = ('<data id="callerTarget" expr="\'#_internal\'" sce:unresolved="caller-target" '
                     'sce:unresolved-reason="the specification does not say who the caller is"/>')
        engine, _ = self.engine_after(reachable, NOTICE + NOWHERE, "", "go")
        self.assertEqual(1, engine.unhandled_error_events())
        self.assertEqual((), engine.last_unhandled_error_rests_on())


if __name__ == "__main__":
    unittest.main()
