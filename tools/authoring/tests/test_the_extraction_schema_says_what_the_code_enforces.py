"""The schema file of an extraction and the code that makes one say the same.

An extraction is a wire form (`SCHEMA_STATUS`), so something outside this tool may read it by its schema
alone. A schema that is looser than the code lets a producer write what this tool would refuse, and one
that is stricter refuses what this tool writes. Each rule the code holds is put to the schema as well,
and the enumerations are compared so that a kind added in one place and not the other fails here.
"""

from __future__ import annotations

import json
import pathlib
import unittest

import jsonschema

from sce_author import extraction
from sce_author.extraction import Block, Extraction, NotCarried, Place

SCHEMA_FILE = pathlib.Path(__file__).resolve().parents[1] / "schema" / "extraction.v1.schema.json"
SCHEMA = json.loads(SCHEMA_FILE.read_text(encoding="utf-8"))


def valid(wire: dict) -> bool:
    try:
        jsonschema.validate(wire, SCHEMA)
    except jsonschema.ValidationError:
        return False
    return True


def one(**block) -> dict:
    base = {
        "id": "b1",
        "kind": "paragraph",
        "text": "x",
        "place": {"member": "m", "precision": "none"},
        "how_read": "text-layer",
    }
    base.update(block)
    return {"v": 1, "extractor": "e", "extractor_version": "1", "member": "m", "blocks": [base]}


class TheSchemaAndTheCodeAgree(unittest.TestCase):
    def test_the_schema_is_a_schema_and_says_its_status_as_the_code_does(self):
        jsonschema.Draft202012Validator.check_schema(SCHEMA)
        self.assertEqual(SCHEMA["x-sce-schema-status"], extraction.SCHEMA_STATUS)
        self.assertEqual(SCHEMA["properties"]["v"]["const"], extraction.SCHEMA_VERSION)

    def test_the_enumerations_are_the_same_lists(self):
        defs = SCHEMA["$defs"]
        self.assertEqual(tuple(defs["block"]["properties"]["kind"]["enum"]), extraction.KINDS)
        self.assertEqual(tuple(defs["block"]["properties"]["how_read"]["enum"]), extraction.HOW_READ)
        self.assertEqual(tuple(defs["place"]["properties"]["precision"]["enum"]), extraction.PRECISIONS)

    def test_what_the_code_writes_the_schema_accepts(self):
        whole = Extraction(
            "docx",
            "1",
            "spec.docx",
            (
                Block("b1", "heading", "Lamp", Place("spec.docx", ordinal=1), level=1),
                Block(
                    "b2",
                    "table",
                    "| a | b |",
                    Place("spec.docx", part="word/document.xml", ordinal=2),
                    cells=(("a", "b"),),
                ),
                Block(
                    "b3",
                    "paragraph",
                    "scanned",
                    Place("spec.pdf", page=2, box=(0.0, 1.0, 2.0, 3.0), precision="exact"),
                    how_read="vision",
                    confidence=0.7,
                ),
                Block("b4", "figure", "[picture: a.png]", Place("spec.docx", ordinal=4), picture="ab" * 32),
            ),
            (NotCarried("a table", "not recoverable", 1), NotCarried("a header")),
        )

        self.assertTrue(valid(whole.to_dict()))

    def test_an_empty_extraction_is_a_form_too(self):
        wire = Extraction("e", "1", "m", ()).to_dict()

        self.assertTrue(valid(wire))


class EveryRuleTheCodeHoldsIsPutToTheSchemaToo(unittest.TestCase):
    def test_a_recognised_block_must_say_how_sure_it_is_and_a_text_block_must_not(self):
        self.assertFalse(valid(one(how_read="ocr")))
        self.assertTrue(valid(one(how_read="ocr", confidence=0.5)))
        self.assertFalse(valid(one(how_read="text-layer", confidence=1.0)))
        self.assertFalse(valid(one(how_read="ocr", confidence=1.5)))

    def test_a_box_says_how_exact_it_is_and_a_claim_of_a_box_needs_one(self):
        box = {"member": "m", "page": 1, "box": [0, 0, 1, 1]}
        self.assertFalse(valid(one(place={**box, "precision": "none"})))
        self.assertTrue(valid(one(place={**box, "precision": "approximate"})))
        self.assertFalse(valid(one(place={"member": "m", "page": 1, "precision": "exact"})))
        self.assertFalse(valid(one(place={**box, "box": [0, 0, 1], "precision": "exact"})))

    def test_the_page_is_not_before_the_first(self):
        self.assertFalse(valid(one(place={"member": "m", "page": 0, "precision": "none"})))

    def test_only_a_heading_has_a_level_a_table_a_grid_and_a_figure_a_picture(self):
        self.assertFalse(valid(one(level=1)))
        self.assertTrue(valid(one(kind="heading", level=1)))
        self.assertFalse(valid(one(cells=[["a"]])))
        self.assertTrue(valid(one(kind="table", cells=[["a"]])))
        self.assertFalse(valid(one(picture="ab" * 32)))
        self.assertTrue(valid(one(kind="figure", picture="ab" * 32)))

    def test_a_kind_nobody_defined_and_a_field_nobody_defined_are_refused(self):
        self.assertFalse(valid(one(kind="sidebar")))
        self.assertFalse(valid(one(extra=1)))
        wire = one()
        wire["extra"] = 1
        self.assertFalse(valid(wire))

    def test_the_version_and_the_extractor_are_required(self):
        for field in ("v", "extractor", "extractor_version", "member", "blocks"):
            with self.subTest(field=field):
                wire = one()
                del wire[field]
                self.assertFalse(valid(wire))
        wire = one()
        wire["v"] = 2
        self.assertFalse(valid(wire))

    def test_a_not_carried_entry_says_what_and_never_counts_a_negative(self):
        wire = one()
        wire["not_carried"] = [{"count": 1}]
        self.assertFalse(valid(wire))
        wire["not_carried"] = [{"what": "x", "count": -1}]
        self.assertFalse(valid(wire))
        wire["not_carried"] = [{"what": "x", "why": "y", "count": 0}]
        self.assertTrue(valid(wire))


if __name__ == "__main__":
    unittest.main()
