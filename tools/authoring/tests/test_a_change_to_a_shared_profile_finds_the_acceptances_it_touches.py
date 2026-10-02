"""A change to a shared profile finds the acceptances it touches.

A profile of house rules is shared by every specification an owner keeps. Editing
one rule raises a question `scxml_acceptance_check` answers one record at a time:
which of the specifications that applied it moved? `scxml_acceptance_impact` asks
it of many records at once and answers from the product's own data.

    a design that applied the edited rule is indexed under that rule, with the
        places it applied it and both wordings
    a design accepted under the profile that did not apply it still lapses (the
        file it was accepted under is other bytes), and is not indexed under any rule
    a design accepted under no profile holds
    a record that cannot be read is unusable, counted apart, and the scan goes on
    a remote server does not offer it
"""

from __future__ import annotations

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
H2 = "A door that is open stays open until told."


def profile(h1: str = H1) -> str:
    return json.dumps({"record": "sce-authoring-profile", "v": 1, "house_rules": [
        {"id": "H1", "rule": h1}, {"id": "H2", "rule": H2}]}) + "\n"


def citing(rule: str | None) -> str:
    """The closed machine with one `sce:assumed` on its first state, or none."""
    if rule is None:
        return statechart(True)
    return statechart(True).replace(
        '<state id="a">',
        f'<state id="a" sce:assumed="{rule}" '
        'sce:assumed-reason="the specification names no event for this state">')


needs_the_generator = unittest.skipUnless(
    _default_codegen().exists(), "the product reads the record; build sce-codegen first")


@needs_the_generator
class AChangeToASharedProfile(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name) / "tree"
        (self.root / "spec").mkdir(parents=True)
        (self.root / "design").mkdir()
        shutil.copy(FIXTURES / MANIFEST, self.root / "spec" / MANIFEST)
        (self.root / "spec" / "prose.md").write_text(PROSE, encoding="utf-8")
        (self.root / "spec" / "profile.json").write_text(profile(), encoding="utf-8")
        # a applies H1, b applies H2, c applies none, d is a accepted under no profile.
        self.accept("a", "H1", under_the_profile=True)
        self.accept("b", "H2", under_the_profile=True)
        self.accept("c", None, under_the_profile=True)
        self.accept("d", "H1", under_the_profile=False)

    def record(self, name: str) -> str:
        return str(self.root / f"{name}.accepted.json")

    def accept(self, name: str, rule: str | None, *, under_the_profile: bool) -> None:
        (self.root / "design" / f"{name}.scxml").write_text(citing(rule), encoding="utf-8")
        extra = {"profile": str(self.root / "spec" / "profile.json")} if under_the_profile else {}
        answer = call_tool("scxml_accept", {
            "document": str(self.root / "design" / f"{name}.scxml"),
            "manifest": str(self.root / "spec" / MANIFEST), "variant": "base",
            "root": str(self.root), "out": self.record(name),
            "sources": [str(self.root / "spec" / "prose.md")], **extra})
        self.assertFalse(answer.get("isError"), answer)

    def impact(self, *names: str, **extra) -> dict:
        answer = call_tool("scxml_acceptance_impact", {
            "records": [self.record(n) if not n.startswith("/") else n for n in names],
            "root": str(self.root), **extra})
        self.assertFalse(answer.get("isError"), answer)
        return body(answer)

    def edit_h1(self) -> None:
        (self.root / "spec" / "profile.json").write_text(
            profile("Every unmentioned event is refused."), encoding="utf-8")

    def test_an_unchanged_profile_touches_nothing(self):
        answer = self.impact("a", "b", "c", "d")
        self.assertEqual({"records": 4, "holding": 4, "lapsed": 0, "unusable": 0},
                         answer["summary"])
        self.assertEqual({}, answer["by_rule"])

    def test_an_edited_rule_is_indexed_under_the_designs_that_applied_it(self):
        self.edit_h1()
        answer = self.impact("a", "b", "c", "d")
        self.assertEqual({"records": 4, "holding": 1, "lapsed": 3, "unusable": 0},
                         answer["summary"])
        self.assertEqual(["H1"], list(answer["by_rule"]), answer["by_rule"])
        [only] = answer["by_rule"]["H1"]
        self.assertEqual(self.record("a"), only["record"])
        self.assertEqual(
            {"places": 1, "recorded": H1, "current": "Every unmentioned event is refused."},
            {key: only[key] for key in ("places", "recorded", "current")})

    def test_a_design_that_did_not_apply_the_rule_still_lapsed_and_is_not_indexed(self):
        self.edit_h1()
        by_record = {entry["record"]: entry for entry in self.impact("a", "b", "c", "d")["records"]}
        for name in ("b", "c"):
            entry = by_record[self.record(name)]
            self.assertFalse(entry["holds"], entry)
            self.assertEqual(["source"], [lapse["kind"] for lapse in entry["lapses"]], entry)
        self.assertTrue(by_record[self.record("d")]["holds"], by_record[self.record("d")])

    def test_the_lapse_is_data_beside_its_sentence(self):
        self.edit_h1()
        entry = self.impact("a")["records"][0]
        rule = next(lapse for lapse in entry["lapses"] if lapse["kind"] == "rule")
        self.assertEqual("H1", rule["id"])
        self.assertIn("house rule `H1`", rule["message"])

    def test_a_rule_that_is_gone_has_no_current_wording(self):
        (self.root / "spec" / "profile.json").write_text(json.dumps({
            "record": "sce-authoring-profile", "v": 1,
            "house_rules": [{"id": "H2", "rule": H2}]}), encoding="utf-8")
        [only] = self.impact("a")["by_rule"]["H1"]
        self.assertIsNone(only["current"])

    def test_a_record_that_cannot_be_read_does_not_end_the_scan(self):
        self.edit_h1()
        (self.root / "garbage.accepted.json").write_text("not a record", encoding="utf-8")
        answer = self.impact("a", str(self.root / "garbage.accepted.json"),
                             str(self.root / "missing.accepted.json"), "d")
        self.assertEqual({"records": 4, "holding": 1, "lapsed": 1, "unusable": 2},
                         answer["summary"])
        unusable = [entry for entry in answer["records"] if "unusable" in entry]
        self.assertEqual(2, len(unusable), answer["records"])
        self.assertTrue(all("holds" not in entry for entry in unusable), unusable)

    def test_the_arguments_are_checked(self):
        for arguments in ({"root": str(self.root)},
                          {"records": [], "root": str(self.root)},
                          {"records": [3], "root": str(self.root)},
                          {"records": [self.record("a")]}):
            with self.subTest(arguments=sorted(arguments)):
                self.assertTrue(call_tool("scxml_acceptance_impact", arguments).get("isError"))


class ARemoteServerDoesNotOfferIt(unittest.TestCase):
    def test_a_remote_caller_is_told_to_check_one_record_at_a_time(self):
        result = call_tool("scxml_acceptance_impact",
                           {"records": ["a.json"], "root": "."}, remote=True)
        self.assertTrue(result.get("isError"), result)
        self.assertIn("local server only", result["content"][0]["text"])
        self.assertIn("scxml_acceptance_check", result["content"][0]["text"])


if __name__ == "__main__":
    unittest.main()
