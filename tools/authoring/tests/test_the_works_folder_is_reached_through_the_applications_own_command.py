"""An authoring client reads a work and saves its model through the application's own command.

The workbench application keeps a specification as a work, and the owner asks an AI
client to write the model of it. These cases drive the three tools that connect the
two (`works_list`, `works_read`, `works_save_model`) against the REAL `sce-work`
and the real generator, because what they promise is a property of the pair: the
text the owner saved is the text the client reads, the model the client saves is the
model the owner sees, and the folder has one writer.

Skipped, and saying so, where either binary is not built. The authoring lane that
builds the generator builds `sce-work` beside it and refuses to run without it, so
in CI a skip here is a broken build step and not a reason to judge less.
"""

from __future__ import annotations

import json
import os
import pathlib
import tempfile
import unittest
import unittest.mock

from sce_author import mcp, works
from sce_author.verify import _default_codegen

BUILT = unittest.skipUnless(
    works.default_work_binary().is_file() and _default_codegen().is_file(),
    "sce-work and the product's code generator are not both built")

# Five Hangul syllables across a newline, built from their code points so this file
# stays ASCII: what is measured is that a text of three-byte scalars crosses two
# pipes unchanged, not what it says.
KOREAN = "".join(chr(code) for code in (0xB3C4, 0xC5B4, 0xB77D, 0x0A, 0xC5F4, 0xB9BC))

SPEC = "The door opens when the card matches.\n" + KOREAN + "\n"

DOOR = ('<?xml version="1.0" encoding="UTF-8"?>\n'
        '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
        'xmlns:sce="http://sce.dev/ext" sce:kind="statechart" version="1.0" '
        'initial="closed">\n'
        '  <state id="closed"><transition event="open" target="opened"/></state>\n'
        '  <state id="opened"><transition event="close" target="closed"/></state>\n'
        '</scxml>\n')

REFUSED = DOOR.replace('target="opened"', 'target="nowhere"')

# The same door, asking the owner what the specification does not say.
ASKS = DOOR.replace(
    '<transition event="open" target="opened"/>',
    '<transition event="open" target="opened" sce:unresolved="open-guard" '
    'sce:unresolved-reason="Which cards open the door?"/>')

# ... and the same door, having applied the owner's answer to that question.
APPLIES = DOOR.replace(
    '<transition event="open" target="opened"/>',
    '<transition event="open" target="opened" sce:assumed="open-guard"/>')

# ... and asking a question nobody had asked yet, beside the one that was answered.
ASKS_MORE = APPLIES.replace(
    '<transition event="close" target="closed"/>',
    '<transition event="close" target="closed" sce:unresolved="close-delay" '
    'sce:unresolved-reason="How long before it closes again?"/>')


# A statechart with a CLOSED interface and the event schema it imports, as the
# authoring tools ask a statechart to be written: two documents that name each other.
ROOT = pathlib.Path(__file__).resolve().parents[3]
SCHEMA_NAME = "schema_job_completed_minimal.scxml"
SCHEMA = (ROOT / "sce-build" / "tests" / "fixtures" / "event_schema" / SCHEMA_NAME
          ).read_text(encoding="utf-8")
GATE_NAME = "gate.scxml"
GATE = ('<?xml version="1.0" encoding="UTF-8"?>\n'
        '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
        'xmlns:sce="http://sce.dev/ext" sce:kind="statechart" sce:interface="closed" '
        'version="1.0" initial="waiting">\n'
        f'  <sce:import src="{SCHEMA_NAME}" kind="event-schema" as="JobCompletedSchema"/>\n'
        '  <state id="waiting"><transition event="job.completed" target="done"/></state>\n'
        '  <state id="done"/>\n'
        '</scxml>\n')
SET = [{"name": GATE_NAME, "text": GATE}, {"name": SCHEMA_NAME, "text": SCHEMA}]


def call(name: str, remote: bool = False, **arguments) -> dict:
    return mcp.call_tool(name, arguments, remote=remote)


def body(answer: dict) -> str:
    return answer["content"][0]["text"]


def data(answer: dict) -> dict:
    return json.loads(body(answer))


@BUILT
class TheWorksFolder(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        patch = unittest.mock.patch.dict(
            os.environ, {"SCE_WORKS_DIR": str(pathlib.Path(self._tmp.name) / "works")})
        patch.start()
        self.addCleanup(patch.stop)
        self.addCleanup(self._tmp.cleanup)
        # The owner's side, made the way the application makes it.
        self.work = works.call_work("create_work", {"title": "Door lock"})["id"]
        self.revision = works.call_work(
            "save_source", {"id": self.work, "text": SPEC})["revision"]

    def test_the_works_are_listed_with_their_ids(self):
        listing = data(call("works_list"))
        self.assertEqual([self.work], [w["id"] for w in listing["works"]])
        self.assertEqual("Door lock", listing["works"][0]["title"])
        self.assertEqual([], listing["unreadable"])

    def test_a_work_is_read_with_the_text_the_owner_saved_unchanged(self):
        read = data(call("works_read", work=self.work))
        self.assertEqual(SPEC, read["source"]["text"])
        self.assertEqual(self.revision, read["source"]["revision"])
        self.assertIsNone(read["model"])
        # It says what to do with what it gave: the revision to hand back.
        self.assertIn("source_revision = source.revision", read["next"])
        self.assertNotIn("base = model.revision", read["next"])

    def test_a_model_saved_for_the_text_read_stands_current_in_the_next_read(self):
        saved = call("works_save_model", work=self.work, model_text=DOOR,
                     source_revision=self.revision)
        self.assertFalse(saved.get("isError"), body(saved))
        self.assertEqual("saved", data(saved)["outcome"])

        read = data(call("works_read", work=self.work))
        self.assertEqual(DOOR, read["model"]["text"])
        self.assertEqual("current", read["model"]["standing"])
        self.assertEqual(self.revision, read["model"]["written_for"])
        self.assertIn("base = model.revision", read["next"])

    def test_a_model_saved_without_the_text_it_was_written_for_is_unstated_not_current(self):
        call("works_save_model", work=self.work, model_text=DOOR)
        read = data(call("works_read", work=self.work))
        self.assertEqual("unstated", read["model"]["standing"])

    def test_a_model_of_a_text_that_moved_on_stands_behind(self):
        call("works_save_model", work=self.work, model_text=DOOR,
             source_revision=self.revision)
        works.call_work("save_source", {"id": self.work, "text": SPEC + "More.\n",
                                        "base": self.revision})
        read = data(call("works_read", work=self.work))
        self.assertEqual("behind", read["model"]["standing"])

    def test_a_model_the_product_refuses_is_not_saved(self):
        refused = call("works_save_model", work=self.work, model_text=REFUSED,
                       source_revision=self.revision)
        self.assertTrue(refused.get("isError"))
        self.assertIn("not saved", body(refused))
        # The product's own records come back, to fix the draft by.
        self.assertIn("nowhere", body(refused))
        self.assertIsNone(data(call("works_read", work=self.work))["model"])

    def test_a_save_from_a_model_that_is_no_longer_current_is_a_conflict_and_writes_nothing(self):
        first = data(call("works_save_model", work=self.work, model_text=DOOR,
                          source_revision=self.revision))
        # A client that never read the model has not seen what it would replace.
        again = call("works_save_model", work=self.work,
                     model_text=DOOR + "<!-- second -->\n", source_revision=self.revision)
        self.assertTrue(again.get("isError"))
        refusal = data(again)
        self.assertEqual("conflict", refusal["refused"])
        self.assertEqual(first["revision"], refusal["detail"]["current"])
        self.assertEqual(DOOR, data(call("works_read", work=self.work))["model"]["text"])

        # From the revision it read, the same save is taken.
        ok = call("works_save_model", work=self.work, model_text=DOOR + "<!-- second -->\n",
                  source_revision=self.revision, base=first["revision"])
        self.assertFalse(ok.get("isError"), body(ok))

    def test_a_model_of_three_byte_scalars_crosses_both_pipes_unchanged(self):
        model = DOOR.replace("<state id=\"closed\">",
                             f"<!-- {KOREAN} -->\n  <state id=\"closed\">")
        saved = call("works_save_model", work=self.work, model_text=model,
                     source_revision=self.revision)
        self.assertFalse(saved.get("isError"), body(saved))
        self.assertEqual(model, data(call("works_read", work=self.work))["model"]["text"])

    def test_a_model_of_several_documents_is_checked_as_a_set_and_read_back_as_its_documents(self):
        saved = call("works_save_model", work=self.work, documents_text=SET,
                     entry_name=GATE_NAME, source_revision=self.revision)
        self.assertFalse(saved.get("isError"), body(saved))
        self.assertEqual("saved", data(saved)["outcome"])

        model = data(call("works_read", work=self.work))["model"]
        self.assertEqual(GATE_NAME, model["entry"])
        self.assertEqual([GATE_NAME, SCHEMA_NAME], [d["name"] for d in model["documents"]])
        self.assertEqual(GATE, model["documents"][0]["text"])
        self.assertEqual(SCHEMA, model["documents"][1]["text"])
        # A model of several is its documents, not the entry twice.
        self.assertNotIn("text", model)
        self.assertEqual("current", model["standing"])

    def test_a_model_of_one_document_is_still_read_back_as_its_text(self):
        call("works_save_model", work=self.work, model_text=DOOR, source_revision=self.revision)
        model = data(call("works_read", work=self.work))["model"]
        self.assertEqual(DOOR, model["text"])
        self.assertNotIn("documents", model)
        self.assertNotIn("entry", model)

    def test_a_set_whose_import_is_not_among_its_documents_is_not_saved(self):
        refused = call("works_save_model", work=self.work,
                       documents_text=[{"name": GATE_NAME, "text": GATE}],
                       source_revision=self.revision)
        # One document is checked as one: the product, reading the entry alone, cannot
        # find the import it names, and says so.
        self.assertTrue(refused.get("isError"), body(refused))
        self.assertIn("not saved", body(refused))
        self.assertIn("import/file-not-found", body(refused))
        self.assertIsNone(data(call("works_read", work=self.work))["model"])

    def test_a_model_given_both_ways_or_neither_way_is_refused_before_anything_runs(self):
        for arguments in ({"model_text": DOOR, "documents_text": SET}, {}):
            with self.subTest(arguments=sorted(arguments)):
                answer = call("works_save_model", work=self.work, **arguments)
                self.assertTrue(answer.get("isError"))
                self.assertIn("exactly one of", body(answer))
        for bad, wanted in (
                ([{"name": "../escape.scxml", "text": "x"}], "plain file name"),
                ([{"name": "a.scxml", "text": "x"}, {"name": "a.scxml", "text": "y"}],
                 "two files are named"),
                ("not a list", "non-empty list"),
                ([], "non-empty list")):
            with self.subTest(documents=bad):
                answer = call("works_save_model", work=self.work, documents_text=bad)
                self.assertTrue(answer.get("isError"))
                self.assertIn(wanted, body(answer))
        named = call("works_save_model", work=self.work, documents_text=SET,
                     entry_name="nowhere.scxml")
        self.assertTrue(named.get("isError"))
        self.assertIn("entry_name", body(named))

    def test_a_revision_that_is_not_one_is_refused_by_shape_before_anything_runs(self):
        answer = call("works_save_model", work=self.work, model_text=DOOR,
                      source_revision="latest")
        self.assertTrue(answer.get("isError"))
        self.assertIn("64 lowercase hexadecimal digits", body(answer))

    def test_a_work_that_is_not_there_is_refused_in_the_folders_own_words(self):
        answer = call("works_read", work="no-such-work")
        self.assertTrue(answer.get("isError"))
        refusal = data(answer)
        self.assertEqual("not-found", refusal["refused"])
        self.assertIn("no-such-work", refusal["message"])

    def test_a_removed_work_is_not_offered_to_an_authoring_client(self):
        works.call_work("remove_work", {"id": self.work})
        self.assertEqual([], data(call("works_list"))["works"])
        refused = call("works_save_model", work=self.work, model_text=DOOR)
        self.assertTrue(refused.get("isError"))
        self.assertIn("removed", body(refused))

    def test_there_is_no_tool_that_writes_the_owners_text_or_removes_a_work(self):
        names = {t["name"] for t in mcp.TOOLS}
        for forbidden in ("works_save_source", "works_write", "works_remove",
                          "works_delete", "works_create", "works_save_answers",
                          "works_answer"):
            self.assertNotIn(forbidden, names)


@BUILT
class TheOwnersAnswers(unittest.TestCase):
    """What the owner answered in the application reaches the next draft, and a
    draft is held to it before it is saved."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        patch = unittest.mock.patch.dict(
            os.environ, {"SCE_WORKS_DIR": str(pathlib.Path(self._tmp.name) / "works")})
        patch.start()
        self.addCleanup(patch.stop)
        self.addCleanup(self._tmp.cleanup)
        self.work = works.call_work("create_work", {"title": "Door lock"})["id"]
        self.revision = works.call_work(
            "save_source", {"id": self.work, "text": SPEC})["revision"]

    def answer(self, **entries):
        """The owner's side: what the application saves when they answer."""
        held = works.call_work("read_answers", {"id": self.work})["answers"]
        works.call_work("save_answers", {
            "id": self.work, "answers": entries,
            "base": held["revision"] if held else None})

    def record_of(self, read: dict):
        """The decision record `works_read` handed back, read by the package that
        owns its format: a record that does not load is a bug here."""
        path = pathlib.Path(self._tmp.name) / "decisions.json"
        path.write_text(read["decisions_text"], encoding="utf-8")
        from sce_author.decisions import load_record
        return load_record(path)

    def test_a_work_nobody_answered_hands_back_no_answers_and_no_record(self):
        read = data(call("works_read", work=self.work))
        self.assertIsNone(read["answers"])
        self.assertNotIn("decisions_text", read)

    def test_the_owners_answer_is_handed_back_as_the_decision_record_it_makes(self):
        call("works_save_model", work=self.work, model_text=ASKS,
             source_revision=self.revision)
        # The model asks; the owner has not answered: the record lists the question.
        open_read = data(call("works_read", work=self.work))
        record = self.record_of(open_read)
        self.assertEqual(["open-guard"], list(record.decisions))
        self.assertEqual("Which cards open the door?", record.decisions["open-guard"].question)
        self.assertFalse(record.decisions["open-guard"].answered)

        self.answer(**{"open-guard": "Any card on the list opens it."})
        read = data(call("works_read", work=self.work))
        self.assertEqual("Any card on the list opens it.",
                         read["answers"]["entries"]["open-guard"]["answer"])
        decision = self.record_of(read).decisions["open-guard"]
        self.assertEqual("Any card on the list opens it.", decision.answer)
        self.assertEqual("Which cards open the door?", decision.question)
        self.assertIn("never leave an answered question sce:unresolved", read["next"])

    def test_an_answer_to_a_question_the_model_no_longer_asks_is_kept_for_the_next_draft(self):
        call("works_save_model", work=self.work, model_text=DOOR,
             source_revision=self.revision)
        self.answer(**{"open-guard": "Any card on the list opens it."})
        decision = self.record_of(data(call("works_read", work=self.work))).decisions["open-guard"]
        self.assertTrue(decision.answered)
        self.assertIn("earlier model", decision.question)

    def test_a_draft_that_leaves_an_answered_question_open_is_not_saved(self):
        self.answer(**{"open-guard": "Any card on the list opens it."})
        refused = call("works_save_model", work=self.work, model_text=ASKS,
                       source_revision=self.revision)
        self.assertTrue(refused.get("isError"), body(refused))
        self.assertIn("does not keep to the owner's answers", body(refused))
        self.assertIn("answered-left-open", body(refused))
        self.assertIsNone(data(call("works_read", work=self.work))["model"])

    def test_a_draft_that_applies_the_answer_is_saved(self):
        self.answer(**{"open-guard": "Any card on the list opens it."})
        saved = call("works_save_model", work=self.work, model_text=APPLIES,
                     source_revision=self.revision)
        self.assertFalse(saved.get("isError"), body(saved))
        held = data(saved)["decisions"]
        self.assertEqual("holds", held["verdict"])
        self.assertEqual(1, held["counts"]["applied"])

    def test_a_question_nobody_asked_yet_is_saved_and_reported_for_the_owner(self):
        self.answer(**{"open-guard": "Any card on the list opens it."})
        saved = call("works_save_model", work=self.work, model_text=ASKS_MORE,
                     source_revision=self.revision)
        self.assertFalse(saved.get("isError"), body(saved))
        held = data(saved)["decisions"]
        self.assertEqual(1, held["counts"]["new-question"])
        self.assertIn("ask the owner each new question", held["next"])
        # The model is saved, and what it asks is now in the record for the owner to answer.
        record = self.record_of(data(call("works_read", work=self.work)))
        self.assertEqual({"open-guard", "close-delay"}, set(record.decisions))

    def test_a_work_with_no_answers_is_not_held_to_a_record_it_does_not_have(self):
        saved = call("works_save_model", work=self.work, model_text=APPLIES,
                     source_revision=self.revision)
        self.assertFalse(saved.get("isError"), body(saved))
        self.assertNotIn("decisions", data(saved))


class TheWorksFolderIsThisMachinesOwn(unittest.TestCase):
    """What needs no binary: the refusals that come before one is run."""

    def test_a_remote_caller_is_not_offered_it(self):
        for name, arguments in (("works_list", {}),
                                ("works_read", {"work": "x"}),
                                ("works_save_model", {"work": "x", "model_text": DOOR})):
            with self.subTest(tool=name):
                answer = mcp.call_tool(name, arguments, remote=True)
                self.assertTrue(answer.get("isError"))
                self.assertIn("remote caller", body(answer))

    def test_a_missing_binary_says_how_to_have_one(self):
        with unittest.mock.patch.dict(os.environ, {"SCE_WORK": "/nowhere/sce-work"}):
            answer = call("works_list")
        self.assertTrue(answer.get("isError"))
        refusal = data(answer)
        self.assertEqual("unavailable", refusal["refused"])
        self.assertIn("SCE_WORK", refusal["message"])
        self.assertIn("cargo build", refusal["message"])

    def test_the_binary_the_environment_names_is_the_one_run(self):
        with unittest.mock.patch.dict(os.environ, {"SCE_WORK": "relative/sce-work"}):
            named = works.default_work_binary()
        self.assertTrue(named.is_absolute())
        self.assertEqual("sce-work", named.name)

    def test_what_the_command_layer_refuses_keeps_its_kind_and_its_facts(self):
        refusal = works._refusal(
            json.dumps({"v": 1, "error": {"kind": "conflict", "message": "stale",
                                          "detail": {"base": None, "current": "a" * 64}}}),
            3)
        self.assertEqual("conflict", refusal.kind)
        self.assertEqual("a" * 64, refusal.detail["current"])
        # A command line it could not read is not a refusal and does not pretend to be.
        usage = works._refusal("error: unrecognized subcommand\n", 2)
        self.assertEqual("failed", usage.kind)
        self.assertIn("status 2", str(usage))


if __name__ == "__main__":
    unittest.main()
