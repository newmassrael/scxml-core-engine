"""A check measures the design against the owner's requirement list.

The list is the owner's, made once from quotes they can read against their own
words (`scxml_requirement_set`), so every draft of one specification is measured
against the same R1..Rn. Measured 2026-10-01: the lists a client builds itself
agree in content and not in where a clause is cut, and a client left to number
its own `sce:req` made the ids up in twelve drafts of fifteen. The product
already answers `requirements --manifest` with an outcome for each id; what was
missing was putting that answer in the call the client already makes.

What the answer is, and is not: the product's own records, passed through whole,
with a count and the ids per outcome. It is data and never a refusal, because the
product answers it with a success status and a verdict made up here would be a
second author of what the check means.
"""

from __future__ import annotations

import json
import pathlib
import tempfile
import unittest
from unittest import mock

from sce_author import mcp, requirement_set
from sce_author.verify import _default_codegen

SPEC = ("The lamp starts off. Pressing the switch turns it on; pressing it "
        "again turns it off. After 30 seconds on, it turns itself off. "
        "Nothing else changes it.")

QUOTES = [
    {"quote": "The lamp starts off.", "statement": "initial state is off"},
    {"quote": "Pressing the switch turns it on", "statement": "switch turns on"},
    {"quote": "pressing it again turns it off", "statement": "switch turns off"},
    {"quote": "After 30 seconds on, it turns itself off.", "statement": "timeout"},
    {"quote": "Nothing else changes it.", "statement": "no other change",
     "modality": "shall_not"},
]

NS = ('xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" '
      'version="1.0" datamodel="ecmascript"')


def lamp(first: str = "R1", press_on: str = "R2", press_off: str = "R3") -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml {NS} name="lamp" initial="off">
  <state id="off" sce:req="{first}">
    <transition event="press" target="on" sce:req="{press_on}"/>
  </state>
  <state id="on" sce:req="R2">
    <transition event="press" target="off" sce:req="{press_off}"/>
  </state>
</scxml>
"""


def call(name: str, **arguments) -> dict:
    return mcp.call_tool(name, arguments)


def manifest_text() -> str:
    return json.loads(call("scxml_requirement_set", specification_text=SPEC,
                           requirements=QUOTES, doc_id="lamp")["content"][0]["text"]
                      )["manifest_text"]


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class TheOwnersListIsTheDenominator(unittest.TestCase):
    def check(self, document: str, **extra) -> dict:
        result = call("validate_scxml", document_text=document,
                      document_name="lamp.scxml", manifest_text=manifest_text(), **extra)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        return json.loads(result["content"][0]["text"])

    def test_each_id_gets_the_products_outcome_and_the_ids_are_listed_by_outcome(self):
        answer = self.check(lamp())
        measured = answer["requirements"]
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual("measured", measured["verdict"])
        # R1..R3 are claimed; R4 (the timeout) is claimed by nothing, and R5 is
        # met by absence so the product asks for a scenario instead of calling
        # it missing.
        self.assertEqual({"implemented": 3, "missing": 1, "needs-scenario": 1},
                         measured["counts"])
        self.assertEqual({"missing": ["R4"], "needs-scenario": ["R5"]}, measured["ids"])

    def test_the_products_records_are_passed_through_whole(self):
        measured = self.check(lamp())["requirements"]
        kinds = {record.get("kind") for record in measured["records"]}
        self.assertEqual({"extraction", "requirement", "section-coverage"}, kinds)
        by_id = {r["id"]: r for r in measured["records"] if r.get("kind") == "requirement"}
        self.assertEqual("implemented", by_id["R2"]["outcome"])
        # The node a requirement is carried by, as the product names it, which
        # is what lets an owner go and look.
        self.assertIn("states.off.transitions[0]", by_id["R2"]["node_paths"])

    def test_an_id_the_list_does_not_hold_is_dangling_and_named(self):
        measured = self.check(lamp(press_off="R99"))["requirements"]
        self.assertEqual(["R99"], measured["ids"]["dangling"])
        by_id = {r["id"]: r for r in measured["records"] if r.get("kind") == "requirement"}
        self.assertEqual(["states.on.transitions[0]"], by_id["R99"]["node_paths"])
        # And R3, which nothing carries now, is missing: dropping a claim does
        # not hide the requirement.
        self.assertIn("R3", measured["ids"]["missing"])

    def test_the_list_says_it_is_a_reading_and_not_the_specifications_own(self):
        self.assertEqual("synthesized", self.check(lamp())["requirements"]["denominator"])

    def test_the_outcome_is_data_and_the_verdict_stays_the_checks(self):
        answer = self.check(lamp(press_off="R99"))
        # A dangling id and eight missing requirements, and the product's check
        # of the document is still what it was.
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual([], [d for d in answer["diagnostics"]
                              if d.get("code", "").startswith("requirement")])

    def test_a_design_the_product_refuses_is_not_measured(self):
        orphan = lamp().replace("</scxml>", '  <state id="orphan"/>\n</scxml>')
        result = call("validate_scxml", document_text=orphan, document_name="lamp.scxml",
                      manifest_text=manifest_text())
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("refused", answer["verdict"])
        self.assertNotIn("requirements", answer)

    def test_without_a_list_the_answer_says_it_measured_nothing_and_what_to_say(self):
        result = call("validate_scxml", document_text=lamp(), document_name="lamp.scxml")
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("accepted", answer["verdict"])
        # A program can read that nothing was measured, and the client is told
        # to label a requirement table it writes as its own reading.
        self.assertEqual({"verdict": "not measured",
                          "reason": "no requirement list was given"},
                         answer["requirements"])
        self.assertIn("SCE has not measured it", answer["show"])
        self.assertIn("your own reading", answer["show"])
        self.assertIn("scxml_requirement_set", answer["show"])
        self.assertNotIn("does what the sentence says", answer["show"])

    def test_a_measured_answer_says_what_implemented_does_not_show(self):
        result = call("validate_scxml", document_text=lamp(), document_name="lamp.scxml",
                      manifest_text=manifest_text())
        answer = json.loads(result["content"][0]["text"])
        # Said where the client reads it: `implemented` is a node carrying an id,
        # and the product has not looked at what that node does.
        self.assertIn("carries the id", answer["show"])
        self.assertIn("has not checked that it does what the sentence says",
                      answer["show"])
        # ...and not the sentence for a design nobody measured.
        self.assertNotIn("has not measured it", answer["show"])

    def test_a_measured_answer_tells_the_client_to_save_the_list_and_its_sidecar(self):
        result = call("validate_scxml", document_text=lamp(), document_name="lamp.scxml",
                      manifest_text=manifest_text())
        shown = json.loads(result["content"][0]["text"])["show"]
        # The ids in the design mean what the list says; without the saved files
        # they mean nothing once the conversation ends.
        self.assertIn("requirements.manifest.json", shown)
        self.assertIn("requirements.sidecar.json", shown)
        self.assertIn("tell the owner where they are", shown)

    def test_the_list_input_says_what_file_it_is(self):
        # A client given only "the requirement manifest" wrote one by hand, as
        # YAML, and was refused three times (a GPT run, 2026-10-01).
        for name in ("validate_scxml", "validate_scxml_set", "scxml_requirements"):
            tool = next(t for t in mcp.TOOLS if t["name"] == name)
            for key in ("manifest", "manifest_text"):
                said = tool["inputSchema"]["properties"][key]["description"]
                self.assertIn("scxml_requirement_set", said, f"{name}.{key}")
                self.assertIn("never written or edited by hand", said, f"{name}.{key}")
                self.assertIn("JSON", said, f"{name}.{key}")

    def test_an_unreadable_list_is_not_presented_as_a_measurement(self):
        result = call("validate_scxml", document_text=lamp(), document_name="lamp.scxml",
                      manifest_text='{"not": "a manifest"}')
        answer = json.loads(result["content"][0]["text"])
        self.assertIn("could not be read", answer["show"])
        self.assertIn("do not present a requirement table as SCE's", answer["show"])
        self.assertNotIn("carries the id", answer["show"])

    def test_a_list_handed_as_a_path_is_read_in_place(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "lamp.manifest.json"
            path.write_text(manifest_text(), encoding="utf-8")
            result = call("validate_scxml", document_text=lamp(),
                          document_name="lamp.scxml", manifest=str(path))
        self.assertEqual("measured", json.loads(result["content"][0]["text"])
                         ["requirements"]["verdict"])

    def test_a_list_the_product_cannot_read_is_said_and_the_check_stands(self):
        result = call("validate_scxml", document_text=lamp(), document_name="lamp.scxml",
                      manifest_text='{"not": "a manifest"}')
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual("refused", answer["requirements"]["verdict"])
        self.assertTrue(answer["requirements"]["refusal"])

    def test_the_tool_offers_the_list_and_says_what_comes_back(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "validate_scxml")
        self.assertIn("manifest", tool["inputSchema"]["properties"])
        self.assertIn("manifest_text", tool["inputSchema"]["properties"])
        self.assertIn("dangling", tool["description"])
        self.assertIn("synthesized", tool["description"])


def closed_lamp(press_off: str = "R3") -> str:
    """The lamp with its interface closed, so it imports the event it admits and
    is checked with that schema as a set."""
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml {NS} name="lamp" initial="off" sce:interface="closed">
  <sce:import as="press" src="press.scxml" kind="event-schema"/>
  <state id="off" sce:req="R1">
    <transition event="press" target="on" sce:req="R2"/>
  </state>
  <state id="on" sce:req="R2">
    <transition event="press" target="off" sce:req="{press_off}"/>
  </state>
</scxml>
"""


PRESS = f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml {NS} sce:kind="event-schema" name="press" sce:event-name="press">
  <sce:kind-basis>
    <sce:evidence>the specification names the switch press</sce:evidence>
    <sce:rejected kind="codec">no wire layout is stated</sce:rejected>
  </sce:kind-basis>
  <datamodel sce:unresolved="payload" sce:unresolved-reason="the payload is not stated"/>
</scxml>
"""


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class TheListReachesTheCheckADesignWithCompanionsGoesThrough(unittest.TestCase):
    """A statechart that closes its interface imports one event schema for each
    event it admits, so the check it goes through is `validate_scxml_set`. The
    list landed on `validate_scxml` alone, and for that design it could not be
    given: measured on a real client's draft (2026-10-01) the only call that
    accepted the document had no place for it."""

    def check(self, document: str, **extra) -> dict:
        result = call("validate_scxml_set",
                      documents_text=[{"name": "lamp.scxml", "text": document},
                                      {"name": "press.scxml", "text": PRESS}], **extra)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        return json.loads(result["content"][0]["text"])

    def test_the_set_is_measured_as_one_design_and_the_schema_takes_nothing_away(self):
        answer = self.check(closed_lamp(), manifest_text=manifest_text())
        measured = answer["requirements"]
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual("measured", measured["verdict"])
        # The schema claims nothing. Measured on its own it would read R1..R3
        # missing; with the statechart it is part of the design that carries them.
        self.assertEqual({"implemented": 3, "missing": 1, "needs-scenario": 1},
                         measured["counts"])
        self.assertEqual({"missing": ["R4"], "needs-scenario": ["R5"]}, measured["ids"])

    def test_the_order_the_documents_are_given_in_does_not_decide_what_is_measured(self):
        result = call("validate_scxml_set",
                      documents_text=[{"name": "press.scxml", "text": PRESS},
                                      {"name": "lamp.scxml", "text": closed_lamp()}],
                      manifest_text=manifest_text())
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        measured = json.loads(result["content"][0]["text"])["requirements"]
        self.assertEqual({"implemented": 3, "missing": 1, "needs-scenario": 1},
                         measured["counts"])

    def test_a_node_path_names_the_document_it_is_in(self):
        measured = self.check(closed_lamp(), manifest_text=manifest_text())["requirements"]
        by_id = {r["id"]: r for r in measured["records"] if r.get("kind") == "requirement"}
        self.assertIn("lamp.scxml#states.off.transitions[0]", by_id["R2"]["node_paths"])

    def test_an_id_the_list_does_not_hold_is_dangling_in_a_set_too(self):
        measured = self.check(closed_lamp(press_off="R99"),
                              manifest_text=manifest_text())["requirements"]
        self.assertEqual(["R99"], measured["ids"]["dangling"])
        self.assertIn("R3", measured["ids"]["missing"])

    def test_without_a_list_the_set_answer_says_it_measured_nothing_too(self):
        answer = self.check(closed_lamp())
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual("not measured", answer["requirements"]["verdict"])
        self.assertIn("SCE has not measured it", answer["show"])

    def test_a_measured_set_answer_says_what_implemented_does_not_show(self):
        answer = self.check(closed_lamp(), manifest_text=manifest_text())
        self.assertIn("has not checked that it does what the sentence says",
                      answer["show"])

    def test_a_set_the_product_refuses_is_not_measured(self):
        refused = closed_lamp().replace("</scxml>", '  <state id="orphan"/>\n</scxml>')
        result = call("validate_scxml_set",
                      documents_text=[{"name": "lamp.scxml", "text": refused},
                                      {"name": "press.scxml", "text": PRESS}],
                      manifest_text=manifest_text())
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("refused", answer["verdict"])
        self.assertNotIn("requirements", answer)

    def test_the_set_tool_offers_the_list_and_says_what_comes_back(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "validate_scxml_set")
        self.assertIn("manifest", tool["inputSchema"]["properties"])
        self.assertIn("manifest_text", tool["inputSchema"]["properties"])
        self.assertIn("requirements", tool["description"])
        self.assertIn("ONE design", tool["description"])


class TheCountIsArithmeticOverTheProductsRecords(unittest.TestCase):
    """No product needed: the summary is a fold over records the product wrote,
    and it must not invent an outcome or drop one."""

    def summarise(self, records: list) -> dict:
        report = json.dumps({"verdict": "done", "records": records})
        with mock.patch.object(mcp, "design_requirement_records",
                               lambda *a, **k: (report, "")):
            return mcp._requirement_outcomes([pathlib.Path("d.scxml")],
                                             pathlib.Path("m.json"), mock.Mock())

    def test_each_outcome_is_counted_and_listed_in_the_order_the_product_wrote(self):
        measured = self.summarise([
            {"kind": "extraction", "denominator": "derived"},
            {"kind": "requirement", "id": "A2", "outcome": "missing"},
            {"kind": "requirement", "id": "A1", "outcome": "missing"},
            {"kind": "requirement", "id": "B1", "outcome": "implemented"},
            {"kind": "requirement", "id": "X", "outcome": "dangling"},
            {"kind": "section-coverage", "section": "S1", "requirements": 1},
        ])
        self.assertEqual({"dangling": 1, "implemented": 1, "missing": 2}, measured["counts"])
        self.assertEqual({"dangling": ["X"], "missing": ["A2", "A1"]}, measured["ids"])
        self.assertEqual("derived", measured["denominator"])

    def test_an_implemented_requirement_is_counted_and_not_listed(self):
        measured = self.summarise([{"kind": "requirement", "id": "A", "outcome": "implemented"}])
        self.assertEqual({"implemented": 1}, measured["counts"])
        self.assertEqual({}, measured["ids"])

    def test_an_outcome_this_module_has_never_heard_of_is_counted_too(self):
        # The product adds an outcome, and the count says so with no edit here.
        measured = self.summarise([{"kind": "requirement", "id": "A", "outcome": "contradicted"}])
        self.assertEqual({"contradicted": 1}, measured["counts"])
        self.assertEqual({"contradicted": ["A"]}, measured["ids"])

    def test_a_product_that_refuses_the_list_is_reported_as_its_refusal(self):
        with mock.patch.object(mcp, "design_requirement_records",
                               lambda *a, **k: ("", "bad manifest\nwhy")):
            measured = mcp._requirement_outcomes([pathlib.Path("d.scxml")],
                                                 pathlib.Path("m.json"), mock.Mock())
        self.assertEqual({"verdict": "refused", "refusal": "bad manifest"}, measured)


if __name__ == "__main__":
    unittest.main()
