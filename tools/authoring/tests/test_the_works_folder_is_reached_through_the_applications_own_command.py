"""An authoring client reads a work and saves its model through the application's own command.

The workbench application keeps a specification as a work, and the owner asks an AI
client to write the model of it. These cases drive the tools that connect the two
(`works_list`, `works_read`, `works_save_model`, `works_save_requirements`) against
the REAL `sce-work`
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


@BUILT
class TheRequirementsAndTheOwnersAcceptance(unittest.TestCase):
    """A client saves the requirement list it read from the text, and is told whether the
    owner accepted the design in the application; it cannot accept for them."""

    QUOTE = "The door opens when the card matches."

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
        built = data(call("scxml_requirement_set", specification_text=SPEC, requirements=[
            {"quote": self.QUOTE, "statement": "The door opens for a matching card."}]))
        self.manifest, self.sidecar = built["manifest_text"], built["sidecar_text"]

    def save_list(self, **extra):
        return call("works_save_requirements", work=self.work, manifest_text=self.manifest,
                    sidecar_text=self.sidecar, source_revision=self.revision, **extra)

    def owner_accepts(self):
        """The owner's side: the press of the application's button, on what it shows."""
        shown = works.call_work("requirements_report", {"id": self.work})["basis"]
        return works.call_work("accept", {"id": self.work, "expect": shown})

    def test_a_work_nobody_listed_requirements_for_says_so_and_says_what_to_do(self):
        call("works_save_model", work=self.work, model_text=DOOR, source_revision=self.revision)
        read = data(call("works_read", work=self.work))
        self.assertIsNone(read["requirements"])
        self.assertIsNone(read["acceptance"])
        self.assertIn("works_save_requirements", read["next"])

    def test_a_list_saved_for_the_text_read_comes_back_as_the_files_the_tools_take(self):
        saved = self.save_list()
        self.assertFalse(saved.get("isError"), body(saved))
        self.assertEqual("saved", data(saved)["outcome"])
        self.assertIn("not accepted", data(saved)["next"])

        read = data(call("works_read", work=self.work))
        requirements = read["requirements"]
        # Byte for byte: an acceptance pins these files by their hash.
        self.assertEqual(self.manifest, requirements["manifest_text"])
        self.assertEqual(self.sidecar, requirements["sidecar_text"])
        self.assertEqual("current", requirements["standing"])
        self.assertEqual(self.revision, requirements["written_for"])
        self.assertNotIn("requirement list", read["next"])

    def test_a_list_saved_without_the_text_it_was_read_from_is_unstated_not_current(self):
        call("works_save_requirements", work=self.work, manifest_text=self.manifest)
        requirements = data(call("works_read", work=self.work))["requirements"]
        self.assertEqual("unstated", requirements["standing"])
        self.assertNotIn("sidecar_text", requirements)

    def test_a_list_of_a_text_that_moved_on_stands_behind_and_the_client_is_told_to_build_it_again(self):
        self.save_list()
        works.call_work("save_source", {"id": self.work, "text": SPEC + "More.\n",
                                        "base": self.revision})
        read = data(call("works_read", work=self.work))
        self.assertEqual("behind", read["requirements"]["standing"])
        self.assertIn("base = requirements.revision", read["next"])

    def test_a_list_the_product_will_not_load_is_not_saved(self):
        refused = call("works_save_requirements", work=self.work,
                       manifest_text='{"requirements": "nope"}\n', source_revision=self.revision)
        self.assertTrue(refused.get("isError"), body(refused))
        self.assertIn("not saved", body(refused))
        # The product's own words come back, to build the list again by.
        self.assertIn("cli/closure-input-unusable", body(refused))
        self.assertIsNone(data(call("works_read", work=self.work))["requirements"])

    def test_a_manifest_that_is_not_json_is_refused_and_not_kept(self):
        refused = call("works_save_requirements", work=self.work, manifest_text="not json at all",
                       source_revision=self.revision)
        self.assertTrue(refused.get("isError"), body(refused))
        self.assertIsNone(data(call("works_read", work=self.work))["requirements"])

    def test_a_save_from_a_list_that_is_not_current_is_a_conflict_and_writes_nothing(self):
        first = data(self.save_list())
        again = self.save_list()
        self.assertTrue(again.get("isError"))
        refusal = data(again)
        self.assertEqual("conflict", refusal["refused"])
        self.assertEqual(first["revision"], refusal["detail"]["current"])
        self.assertFalse(self.save_list(base=first["revision"]).get("isError"))

    def test_an_arguments_shape_is_refused_before_anything_runs(self):
        for arguments, wanted in (({"manifest_text": ""}, "manifest_text"),
                                  ({"manifest_text": 3}, "manifest_text"),
                                  ({"manifest_text": "{}", "sidecar_text": 3}, "sidecar_text"),
                                  ({"manifest_text": "{}", "source_revision": "latest"},
                                   "64 lowercase hexadecimal digits")):
            with self.subTest(arguments=sorted(arguments)):
                answer = call("works_save_requirements", work=self.work, **arguments)
                self.assertTrue(answer.get("isError"))
                self.assertIn(wanted, body(answer))

    def test_a_design_the_owner_accepted_is_told_to_the_client_and_no_new_draft_is_asked_for(self):
        call("works_save_model", work=self.work, model_text=DOOR, source_revision=self.revision)
        self.save_list()
        read = data(call("works_read", work=self.work))
        self.assertEqual({"standing": "none"}, read["acceptance"])
        self.assertIn("base = model.revision", read["next"])

        self.owner_accepts()
        read = data(call("works_read", work=self.work))
        accepted = read["acceptance"]
        self.assertEqual("holds", accepted["standing"])
        self.assertEqual("direct", accepted["channel"], "the owner's own press in the application")
        self.assertNotIn("lapse", accepted)
        self.assertIn("write no new draft unless they ask for one", read["next"])
        self.assertNotIn("source_revision = source.revision", read["next"])

    def test_an_acceptance_that_lapsed_says_what_moved_and_the_owner_accepts_again(self):
        saved = data(call("works_save_model", work=self.work, model_text=DOOR,
                          source_revision=self.revision))
        self.save_list()
        self.owner_accepts()

        call("works_save_model", work=self.work, model_text=DOOR + "<!-- second -->\n",
             source_revision=self.revision, base=saved["revision"])
        read = data(call("works_read", work=self.work))
        accepted = read["acceptance"]
        self.assertEqual("lapsed", accepted["standing"])
        # The product's own sentence, whole: it names the file that moved.
        self.assertIn("design/model.scxml", accepted["lapse"])
        self.assertIn("no longer holds", read["next"])
        self.assertIn("nothing here records an acceptance", read["next"])
        # The client's way back is the normal one: it may write the model again.
        self.assertIn("source_revision = source.revision", read["next"])

    def test_the_product_not_answering_does_not_stop_a_work_being_read(self):
        call("works_save_model", work=self.work, model_text=DOOR, source_revision=self.revision)
        self.save_list()
        self.owner_accepts()
        # The work is read through `works` itself: the generator the application would run is
        # named by the same variable the package runs its own checks with, and what is under
        # test is the application's answer, not those checks.
        with unittest.mock.patch.dict(os.environ, {"SCE_CODEGEN": "/nowhere/sce-codegen"}):
            read = works.read_work(self.work)
        self.assertEqual("unavailable", read["acceptance"]["standing"])
        self.assertTrue(read["acceptance"]["kind"].startswith("sce-"))
        # The rest of the work is still there to read.
        self.assertEqual(DOOR, read["model"]["text"])

    def test_there_is_no_tool_that_states_an_acceptance_for_the_owner(self):
        names = {t["name"] for t in mcp.TOOLS}
        for forbidden in ("works_accept", "works_save_acceptance", "works_acceptance"):
            self.assertNotIn(forbidden, names)


@BUILT
class TheClientTakesTheOwnersRequestForAModel(unittest.TestCase):
    """The owner asks for a model in the application, and the client that writes it takes the
    request, writes for it, and says it is done: what it wrote is the work's only then, the
    model and the requirement list together."""

    QUOTE = "The door opens when the card matches."

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        patch = unittest.mock.patch.dict(
            os.environ, {"SCE_WORKS_DIR": str(pathlib.Path(self._tmp.name) / "works")})
        patch.start()
        self.addCleanup(patch.stop)
        self.addCleanup(self._tmp.cleanup)
        # A keeper of this test's own, that renews nothing while the case runs: what is held
        # is the case's, and the process's other tests do not find it.
        keeper = works.Generations(interval=3600.0)
        self.addCleanup(keeper.close)
        patch_keeper = unittest.mock.patch.object(works, "_GENERATIONS", keeper)
        patch_keeper.start()
        self.addCleanup(patch_keeper.stop)
        self.work = works.call_work("create_work", {"title": "Door lock"})["id"]
        self.revision = works.call_work(
            "save_source", {"id": self.work, "text": SPEC})["revision"]
        built = data(call("scxml_requirement_set", specification_text=SPEC, requirements=[
            {"quote": self.QUOTE, "statement": "The door opens for a matching card."}]))
        self.manifest, self.sidecar = built["manifest_text"], built["sidecar_text"]

    def owner_asks(self, key: str = "press-1") -> str:
        """The owner's side: the press of the application's button."""
        return works.call_work("request_generation", {
            "id": self.work, "key": key, "origin": "gui",
            "expect": {"source": self.revision}})["request"]["id"]

    def begin(self) -> dict:
        answer = call("works_begin_generation", work=self.work)
        self.assertFalse(answer.get("isError"), body(answer))
        return data(answer)

    def write_both(self, request: str, model: str = DOOR) -> None:
        saved = call("works_save_model", work=self.work, model_text=model, request=request)
        self.assertFalse(saved.get("isError"), body(saved))
        listed = call("works_save_requirements", work=self.work, manifest_text=self.manifest,
                      sidecar_text=self.sidecar, request=request)
        self.assertFalse(listed.get("isError"), body(listed))

    def request_of(self, request: str) -> dict:
        return works.call_work("read_request", {"id": self.work, "request": request})["request"]

    def test_the_request_the_owner_made_is_the_one_the_client_takes(self):
        asked = self.owner_asks()

        read = data(call("works_read", work=self.work))
        self.assertEqual(asked, read["request"]["id"])
        self.assertEqual("queued", read["request"]["state"])
        self.assertIn("works_begin_generation", read["next"])

        begun = self.begin()

        self.assertEqual(asked, begun["generation"]["request"])
        self.assertEqual(1, begun["generation"]["attempt"])
        self.assertEqual(self.revision, begun["generation"]["source"])
        self.assertIn(f"request = {asked}", begun["next"])
        seen = self.request_of(asked)
        self.assertEqual("running", seen["state"])
        self.assertTrue(seen["lease"]["holder"].startswith("mcp-"), seen["lease"])

    def test_a_work_nobody_asked_for_a_model_is_asked_for_one_by_the_client_that_begins(self):
        begun = self.begin()

        made = self.request_of(begun["generation"]["request"])
        self.assertEqual("mcp", made["origin"])
        self.assertEqual({"source": self.revision, "answers": None}, made["inputs"])

    def test_what_is_written_for_the_request_is_the_works_only_when_it_is_finished(self):
        request = self.begin()["generation"]["request"]

        saved = data(call("works_save_model", work=self.work, model_text=DOOR, request=request))
        self.assertIn("requirement list", saved["next"], "the other half is still to write")
        read = data(call("works_read", work=self.work))
        self.assertIsNone(read["model"], "a candidate is not the work's model")

        self.write_both(request)
        finished = call("works_finish_generation", work=self.work, request=request)
        self.assertFalse(finished.get("isError"), body(finished))
        done = data(finished)

        self.assertEqual("completed", done["request"]["state"])
        self.assertRegex(done["bundle"], r"^[0-9a-f]{64}$")
        read = data(call("works_read", work=self.work))
        self.assertEqual(DOOR, read["model"]["text"])
        self.assertEqual("current", read["model"]["standing"])
        self.assertEqual(self.manifest, read["requirements"]["manifest_text"])
        self.assertEqual(done["bundle"], read["bundle"])
        # The core ran its own check of the model, and the bundle says so.
        bundle = works.call_work("read_bundle", {"id": self.work})["bundle"]["bundle"]
        self.assertEqual(["core"], [c["by"] for c in bundle["checks"]])
        self.assertEqual("accepted", bundle["checks"][0]["verdict"])
        # The bundle says what the client was told to do, named by the wording of it.
        self.assertEqual(mcp.INSTRUCTIONS_VERSION, bundle["instructions"])
        self.assertRegex(bundle["instructions"], r"^sce-author-mcp/[0-9a-f]{12}$")
        self.assertIsNone(bundle["replaces"], "the first bundle replaced none")

    def test_a_work_that_had_a_model_published_refuses_the_plain_saves_and_says_what_to_do(self):
        request = self.begin()["generation"]["request"]
        self.write_both(request)
        call("works_finish_generation", work=self.work, request=request)

        refused = call("works_save_model", work=self.work, model_text=DOOR + "<!-- by hand -->\n",
                       source_revision=self.revision)

        self.assertTrue(refused.get("isError"), body(refused))
        self.assertEqual("bundled-work", data(refused)["refused"])
        self.assertIn("works_begin_generation", data(refused)["next"])
        self.assertIn("works_begin_generation", data(call("works_read", work=self.work))["next"])

    def test_finishing_with_one_half_written_is_refused_and_the_request_stays_the_clients(self):
        request = self.begin()["generation"]["request"]
        call("works_save_model", work=self.work, model_text=DOOR, request=request)

        refused = call("works_finish_generation", work=self.work, request=request)

        self.assertEqual("no-candidate", data(refused)["refused"])
        self.assertEqual(["requirements"], data(refused)["detail"]["missing"])
        self.assertEqual("running", self.request_of(request)["state"])
        # Written, and said again: the same generation finishes.
        call("works_save_requirements", work=self.work, manifest_text=self.manifest,
             sidecar_text=self.sidecar, request=request)
        self.assertFalse(call("works_finish_generation", work=self.work,
                              request=request).get("isError"))

    def test_a_request_the_owner_called_off_is_told_at_the_next_word_and_then_not_spoken_for(self):
        request = self.begin()["generation"]["request"]
        works.call_work("cancel_request", {"id": self.work, "request": request})

        told = call("works_save_model", work=self.work, model_text=DOOR, request=request)
        self.assertEqual("request-ended", data(told)["refused"])
        self.assertEqual("cancelled", data(told)["detail"]["state"])
        again = call("works_save_model", work=self.work, model_text=DOOR, request=request)

        self.assertEqual("generation-ended", data(again)["refused"])
        self.assertIn("works_read", data(again)["next"])

    def test_a_text_the_owner_saved_after_the_client_began_ends_the_request_it_was_asked_about(self):
        request = self.begin()["generation"]["request"]
        works.call_work("save_source", {"id": self.work, "text": SPEC + "A second sentence.\n",
                                        "base": self.revision})

        told = call("works_save_model", work=self.work, model_text=DOOR, request=request)

        self.assertEqual("request-ended", data(told)["refused"])
        self.assertEqual("superseded", data(told)["detail"]["state"])

    def test_a_request_the_applications_own_executor_holds_is_not_taken(self):
        asked = self.owner_asks()
        works.call_work("claim_request",
                        {"id": self.work, "request": asked, "holder": "adapter-a"})

        refused = call("works_begin_generation", work=self.work)

        self.assertEqual("request-held", data(refused)["refused"])
        self.assertIn("write nothing", data(refused)["next"])

    def test_a_work_with_no_text_is_not_begun(self):
        empty = works.call_work("create_work", {"title": "Empty"})["id"]

        refused = call("works_begin_generation", work=empty)

        self.assertEqual("no-text", data(refused)["refused"])

    def test_a_draft_is_written_for_the_text_its_request_is_about(self):
        request = self.begin()["generation"]["request"]

        refused = call("works_save_model", work=self.work, model_text=DOOR, request=request,
                       source_revision="f" * 64)

        self.assertTrue(refused.get("isError"))
        self.assertIn("source_revision", body(refused))

    def test_a_request_is_spoken_for_only_in_its_own_work(self):
        request = self.begin()["generation"]["request"]
        other = works.call_work("create_work", {"title": "Other"})["id"]

        refused = call("works_save_model", work=other, model_text=DOOR, request=request)

        self.assertTrue(refused.get("isError"))
        self.assertIn(f"`{self.work}`", body(refused))

    def test_a_generation_this_server_did_not_begin_is_not_spoken_for(self):
        asked = self.owner_asks()

        refused = call("works_save_model", work=self.work, model_text=DOOR, request=asked)

        self.assertEqual("unknown-generation", data(refused)["refused"])

    def test_a_client_that_cannot_write_the_model_says_why_and_the_owner_is_told(self):
        request = self.begin()["generation"]["request"]

        failed = call("works_fail_generation", work=self.work, request=request,
                      reason="The text never says which cards open the door.")

        self.assertFalse(failed.get("isError"), body(failed))
        seen = self.request_of(request)
        self.assertEqual("failed", seen["state"])
        self.assertEqual("The text never says which cards open the door.", seen["note"])

    def test_the_decisions_the_draft_was_held_to_are_kept_beside_the_cores_check(self):
        works.call_work("save_answers", {
            "id": self.work, "answers": {"open-guard": "Any card on the list opens it."}})
        request = self.begin()["generation"]["request"]
        # The same door, applying the owner's answer to the question it asked.
        call("works_save_model", work=self.work, model_text=APPLIES, request=request)
        call("works_save_requirements", work=self.work, manifest_text=self.manifest,
             sidecar_text=self.sidecar, request=request)

        finished = call("works_finish_generation", work=self.work, request=request)

        self.assertFalse(finished.get("isError"), body(finished))
        bundle = works.call_work("read_bundle", {"id": self.work})["bundle"]["bundle"]
        self.assertEqual([("core", "model"), ("client", "decisions")],
                         [(c["by"], c["name"]) for c in bundle["checks"]])


class TheWorksFolderIsThisMachinesOwn(unittest.TestCase):
    """What needs no binary: the refusals that come before one is run."""

    def test_a_remote_caller_is_not_offered_it(self):
        for name, arguments in (("works_list", {}),
                                ("works_read", {"work": "x"}),
                                ("works_begin_generation", {"work": "x"}),
                                ("works_finish_generation", {"work": "x", "request": "req-1"}),
                                ("works_fail_generation",
                                 {"work": "x", "request": "req-1", "reason": "no"}),
                                ("works_save_requirements", {"work": "x", "manifest_text": "{}"}),
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
