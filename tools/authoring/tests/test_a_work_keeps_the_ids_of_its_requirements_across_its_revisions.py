"""A work keeps the ids of its requirements when its text is revised, through the real `sce-work`.

The workbench keeps a specification as a work, and the owner revises its text there. What the
authoring client can hand back to the next call of `scxml_requirement_set` is whatever the work
gives it, and measured on three revisions of a three-sentence text, a manifest and a sidecar alone
gave the third revision's new requirement the id of the sentence the second had dropped
(`docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md`). These cases
drive `works_read`, `scxml_requirement_set` and `works_save_requirements` against the real
`sce-work` and hold that:

* the lineage a list was built against is kept with the list and read back with it;
* a revision built from what `works_read` gave keeps every id and issues the new one from where the
  last left off, with nothing handed over by hand;
* a list without the lineage the work holds is refused, by a direct save and by a generation's,
  and so is a lineage that is another history's or another list's;
* the same flow reaches the same ids by a generation, which is the application's main path.

Skipped, and saying so, where either binary is not built; the authoring lane builds both.
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

LAMP = "The lamp starts off. Pressing the switch turns it on. A fuse protects it."
DROPPED = "The lamp starts off. Pressing the switch turns it on."
ADDED = "The lamp starts off. Pressing the switch turns it on. A reset key clears it."
ON = "The lamp starts off."
SWITCH = "Pressing the switch turns it on."
FUSE = "A fuse protects it."
RESET = "A reset key clears it."

DOOR = ('<?xml version="1.0" encoding="UTF-8"?>\n'
        '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
        'xmlns:sce="http://sce.dev/ext" sce:kind="statechart" version="1.0" '
        'initial="closed">\n'
        '  <state id="closed"><transition event="open" target="opened"/></state>\n'
        '  <state id="opened"><transition event="close" target="closed"/></state>\n'
        '</scxml>\n')


def call(name: str, **arguments) -> dict:
    return mcp.call_tool(name, arguments)


def body(answer: dict) -> str:
    return answer["content"][0]["text"]


def data(answer: dict) -> dict:
    return json.loads(body(answer))


def quotes(*sentences):
    return [{"quote": s, "statement": "s"} for s in sentences]


@BUILT
class AWorkKeepsTheIdsOfItsRequirements(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        patch = unittest.mock.patch.dict(
            os.environ, {"SCE_WORKS_DIR": str(pathlib.Path(self._tmp.name) / "works")})
        patch.start()
        self.addCleanup(patch.stop)
        self.addCleanup(self._tmp.cleanup)
        self.work = works.call_work("create_work", {"title": "Lamp"})["id"]
        self.source = self.write_text(LAMP, None)

    # -- the owner's side ------------------------------------------------------------------
    def write_text(self, text: str, base: str | None) -> str:
        return works.call_work("save_source", {"id": self.work, "text": text, "base": base}
                               )["revision"]

    # -- the client's side -----------------------------------------------------------------
    def read(self) -> dict:
        answer = call("works_read", work=self.work)
        self.assertFalse(answer.get("isError"), body(answer))
        return data(answer)

    def build(self, text: str, sentences, read: dict | None = None, **by_hand) -> dict:
        """The list for `text`, built the way works_read's `next` says: against what the work holds."""
        arguments = {"specification_text": text, "requirements": quotes(*sentences),
                     "doc_id": "lamp", **by_hand}
        held = (read or {}).get("requirements")
        if held is not None and not by_hand:
            arguments["previous_sidecar_text"] = held["sidecar_text"]
            if "lineage_text" in held:
                arguments["lineage_text"] = held["lineage_text"]
            else:
                arguments["previous_manifest_text"] = held["manifest_text"]
        answer = call("scxml_requirement_set", **arguments)
        self.assertFalse(answer.get("isError"), body(answer))
        return data(answer)

    def save(self, built: dict, read: dict | None, source: str, lineage: bool = True, **more) -> dict:
        arguments = {"work": self.work, "manifest_text": built["manifest_text"],
                     "sidecar_text": built["sidecar_text"], "source_revision": source, **more}
        if lineage:
            arguments["lineage_text"] = built["lineage_text"]
        if read is not None and read.get("requirements") is not None and "request" not in more:
            arguments["base"] = read["requirements"]["revision"]
        return call("works_save_requirements", **arguments)

    def ids_of(self, read: dict) -> list[str]:
        return [r["id"] for r in json.loads(read["requirements"]["manifest_text"])["requirements"]]

    def three_revisions(self) -> dict:
        """The text revised twice: the last sentence dropped, then another added."""
        first = self.build(LAMP, [ON, SWITCH, FUSE])
        self.assertFalse(self.save(first, None, self.source).get("isError"))
        second_source = self.write_text(DROPPED, self.source)
        read = self.read()
        second = self.build(DROPPED, [ON, SWITCH], read)
        self.assertFalse(self.save(second, read, second_source).get("isError"))
        third_source = self.write_text(ADDED, second_source)
        return {"read": self.read(), "source": third_source}

    # -- what the work keeps ---------------------------------------------------------------
    def test_the_lineage_is_kept_with_the_list_and_read_back_with_it(self):
        built = self.build(LAMP, [ON, SWITCH, FUSE])
        saved = self.save(built, None, self.source)
        self.assertFalse(saved.get("isError"), body(saved))
        read = self.read()
        self.assertEqual(built["lineage_text"], read["requirements"]["lineage_text"])
        self.assertEqual(built["manifest_text"], read["requirements"]["manifest_text"])

    def test_a_list_saved_without_a_lineage_reads_back_with_the_one_adopting_it_makes(self):
        # A list made before lineages stands on the lineage adopting it makes (ADR 0012): the core
        # derives it from the manifest and the sidecar, hands it over as `lineage_text` and says it
        # was derived, so a client that is told only to build against `lineage_text` adopts the
        # list without being told to. Nothing of it is kept beside the list.
        built = self.build(LAMP, [ON, SWITCH, FUSE])
        self.assertFalse(self.save(built, None, self.source, lineage=False).get("isError"))
        read = self.read()["requirements"]
        self.assertIn("lineage_text", read)
        self.assertTrue(read.get("lineage_adopted"), read)

    # -- the measurement -------------------------------------------------------------------
    def test_the_third_revision_keeps_every_id_and_issues_the_new_one_from_where_the_last_left_off(self):
        held = self.three_revisions()
        third = self.build(ADDED, [ON, SWITCH, RESET], held["read"])
        by_quote = {r["quote"]: r["id"] for r in third["requirements"]}
        self.assertEqual({ON: "R1", SWITCH: "R2", RESET: "R4"}, by_quote,
                         "the sentence dropped in the second revision was R3, and is not issued again")
        saved = self.save(third, held["read"], held["source"])
        self.assertFalse(saved.get("isError"), body(saved))
        self.assertEqual(["R1", "R2", "R4"], self.ids_of(self.read()))

    def test_from_a_manifest_and_a_sidecar_alone_the_dropped_id_would_be_issued_again(self):
        # What a work that could hand back no lineage did, measured as the control that shows
        # what the lineage is for. Not a path any more: the lineage is read from the work.
        held = self.three_revisions()
        without = call("scxml_requirement_set", specification_text=ADDED,
                       requirements=quotes(ON, SWITCH, RESET), doc_id="lamp",
                       previous_manifest_text=held["read"]["requirements"]["manifest_text"],
                       previous_sidecar_text=held["read"]["requirements"]["sidecar_text"])
        by_quote = {r["quote"]: r["id"] for r in data(without)["requirements"]}
        self.assertEqual("R3", by_quote[RESET], "the control no longer measures what it did")

    def test_a_list_that_had_no_lineage_is_adopted_and_keeps_its_ids_from_then_on(self):
        first = self.build(LAMP, [ON, SWITCH, FUSE])
        self.assertFalse(self.save(first, None, self.source, lineage=False).get("isError"))
        second_source = self.write_text(DROPPED, self.source)
        read = self.read()
        # The core hands the client the lineage adopting the list makes, so the client builds
        # against it as against a kept one and keeps every id.
        self.assertIn("lineage_text", read["requirements"])
        self.assertTrue(read["requirements"].get("lineage_adopted"))
        second = self.build(DROPPED, [ON, SWITCH], read)
        saved = self.save(second, read, second_source)
        self.assertFalse(saved.get("isError"), body(saved))
        third_source = self.write_text(ADDED, second_source)
        read = self.read()
        self.assertIn("lineage_text", read["requirements"], "the adoption's lineage is kept")
        third = self.build(ADDED, [ON, SWITCH, RESET], read)
        self.assertEqual("R4", {r["quote"]: r["id"] for r in third["requirements"]}[RESET])
        self.assertFalse(self.save(third, read, third_source).get("isError"))

    # -- what is refused -------------------------------------------------------------------
    def refusal(self, answer: dict) -> dict:
        self.assertTrue(answer.get("isError"), body(answer))
        return json.loads(body(answer)) if body(answer).lstrip().startswith("{") else {
            "message": body(answer)}

    def test_a_list_without_the_lineage_the_work_holds_is_refused_and_the_work_keeps_its_list(self):
        held = self.three_revisions()
        third = self.build(ADDED, [ON, SWITCH, RESET], held["read"])
        refused = self.refusal(self.save(third, held["read"], held["source"], lineage=False))
        self.assertEqual("lineage-dropped", refused.get("refused"), refused)
        self.assertIn("lineage_text", refused["next"])
        self.assertEqual(held["read"]["requirements"], self.read()["requirements"])

    def test_a_lineage_that_is_another_lists_is_refused(self):
        held = self.three_revisions()
        third = self.build(ADDED, [ON, SWITCH, RESET], held["read"])
        other = self.build(ADDED, [ON, SWITCH], held["read"])
        refused = self.refusal(self.save(third, held["read"], held["source"],
                                         lineage=False, lineage_text=other["lineage_text"]))
        self.assertIn("another manifest than this one", refused["message"])

    def test_a_lineage_of_another_history_is_refused_though_it_is_well_formed(self):
        held = self.three_revisions()
        # A fresh start for the same text: it issues R1.. again, so it forgets R3's retirement.
        fresh = self.build(ADDED, [ON, SWITCH, RESET])
        refused = self.refusal(self.save(fresh, held["read"], held["source"]))
        self.assertIn("cannot be kept with this list", refused["message"])
        self.assertEqual(held["read"]["requirements"], self.read()["requirements"])

    def test_a_save_from_a_stale_base_is_a_conflict_and_not_a_lineage_complaint(self):
        held = self.three_revisions()
        third = self.build(ADDED, [ON, SWITCH, RESET], held["read"])
        stale = self.save(third, held["read"], held["source"])
        self.assertFalse(stale.get("isError"), body(stale))
        again = self.build(ADDED, [ON, SWITCH], held["read"])
        refused = self.refusal(self.save(again, held["read"], held["source"]))
        self.assertEqual("conflict", refused.get("refused"), refused)

    # -- the application's main path -------------------------------------------------------
    def test_a_generation_carries_the_lineage_and_publishes_it_with_the_list(self):
        held = self.three_revisions()
        request = data(call("works_begin_generation", work=self.work))["generation"]["request"]
        third = self.build(ADDED, [ON, SWITCH, RESET], held["read"])
        self.assertFalse(call("works_save_model", work=self.work, model_text=DOOR,
                              request=request).get("isError"))
        listed = self.save(third, None, held["source"], request=request)
        self.assertFalse(listed.get("isError"), body(listed))
        done = call("works_finish_generation", work=self.work, request=request)
        self.assertFalse(done.get("isError"), body(done))
        read = self.read()
        self.assertEqual(third["lineage_text"], read["requirements"]["lineage_text"])
        self.assertEqual(["R1", "R2", "R4"], self.ids_of(read))

    def test_a_generation_that_would_lose_the_lineage_is_told_before_it_is_finished(self):
        held = self.three_revisions()
        request = data(call("works_begin_generation", work=self.work))["generation"]["request"]
        third = self.build(ADDED, [ON, SWITCH, RESET], held["read"])
        refused = self.refusal(self.save(third, None, held["source"], lineage=False, request=request))
        self.assertIn("keeps a requirement lineage", refused["message"])
        # Nothing was written for the request: it is still the client's to write.
        self.assertFalse(self.save(third, None, held["source"], request=request).get("isError"))


if __name__ == "__main__":
    unittest.main()
