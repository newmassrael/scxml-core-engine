"""The same specification asked for again is answered with the design the
owner accepted, not with a new draft.

The model that writes a draft is the owner's client's, and nothing makes two
of its drafts equal: measured 2026-09-29, no two of thirty drafts of six
specifications were byte-identical, and no two of their review pages were.
So an owner who asks twice gets two documents unless the second request is
answered from the first acceptance. `scxml_accept` pins what the design was
authored from; `scxml_accepted_for` asks, before anything is written, whether
an accepted design answers for these files -- by content, so the owner's copy
of the same prose under another name still matches, and a revised one does
not.
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
DECISIONS = json.dumps({
    "record": "sce-decision-record", "v": 1,
    "specification": {"doc_id": "doip-nl"},
    "decisions": [{"id": "D1", "question": "How long is the inactivity timeout?",
                   "answer": "5 minutes"}]}) + "\n"


def body(answer: dict) -> dict:
    return json.loads(answer["content"][0]["text"])


@unittest.skipUnless(_default_codegen().exists(),
                     "the record is the product's; build sce-codegen first")
class AnAcceptedDesignIsHandedBack(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name) / "tree"
        (self.root / "spec").mkdir(parents=True)
        (self.root / "design").mkdir()
        shutil.copy(FIXTURES / MANIFEST, self.root / "spec" / MANIFEST)
        shutil.copy(FIXTURES / DOCUMENT, self.root / "design" / DOCUMENT)
        (self.root / "spec" / "prose.md").write_text(PROSE, encoding="utf-8")
        (self.root / "spec" / "decisions.json").write_text(DECISIONS, encoding="utf-8")
        self.record = self.root / "acceptance.json"
        accepted = call_tool("scxml_accept", {
            "document": str(self.root / "design" / DOCUMENT),
            "manifest": str(self.root / "spec" / MANIFEST),
            "variant": "base", "root": str(self.root), "out": str(self.record),
            "sources": [str(self.root / "spec" / "prose.md")],
            "decisions": str(self.root / "spec" / "decisions.json"),
        })
        self.assertFalse(accepted.get("isError"), accepted)
        self.accepted_answer = body(accepted)
        # The owner's own copies, somewhere else and under other names.
        self.elsewhere = pathlib.Path(temporary.name) / "owner"
        self.elsewhere.mkdir()
        self.prose = self.elsewhere / "the spec.md"
        self.decisions = self.elsewhere / "answers.json"
        self.prose.write_text(PROSE, encoding="utf-8")
        self.decisions.write_text(DECISIONS, encoding="utf-8")

    def ask(self, **extra) -> dict:
        arguments = {"record": str(self.record), "root": str(self.root), "variant": "base",
                     "sources": [str(self.prose)], "decisions": str(self.decisions)}
        arguments.update(extra)
        answer = call_tool("scxml_accepted_for", arguments)
        self.assertFalse(answer.get("isError"), answer)
        return body(answer)

    def test_the_same_specification_gets_the_accepted_design_and_its_page(self):
        answer = self.ask()
        self.assertEqual(answer["verdict"], "accepted-design")
        self.assertEqual(answer["document"], f"design/{DOCUMENT}")
        self.assertEqual(answer["document_text"],
                         (self.root / "design" / DOCUMENT).read_text(encoding="utf-8"))
        self.assertTrue(answer["page"], "the owner is shown the accepted page")
        self.assertIn("no new draft", answer["next"])

    def test_accepting_a_design_with_open_matters_says_what_it_was_accepted_with(self):
        # This design leaves its two inactivity timers `sce:unresolved`: the
        # owner may accept it, and the answer and the record both say they
        # did so with those questions open.
        answer = self.accepted_answer
        self.assertEqual("done", answer["verdict"])
        self.assertTrue(answer["accepted_with"], answer)
        self.assertTrue(any("question(s)" in line for line in answer["accepted_with"]),
                        answer["accepted_with"])
        self.assertIn("accepted_with", answer["next"])
        kept = json.loads(self.record.read_text(encoding="utf-8"))["open_at_acceptance"]
        self.assertEqual(answer["accepted_with"], [m["message"] for m in kept])

    def test_a_design_that_left_nothing_open_is_answered_as_it_was(self):
        # The control: no new field on the answer, so a finished design's
        # acceptance reads exactly as before.
        from sce_author.mcp import _with_open_at_acceptance

        record = self.root / "finished.json"
        record.write_text(json.dumps({"record": "sce-acceptance-record", "v": 1}),
                          encoding="utf-8")
        answer = json.loads(_with_open_at_acceptance(
            json.dumps({"verdict": "done", "record": str(record)}), record))
        self.assertEqual({"verdict": "done", "record": str(record)}, answer)

    def test_a_revised_specification_is_not_answered_by_the_old_design(self):
        self.prose.write_text("The connection closes after two timeouts.\n", encoding="utf-8")
        answer = self.ask()
        self.assertEqual(answer["verdict"], "lapsed")
        self.assertNotIn("document_text", answer)
        self.assertTrue(any("authored from" in d.get("message", "")
                            for d in answer["diagnostics"]), answer)

    def test_leaving_out_the_decision_record_is_a_different_question(self):
        answer = self.ask(decisions=None)
        self.assertEqual(answer["verdict"], "lapsed")

    def test_a_file_that_is_not_a_decision_record_is_not_pinned_as_one(self):
        # The specification and the record sit side by side; handing over the
        # first as the second would pin it, and every later question asked
        # with the real record would then read as a revision.
        answer = call_tool("scxml_accept", {
            "document": str(self.root / "design" / DOCUMENT),
            "manifest": str(self.root / "spec" / MANIFEST),
            "variant": "base", "root": str(self.root),
            "out": str(self.root / "second.json"),
            "sources": [str(self.root / "spec" / "prose.md")],
            "decisions": str(self.root / "spec" / "prose.md"),
        })
        self.assertTrue(answer.get("isError"))
        self.assertIn("prose.md", answer["content"][0]["text"])
        self.assertFalse((self.root / "second.json").exists())

    def test_the_question_needs_a_specification(self):
        answer = call_tool("scxml_accepted_for", {
            "record": str(self.record), "root": str(self.root), "variant": "base"})
        self.assertTrue(answer.get("isError"))

    def test_a_remote_client_hands_every_file_over_as_text(self):
        # Over HTTP there is no tree: the record, every file it names, and the
        # files the question is about all arrive as text -- the asked prose
        # under the very name the record pins, which must not be mistaken
        # for the pinned one.
        record = json.loads(self.record.read_text(encoding="utf-8"))
        named = [pin["path"] for pin in record["inputs"]] + [record["manifest"]["path"]] \
            + [pin["path"] for pin in record["authored_from"]]
        files = []
        for rel in named:
            files.append({"name": pathlib.Path(rel).name,
                          "text": (self.root / rel).read_text(encoding="utf-8")})
        # Staged flat, so the record is rewritten to the names they arrive by.
        flat = json.loads(self.record.read_text(encoding="utf-8"))
        flat["document"] = pathlib.Path(flat["document"]).name
        flat["manifest"]["path"] = pathlib.Path(flat["manifest"]["path"]).name
        for pin in flat["inputs"] + flat["authored_from"]:
            pin["path"] = pathlib.Path(pin["path"]).name
        answer = call_tool("scxml_accepted_for", {
            "record_text": json.dumps(flat), "files_text": files, "variant": "base",
            "sources_text": [{"name": "prose.md", "text": PROSE}],
            "decisions_text": DECISIONS,
        }, remote=True)
        self.assertFalse(answer.get("isError"), answer)
        self.assertEqual(body(answer)["verdict"], "accepted-design")


if __name__ == "__main__":
    unittest.main()
