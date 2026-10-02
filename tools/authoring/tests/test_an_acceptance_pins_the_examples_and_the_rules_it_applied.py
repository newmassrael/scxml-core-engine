"""An acceptance pins the examples a design was held to and the rules it applied.

The product writes the record and holds it (`sce-build/src/acceptance_record.rs`,
`SourceRole::Examples`, `applied_rules`, `Lapse::Rule`); what this module holds is
that the tools reach it and relay it untouched: the scenario set is handed to
`scxml_accept`, `scxml_acceptance_check` and `scxml_accepted_for` like the profile
is, the rules the design applied come back in the answer so a reader need not ask
for the profile, and a profile that changes is reported rule by rule.

    the examples are pinned, and a set edited afterwards, or left out, lapses
    the answer to `scxml_accept` carries the house rules the design applied, as
        the profile worded them, with the number of places
    a profile that moved names the rule the design relied on, and the words it
        had, and says nothing of a rule the design never applied
    a client with no tree hands the set over as text, under the name the record pins
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import shutil
import tempfile
import unittest

from sce_author.mcp import call_tool
from sce_author.verify import _default_codegen
from tests.test_an_owner_profile_reaches_the_product import (
    FIXTURES, MANIFEST, PROSE, body, statechart)

H1 = "An event a state does not mention is ignored."
PROFILE = json.dumps({"record": "sce-authoring-profile", "v": 1, "house_rules": [
    {"id": "H1", "rule": H1},
    {"id": "H2", "rule": "A door that is open stays open until told."}]}) + "\n"
SET = json.dumps({
    "record": "sce-scenario-set", "v": 1,
    "specification": {"doc_id": "nl", "rev": "1"}, "origin": "ai-proposed",
    "interface": {"inputs": [], "outputs": []},
    "scenarios": [{"id": "S1", "quote": "Waiting ends on a tick.",
                   "steps": [{"advance_ms": 5, "expect": {"outbound": []}}]}]}) + "\n"


def citing(rule: str) -> str:
    """The closed machine with one `sce:assumed` on its first state."""
    return statechart(True).replace(
        '<state id="a">',
        f'<state id="a" sce:assumed="{rule}" '
        'sce:assumed-reason="the specification names no event for this state">')


needs_the_generator = unittest.skipUnless(
    _default_codegen().exists(), "the product reads the record; build sce-codegen first")


@needs_the_generator
class AnAcceptancePinsTheExamplesAndTheRulesItApplied(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name) / "tree"
        (self.root / "spec").mkdir(parents=True)
        (self.root / "design").mkdir()
        shutil.copy(FIXTURES / MANIFEST, self.root / "spec" / MANIFEST)
        for name, text in (("prose.md", PROSE), ("profile.json", PROFILE),
                           ("examples.json", SET)):
            (self.root / "spec" / name).write_text(text, encoding="utf-8")
        (self.root / "design" / "door.scxml").write_text(citing("H1"), encoding="utf-8")
        self.record = self.root / "acceptance.json"

    def spec(self, name: str) -> str:
        return str(self.root / "spec" / name)

    def accept(self, **extra) -> dict:
        answer = call_tool("scxml_accept", {
            "document": str(self.root / "design" / "door.scxml"),
            "manifest": self.spec(MANIFEST), "variant": "base",
            "root": str(self.root), "out": str(self.record),
            "sources": [self.spec("prose.md")],
            "profile": self.spec("profile.json"), "scenarios": self.spec("examples.json"),
            **extra})
        self.assertFalse(answer.get("isError"), answer)
        return body(answer)

    def ask(self, tool: str = "scxml_accepted_for", **extra) -> dict:
        answer = call_tool(tool, {
            "record": str(self.record), "root": str(self.root), "variant": "base",
            "sources": [self.spec("prose.md")], **extra})
        self.assertFalse(answer.get("isError"), answer)
        return body(answer)

    def messages(self, answer: dict) -> str:
        return " | ".join(d.get("message", "") for d in answer.get("diagnostics", []))

    def test_the_answer_carries_the_rules_the_design_applied_as_the_profile_worded_them(self):
        accepted = self.accept()
        self.assertEqual([{"id": "H1", "rule": H1, "places": 1}], accepted["applied_rules"])
        pinned = json.loads(self.record.read_text(encoding="utf-8"))
        self.assertEqual(accepted["applied_rules"], pinned["applied_rules"])
        self.assertIn({"role": "examples", "path": "spec/examples.json",
                       "sha256": hashlib.sha256(SET.encode("utf-8")).hexdigest()},
                      pinned["authored_from"])

    def test_an_acceptance_with_no_rule_applied_answers_as_it_did(self):
        (self.root / "design" / "door.scxml").write_text(statechart(True), encoding="utf-8")
        self.assertNotIn("applied_rules", self.accept())

    def test_the_set_is_part_of_what_the_acceptance_answers_for(self):
        self.accept()
        both = {"profile": self.spec("profile.json"), "scenarios": self.spec("examples.json")}
        self.assertEqual("accepted-design", self.ask(**both)["verdict"])

        without = self.ask(profile=self.spec("profile.json"))
        self.assertEqual("lapsed", without["verdict"])
        self.assertIn("scenario set", self.messages(without))

        (self.root / "spec" / "examples.json").write_text(
            SET.replace("a tick", "a tock"), encoding="utf-8")
        edited = self.ask("scxml_acceptance_check", **both)
        self.assertEqual("lapsed", edited["verdict"])
        self.assertIn("the scenario set this design was held to changed",
                      self.messages(edited))

    def test_a_profile_that_moved_names_the_rule_the_design_relied_on(self):
        self.accept()
        (self.root / "spec" / "profile.json").write_text(
            PROFILE.replace(H1, "An unmentioned event is an error."), encoding="utf-8")
        lapsed = self.ask("scxml_acceptance_check", profile=self.spec("profile.json"),
                          scenarios=self.spec("examples.json"))
        self.assertEqual("lapsed", lapsed["verdict"])
        said = self.messages(lapsed)
        self.assertIn("house rule `H1`, which the design applied at 1 place", said)
        self.assertIn("now says \"An unmentioned event is an error.\"", said)
        self.assertIn(f"it said {json.dumps(H1)} when the design was accepted", said)

    def test_a_rule_the_design_never_applied_moves_the_profile_and_names_no_rule(self):
        self.accept()
        (self.root / "spec" / "profile.json").write_text(
            PROFILE.replace("stays open until told", "closes by itself"), encoding="utf-8")
        lapsed = self.ask("scxml_acceptance_check", profile=self.spec("profile.json"),
                          scenarios=self.spec("examples.json"))
        self.assertEqual("lapsed", lapsed["verdict"])
        self.assertNotIn("house rule", self.messages(lapsed))
        self.assertIn("authoring profile", self.messages(lapsed))

    def test_a_file_that_is_no_scenario_set_is_refused_and_not_pinned(self):
        refused = call_tool("scxml_accept", {
            "document": str(self.root / "design" / "door.scxml"),
            "manifest": self.spec(MANIFEST), "variant": "base",
            "root": str(self.root), "out": str(self.record),
            "sources": [self.spec("prose.md")], "scenarios": self.spec("prose.md")})
        self.assertTrue(refused.get("isError"), refused)
        self.assertFalse(self.record.exists(), "a refused acceptance writes no record")

    def test_a_client_with_no_tree_hands_the_set_over_as_text(self):
        self.accept()
        record = json.loads(self.record.read_text(encoding="utf-8"))
        flat = json.loads(json.dumps(record))
        named = [pin["path"] for pin in record["inputs"]] + [record["manifest"]["path"]] \
            + [pin["path"] for pin in record["authored_from"]]
        files = [{"name": pathlib.Path(rel).name,
                  "text": (self.root / rel).read_text(encoding="utf-8")} for rel in named]
        flat["document"] = pathlib.Path(flat["document"]).name
        flat["manifest"]["path"] = pathlib.Path(flat["manifest"]["path"]).name
        for pin in flat["inputs"] + flat["authored_from"]:
            pin["path"] = pathlib.Path(pin["path"]).name

        def ask(scenarios: str) -> dict:
            answer = call_tool("scxml_accepted_for", {
                "record_text": json.dumps(flat), "files_text": files, "variant": "base",
                "sources_text": [{"name": "prose.md", "text": PROSE}],
                "profile_text": PROFILE, "profile_name": "profile.json",
                "scenarios_text": scenarios, "scenarios_name": "examples.json"},
                remote=True)
            self.assertFalse(answer.get("isError"), answer)
            return body(answer)

        self.assertEqual("accepted-design", ask(SET)["verdict"])
        self.assertEqual("lapsed", ask(SET.replace("a tick", "a tock"))["verdict"])


class TheToolsTakeTheSet(unittest.TestCase):
    def test_the_three_acceptance_tools_take_the_scenario_set(self):
        from sce_author import mcp

        for name in ("scxml_accept", "scxml_acceptance_check", "scxml_accepted_for"):
            properties = next(t for t in mcp.TOOLS if t["name"] == name)[
                "inputSchema"]["properties"]
            self.assertIn("scenarios", properties, name)
            self.assertIn("scenarios_text", properties, name)


if __name__ == "__main__":
    unittest.main()
