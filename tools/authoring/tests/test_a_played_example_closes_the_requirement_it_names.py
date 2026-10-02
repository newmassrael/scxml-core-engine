"""A played example closes the requirement it names, through the tool a client calls.

A requirement met by something NOT happening ("nothing is sent after the
response") has no node to point at, so the annotation can only call it
`needs-scenario`. `scxml_scenarios` takes the owner's requirement list
(`manifest`), and the product holds the list against the examples it played:
a `shall_not` whose every scenario passed is `scenario-passed`, one with a
failed scenario is `scenario-failed`.

What is asserted here is the tool's part, which is small on purpose: the
verdicts that move a requirement are the PRODUCT's, judged from the trace the
driver wrote (`sce-codegen requirements --scenarios --trace`), and this module
types none of them. So the tests hold that the list reaches the product, that
the product's records come back whole, and that what a pass is NOT -- not
`implemented`, not the owner's examples, not another engine -- reaches the
client in the same answer.
"""

from __future__ import annotations

import io
import json
import pathlib
import unittest

from sce_author import mcp
from sce_author.verify import _default_codegen
from tests.test_a_scenario_set_is_played_into_a_design_and_judged import RETRY_SET, machine

needs_the_generator = unittest.skipUnless(
    _default_codegen().exists(), "the verdicts are the product's; build sce-codegen first")

MANIFEST = json.dumps({
    "doc_id": "retry-client", "rev": "1",
    "extraction": {"ids": "native", "trace": "none",
                   "modality_convention": "english-modal-verbs", "method": "ai-pass-1"},
    "sections": [{"id": "1", "title": "Retry"}],
    "requirements": [
        {"id": "R1", "section": "1", "modality": "shall_not", "at": {"page": 1}},
        {"id": "R2", "section": "1", "modality": "shall_not", "at": {"page": 1}},
        {"id": "R3", "section": "1", "at": {"page": 1}},
        {"id": "R4", "section": "1", "modality": "shall_not", "at": {"page": 1}},
    ],
})


def call(**arguments) -> dict:
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": "scxml_scenarios", "arguments": arguments}}
    output = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
    return json.loads(output.getvalue())["result"]


def said(result: dict) -> dict:
    return json.loads(result["content"][0]["text"])


def linked(spec: dict) -> None:
    """Name the requirement each scenario is about: the at-most-three limit and the
    timer stopped by a response are prohibitions, the boundary is a `shall`."""
    for scenario in spec["scenarios"]:
        scenario["requirements"] = {"T3-at-most-three": ["R1"],
                                    "T5-response-stops-the-timer": ["R2"],
                                    "T2-boundary": ["R3"]}.get(scenario["id"], [])


@needs_the_generator
class APlayedExampleClosesTheRequirementItNames(unittest.TestCase):
    def ask(self, design: str | None = None, with_list: bool = True) -> dict:
        spec = json.loads(RETRY_SET.read_text(encoding="utf-8"))
        linked(spec)
        arguments = {"scenarios_text": json.dumps(spec),
                     "documents_text": [{"name": "retry.scxml",
                                         "text": machine() if design is None else design}]}
        if with_list:
            arguments["manifest_text"] = MANIFEST
        result = call(**arguments)
        self.assertFalse(result.get("isError"), result)
        return said(result)

    @staticmethod
    def outcomes(reply: dict) -> dict:
        return {r["id"]: r["outcome"] for r in reply["requirements"]["records"]
                if r.get("kind") == "requirement"}

    def test_a_prohibition_every_example_of_which_passed_is_closed_by_them(self):
        reply = self.ask()
        self.assertEqual("measured", reply["requirements"]["verdict"])
        outcomes = self.outcomes(reply)
        self.assertEqual("scenario-passed", outcomes["R1"], outcomes)
        self.assertEqual("scenario-passed", outcomes["R2"], outcomes)
        self.assertEqual(["R1", "R2"], reply["requirements"]["ids"]["scenario-passed"])

    def test_a_pass_does_not_make_a_shall_implemented_and_an_unnamed_one_stays_open(self):
        outcomes = self.outcomes(self.ask())
        # R3 is a `shall` nothing cites: a passing example is evidence beside it,
        # not a node that carries it.
        self.assertEqual("missing", outcomes["R3"], outcomes)
        # R4 is a prohibition no example names, and nothing was said about it.
        self.assertEqual("needs-scenario", outcomes["R4"], outcomes)

    def test_a_design_that_fails_an_example_fails_the_requirement_it_names(self):
        reply = self.ask(machine(attempts=4))
        self.assertEqual("scenario-failed", self.outcomes(reply)["R1"])
        row = next(r for r in reply["requirements"]["records"]
                   if r.get("kind") == "requirement" and r["id"] == "R1")
        self.assertEqual([("T3-at-most-three", "fail")],
                         [(s["scenario"], s["verdict"]) for s in row["scenarios"]])

    def test_the_answer_says_whose_examples_on_which_engine_and_what_a_pass_is_not(self):
        requirements = self.ask()["requirements"]
        evidence = requirements["scenario_evidence"]
        self.assertIs(True, evidence["used"], evidence)
        self.assertEqual("Python lowering", evidence["engine"]["name"])
        self.assertEqual("ai-proposed", evidence["origin"])
        self.assertEqual(64, len(evidence["set_sha256"]))
        self.assertIn("`scenario-passed` is not `implemented`", requirements["says"])
        self.assertIn("another engine may differ", requirements["says"])

    def test_without_the_list_the_answer_is_what_it_was(self):
        reply = self.ask(with_list=False)
        self.assertNotIn("requirements", reply)
        self.assertEqual("judged", reply["verdict"])

    def test_the_trace_the_product_judges_is_the_one_the_driver_wrote(self):
        """The set is judged by the product from the trace, so a design that
        breaks the limit cannot be reported as holding it by anything this
        module says: R1 follows the design."""
        passed = self.outcomes(self.ask())["R1"]
        failed = self.outcomes(self.ask(machine(attempts=4)))["R1"]
        self.assertEqual(("scenario-passed", "scenario-failed"), (passed, failed))

    def test_a_design_that_was_not_run_closes_nothing(self):
        """A remote caller's design is not played unless the operator said so;
        then there is no trace, and no requirement is moved by examples."""
        spec = json.loads(RETRY_SET.read_text(encoding="utf-8"))
        linked(spec)
        result = mcp.call_tool(
            "scxml_scenarios",
            {"scenarios_text": json.dumps(spec), "manifest_text": MANIFEST,
             "documents_text": [{"name": "retry.scxml", "text": machine()}]},
            remote=True)
        reply = json.loads(result["content"][0]["text"])
        self.assertEqual("not run", reply["verdict"])
        self.assertNotIn("requirements", reply)


class TheToolDescribesWhatItCloses(unittest.TestCase):
    def test_the_tool_takes_the_list_and_says_a_pass_is_not_implemented(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "scxml_scenarios")
        self.assertIn("manifest", tool["inputSchema"]["properties"])
        self.assertIn("manifest_text", tool["inputSchema"]["properties"])
        self.assertIn("`scenario-passed` is not `implemented`", tool["description"])


if __name__ == "__main__":
    unittest.main()
