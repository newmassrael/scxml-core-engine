"""An accepted statechart says that the check did not run it.

Measured 2026-10-02 (Sonnet, headless, only this server's tools): asked to
"make pseudocode from this specification", three runs out of three never called
`scxml_scenarios`; asked also to check that the design behaves as the
specification says, three out of three did. The tool was in the list and in no
instruction the neutral request reached. What a client reads on every check is
the answer, so the answer is where it is told that the design was checked and
not run, and what to do before calling it finished.

The field and the sentence come from one function, so they cannot disagree, and
neither is there when there is nothing to play: a refused document, or one the
product accepted as a kind that is not a statechart.
"""

from __future__ import annotations

import json
import unittest

from sce_author import mcp
from sce_author.mcp import call_tool
from sce_author.verify import _default_codegen

NS = ('xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" '
      'version="1.0"')

STATECHART = f"""<scxml {NS} initial="Idle">
  <state id="Idle"><transition event="go" target="Busy"/></state>
  <state id="Busy"><transition event="done" target="Idle"/></state>
</scxml>
"""

ORPHAN = f"""<scxml {NS} initial="Idle">
  <state id="Idle"/>
  <state id="Orphan"/>
</scxml>
"""

CLOSED_STATECHART = f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml {NS} name="job" initial="idle" datamodel="ecmascript" sce:interface="closed">
  <sce:import src="job_requested.scxml" kind="event-schema" as="Requested"/>
  <state id="idle">
    <transition event="job.requested" target="running"/>
  </state>
  <state id="running">
    <transition event="job.requested" target="idle"/>
  </state>
</scxml>
"""

EVENT_SCHEMA = f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml {NS} sce:kind="event-schema" name="job_requested" sce:event-name="job.requested">
  <datamodel sce:payload="none"/>
</scxml>
"""


def answered(name: str, arguments: dict) -> dict:
    """The JSON block of the answer. A refusal is an answer too: it comes back
    flagged `isError` with the product's verdict in it."""
    return json.loads(call_tool(name, arguments)["content"][0]["text"])


@unittest.skipUnless(_default_codegen().exists(),
                     "the verdict is the product's; build sce-codegen first")
class AnAcceptedStatechartSaysItWasCheckedAndNotRun(unittest.TestCase):
    def test_an_accepted_statechart_is_told_to_play_examples_before_it_is_finished(self):
        answer = answered("validate_scxml", {"document_text": STATECHART,
                                             "document_name": "client.scxml"})
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual({"verdict": "not played",
                          "reason": "no scenario set was played into this design"},
                         answer["behaviour"])
        self.assertIn("a check of the document, not a run of it", answer["show"])
        self.assertIn("scxml_scenarios", answer["show"])
        self.assertIn("word for word", answer["show"])
        self.assertIn("THESE examples", answer["show"])

    def test_a_refused_statechart_has_nothing_to_play(self):
        answer = answered("validate_scxml", {"document_text": ORPHAN,
                                             "document_name": "client.scxml"})
        self.assertEqual("refused", answer["verdict"])
        self.assertNotIn("behaviour", answer)
        self.assertNotIn("show", answer)

    def test_a_document_accepted_as_another_kind_has_nothing_to_play(self):
        answer = answered("validate_scxml", {"document_text": EVENT_SCHEMA,
                                             "document_name": "job_requested.scxml"})
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual("event-schema", answer["manifest"]["document_kind"]["name"])
        self.assertNotIn("behaviour", answer)
        self.assertNotIn("scxml_scenarios", answer["show"])

    def test_a_set_with_a_statechart_is_told_the_same(self):
        answer = answered("validate_scxml_set", {"documents_text": [
            {"name": "job.scxml", "text": CLOSED_STATECHART},
            {"name": "job_requested.scxml", "text": EVENT_SCHEMA}]})
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual("not played", answer["behaviour"]["verdict"])
        self.assertIn("scxml_scenarios", answer["show"])

    def test_the_requirements_sentence_is_still_there_beside_it(self):
        answer = answered("validate_scxml", {"document_text": STATECHART,
                                             "document_name": "client.scxml"})
        self.assertIn("SCE has not measured it", answer["show"])
        self.assertIn("not a run of it", answer["show"])

    def test_the_step_before_the_reply_comes_first_in_what_to_show(self):
        """Measured 2026-10-02 (Sonnet, headless, four specifications, a neutral
        request, twelve runs each): with the play instruction LAST in this text no
        run called scxml_scenarios (0/12); first, as the step before replying, seven
        did (7/12); first, with the scenario set's interface filled in, ten (10/12).
        A client reads `show` as what to show, in order, and an instruction at its
        end reads as one more thing to offer the owner."""
        answer = answered("validate_scxml", {"document_text": STATECHART,
                                             "document_name": "client.scxml"})
        shown = answer["show"]
        self.assertTrue(shown.startswith("BEFORE you write your reply to the owner:"), shown)
        self.assertLess(shown.index("scxml_scenarios"), shown.index("Show each page verbatim"))
        self.assertLess(shown.index("not a run of it"), shown.index("SCE has not measured it"))


@unittest.skipUnless(_default_codegen().exists(),
                     "the verdict is the product's; build sce-codegen first")
class TheScenarioSetIsHandedOverWithItsInterfaceFilledIn(unittest.TestCase):
    def test_the_designs_own_inputs_and_outputs_are_the_interface_of_the_skeleton(self):
        answer = answered("validate_scxml", {"document_text": STATECHART,
                                             "document_name": "client.scxml"})
        surface = answer["manifest"]["surface"]
        skeleton = answer["scenarios_skeleton"]
        self.assertEqual("sce-scenario-set", skeleton["record"])
        self.assertEqual("ai-proposed", skeleton["origin"])
        self.assertEqual(surface["inputs"], [i["name"] for i in skeleton["interface"]["inputs"]])
        self.assertEqual(surface["outputs"],
                         [o["name"] for o in skeleton["interface"]["outputs"]])
        self.assertIn("`scenarios_skeleton` is the scenario set to fill in", answer["show"])
        # What the client writes is marked as the client's, not filled in.
        [scenario] = skeleton["scenarios"]
        self.assertTrue(scenario["quote"].startswith("<"), scenario)

    def test_no_skeleton_where_nothing_is_to_be_played(self):
        for name, document in (("refused", ORPHAN), ("event-schema", EVENT_SCHEMA)):
            with self.subTest(name):
                answer = answered("validate_scxml", {"document_text": document,
                                                     "document_name": "client.scxml"})
                self.assertNotIn("scenarios_skeleton", answer)

    def test_the_skeleton_is_a_set_the_scenario_tool_reads(self):
        """Filled in with one real scenario it is a usable set: the names it offers are
        the names the design presents, so no mismatch with the interface can come of it."""
        answer = answered("validate_scxml", {"document_text": STATECHART,
                                             "document_name": "client.scxml"})
        skeleton = json.loads(json.dumps(answer["scenarios_skeleton"]))
        skeleton["specification"]["doc_id"] = "client"
        skeleton["scenarios"] = [{"id": "S1", "quote": "Going makes it busy.",
                                  "steps": [{"send": "go", "expect": {"finished": False}}]}]
        reply = answered("scxml_scenarios", {
            "scenarios_text": json.dumps(skeleton),
            "documents_text": [{"name": "client.scxml", "text": STATECHART}]})
        interface = reply["interface"]
        self.assertEqual([], interface["unserved_inputs"], reply)
        self.assertEqual([], interface["unaccepted_inputs"], reply)
        self.assertTrue(interface["matches"], reply)


class TheToolTheAnswerNamesIsOneTheServerHas(unittest.TestCase):
    def test_the_instructions_and_the_sentence_name_a_tool_that_is_listed(self):
        listed = {tool["name"] for tool in mcp.TOOLS}
        self.assertIn("scxml_scenarios", listed)
        self.assertIn("scxml_scenarios", mcp.SERVER_INSTRUCTIONS)
        self.assertIn("scxml_scenarios", mcp._UNPLAYED)


if __name__ == "__main__":
    unittest.main()
