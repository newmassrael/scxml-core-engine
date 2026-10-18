"""An extraction: blocks, and what was not carried, in one form for every format.

ADR 0026 holds a conversion to its original block by block, so everything after an extractor reads this
form and nothing else. Each case makes the situation the form exists to refuse, and pairs it with the
nearest one that must be accepted: a form that refuses everything passes the first half and is worthless.
"""

from __future__ import annotations

import json
import unittest

from sce_author.errors import IngestError
from sce_author.extraction import SCHEMA_VERSION, Block, Extraction, NotCarried, Place


def place(**kwargs) -> Place:
    return Place(member="spec.docx", **kwargs)


def block(identifier: str = "b0001", **kwargs) -> Block:
    kwargs.setdefault("kind", "paragraph")
    kwargs.setdefault("text", "The lamp is on.")
    kwargs.setdefault("place", place(ordinal=1))
    return Block(id=identifier, **kwargs)


def extraction(*blocks: Block, carried: tuple[NotCarried, ...] = ()) -> Extraction:
    return Extraction("docx", "1", "spec.docx", tuple(blocks), carried)


class AnExtractionIsKeptAndReadBackExactly(unittest.TestCase):
    def test_every_field_survives_the_wire_form(self):
        whole = extraction(
            block("b1", kind="heading", text="Lamp", level=2),
            block(
                "b2",
                kind="table",
                text="| a | b |\n| c | d |",
                cells=(("a", "b"), ("c", "d")),
                place=place(part="word/document.xml", ordinal=2),
            ),
            block(
                "b3",
                text="Scanned line",
                how_read="ocr",
                confidence=0.82,
                place=place(page=3, box=(1.0, 2.0, 30.0, 40.5), precision="approximate"),
            ),
            block("b4", kind="figure", text="[picture: image1.png]", picture="ab" * 32),
            carried=(NotCarried("a table", "its cells were not recoverable", 2), NotCarried("a header")),
        )

        again = Extraction.from_json(whole.to_json())

        self.assertEqual(again, whole)
        self.assertEqual(again.revision(), whole.revision())

    def test_the_revision_is_the_same_for_the_same_content_and_moves_with_any_word_of_it(self):
        one = extraction(block(text="The lamp is on."))
        same = extraction(block(text="The lamp is on."))
        other = extraction(block(text="The lamp is off."))

        self.assertEqual(one.revision(), same.revision())
        self.assertNotEqual(one.revision(), other.revision())
        # What was not carried is part of what the extraction is.
        self.assertNotEqual(one.revision(), extraction(block(), carried=(NotCarried("a table"),)).revision())

    def test_the_wire_form_says_its_version_and_is_text_in_a_fixed_order(self):
        text = extraction(block()).to_json()

        self.assertEqual(json.loads(text)["v"], SCHEMA_VERSION)
        self.assertEqual(text, extraction(block()).to_json())
        self.assertNotIn(" ", text.replace("The lamp is on.", ""))


class WhatWasNotCarriedIsPartOfTheAnswer(unittest.TestCase):
    def test_an_extraction_that_left_something_behind_is_not_clean(self):
        self.assertTrue(extraction(block()).clean)
        self.assertFalse(extraction(block(), carried=(NotCarried("a picture"),)).clean)

    def test_it_is_listed_with_its_reason_and_count_after_the_round_trip(self):
        whole = extraction(block(), carried=(NotCarried("pictures", "no model is called", 4),))

        again = Extraction.from_json(whole.to_json())

        self.assertEqual(again.not_carried, (NotCarried("pictures", "no model is called", 4),))

    def test_an_entry_says_what_it_is_about(self):
        with self.assertRaises(IngestError):
            NotCarried("")
        with self.assertRaises(IngestError):
            NotCarried("pictures", count=-1)


class ReadingByRecognitionIsNeverExact(unittest.TestCase):
    def test_a_block_read_from_an_image_must_say_how_sure_it_is(self):
        for how in ("ocr", "vision"):
            with self.subTest(how=how), self.assertRaises(IngestError):
                block(how_read=how)
            self.assertEqual(block(how_read=how, confidence=0.5).confidence, 0.5)

    def test_a_block_read_from_the_documents_own_text_carries_no_confidence(self):
        with self.assertRaises(IngestError):
            block(how_read="text-layer", confidence=1.0)
        self.assertIsNone(block().confidence)

    def test_a_confidence_is_between_nothing_and_certain(self):
        for bad in (-0.01, 1.01):
            with self.subTest(bad=bad), self.assertRaises(IngestError):
                block(how_read="ocr", confidence=bad)
        block(how_read="ocr", confidence=0.0)
        block(how_read="ocr", confidence=1.0)


class APlaceSaysHowExactItIs(unittest.TestCase):
    def test_a_box_says_how_exact_it_is_and_a_claim_of_a_box_needs_one(self):
        with self.assertRaises(IngestError):
            place(page=1, box=(0.0, 0.0, 1.0, 1.0))
        with self.assertRaises(IngestError):
            place(page=1, precision="exact")
        self.assertEqual(place(page=1, box=(0.0, 0.0, 1.0, 1.0), precision="exact").precision, "exact")

    def test_the_page_is_enough_and_the_box_is_not_required(self):
        # ADR 0035: a reader chosen for how well it reads may give no box.
        self.assertIsNone(place(page=2).box)

    def test_a_precision_nobody_defined_and_a_page_before_the_first_are_refused(self):
        with self.assertRaises(IngestError):
            place(page=1, box=(0.0, 0.0, 1.0, 1.0), precision="roughly")
        with self.assertRaises(IngestError):
            place(page=0)

    def test_a_place_names_its_member(self):
        with self.assertRaises(IngestError):
            Place(member="")


class ABlockIsOfTheKindItClaims(unittest.TestCase):
    def test_a_kind_nobody_defined_is_refused(self):
        with self.assertRaises(IngestError):
            block(kind="sidebar")

    def test_only_a_heading_has_a_level_only_a_table_a_grid_and_only_a_figure_a_picture(self):
        with self.assertRaises(IngestError):
            block(kind="paragraph", level=1)
        with self.assertRaises(IngestError):
            block(kind="paragraph", cells=(("a",),))
        with self.assertRaises(IngestError):
            block(kind="paragraph", picture="ab" * 32)
        block(kind="heading", level=1)

    def test_every_row_of_a_table_is_as_wide_as_the_table(self):
        with self.assertRaises(IngestError):
            block(kind="table", cells=(("a", "b"), ("c",)))
        block(kind="table", cells=(("a", "b"), ("c", "d")))

    def test_two_blocks_do_not_share_an_id(self):
        with self.assertRaises(IngestError):
            extraction(block("b1"), block("b1"))

    def test_an_extraction_says_who_made_it_and_of_what(self):
        with self.assertRaises(IngestError):
            Extraction("", "1", "spec.docx", ())
        with self.assertRaises(IngestError):
            Extraction("docx", "", "spec.docx", ())
        with self.assertRaises(IngestError):
            Extraction("docx", "1", "", ())


class AFormFromAnotherToolIsNeverHalfRead(unittest.TestCase):
    def read(self, mutate) -> Extraction:
        wire = json.loads(extraction(block()).to_json())
        mutate(wire)
        return Extraction.from_json(json.dumps(wire))

    def test_a_field_this_tool_does_not_know_refuses_the_whole(self):
        for where, mutate in {
            "extraction": lambda w: w.update(extra=1),
            "block": lambda w: w["blocks"][0].update(extra=1),
            "place": lambda w: w["blocks"][0]["place"].update(extra=1),
        }.items():
            with self.subTest(where=where), self.assertRaisesRegex(IngestError, "extra"):
                self.read(mutate)

    def test_another_version_is_refused_and_says_which(self):
        with self.assertRaisesRegex(IngestError, "version 99"):
            self.read(lambda w: w.update(v=99))

    def test_what_is_not_json_or_not_an_object_is_refused(self):
        for text in ("not json", "[]", "7", "null"):
            with self.subTest(text=text), self.assertRaises(IngestError):
                Extraction.from_json(text)

    def test_a_field_of_the_wrong_type_is_refused(self):
        for mutate in (
            lambda w: w["blocks"][0].update(text=5),
            lambda w: w["blocks"][0]["place"].update(ordinal="1"),
            lambda w: w["blocks"][0]["place"].update(ordinal=True),
            lambda w: w["blocks"][0].update(confidence="high"),
            lambda w: w.update(blocks="none"),
            lambda w: w.update(not_carried="none"),
            lambda w: w["blocks"][0]["place"].update(box=[1, 2, 3], precision="exact"),
        ):
            with self.subTest(), self.assertRaises(IngestError):
                self.read(mutate)

    def test_the_rules_hold_on_the_way_in_as_on_the_way_out(self):
        # A recognised block written as exact is refused when it is read, not only when it is made.
        with self.assertRaises(IngestError):
            self.read(lambda w: w["blocks"][0].update(how_read="ocr"))


if __name__ == "__main__":
    unittest.main()
