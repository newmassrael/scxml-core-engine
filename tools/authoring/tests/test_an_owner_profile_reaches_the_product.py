"""What the owner keeps in an authoring profile reaches the product untouched.

A statechart that imports event-schemas and leaves its interface open is valid,
and nothing in it says whether that was meant: it may be a conformance
document, a legacy machine, or a design about to be shown to an owner. The
product does not guess from a feature of the document; the owner states it in
a profile kept beside the specification, and the tools hand that file to the
product and relay what it says.

This module holds only the relay. Which settings a profile has, what each one
refuses and how it is worded are the product's (`sce-build/src/authoring_profile.rs`,
`docs/SCE_ACCEPTED_SUBSET.md` §2.17), and nothing here re-reads the file: a
setting the product adds must reach these tools with no edit to them.
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

REPO = pathlib.Path(__file__).resolve().parents[3]
FIXTURES = REPO / "sce-build" / "tests" / "fixtures" / "requirement_closure"
MANIFEST = "iso13400_2_nl_socket_handling.manifest.json"

PROFILE = json.dumps({"record": "sce-authoring-profile", "v": 1,
                      "name": "owner-review", "interface": "closed"}) + "\n"
OTHER_PROFILE = json.dumps({"record": "sce-authoring-profile", "v": 1,
                            "name": "house-review", "interface": "closed"}) + "\n"
PROSE = "The machine moves on when it is told to.\n"


def statechart(closed: bool) -> str:
    """A machine that only takes what it raises itself, so closing its
    interface needs no schema and the two differ in the declaration alone."""
    declaration = ' sce:interface="closed"' if closed else ""
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="loop" initial="a" datamodel="ecmascript"{declaration}>
  <state id="a">
    <onentry><raise event="go"/></onentry>
    <transition event="go" target="b"/>
  </state>
  <final id="b"/>
</scxml>
"""


SET_STATECHART = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="job" initial="idle" datamodel="ecmascript"{declaration}>
  <sce:import src="job_requested.scxml" kind="event-schema" as="Requested"/>
  <state id="idle">
    <transition event="job.requested" target="running"/>
  </state>
  <state id="running"/>
</scxml>
"""

SCHEMA = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="job_requested"
       sce:event-name="job.requested">
  <datamodel sce:payload="none"/>
</scxml>
"""


def body(answer: dict) -> dict:
    """The tool's JSON object, whether it is an answer or a refusal."""
    return json.loads(answer["content"][0]["text"])


def codes(answer: dict) -> list[str]:
    return [record.get("code") for record in body(answer).get("diagnostics", [])]


@unittest.skipUnless(_default_codegen().exists(),
                     "the profile is the product's; build sce-codegen first")
class AnOwnerProfileReachesTheProduct(unittest.TestCase):
    def validate(self, closed: bool, **extra) -> dict:
        return call_tool("validate_scxml", {
            "document_text": statechart(closed), "document_name": "loop.scxml", **extra})

    def validate_set(self, closed: bool, **extra) -> dict:
        declaration = ' sce:interface="closed"' if closed else ""
        return call_tool("validate_scxml_set", {"documents_text": [
            {"name": "job.scxml", "text": SET_STATECHART.format(declaration=declaration)},
            {"name": "job_requested.scxml", "text": SCHEMA},
        ], **extra})

    def test_a_statechart_that_leaves_its_interface_open_is_refused_under_a_profile(self):
        refused = self.validate(False, profile_text=PROFILE)
        self.assertEqual("refused", body(refused)["verdict"])
        self.assertEqual(["profile/interface-not-closed"], codes(refused))

    def test_the_same_statechart_is_accepted_without_a_profile_and_names_none(self):
        # The control: the refusal is the profile's, not the document's, and a
        # run held to nothing says so by carrying no `profile`.
        accepted = self.validate(False)
        self.assertEqual("accepted", body(accepted)["verdict"])
        self.assertNotIn("profile", body(accepted)["manifest"])

    def test_a_statechart_that_keeps_to_the_profile_is_accepted_and_names_it(self):
        accepted = body(self.validate(True, profile_text=PROFILE))
        self.assertEqual("accepted", accepted["verdict"])
        self.assertEqual(
            {"name": "owner-review", "judged": 1,
             "sha256": hashlib.sha256(PROFILE.encode("utf-8")).hexdigest()},
            accepted["manifest"]["profile"])

    def test_every_statechart_of_a_set_is_held_to_it(self):
        refused = self.validate_set(False, profile_text=PROFILE)
        self.assertEqual("refused", body(refused)["verdict"])
        self.assertEqual(["profile/interface-not-closed"], codes(refused))
        held = body(self.validate_set(True, profile_text=PROFILE))
        self.assertEqual("accepted", held["verdict"])
        self.assertEqual(1, held["manifest"]["profile"]["judged"])

    def test_a_profile_the_product_cannot_read_is_refused_and_not_applied_in_part(self):
        # A setting this build does not know refuses the whole profile: a run
        # that applied the part it understood would say the design was held
        # to something it was not. The closed statechart, which the profile
        # would pass, is the control that the refusal is the profile's.
        unknown = json.dumps({"record": "sce-authoring-profile", "v": 1,
                              "interface": "closed", "naming": "camel"})
        refused = self.validate(True, profile_text=unknown)
        self.assertEqual("refused", body(refused)["verdict"])
        self.assertEqual(["cli/profile-unusable"], codes(refused))

    def test_a_profile_is_given_either_as_a_path_or_as_text_and_never_both(self):
        answer = self.validate(True, profile_text=PROFILE, profile="profile.json")
        self.assertTrue(answer.get("isError"), answer)
        self.assertIn("not both", answer["content"][0]["text"])

    def test_a_name_the_document_defines_is_held_to_the_owners_spelling(self):
        # A state id in the wrong case is refused and the record says how the
        # style would spell it; the same document with the id respelled, under
        # the same profile, is the control that the profile is what refuses.
        snake = json.dumps({"record": "sce-authoring-profile", "v": 1, "name": "owner-review",
                            "names": {"state": {"style": "snake"}}})
        crooked = statechart(True).replace('id="a"', 'id="DoorOpen"') \
            .replace('initial="a"', 'initial="DoorOpen"')
        refused = call_tool("validate_scxml", {
            "document_text": crooked, "document_name": "loop.scxml", "profile_text": snake})
        self.assertEqual("refused", body(refused)["verdict"])
        self.assertEqual(["profile/name-style"], codes(refused))
        self.assertIn("door_open", body(refused)["diagnostics"][0]["message"])
        respelled = statechart(True).replace('id="a"', 'id="door_open"') \
            .replace('initial="a"', 'initial="door_open"')
        kept = body(call_tool("validate_scxml", {
            "document_text": respelled, "document_name": "loop.scxml", "profile_text": snake}))
        self.assertEqual("accepted", kept["verdict"])
        self.assertEqual(1, kept["manifest"]["profile"]["judged"])

    def test_guidance_is_counted_and_nothing_pretends_it_was_checked(self):
        guided = json.dumps({"record": "sce-authoring-profile", "v": 1,
                             "guidance": ["Ask before writing."]})
        held = body(self.validate(True, profile_text=guided))
        self.assertEqual("accepted", held["verdict"])
        self.assertEqual(1, held["manifest"]["profile"]["guidance"])
        # A profile with no guidance names none, so a count of zero is not
        # said as though it were a finding.
        plain = body(self.validate(True, profile_text=PROFILE))
        self.assertNotIn("guidance", plain["manifest"]["profile"])


@unittest.skipUnless(_default_codegen().exists(),
                     "the record is the product's; build sce-codegen first")
class AnAcceptanceAnswersOnlyForTheProfileItWasTakenUnder(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name) / "tree"
        (self.root / "spec").mkdir(parents=True)
        (self.root / "design").mkdir()
        shutil.copy(FIXTURES / MANIFEST, self.root / "spec" / MANIFEST)
        (self.root / "spec" / "prose.md").write_text(PROSE, encoding="utf-8")
        (self.root / "spec" / "profile.json").write_text(PROFILE, encoding="utf-8")
        (self.root / "spec" / "other.json").write_text(OTHER_PROFILE, encoding="utf-8")
        for name, closed in (("closed.scxml", True), ("open.scxml", False)):
            (self.root / "design" / name).write_text(statechart(closed), encoding="utf-8")
        self.record = self.root / "acceptance.json"

    def accept(self, document: str, **extra) -> dict:
        return call_tool("scxml_accept", {
            "document": str(self.root / "design" / document),
            "manifest": str(self.root / "spec" / MANIFEST),
            "variant": "base", "root": str(self.root), "out": str(self.record),
            "sources": [str(self.root / "spec" / "prose.md")], **extra})

    def ask(self, **extra) -> dict:
        answer = call_tool("scxml_accepted_for", {
            "record": str(self.record), "root": str(self.root), "variant": "base",
            "sources": [str(self.root / "spec" / "prose.md")], **extra})
        self.assertFalse(answer.get("isError"), answer)
        return body(answer)

    def test_a_design_that_departs_from_the_profile_is_not_accepted_under_it(self):
        refused = self.accept("open.scxml", profile=str(self.root / "spec" / "profile.json"))
        self.assertTrue(refused.get("isError"), refused)
        self.assertEqual(["profile/interface-not-closed"], codes(refused))
        self.assertFalse(self.record.exists(), "a refused acceptance writes no record")

    def test_the_accepted_design_answers_for_this_profile_and_no_other(self):
        accepted = self.accept("closed.scxml", profile=str(self.root / "spec" / "profile.json"))
        self.assertFalse(accepted.get("isError"), accepted)
        pinned = json.loads(self.record.read_text(encoding="utf-8"))["authored_from"]
        self.assertIn({"role": "profile", "path": "spec/profile.json",
                       "sha256": hashlib.sha256(PROFILE.encode("utf-8")).hexdigest()}, pinned)

        held = self.ask(profile=str(self.root / "spec" / "profile.json"))
        self.assertEqual("accepted-design", held["verdict"])

        # Another profile is another request, and the product says the design
        # was held to a profile, not authored from one.
        other = self.ask(profile=str(self.root / "spec" / "other.json"))
        self.assertEqual("lapsed", other["verdict"])
        self.assertTrue(any("held to the authoring profile" in d.get("message", "")
                            for d in other["diagnostics"]), other)

        # A role left out is part of the answer.
        without = self.ask()
        self.assertEqual("lapsed", without["verdict"])
        self.assertTrue(any("authoring profile" in d.get("message", "")
                            for d in without["diagnostics"]), without)

    def test_a_remote_client_hands_the_profile_over_as_text(self):
        # Over HTTP there is no tree, and the profile asked about arrives as
        # text -- under the very name the record pins, which must not be
        # mistaken for the pinned one.
        self.accept("closed.scxml", profile=str(self.root / "spec" / "profile.json"))
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

        def ask(profile: str) -> dict:
            answer = call_tool("scxml_accepted_for", {
                "record_text": json.dumps(flat), "files_text": files, "variant": "base",
                "sources_text": [{"name": "prose.md", "text": PROSE}],
                "profile_text": profile, "profile_name": "profile.json"}, remote=True)
            self.assertFalse(answer.get("isError"), answer)
            return body(answer)

        self.assertEqual("accepted-design", ask(PROFILE)["verdict"])
        self.assertEqual("lapsed", ask(OTHER_PROFILE)["verdict"])


if __name__ == "__main__":
    unittest.main()
