"""SCE's check of a conversion: block by block, against the extraction it cites.

ADR 0026 holds a conversion to its original and does not take its maker's word for how much of the
original it carries. Each case damages a conversion the way a maker damages one (a word one syllable
away, a block left out, a block invented, a block that cites something that is not there) and asserts
the check sees it, and pairs it with the nearest conversion that must pass: a check that refuses
everything passes the first half and is worthless, and one that accepts everything passes the second.
"""

from __future__ import annotations

import json
import unittest

from sce_author.conversion import (
    CHECK,
    MATCHED,
    NOT_IN_ORIGINAL,
    Conversion,
    Converter,
    MarkdownBlock,
    check_conversion,
    convert_by_rules,
    plain_of_markdown,
)
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place


def block(identifier: str, kind: str = "paragraph", text: str = "", **kwargs) -> Block:
    kwargs.setdefault("place", Place("spec.md", ordinal=int(identifier[1:])))
    return Block(identifier, kind, text, **kwargs)


def original() -> Extraction:
    return Extraction(
        "plain",
        "1",
        "spec.md",
        (
            block("b1", "heading", "Lamp controller", level=1),
            block("b2", text="The lamp is on while the supply is present."),
            block("b3", "list_item", "- turns off when the supply is lost"),
            block("b4", "table", "| state | lamp |\n| on | lit |", cells=(("state", "lamp"), ("on", "lit"))),
        ),
    )


def conversion(extraction: Extraction, *blocks: MarkdownBlock) -> Conversion:
    return Conversion(Converter("model", "a-model", "1"), extraction.revision(), tuple(blocks))


FAITHFUL = (
    MarkdownBlock("m1", "# Lamp controller", ("b1",)),
    MarkdownBlock("m2", "The lamp is on while the supply is present.", ("b2",)),
    MarkdownBlock("m3", "- turns off when the supply is lost", ("b3",)),
    MarkdownBlock("m4", "| state | lamp |\n| --- | --- |\n| on | lit |", ("b4",)),
)


class AFaithfulConversionIsFoundComplete(unittest.TestCase):
    def test_every_block_matched_and_nothing_left_out(self):
        extraction = original()

        report = check_conversion(extraction, conversion(extraction, *FAITHFUL))

        self.assertTrue(report.complete)
        self.assertEqual(report.matched, 4)
        self.assertEqual(report.missing, ())
        self.assertEqual(report.queue(), [])

    def test_markup_is_not_what_is_compared_only_the_words(self):
        extraction = original()
        dressed = (
            MarkdownBlock("m1", "## Lamp controller", ("b1",)),  # another level of heading
            MarkdownBlock("m2", "The  lamp is on\nwhile the supply is present.", ("b2",)),  # wrapped, spaced
            MarkdownBlock("m3", "* turns off when the supply is lost", ("b3",)),  # another bullet
            MarkdownBlock("m4", "|state|lamp|\n|:--|--:|\n|on|lit|", ("b4",)),  # a terse table
        )

        self.assertTrue(check_conversion(extraction, conversion(extraction, *dressed)).complete)

    def test_a_block_may_cite_several_blocks_and_is_compared_with_them_in_the_order_it_cites(self):
        extraction = original()
        joined = conversion(
            extraction,
            MarkdownBlock("m1", "# Lamp controller", ("b1",)),
            MarkdownBlock(
                "m2",
                "The lamp is on while the supply is present. turns off when the supply is lost",
                ("b2", "b3"),
            ),
            MarkdownBlock("m3", "| state | lamp |\n| --- | --- |\n| on | lit |", ("b4",)),
        )
        reversed_order = conversion(
            extraction,
            MarkdownBlock("m1", "# Lamp controller", ("b1",)),
            MarkdownBlock(
                "m2",
                "The lamp is on while the supply is present. turns off when the supply is lost",
                ("b3", "b2"),
            ),
            MarkdownBlock("m3", "| state | lamp |\n| --- | --- |\n| on | lit |", ("b4",)),
        )

        self.assertTrue(check_conversion(extraction, joined).complete)
        self.assertEqual(check_conversion(extraction, reversed_order).ids(CHECK), ["m2"])

    def test_a_pdf_bullet_is_the_same_item_as_a_markdown_one(self):
        extraction = Extraction("pdf-text", "1", "s.pdf", (block("b1", "list_item", "• first item"),))

        report = check_conversion(extraction, conversion(extraction, MarkdownBlock("m1", "- first item", ("b1",))))

        self.assertTrue(report.complete)

    def test_the_words_of_an_escaped_paragraph_are_the_words(self):
        extraction = Extraction("plain", "1", "s.md", (block("b1", text="- not a list, a sentence"),))

        report = check_conversion(
            extraction, conversion(extraction, MarkdownBlock("m1", "\\- not a list, a sentence", ("b1",)))
        )

        self.assertTrue(report.complete)


class ADamagedConversionIsSeen(unittest.TestCase):
    def damaged(self, index: int, markdown: str) -> Conversion:
        extraction = original()
        blocks = list(FAITHFUL)
        blocks[index] = MarkdownBlock(blocks[index].id, markdown, blocks[index].cites)
        return conversion(extraction, *blocks)

    def test_one_word_different_is_a_block_to_check_and_the_rest_still_match(self):
        extraction = original()

        report = check_conversion(extraction, self.damaged(1, "The lamp is off while the supply is present."))

        self.assertEqual(report.ids(CHECK), ["m2"])
        self.assertEqual(report.matched, 3)
        self.assertFalse(report.complete)

    def test_a_syllable_away_is_seen_the_failure_of_a_recogniser_that_replaces_a_character(self):
        # "close" for "closed": the recogniser's reading said it was transcribed exactly.
        extraction = Extraction("scan", "1", "s.pdf", (block("b1", text="The valve is closed."),))

        report = check_conversion(
            extraction, conversion(extraction, MarkdownBlock("m1", "The valve is close.", ("b1",)))
        )

        self.assertEqual(report.ids(CHECK), ["m1"])

    def test_a_look_alike_character_is_seen(self):
        extraction = Extraction("scan", "1", "s.pdf", (block("b1", text="Limit is 100."),))

        report = check_conversion(
            extraction, conversion(extraction, MarkdownBlock("m1", "Limit is 1O0.", ("b1",)))
        )

        self.assertEqual(report.ids(CHECK), ["m1"])

    def test_a_case_that_changed_is_a_difference(self):
        extraction = Extraction("plain", "1", "s.md", (block("b1", text="The lamp is ON."),))

        report = check_conversion(extraction, conversion(extraction, MarkdownBlock("m1", "The lamp is on.", ("b1",))))

        self.assertEqual(report.ids(CHECK), ["m1"])

    def test_a_cell_that_changed_in_a_table_is_seen(self):
        report = check_conversion(
            original(), self.damaged(3, "| state | lamp |\n| --- | --- |\n| on | dark |")
        )

        self.assertEqual(report.ids(CHECK), ["m4"])

    def test_a_table_that_lost_a_row_is_seen(self):
        report = check_conversion(original(), self.damaged(3, "| state | lamp |\n| --- | --- |"))

        self.assertEqual(report.ids(CHECK), ["m4"])

    def test_a_block_left_out_is_missing_and_named(self):
        extraction = original()

        report = check_conversion(extraction, conversion(extraction, *FAITHFUL[:2], FAITHFUL[3]))

        self.assertEqual(report.missing, ("b3",))
        self.assertFalse(report.complete)
        self.assertEqual(report.matched, 3)

    def test_a_block_that_cites_nothing_is_not_in_the_original(self):
        extraction = original()

        report = check_conversion(
            extraction, conversion(extraction, *FAITHFUL, MarkdownBlock("m5", "A sentence the maker added.", ()))
        )

        self.assertEqual(report.ids(NOT_IN_ORIGINAL), ["m5"])
        self.assertFalse(report.complete)
        self.assertEqual(report.missing, ())

    def test_a_block_that_cites_what_the_original_does_not_have_is_to_check_and_says_which(self):
        extraction = original()

        report = check_conversion(
            extraction, conversion(extraction, *FAITHFUL[:3], MarkdownBlock("m4", "x", ("b99",)))
        )

        verdict = next(v for v in report.verdicts if v.id == "m4")
        self.assertEqual(verdict.status, CHECK)
        self.assertIn("b99", verdict.why)
        # What it should have cited is then missing, and not counted as carried.
        self.assertEqual(report.missing, ("b4",))

    def test_citing_a_true_block_for_words_it_does_not_hold_is_seen(self):
        # The citation is real and does not support the words: the check reads the words.
        extraction = original()

        report = check_conversion(
            extraction,
            conversion(
                extraction,
                MarkdownBlock("m1", "# Lamp controller", ("b1",)),
                MarkdownBlock("m2", "The lamp is on while the supply is present.", ("b3",)),
                MarkdownBlock("m3", "- turns off when the supply is lost", ("b3",)),
                MarkdownBlock("m4", "| state | lamp |\n| --- | --- |\n| on | lit |", ("b4",)),
            ),
        )

        self.assertEqual(report.ids(CHECK), ["m2"])
        self.assertEqual(report.missing, ("b2",))


class ARecognisedBlockIsAskedAboutWhateverItsTextSays(unittest.TestCase):
    def test_a_block_read_from_an_image_is_queued_even_when_the_conversion_matches_it(self):
        extraction = Extraction(
            "scan",
            "1",
            "s.pdf",
            (
                block("b1", text="Read from text."),
                block("b2", text="Read from a picture.", how_read="ocr", confidence=0.93),
            ),
        )

        report = check_conversion(
            extraction,
            conversion(
                extraction,
                MarkdownBlock("m1", "Read from text.", ("b1",)),
                MarkdownBlock("m2", "Read from a picture.", ("b2",)),
            ),
        )

        self.assertTrue(report.complete)
        self.assertEqual(report.recognised, ("b2",))
        self.assertEqual(report.queue(), [("recognised", "b2")])

    def test_the_queue_asks_about_what_could_not_be_confirmed_before_what_was_recognised(self):
        extraction = Extraction(
            "scan",
            "1",
            "s.pdf",
            (
                block("b1", text="Alpha."),
                block("b2", text="Beta.", how_read="vision", confidence=0.5),
                block("b3", text="Gamma."),
            ),
        )

        report = check_conversion(
            extraction,
            conversion(
                extraction,
                MarkdownBlock("m1", "Alpha changed.", ("b1",)),
                MarkdownBlock("m2", "Beta.", ("b2",)),
                MarkdownBlock("m3", "Invented.", ()),
            ),
        )

        self.assertEqual(
            report.queue(),
            [("check", "m1"), ("recognised", "b2"), ("not-in-original", "m3"), ("missing", "b3")],
        )


class ACheckIsOfTheExtractionTheConversionNames(unittest.TestCase):
    def test_a_conversion_of_another_extraction_is_refused_and_counts_nothing(self):
        extraction = original()
        other = Extraction("plain", "1", "other.md", (block("b1", text="Different."),))

        with self.assertRaisesRegex(IngestError, "not of this one"):
            check_conversion(extraction, conversion(other, MarkdownBlock("m1", "Different.", ("b1",))))

    def test_there_is_nowhere_in_the_form_for_a_conversion_to_say_it_is_complete(self):
        extraction = original()
        wire = json.loads(conversion(extraction, *FAITHFUL).to_json())
        wire["complete"] = True

        with self.assertRaisesRegex(IngestError, "complete"):
            Conversion.from_dict(wire)

    def test_a_block_with_no_words_has_nothing_to_leave_out(self):
        extraction = Extraction(
            "plain", "1", "s.md", (block("b1", text="Words."), block("b2", "table", "|  |", cells=(("",),)))
        )

        report = check_conversion(extraction, conversion(extraction, MarkdownBlock("m1", "Words.", ("b1",))))

        self.assertTrue(report.complete)

    def test_the_report_says_its_counts_as_data(self):
        extraction = original()

        made = conversion(extraction, *FAITHFUL[:3])
        said = check_conversion(extraction, made).to_dict()

        self.assertEqual(said["extraction"], extraction.revision())
        self.assertEqual(said["conversion"], made.revision())
        self.assertEqual(said["complete"], False)
        self.assertEqual(said["matched"], 3)
        self.assertEqual(said["missing"], ["b4"])
        self.assertEqual(said["queue"], [{"why": "missing", "id": "b4"}])


class TheRulesConverterIsTheChecksControl(unittest.TestCase):
    def test_what_it_makes_matches_every_block(self):
        extraction = original()

        made = convert_by_rules(extraction)
        report = check_conversion(extraction, made)

        self.assertTrue(report.complete, report.to_dict())
        self.assertEqual(report.matched, len(extraction.blocks))
        self.assertEqual(made.converter.kind, "rules")

    def test_the_markdown_it_writes_is_the_one_a_reader_expects(self):
        made = convert_by_rules(original())

        self.assertEqual(
            made.markdown(),
            "# Lamp controller\n\n"
            "The lamp is on while the supply is present.\n\n"
            "- turns off when the supply is lost\n\n"
            "| state | lamp |\n| --- | --- |\n| on | lit |\n",
        )

    def test_a_paragraph_that_would_read_as_markup_is_escaped_and_still_matches(self):
        extraction = Extraction(
            "plain",
            "1",
            "s.md",
            tuple(
                block(f"b{i}", text=text)
                for i, text in enumerate(
                    ["# not a heading", "> not a quote", "1. not a list", "| not a table", "- not a bullet", "plain"],
                    start=1,
                )
            ),
        )

        made = convert_by_rules(extraction)

        self.assertTrue(check_conversion(extraction, made).complete)
        self.assertTrue(made.blocks[0].markdown.startswith("\\#"))
        self.assertEqual(made.blocks[5].markdown, "plain")

    def test_a_pipe_in_a_cell_stays_in_its_cell(self):
        extraction = Extraction(
            "plain",
            "1",
            "s.md",
            (block("b1", "table", "x", cells=(("a|b", "c"), ("d", "e"))),),
        )

        made = convert_by_rules(extraction)

        self.assertIn("a\\|b", made.blocks[0].markdown)
        self.assertTrue(check_conversion(extraction, made).complete)

    def test_a_row_of_dashes_that_is_a_value_or_a_rule_between_paragraphs_is_kept_as_words(self):
        extraction = Extraction(
            "plain",
            "1",
            "s.md",
            (
                block("b1", "table", "x", cells=(("state", "limit"), ("-", "-"), ("on", "5"))),
                block("b2", text="----"),
            ),
        )

        made = convert_by_rules(extraction)
        report = check_conversion(extraction, made)

        self.assertTrue(report.complete, report.to_dict())
        # Damage it: the dash row is a row of the table, and losing it is seen.
        lost = Conversion(
            made.converter,
            made.extraction,
            (
                MarkdownBlock("m1", "| state | limit |\n| --- | --- |\n| on | 5 |", ("b1",)),
                made.blocks[1],
            ),
        )
        self.assertEqual(check_conversion(extraction, lost).ids(CHECK), ["m1"])

    def test_a_merged_cell_repeated_in_each_column_it_covers_is_written_in_each(self):
        extraction = Extraction(
            "docx", "1", "s.docx", (block("b1", "table", "x", cells=(("Header", "Header"), ("left", "right"))),)
        )

        made = convert_by_rules(extraction)

        self.assertEqual(made.blocks[0].markdown.splitlines()[0], "| Header | Header |")
        self.assertTrue(check_conversion(extraction, made).complete)

    def test_a_block_with_no_words_is_not_written_and_not_missing(self):
        extraction = Extraction(
            "plain", "1", "s.md", (block("b1", text="Words."), block("b2", "table", "|  |", cells=(("",),)))
        )

        made = convert_by_rules(extraction)

        self.assertEqual(len(made.blocks), 1)
        self.assertTrue(check_conversion(extraction, made).complete)

    def test_a_document_with_no_words_in_it_has_nothing_to_convert_and_is_refused(self):
        for blocks in ((), (block("b1", "table", "|  |", cells=(("",),)),)):
            with self.subTest(blocks=len(blocks)):
                with self.assertRaisesRegex(IngestError, "nothing to convert"):
                    convert_by_rules(Extraction("plain", "1", "s.md", blocks))
        # The nearest document that has words is converted.
        self.assertEqual(len(convert_by_rules(Extraction("plain", "1", "s.md", (block("b1", text="Words."),))).blocks), 1)

    def test_a_table_it_cannot_write_from_its_cells_is_refused_and_not_guessed_at(self):
        extraction = Extraction("plain", "1", "s.md", (block("b1", "table", "| a |"),))

        with self.assertRaisesRegex(IngestError, "no grid of cells"):
            convert_by_rules(extraction)

    def test_figures_and_attachments_are_written_as_the_marks_they_are(self):
        extraction = Extraction(
            "docx",
            "1",
            "s.docx",
            (
                block("b1", "figure", "[picture: image1.png]"),
                block("b2", "attachment", "[enclosed object: sheet.xlsx (xlsx)]"),
            ),
        )

        made = convert_by_rules(extraction)

        self.assertEqual([b.markdown for b in made.blocks], ["[picture: image1.png]", "[enclosed object: sheet.xlsx (xlsx)]"])
        self.assertTrue(check_conversion(extraction, made).complete)

    def test_blocks_are_cited_one_for_one_in_order_and_named_the_same_each_time(self):
        extraction = original()

        first, second = convert_by_rules(extraction), convert_by_rules(extraction)

        self.assertEqual([b.cites for b in first.blocks], [("b1",), ("b2",), ("b3",), ("b4",)])
        self.assertEqual(first.revision(), second.revision())
        self.assertEqual(first.extraction, extraction.revision())


class APlainReadingOfMarkdownOnly(unittest.TestCase):
    def test_what_is_taken_off_and_what_is_kept(self):
        self.assertEqual(plain_of_markdown("# Title"), "Title")
        self.assertEqual(plain_of_markdown("  - item"), "item")
        self.assertEqual(plain_of_markdown("12) item"), "item")
        self.assertEqual(plain_of_markdown("| a | b |\n| --- | --- |\n| c | d |"), "a b c d")
        self.assertEqual(plain_of_markdown("a \\| b"), "a | b")
        # What looks like a marker and is not one is kept.
        self.assertEqual(plain_of_markdown("-5 degrees"), "-5 degrees")
        self.assertEqual(plain_of_markdown("3.5 volts"), "3.5 volts")
        self.assertEqual(plain_of_markdown("#hashtag"), "#hashtag")


if __name__ == "__main__":
    unittest.main()
