"""A draft cites the owner's answers, and is held to them.

Measured 2026-09-29: five drafts of one specification marked between four
and eleven open questions each, the same question spelled up to three ways,
and no two drafts the same set. The decision record is where the owner
answers each question once; a draft cites the answer by its id, and
`decisions` refuses a draft that guessed where no answer licenses it.

Every case here is judged by the product's own reading of the markers
(`sce-codegen unresolved`) and anchors (`sce-codegen requirements`), so a
case that passes against a hand-rolled reader and fails against the product
cannot exist.
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.decisions import DecisionRecordError, hold, load_record
from sce_author.mcp import call_tool
from sce_author.verify import _default_codegen

# A door whose restart policy is a decision variable, whose close is a
# guess on a transition, and which asks one question on the closed state's
# clause.
DOOR = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="door" initial="closed" datamodel="ecmascript">
  <datamodel>
    <data id="restart" sce:assumed="D1" sce:assumed-reason="an open request while open"
          sce:assumed-candidates="true false" expr="true"/>
  </datamodel>
  <state id="closed" sce:provenance="door-spec#1:page=2">
    <transition event="open" target="open" sce:unresolved="while-moving"
                sce:unresolved-reason="what an open request does while it moves"/>
  </state>
  <state id="open">
    <transition event="close" target="closed" sce:assumed="D2"/>
  </state>
</scxml>
"""

CLAUSE = {"doc_id": "door-spec", "section": "1", "at": {"page": 2}}


def record(*decisions: dict) -> dict:
    return {"record": "sce-decision-record", "v": 1,
            "specification": {"doc_id": "door-spec", "rev": "3"},
            "decisions": list(decisions)}


D1 = {"id": "D1", "anchor": CLAUSE,
      "question": "Does an open request while open restart the timer?",
      "answer": "Yes.", "candidates": ["true", "false"], "chosen": "true"}
D2 = {"id": "D2", "question": "Does close always close?", "answer": "Yes."}
ANSWERED = record(D1, D2)


@unittest.skipUnless(_default_codegen().exists(),
                     "the markers are the product's to read; build sce-codegen first")
class ADraftIsHeldToTheDecisionRecord(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def write(self, name: str, text: str) -> pathlib.Path:
        path = self.dir / name
        path.write_text(text, encoding="utf-8")
        return path

    def held(self, document: str, decisions: dict) -> tuple[str, dict]:
        report, refused = hold(self.write("door.scxml", document),
                               self.write("decisions.json", json.dumps(decisions)))
        answer = json.loads(report or refused)
        return ("refused" if refused else "holds"), answer

    def findings(self, answer: dict) -> dict:
        return {(f["finding"], f.get("marker") or f.get("decision")): f
                for f in answer["findings"]}

    # ------------------------------------------------------------ holds

    def test_a_draft_that_cites_the_answers_holds(self):
        verdict, answer = self.held(DOOR, ANSWERED)
        self.assertEqual(("holds", "holds"), (verdict, answer["verdict"]))
        found = self.findings(answer)
        self.assertIn(("applied", "D1"), found)
        self.assertIn(("applied", "D2"), found)
        self.assertEqual(answer["specification"], {"doc_id": "door-spec", "rev": "3"})

    def test_a_new_question_is_reported_beside_the_recorded_one_on_its_clause(self):
        # `while-moving` sits on a transition of the `closed` state, whose
        # clause D1 is recorded on: the owner is shown both, and asked.
        verdict, answer = self.held(DOOR, ANSWERED)
        self.assertEqual("holds", verdict, "a new question is never refused")
        new = self.findings(answer)[("new-question", "while-moving")]
        self.assertFalse(new["refuses"])
        self.assertEqual(["D1"], new["alongside"])
        self.assertIn("ask the owner each new question", answer["next"])

    def test_a_new_question_on_another_clause_is_shown_alone(self):
        other = dict(D1, anchor={"doc_id": "door-spec", "section": "4"})
        _, answer = self.held(DOOR, record(other, D2))
        self.assertNotIn("alongside", self.findings(answer)[("new-question", "while-moving")])

    def test_a_question_asked_and_still_open_is_listed(self):
        asked = {"id": "while-moving", "question": "What does open do while moving?"}
        verdict, answer = self.held(DOOR, record(D1, D2, asked))
        self.assertEqual("holds", verdict)
        self.assertIn(("still-open", "while-moving"), self.findings(answer))

    def test_an_answer_no_marker_cites_is_reported(self):
        unused = {"id": "D9", "question": "Is there a light?", "answer": "No."}
        verdict, answer = self.held(DOOR, record(D1, D2, unused))
        self.assertEqual("holds", verdict)
        self.assertFalse(self.findings(answer)[("uncited-answer", "D9")]["refuses"])

    # ------------------------------------------------------------ refused

    def test_a_guess_that_cites_no_decision_is_refused(self):
        verdict, answer = self.held(DOOR, record(D1))
        self.assertEqual("refused", verdict)
        self.assertTrue(self.findings(answer)[("uncited-guess", "D2")]["refuses"])

    def test_a_guess_on_a_question_not_answered_yet_is_refused(self):
        open_d2 = {"id": "D2", "question": "Does close always close?"}
        verdict, answer = self.held(DOOR, record(D1, open_d2))
        self.assertEqual("refused", verdict)
        found = self.findings(answer)[("guess-on-an-open-question", "D2")]
        self.assertIn('sce:unresolved="D2"', found["message"])

    def test_a_question_the_owner_already_answered_is_refused(self):
        answered = {"id": "while-moving", "question": "What does open do while moving?",
                    "answer": "It reverses."}
        verdict, answer = self.held(DOOR, record(D1, D2, answered))
        self.assertEqual("refused", verdict)
        found = self.findings(answer)[("answered-left-open", "while-moving")]
        self.assertIn('sce:assumed="while-moving"', found["message"])

    def test_a_decision_variable_holding_another_value_is_refused(self):
        verdict, answer = self.held(DOOR, record(dict(D1, chosen="false"), D2))
        self.assertEqual("refused", verdict)
        found = self.findings(answer)[("holds-another-value", "D1")]
        self.assertIn("'true'", found["message"])
        self.assertEqual("variables[0]", found["node_path"])

    def test_an_answer_the_draft_does_not_list_as_a_candidate_is_refused(self):
        widened = dict(D1, candidates=["true", "false", "never"], chosen="never")
        verdict, answer = self.held(DOOR, record(widened, D2))
        self.assertEqual("refused", verdict)
        self.assertIn(("not-a-candidate", "D1"), self.findings(answer))

    def test_a_value_answer_on_a_marker_with_no_value_is_left_to_a_person(self):
        valued = dict(D2, candidates=["always", "never"], chosen="always")
        verdict, answer = self.held(DOOR, record(D1, valued))
        self.assertEqual("holds", verdict)
        self.assertFalse(self.findings(answer)[("answer-not-read", "D2")]["refuses"])

    def test_a_forge_documents_markers_are_held_too(self):
        # The product reads a forge document's markers through its own
        # expansion; the check reads what the product reads.
        transform = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       name="scaled" sce:kind="transform">
  <datamodel>
    <data id="raw" sce:type="int32" sce:direction="in"/>
    <data id="scaled" sce:type="int32" sce:direction="out" expr="raw * 2"
          sce:assumed="GAIN" sce:assumed-candidates="2 3"/>
  </datamodel>
</scxml>
"""
        gain = {"id": "GAIN", "question": "What is the gain?", "answer": "Three.",
                "candidates": ["2", "3"], "chosen": "3"}
        verdict, answer = self.held(transform, record(gain))
        self.assertEqual("refused", verdict)
        self.assertIn(("holds-another-value", "GAIN"), self.findings(answer))
        verdict, _ = self.held(transform, record(dict(gain, chosen="2", answer="Two.")))
        self.assertEqual("holds", verdict)

    # ------------------------------------------------------------ the record

    def test_a_record_that_is_not_one_is_refused_naming_it(self):
        cases = {
            "not JSON": "{",
            "no record marker": json.dumps({"v": 1, "specification": {"doc_id": "x"},
                                            "decisions": []}),
            "a choice without an answer": json.dumps(record(
                {"id": "D1", "question": "q?", "chosen": "true"})),
            "an id twice": json.dumps(record(D2, D2)),
            "a choice outside its candidates": json.dumps(record(dict(D1, chosen="maybe"))),
            "an answered date that is not a date": json.dumps(record(
                dict(D2, answered="yesterday"))),
            "an id with a space": json.dumps(record(dict(D2, id="D 2"))),
        }
        for why, text in cases.items():
            with self.subTest(why=why):
                path = self.write("bad.json", text)
                with self.assertRaises(DecisionRecordError) as caught:
                    load_record(path)
                self.assertIn("bad.json", str(caught.exception))

    def test_the_answered_record_loads_in_its_own_order(self):
        path = self.write("decisions.json", json.dumps(ANSWERED))
        loaded = load_record(path)
        self.assertEqual(["D1", "D2"], list(loaded.decisions))
        self.assertTrue(loaded.decisions["D1"].answered)

    # ------------------------------------------------------------ surfaces

    def test_the_command_line_exits_one_only_on_a_refusal(self):
        document = self.write("door.scxml", DOOR)

        def run(decisions: dict) -> tuple[int, str]:
            path = self.write("decisions.json", json.dumps(decisions))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = main(["decisions", "--document", str(document),
                             "--decisions", str(path),
                             "--out", str(self.dir / "report.json")])
            return code, out.getvalue()

        code, printed = run(ANSWERED)
        self.assertEqual(0, code, printed)
        self.assertIn("new-question", printed)
        code, printed = run(record(D1))
        self.assertEqual(1, code)
        self.assertIn("REFUSED", printed)
        self.assertEqual("refused", json.loads(
            (self.dir / "report.json").read_text(encoding="utf-8"))["verdict"])

    def test_a_remote_client_hands_the_draft_and_the_record_over_as_text(self):
        schema = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       name="door_open" sce:kind="event-schema" sce:event-name="open">
  <datamodel><data id="by" sce:type="int32" sce:direction="in"/></datamodel>
</scxml>
"""
        # The draft imports a schema, which has to arrive beside it.
        importing = DOOR.replace(
            '<datamodel>',
            '<sce:import src="door_open.scxml" kind="event-schema" as="Open"/>\n'
            '  <datamodel>', 1)
        answer = call_tool("decisions", {
            "document_text": importing, "document_name": "door.scxml",
            "decisions_text": json.dumps(ANSWERED),
            "files_text": [{"name": "door_open.scxml", "text": schema}],
        }, remote=True)
        self.assertFalse(answer.get("isError"), answer)
        self.assertEqual("holds", json.loads(answer["content"][0]["text"])["verdict"])

        refused = call_tool("decisions", {
            "document_text": importing, "document_name": "door.scxml",
            "decisions_text": json.dumps(record(D1)),
            "files_text": [{"name": "door_open.scxml", "text": schema}],
        }, remote=True)
        self.assertTrue(refused.get("isError"))
        self.assertEqual("refused", json.loads(refused["content"][0]["text"])["verdict"])

    def test_a_remote_client_cannot_name_this_machines_files(self):
        answer = call_tool("decisions", {
            "document": str(self.write("door.scxml", DOOR)),
            "decisions_text": json.dumps(ANSWERED)}, remote=True)
        self.assertTrue(answer.get("isError"))
        self.assertIn("document_text", answer["content"][0]["text"])


if __name__ == "__main__":
    unittest.main()
