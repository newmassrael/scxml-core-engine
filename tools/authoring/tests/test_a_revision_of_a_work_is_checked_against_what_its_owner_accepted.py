"""A revision of a work is checked against what its owner accepted, through the real `sce-work`.

The owner accepts a design in the application, and revises the text there afterwards. What an
authoring client owes the owner is what the revision did to each requirement: whether the
design moved where the words asked it to, and whether it kept what the specification dropped.
ADR 0009 joins the two facts for a design and a delta handed over by hand; for a work both are
the work's own (`docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md`):

* the WORDS of each requirement come from the lineage of the list the owner accepted and the
  lineage of the list the work has now, so any two revisions can be compared;
* the EVIDENCE comes from the product, asked by the application's command layer of the work
  laid out the way the product is always shown one, and read with the acceptance as one state.

These cases drive `works_revision_check` and `works_revision_report` against the REAL `sce-work`
and generator, with the acceptance taken by the application's own commands, because what they
promise is a property of the pair. Skipped, and saying so, where either binary is not built.
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

ON = "The lamp starts off."
SWITCH = "Pressing the switch turns it on."
FUSE = "A fuse protects it."
RESET = "A reset key clears it."
FIRST = f"{ON} {SWITCH} {FUSE}"
SECOND = f"{ON} {SWITCH}"
THIRD = f"{ON} {SWITCH} {RESET}"

HEAD = ('<?xml version="1.0" encoding="UTF-8"?>\n'
        '<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" '
        'sce:kind="statechart" version="1.0" initial="off">\n')


def design(event: str = "press", on: str = '', extra: str = "") -> str:
    """The lamp: R1 on the state it starts in, R2 on the transition that turns it on; `on` is the
    citation the second state carries (R3 in the first revision, R4 in the third)."""
    return (HEAD
            + f'  <state id="off" sce:req="R1"><transition event="{event}" target="on" '
              'sce:req="R2"/></state>\n'
            + f'  <state id="on"{on}/>\n{extra}</scxml>\n')


def call(name: str, **arguments) -> dict:
    return mcp.call_tool(name, arguments)


def body(answer: dict) -> str:
    return answer["content"][0]["text"]


def data(answer: dict) -> dict:
    return json.loads(body(answer))


def quotes(*sentences):
    return [{"quote": s, "statement": "s"} for s in sentences]


@BUILT
class ARevisionOfAWork(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        patch = unittest.mock.patch.dict(
            os.environ, {"SCE_WORKS_DIR": str(pathlib.Path(self._tmp.name) / "works")})
        patch.start()
        self.addCleanup(patch.stop)
        self.addCleanup(self._tmp.cleanup)
        self.work = works.call_work("create_work", {"title": "Lamp"})["id"]

    # -- the owner's side ------------------------------------------------------------------
    def write_text(self, text: str) -> str:
        held = works.call_work("read_source", {"id": self.work})["source"]
        return works.call_work("save_source", {
            "id": self.work, "text": text,
            "base": held["revision"] if held else None})["revision"]

    def accept(self) -> None:
        """The owner presses accept on the page they were shown."""
        shown = works.call_work("requirements_report", {"id": self.work})["basis"]
        works.call_work("accept", {"id": self.work, "expect": shown})

    # -- the client's side -----------------------------------------------------------------
    def read(self) -> dict:
        answer = call("works_read", work=self.work)
        self.assertFalse(answer.get("isError"), body(answer))
        return data(answer)

    def build(self, text: str, sentences) -> dict:
        arguments = {"specification_text": text, "requirements": quotes(*sentences), "doc_id": "lamp"}
        held = self.read()["requirements"]
        if held is not None:
            arguments["previous_sidecar_text"] = held["sidecar_text"]
            if "lineage_text" in held:
                arguments["lineage_text"] = held["lineage_text"]
            else:
                arguments["previous_manifest_text"] = held["manifest_text"]
        built = call("scxml_requirement_set", **arguments)
        self.assertFalse(built.get("isError"), body(built))
        return data(built)

    def save_list(self, built: dict, source: str, lineage: bool = True) -> None:
        arguments = {"work": self.work, "manifest_text": built["manifest_text"],
                     "sidecar_text": built["sidecar_text"], "source_revision": source}
        if lineage:
            arguments["lineage_text"] = built["lineage_text"]
        held = self.read()["requirements"]
        if held is not None:
            arguments["base"] = held["revision"]
        saved = call("works_save_requirements", **arguments)
        self.assertFalse(saved.get("isError"), body(saved))

    def save_design(self, text: str, source: str) -> None:
        held = self.read()["model"]
        arguments = {"work": self.work, "model_text": text, "source_revision": source}
        if held is not None:
            arguments["base"] = held["revision"]
        saved = call("works_save_model", **arguments)
        self.assertFalse(saved.get("isError"), body(saved))

    def revise(self, text: str, sentences, model: str, lineage: bool = True) -> str:
        """The owner revises the text; the client reads it again and saves the next list and design."""
        source = self.write_text(text)
        self.save_list(self.build(text, sentences), source, lineage)
        self.save_design(model, source)
        return source

    def accepted_lamp(self, lineage: bool = True) -> str:
        """A work with the first revision of the lamp, accepted by its owner."""
        source = self.write_text(FIRST)
        self.save_list(self.build(FIRST, [ON, SWITCH, FUSE]), source, lineage)
        self.save_design(design(on=' sce:req="R3"'), source)
        self.accept()
        return source

    def check(self) -> dict:
        answer = call("works_revision_check", work=self.work)
        self.assertFalse(answer.get("isError"), body(answer))
        return data(answer)

    def kinds(self, result: dict) -> dict:
        return {r["requirement"]: r["kind"] for r in result["requirements"]}

    # -- what the revision did -------------------------------------------------------------
    def test_a_work_nothing_was_done_to_carries_every_requirement_over(self):
        self.accepted_lamp()
        result = self.check()
        self.assertEqual("within-reach", result["verdict"])
        self.assertEqual({"R1": "carries-over", "R2": "carries-over", "R3": "carries-over"},
                         self.kinds(result))
        self.assertEqual(3, result["summary"]["seen"])

    def test_a_sentence_dropped_and_its_citation_taken_off_is_retired_cleanly(self):
        self.accepted_lamp()
        self.revise(SECOND, [ON, SWITCH], design())
        result = self.check()
        self.assertEqual("within-reach", result["verdict"], result)
        self.assertEqual({"R1": "carries-over", "R2": "carries-over", "R3": "retired-cleanly"},
                         self.kinds(result))

    def test_a_sentence_dropped_and_still_cited_by_the_design_is_outside_reach(self):
        self.accepted_lamp()
        self.revise(SECOND, [ON, SWITCH], design(on=' sce:req="R3"'))
        result = self.check()
        self.assertEqual("outside-reach", result["verdict"], result)
        self.assertEqual("retired-still-cited", self.kinds(result)["R3"])

    def test_a_design_that_moved_where_the_words_did_not_is_outside_reach(self):
        source = self.accepted_lamp()
        self.save_design(design(event="push", on=' sce:req="R3"'), source)
        result = self.check()
        self.assertEqual("outside-reach", result["verdict"], result)
        self.assertEqual("moved-without-reason", self.kinds(result)["R2"])
        moved = next(r for r in result["requirements"] if r["requirement"] == "R2")["moved"]
        self.assertTrue(any("transitions[" in place for place in moved), moved)

    def test_two_revisions_after_the_acceptance_are_compared_as_one_step(self):
        # R3 is dropped in the second revision and the reset key is added in the third; the
        # owner accepted the first. The new requirement is issued R4, not R3 again.
        self.accepted_lamp()
        self.revise(SECOND, [ON, SWITCH], design())
        self.revise(THIRD, [ON, SWITCH, RESET], design(on=' sce:req="R4"'))
        result = self.check()
        self.assertEqual("within-reach", result["verdict"], result)
        self.assertEqual({"R1": "carries-over", "R2": "carries-over", "R3": "retired-cleanly",
                          "R4": "implemented-new"}, self.kinds(result))

    def test_the_answer_names_the_revisions_it_is_about(self):
        self.accepted_lamp()
        before = self.read()
        self.revise(SECOND, [ON, SWITCH], design())
        result = self.check()
        self.assertEqual(before["requirements"]["revision"], result["of"]["accepted"]["requirements"])
        self.assertEqual(self.read()["requirements"]["revision"], result["of"]["now"]["requirements"])
        self.assertNotEqual(result["of"]["accepted"]["requirements"],
                            result["of"]["now"]["requirements"])

    def test_an_acceptance_of_a_list_that_had_no_lineage_is_compared_by_adopting_it(self):
        self.accepted_lamp(lineage=False)
        self.assertNotIn("lineage_text", self.read()["requirements"])
        self.revise(SECOND, [ON, SWITCH], design())
        result = self.check()
        self.assertEqual("within-reach", result["verdict"], result)
        self.assertEqual("retired-cleanly", self.kinds(result)["R3"])

    # -- what is refused -------------------------------------------------------------------
    def test_a_work_nobody_accepted_has_nothing_to_compare_and_says_so(self):
        source = self.write_text(FIRST)
        self.save_list(self.build(FIRST, [ON, SWITCH, FUSE]), source)
        self.save_design(design(on=' sce:req="R3"'), source)
        refused = call("works_revision_check", work=self.work)
        self.assertTrue(refused.get("isError"), body(refused))
        self.assertIn("nothing was accepted", body(refused))

    def test_a_list_with_no_lineage_and_no_sidecar_cannot_be_compared_and_says_why(self):
        # The accepted list was saved as a client that knew of neither might save it: a manifest
        # alone. No lineage is kept and none can be made (the words behind its ids are in the
        # sidecar), so what became of those requirements' words cannot be said, and nothing is
        # guessed. The next list is a fresh one, which the work accepts: it holds no lineage to lose.
        source = self.write_text(FIRST)
        first = self.build(FIRST, [ON, SWITCH, FUSE])
        saved = call("works_save_requirements", work=self.work,
                     manifest_text=first["manifest_text"], source_revision=source)
        self.assertFalse(saved.get("isError"), body(saved))
        self.save_design(design(on=' sce:req="R3"'), source)
        self.accept()
        source = self.write_text(SECOND)
        later = call("scxml_requirement_set", specification_text=SECOND,
                     requirements=quotes(ON, SWITCH), doc_id="lamp")
        self.save_list(data(later), source)
        self.save_design(design(), source)
        refused = call("works_revision_check", work=self.work)
        self.assertTrue(refused.get("isError"), body(refused))
        self.assertIn("keeps no lineage and no sidecar", body(refused))

    # An acceptance that pinned another manifest than the list it was taken of is refused by the
    # product now, and that refusal is held where it is made: `app-core/tests/a_work_says_what_
    # its_revision_did_to_each_requirement.rs` (`an_acceptance_that_pinned_another_manifest_than_
    # the_list_is_refused`), with a stand-in whose record pins another manifest.

    def test_a_remote_caller_is_not_offered_the_works_folder(self):
        # A work that WAS accepted, so that a tool that did not refuse a remote caller would
        # answer: the refusal is what is under test, and a work with nothing to compare would be
        # refused for another reason and pass for the wrong one.
        self.accepted_lamp()
        for tool in ("works_revision_check", "works_revision_report"):
            with self.subTest(tool):
                local = mcp.call_tool(tool, {"work": self.work})
                self.assertFalse(local.get("isError"), body(local))
                refused = mcp.call_tool(tool, {"work": self.work}, remote=True)
                self.assertTrue(refused.get("isError"), body(refused))
                self.assertIn("cannot reach", body(refused))

    # -- the page --------------------------------------------------------------------------
    def test_the_page_lists_only_what_to_look_at_and_prints_sentences_only_when_asked(self):
        self.accepted_lamp()
        source = self.write_text(SECOND)
        self.save_list(self.build(SECOND, [ON, SWITCH]), source)
        self.save_design(design(on=' sce:req="R3"'), source)
        plain = data(call("works_revision_report", work=self.work))
        self.assertEqual("outside-reach", plain["verdict"])
        self.assertIn("retired-still-cited", plain["page"])
        self.assertNotIn(FUSE, plain["page"], "a sentence was printed without being asked")
        self.assertNotIn("local artefact", plain["page"])
        asked = data(call("works_revision_report", work=self.work, sentences=True))
        self.assertIn("local artefact", asked["page"])


if __name__ == "__main__":
    unittest.main()
