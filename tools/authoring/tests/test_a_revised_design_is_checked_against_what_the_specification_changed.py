"""A revised design is started from the accepted one and checked against what the specification changed.

Run against the product's own generator, on the committed design the other acceptance tests use.
When the specification was revised and the design's own files did not move, `scxml_accepted_for`
hands the accepted design back to REVISE rather than asking for a draft afresh; when a design file
moved, the accepted bytes are gone and it says so. `scxml_revision_check` then joins what the
words did (the `delta` of `scxml_requirement_set`) with what the product says the design's
evidence did, and `scxml_revision_report` renders it for the owner
(`docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`).
"""

from __future__ import annotations

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
DOCUMENT = "doip_nl_connection_states.scxml"
PROSE = "The connection closes after the inactivity timeout.\n"
REVISED = "The connection closes after the inactivity timeout, or at once on a fault.\n"
EVENT = 'event="tcp_established"'
# A transition two requirements cite (`sce:req="3.DoIP-124 3.DoIP-081"`): editing it moves both.
SHARED_EVENT = 'event="alive_check_response"'


def body(answer: dict) -> dict:
    return json.loads(answer["content"][0]["text"])


def words(carried=(), changed=(), new=(), retired=(), **extra):
    """A words delta of the committed manifest's specification, from the revision the record pins."""
    return {"doc_id": "ISO-13400-2", "from_rev": "2019",
            "requirements": {"carried": list(carried),
                             "changed": [{"id": i, "how": "near-match"} for i in changed],
                             "new": list(new), "retired": list(retired)}, **extra}


@unittest.skipUnless(_default_codegen().exists(),
                     "the record is the product's; build sce-codegen first")
class ARevisedDesignIsCheckedAgainstWhatTheSpecificationChanged(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name) / "tree"
        (self.root / "spec").mkdir(parents=True)
        (self.root / "design").mkdir()
        shutil.copy(FIXTURES / MANIFEST, self.root / "spec" / MANIFEST)
        self.design = self.root / "design" / DOCUMENT
        shutil.copy(FIXTURES / DOCUMENT, self.design)
        self.original = self.design.read_text(encoding="utf-8")
        self.prose = self.root / "spec" / "prose.md"
        self.prose.write_text(PROSE, encoding="utf-8")
        self.record = self.root / "acceptance.json"
        accepted = call_tool("scxml_accept", {
            "document": str(self.design), "manifest": str(self.root / "spec" / MANIFEST),
            "variant": "base", "root": str(self.root), "out": str(self.record),
            "sources": [str(self.prose)]})
        self.assertFalse(accepted.get("isError"), accepted)

    def ask(self, prose: pathlib.Path) -> dict:
        answer = call_tool("scxml_accepted_for", {
            "record": str(self.record), "root": str(self.root), "variant": "base",
            "sources": [str(prose)]})
        self.assertFalse(answer.get("isError"), answer)
        return body(answer)

    def revised_prose(self) -> pathlib.Path:
        path = self.root / "spec" / "revised.md"
        path.write_text(REVISED, encoding="utf-8")
        return path

    def evidence(self) -> dict:
        answer = call_tool("scxml_acceptance_delta", {"record": str(self.record),
                                                       "root": str(self.root)})
        self.assertFalse(answer.get("isError"), answer)
        return body(answer)

    def edit_the_design(self) -> None:
        self.assertEqual(1, self.original.count(EVENT), "the fixture changed under the test")
        self.design.write_text(self.original.replace(EVENT, 'event="tcp_up"'), encoding="utf-8")

    def edit_the_transition_two_requirements_cite(self) -> None:
        self.assertEqual(1, self.original.count(SHARED_EVENT), "the fixture changed under the test")
        self.design.write_text(self.original.replace(SHARED_EVENT, 'event="alive_check_reply"'),
                               encoding="utf-8")

    def check(self, tool: str, delta: dict, **extra) -> dict:
        answer = call_tool(tool, {"delta": delta, "record": str(self.record),
                                  "root": str(self.root), **extra})
        self.assertFalse(answer.get("isError"), answer)
        return body(answer)

    def moved(self) -> list[str]:
        return [r["requirement"] for r in self.evidence()["requirements"]
                if r["evidence"] != "unchanged"]

    def all_ids(self) -> list[str]:
        return [r["requirement"] for r in self.evidence()["requirements"]]

    def test_a_lapse_of_the_specification_alone_hands_the_accepted_design_back_to_revise(self):
        answer = self.ask(self.revised_prose())
        self.assertEqual("lapsed", answer["verdict"])
        offered = answer["revise_from"]
        self.assertEqual(f"design/{DOCUMENT}", offered["document"])
        self.assertEqual(self.original, offered["document_text"])
        self.assertTrue(offered["page"], "the owner is shown the page of the design to revise")
        self.assertIn("revise it", answer["next"])
        self.assertIn("scxml_revision_check", answer["next"])

    def test_a_design_file_that_moved_leaves_nothing_to_revise_from(self):
        self.edit_the_design()
        answer = self.ask(self.prose)
        self.assertEqual("lapsed", answer["verdict"])
        self.assertNotIn("revise_from", answer)
        self.assertIn("moved since it was accepted", answer["next"])

    def test_the_specification_the_design_was_authored_from_is_still_asked_about_by_content(self):
        answer = self.ask(self.prose)
        self.assertEqual("accepted-design", answer["verdict"])
        self.assertNotIn("revise_from", answer)

    def test_a_design_that_did_not_move_carries_everything_over(self):
        ids = self.all_ids()
        self.assertTrue(ids)
        result = self.check("scxml_revision_check", words(carried=ids))
        self.assertEqual("within-reach", result["verdict"])
        self.assertEqual({"carries-over": len(ids)}, result["summary"]["kinds"])

    def test_a_carried_requirement_no_node_cites_was_not_compared_and_is_not_carried_over(self):
        ids = self.all_ids()
        result = self.check("scxml_revision_check", words(carried=[*ids, "9.NOT-CITED"]))
        self.assertEqual("within-reach", result["verdict"])
        uncited = [r for r in result["requirements"] if r["kind"] == "uncited"]
        self.assertEqual(["9.NOT-CITED"], [r["requirement"] for r in uncited])
        self.assertEqual({"carries-over": len(ids), "uncited": 1}, result["summary"]["kinds"])
        self.assertEqual((len(ids), 1), (result["summary"]["seen"], result["summary"]["uncovered"]))
        page = self.check("scxml_revision_report", words(carried=[*ids, "9.NOT-CITED"]))["page"]
        self.assertIn("## Not covered by this check (1)", page)
        self.assertIn(f"## Carries over ({len(ids)})", page)

    def test_a_design_that_moved_where_the_words_did_not_is_outside_reach(self):
        ids = self.all_ids()
        self.edit_the_design()
        moved = self.moved()
        self.assertIn("3.DoIP-127", moved, "the edited transition's requirement did not move")
        result = self.check("scxml_revision_check", words(carried=ids))
        self.assertEqual("outside-reach", result["verdict"])
        violations = sorted(r["requirement"] for r in result["requirements"]
                            if r["severity"] == "violation")
        self.assertEqual(sorted(moved), violations)
        self.assertTrue(all(r["kind"] == "moved-without-reason" for r in result["requirements"]
                            if r["severity"] == "violation"))

    def test_the_same_edit_is_within_reach_when_the_words_of_what_it_moved_changed(self):
        ids = self.all_ids()
        self.edit_the_design()
        moved = self.moved()
        delta = words(carried=[i for i in ids if i not in moved], changed=moved)
        result = self.check("scxml_revision_check", delta)
        self.assertEqual("within-reach", result["verdict"])
        revised = sorted(r["requirement"] for r in result["requirements"] if r["kind"] == "revised")
        self.assertEqual(sorted(moved), revised)
        # The places that moved are the product's, and the edited transition is one of them.
        places = [p for r in result["requirements"] for p in r.get("moved", ())]
        self.assertTrue(any("transitions[" in p for p in places), places)

    def test_a_requirement_sharing_the_edited_node_with_a_changed_one_is_a_look_that_names_it(self):
        # `3.DoIP-124` and `3.DoIP-081` cite the same transition. The specification asked for the first to
        # change; the product moves the evidence of both, and the second is not a design that wandered.
        ids = self.all_ids()
        self.edit_the_transition_two_requirements_cite()
        self.assertEqual(["3.DoIP-081", "3.DoIP-124"], sorted(self.moved()))
        delta = words(carried=[i for i in ids if i != "3.DoIP-124"], changed=["3.DoIP-124"])
        result = self.check("scxml_revision_check", delta)
        self.assertEqual("within-reach", result["verdict"])
        neighbour = next(r for r in result["requirements"] if r["requirement"] == "3.DoIP-081")
        self.assertEqual(("moved-with-a-changed-neighbour", "look", ["3.DoIP-124"]),
                         (neighbour["kind"], neighbour["severity"], neighbour["shared_with"]))
        self.assertTrue(neighbour["moved"] and all("transitions[" in p for p in neighbour["moved"]))
        page = self.check("scxml_revision_report", delta)["page"]
        self.assertIn("- 3.DoIP-081 -- moved-with-a-changed-neighbour", page)
        self.assertIn("shared with 3.DoIP-124", page)

    def test_the_same_edit_with_neither_requirement_changed_is_two_violations(self):
        ids = self.all_ids()
        self.edit_the_transition_two_requirements_cite()
        result = self.check("scxml_revision_check", words(carried=ids))
        self.assertEqual("outside-reach", result["verdict"])
        self.assertEqual(["3.DoIP-081", "3.DoIP-124"],
                         sorted(r["requirement"] for r in result["requirements"]
                                if r["severity"] == "violation"))

    def test_the_report_is_the_page_the_owner_reads_and_prints_a_sentence_only_when_given_one(self):
        ids = self.all_ids()
        self.edit_the_design()
        moved = self.moved()
        delta = words(carried=[i for i in ids if i not in moved], changed=moved)
        plain = self.check("scxml_revision_report", delta)
        self.assertEqual("within-reach", plain["verdict"])
        self.assertIn("Changed as the words asked", plain["page"])
        self.assertIn(f"## Carries over ({len(ids) - len(moved)})", plain["page"])
        self.assertNotIn("local artefact", plain["page"])
        sidecar = json.dumps({"doc_id": "d", "rev": "2",
                              "text": {moved[0]: "The inactivity timeout is five minutes."}})
        with_words = self.check("scxml_revision_report", delta, sidecar_text=sidecar)
        self.assertIn("The inactivity timeout is five minutes.", with_words["page"])
        self.assertIn("local artefact", with_words["page"])

    def test_a_delta_of_another_specification_is_refused_though_every_id_it_names_exists(self):
        # The ids are the record's own, so a join by id alone would have run. The delta is of another
        # specification, or starts from a revision the record was not taken at: neither describes
        # the step this design was accepted for.
        ids = self.all_ids()
        self.edit_the_design()
        for what, delta, message in (
                ("another specification", words(carried=ids, doc_id="OTHER-SPEC"), "'OTHER-SPEC'"),
                ("another revision", words(carried=ids, from_rev="2020"), "starts from revision 2020")):
            for tool in ("scxml_revision_check", "scxml_revision_report"):
                with self.subTest(what, tool=tool):
                    answer = call_tool(tool, {"delta": delta, "record": str(self.record),
                                              "root": str(self.root)})
                    self.assertTrue(answer.get("isError"), answer)
                    self.assertIn(message, answer["content"][0]["text"])

    def test_a_record_from_before_evidence_is_refused_and_not_read_as_everything_new(self):
        record = json.loads(self.record.read_text(encoding="utf-8"))
        record.pop("evidence", None)
        record.pop("unclaimed", None)
        self.record.write_text(json.dumps(record), encoding="utf-8")
        answer = call_tool("scxml_revision_check", {
            "delta": words(carried=["3.DoIP-127"]), "record": str(self.record),
            "root": str(self.root)})
        self.assertTrue(answer.get("isError"), answer)
        self.assertIn("states no evidence", answer["content"][0]["text"])


if __name__ == "__main__":
    unittest.main()
