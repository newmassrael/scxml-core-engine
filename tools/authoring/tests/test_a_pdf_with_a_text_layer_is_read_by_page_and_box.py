"""A PDF that carries its own text, read into blocks placed by page and box.

ADR 0035 reads a PDF with a text layer by a library and no model. The PDFs here are written byte by byte
by `build_pdf` (one base font, text at positions that are asserted back), so the expected answer is the
fixture's own and not something the extractor's output was copied from.

The library is a pack (`pypdfium2`). Where it is installed the whole file runs; where it is not, the
cases that need it are skipped and the registry's answer is held instead, and a lane that sets
`SCE_REQUIRE_PDF_PACK` fails when the pack is missing, because a skip reads as a pass.
"""

from __future__ import annotations

import os
import pathlib
import pkgutil
import tempfile
import unittest
from unittest import mock

from sce_author import extractors_pdf
from sce_author.errors import IngestError
from sce_author.extractors import PACK_MISSING, Unreadable, builtin

PACK_PRESENT = any(found.name == "pypdfium2" for found in pkgutil.iter_modules())
needs_pack = unittest.skipUnless(PACK_PRESENT, "the pack pypdfium2 is not installed")


def build_pdf(pages: list[dict]) -> bytes:
    """A PDF of `pages`, each `{"lines": [(x, y, size, text)], "images": n, "size": (w, h)}`."""
    objects: list[bytes] = []

    def add(body: bytes) -> int:
        objects.append(body)
        return len(objects)

    add(b"")  # 1: the catalog, written below
    add(b"")  # 2: the page tree
    font = add(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
    image = add(
        b"<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray "
        b"/BitsPerComponent 8 /Length 1 >>\nstream\n\x00\nendstream"
    )
    kids = []
    for page in pages:
        width, height = page.get("size", (612, 792))
        content = "".join(
            f"BT /F1 {size} Tf {x} {y} Td ({text}) Tj ET\n" for x, y, size, text in page.get("lines", [])
        )
        content += "".join(f"q 40 0 0 40 {100 + 60 * i} 300 cm /Im1 Do Q\n" for i in range(page.get("images", 0)))
        data = content.encode("latin-1")
        stream = add(b"<< /Length %d >>\nstream\n" % len(data) + data + b"endstream")
        kids.append(
            add(
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %d %d] "
                b"/Resources << /Font << /F1 %d 0 R >> /XObject << /Im1 %d 0 R >> >> /Contents %d 0 R >>"
                % (width, height, font, image, stream)
            )
        )
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>"
    objects[1] = b"<< /Type /Pages /Kids [" + b" ".join(b"%d 0 R" % k for k in kids) + b"] /Count %d >>" % len(kids)
    out = b"%PDF-1.4\n"
    offsets = []
    for number, body in enumerate(objects, 1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % number + body + b"\nendobj\n"
    table = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1)
    out += b"".join(b"%010d 00000 n \n" % offset for offset in offsets)
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objects) + 1, table)
    return out


LAMP = [
    (72, 720, 18, "Lamp controller"),
    (72, 690, 11, "The lamp is on while the supply is present."),
    (72, 676, 11, "It turns off when the supply is lost."),
    (72, 640, 11, "- first item"),
    (72, 626, 11, "- second item"),
]


class Folder(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.root = pathlib.Path(self._temp.name)

    def pdf(self, pages: list[dict], name: str = "spec.pdf") -> pathlib.Path:
        path = self.root / name
        path.write_bytes(build_pdf(pages))
        return path


class ThePackIsThereWhereItIsRequired(unittest.TestCase):
    def test_a_lane_that_requires_the_pack_does_not_pass_without_it(self):
        if os.environ.get("SCE_REQUIRE_PDF_PACK"):
            self.assertTrue(PACK_PRESENT, "SCE_REQUIRE_PDF_PACK is set and pypdfium2 is not installed")

    def test_the_registry_reads_a_pdf_with_it_and_names_it_without_it(self):
        with tempfile.TemporaryDirectory() as folder:
            path = pathlib.Path(folder) / "spec.pdf"
            path.write_bytes(build_pdf([{"lines": LAMP}]))

            chosen = builtin().choose(path)

        if PACK_PRESENT:
            self.assertEqual(chosen.declaration.name, "pdf-text")
        else:
            self.assertIsInstance(chosen, Unreadable)
            self.assertEqual(chosen.code, PACK_MISSING)
            self.assertEqual(chosen.pack, "pdf-text")

    def test_the_declaration_is_what_the_registry_reads(self):
        declaration = extractors_pdf.DECLARATION

        self.assertEqual(declaration.suffixes, (".pdf",))
        self.assertEqual(declaration.unit, "page")
        self.assertEqual(declaration.needs, ("pypdfium2",))
        self.assertEqual(declaration.pack, "pdf-text")


@needs_pack
class LinesBecomeBlocksByTheirOwnGeometry(Folder):
    def test_a_heading_a_paragraph_and_list_items(self):
        done = builtin().extract(self.pdf([{"lines": LAMP}]))

        self.assertEqual([b.kind for b in done.blocks], ["heading", "paragraph", "list_item", "list_item"])
        self.assertEqual(done.blocks[0].text, "Lamp controller")
        self.assertEqual(done.blocks[0].level, 1)
        # Two lines that follow each other closely at one margin are one paragraph.
        self.assertEqual(done.blocks[1].text, "The lamp is on while the supply is present. It turns off when the supply is lost.")
        self.assertEqual([b.text for b in done.blocks[2:]], ["- first item", "- second item"])

    def test_text_is_the_documents_own_so_it_is_exact_and_has_no_confidence(self):
        done = builtin().extract(self.pdf([{"lines": LAMP}]))

        for block in done.blocks:
            self.assertEqual(block.how_read, "text-layer")
            self.assertIsNone(block.confidence)
            self.assertEqual(block.place.precision, "exact")

    def test_a_box_is_measured_down_from_the_top_left_of_the_page(self):
        done = builtin().extract(self.pdf([{"lines": LAMP}]))

        heading, paragraph = done.blocks[0], done.blocks[1]
        # The heading was set at y=720 of 792 from the bottom: it is near the top, so its top edge is small.
        self.assertLess(heading.place.box[1], 100)
        self.assertLess(heading.place.box[3], paragraph.place.box[1])
        for block in done.blocks:
            left, top, right, bottom = block.place.box
            self.assertTrue(0 <= left < right <= 612 and 0 <= top < bottom <= 792, block.place.box)
            self.assertGreaterEqual(left, 71)

    def test_the_same_size_everywhere_makes_no_heading(self):
        done = builtin().extract(self.pdf([{"lines": [(72, 700, 11, "One."), (72, 640, 11, "Two.")]}]))

        self.assertEqual([b.kind for b in done.blocks], ["paragraph", "paragraph"])

    def test_larger_sizes_are_levels_in_order(self):
        lines = [
            (72, 740, 24, "Title"),
            (72, 700, 16, "Section"),
            (72, 660, 11, "Body of the section."),
        ]

        done = builtin().extract(self.pdf([{"lines": lines}]))

        self.assertEqual([(b.kind, b.level) for b in done.blocks], [("heading", 1), ("heading", 2), ("paragraph", None)])

    def test_a_list_item_holds_the_line_that_wraps_under_it(self):
        lines = [(72, 700, 11, "- an item that is long"), (84, 686, 11, "and wraps here"), (72, 672, 11, "- next")]

        done = builtin().extract(self.pdf([{"lines": lines}]))

        self.assertEqual([b.text for b in done.blocks], ["- an item that is long and wraps here", "- next"])

    def test_what_is_read_converts_by_rule_and_the_check_finds_it_complete(self):
        # The whole route for a PDF that carries its text, with no model: read, convert, check.
        from sce_author.conversion import check_conversion, convert_by_rules

        done = builtin().extract(self.pdf([{"lines": LAMP}]))

        made = convert_by_rules(done)

        self.assertTrue(check_conversion(done, made).complete)
        self.assertEqual(
            made.markdown().splitlines()[0:3],
            ["# Lamp controller", "", "The lamp is on while the supply is present. It turns off when the supply is lost."],
        )

    def test_the_extraction_is_the_same_each_time_it_is_made(self):
        path = self.pdf([{"lines": LAMP}])

        self.assertEqual(builtin().extract(path).revision(), builtin().extract(path).revision())

    def test_its_text_has_every_line_in_order(self):
        done = builtin().extract(self.pdf([{"lines": LAMP}]))

        text = done.text()
        positions = [text.index(word) for word in ("Lamp controller", "supply is lost", "first item", "second item")]
        self.assertEqual(positions, sorted(positions))


# Helvetica's advance widths, in thousandths of the type size (its published metrics): a producer that
# places each glyph puts the next one this far on, which is what makes the gaps between letters real ones.
_HELVETICA = {
    " ": 278, ".": 278, ",": 278, "-": 333, "'": 191, ";": 278, ":": 278, "(": 333, ")": 333,
    **{d: 556 for d in "0123456789"},
    "a": 556, "b": 556, "c": 500, "d": 556, "e": 556, "f": 278, "g": 556, "h": 556, "i": 222, "j": 222,
    "k": 500, "l": 222, "m": 833, "n": 556, "o": 556, "p": 556, "q": 556, "r": 333, "s": 500, "t": 278,
    "u": 556, "v": 500, "w": 722, "x": 500, "y": 500, "z": 500,
    "A": 667, "B": 667, "C": 722, "D": 722, "E": 667, "F": 611, "G": 778, "H": 722, "I": 278, "J": 500,
    "K": 667, "L": 556, "M": 833, "N": 722, "O": 778, "P": 667, "Q": 778, "R": 722, "S": 667, "T": 611,
    "U": 722, "V": 667, "W": 944, "X": 667, "Y": 667, "Z": 611,
}


def glyphs(x: float, y: float, size: float, text: str) -> list[tuple[float, float, float, str]]:
    """`text` drawn as a producer that sets every glyph on its own draws it: one text object per character,
    each at its own position, the next as far on as the glyph is wide. A browser's print-to-PDF does this, and
    the library reports a rectangle for each piece, so a line arrives as dozens of them."""
    out = []
    for character in text:
        if character != " ":
            out.append((round(x, 2), y, size, character))
        x += size * _HELVETICA[character] / 1000
    return out


@needs_pack
class ACharacterIsOnALineByWhereItIsSet(unittest.TestCase):
    """Measured on a browser's PDF of a real specification: every character of the line `... SCXML, or the ...`
    is set at one baseline, 270.2, and the comma's box (268.7 to 271.3) reaches only 1.2 points into the
    letters' (270.1 to 276.2), which by overlap of boxes is not the same line."""

    def line(self, **kwargs):
        from sce_author.extractors_pdf import _Run

        return _Run(
            left=158.2, bottom=270.1, right=182.9, top=276.2, size=16.0, baseline=270.2, text=list("SCXML"), **kwargs
        )

    def test_the_comma_set_on_the_baseline_continues_the_line_though_its_box_hangs_below_it(self):
        self.assertTrue(self.line().continued_by(left=183.5, baseline=270.2, size=16.0))

    def test_a_character_of_the_next_line_does_not(self):
        self.assertFalse(self.line().continued_by(left=183.5, baseline=270.2 - 18.0, size=16.0))

    def test_a_raised_glyph_a_part_of_a_size_off_does_and_one_a_whole_size_off_does_not(self):
        self.assertTrue(self.line().continued_by(left=183.5, baseline=270.2 + 4.0, size=11.0))
        self.assertFalse(self.line().continued_by(left=183.5, baseline=270.2 + 16.0, size=16.0))

    def test_a_column_across_a_gutter_and_a_character_back_over_the_line_do_not(self):
        self.assertFalse(self.line().continued_by(left=182.9 + 5 * 16.0, baseline=270.2, size=16.0))
        self.assertFalse(self.line().continued_by(left=100.0, baseline=270.2, size=16.0))


@needs_pack
class ALineDrawnAGlyphAtATimeIsOneLine(Folder):
    """A rectangle of the library is a piece of drawing and not a line: pieces that sit on one baseline with
    no more than a word's space between them are the same line, and a gap wider than that is not."""

    def test_a_line_set_glyph_by_glyph_is_read_as_the_line_it_is(self):
        lines = glyphs(72, 720, 18, "Lamp controller") + glyphs(72, 690, 11, "The lamp is on while the supply is present.")

        done = builtin().extract(self.pdf([{"lines": lines}]))

        self.assertEqual([(b.kind, b.text) for b in done.blocks], [
            ("heading", "Lamp controller"),
            ("paragraph", "The lamp is on while the supply is present."),
        ])

    def test_the_lines_of_a_paragraph_set_that_way_are_still_one_paragraph(self):
        lines = glyphs(72, 700, 11, "The lamp is on while the supply is present.") + glyphs(72, 686, 11, "It turns off when the supply is lost.")

        done = builtin().extract(self.pdf([{"lines": lines}]))

        self.assertEqual([b.text for b in done.blocks], ["The lamp is on while the supply is present. It turns off when the supply is lost."])

    def test_it_is_the_same_reading_as_the_line_set_whole(self):
        whole = builtin().extract(self.pdf([{"lines": LAMP}], "whole.pdf"))
        pieces = [piece for x, y, size, text in LAMP for piece in glyphs(x, y, size, text)]

        glyphwise = builtin().extract(self.pdf([{"lines": pieces}], "glyphs.pdf"))

        self.assertEqual([(b.kind, b.level, b.text) for b in glyphwise.blocks], [(b.kind, b.level, b.text) for b in whole.blocks])

    def test_two_columns_on_one_row_are_not_joined_across_the_gap_between_them(self):
        # The extraction says it does not untangle columns; reading across one would be worse than that.
        lines = glyphs(72, 700, 11, "Left column text.") + glyphs(340, 700, 11, "Right column text.")

        done = builtin().extract(self.pdf([{"lines": lines}]))

        self.assertEqual(sorted(b.text for b in done.blocks), ["Left column text.", "Right column text."])

    def test_a_comma_that_hangs_below_the_baseline_is_on_the_line_it_ends_a_word_of(self):
        # A glyph is on a line by where it is set (its baseline), not by how much of its box overlaps the
        # others: a comma's box lies below the line, and a full stop's sits on it.
        lines = glyphs(72, 700, 11, "The lamp, which is on, turns off; it is fine.")

        done = builtin().extract(self.pdf([{"lines": lines}]))

        self.assertEqual([b.text for b in done.blocks], ["The lamp, which is on, turns off; it is fine."])

    def test_a_raised_or_lowered_glyph_is_on_the_line_and_a_line_below_is_not(self):
        raised = glyphs(72, 700, 11, "Water is H") + glyphs(126, 703, 7, "2") + glyphs(130, 700, 11, "O.")
        below = glyphs(72, 686, 11, "Next line.")

        done = builtin().extract(self.pdf([{"lines": raised + below}]))

        # The raised glyph is on the first line, not a line of its own, and the line under it is the same
        # paragraph as it always was: one block that says both, in order.
        self.assertEqual(len(done.blocks), 1)
        text = done.blocks[0].text
        self.assertTrue(text.startswith("Water is H"), text)
        self.assertLess(text.index("2"), text.index("O."))
        self.assertTrue(text.endswith("Next line."), text)

    def test_a_box_covers_the_whole_line_and_not_its_first_glyph(self):
        done = builtin().extract(self.pdf([{"lines": glyphs(72, 700, 11, "The lamp is on.")}]))

        left, _, right, _ = done.blocks[0].place.box
        self.assertGreater(right - left, 50)

    def test_a_document_of_glyphs_does_not_make_a_block_of_each(self):
        lines = [piece for row in range(20) for piece in glyphs(72, 740 - 14 * row, 11, "Row of text that is a line.")]

        done = builtin().extract(self.pdf([{"lines": lines}]))

        self.assertEqual(len(done.blocks), 1)


@needs_pack
class AClauseNumberMakesATitleAHeadingWhenTheDocumentSetItNoLarger(Folder):
    """A specification marks its sections by number, and a producer that sets every title in the body's size
    leaves nothing else to tell a title by. The rule is narrow, and each way it must not fire is held."""

    def kinds(self, lines):
        done = builtin().extract(self.pdf([{"lines": lines}]))
        return [(b.kind, b.level, b.text) for b in done.blocks]

    def test_a_dotted_number_and_a_short_title_alone_on_the_line_is_a_heading_under_the_top_level(self):
        lines = [
            (72, 740, 18, "3 Lamp"),
            (72, 690, 11, "3.1 Timing"),
            (72, 640, 11, "It turns off after thirty seconds."),
            (72, 590, 11, "3.1.1 Delay"),
            (72, 540, 11, "The delay is five seconds."),
        ]

        self.assertEqual(self.kinds(lines), [
            ("heading", 1, "3 Lamp"),
            ("heading", 2, "3.1 Timing"),
            ("paragraph", None, "It turns off after thirty seconds."),
            ("heading", 3, "3.1.1 Delay"),
            ("paragraph", None, "The delay is five seconds."),
        ])

    def test_the_level_follows_where_the_document_set_its_own_top_level_numbers(self):
        # The top-level titles are set larger and are the second size here (a title above them is the first):
        # a number of two parts is one level below those, whatever level that is.
        lines = [
            (72, 760, 24, "Lamp specification"),
            (72, 720, 16, "3 Lamp"),
            (72, 670, 11, "3.1 Timing"),
            (72, 620, 11, "It turns off."),
        ]

        self.assertEqual([(k, lv) for k, lv, _ in self.kinds(lines)], [
            ("heading", 1), ("heading", 2), ("heading", 3), ("paragraph", None),
        ])

    def test_a_line_that_is_a_sentence_a_list_item_or_a_single_number_is_not_one(self):
        lines = [
            (72, 740, 11, "3.1 The lamp turns off after thirty seconds."),
            (72, 690, 11, "1. first item"),
            (72, 640, 11, "2 Timing"),
            (72, 590, 11, "3.2 Notes:"),
            (72, 540, 11, "3.3" + " very long title" * 8),
            (72, 490, 11, "3.4 timing"),
        ]

        got = self.kinds(lines)

        self.assertEqual([k for k, _, _ in got], ["paragraph", "list_item", "paragraph", "paragraph", "paragraph", "heading"])
        # The nearest that must fire: the same shape, with a title that begins a word and does not end a sentence.
        self.assertEqual(got[-1][2], "3.4 timing")

    def test_a_numbered_line_that_has_lines_under_it_in_one_paragraph_is_not_a_title_alone(self):
        lines = [(72, 700, 11, "3.1 Timing"), (72, 688, 11, "continues here as the paragraph does")]

        self.assertEqual([k for k, _, _ in self.kinds(lines)], ["paragraph"])


@needs_pack
class PagesArePlacedAndAPageWithNoTextIsSaid(Folder):
    def test_each_block_is_on_its_page_and_ordinals_run_on(self):
        pages = [{"lines": [(72, 700, 11, "First page.")]}, {"lines": [(72, 700, 11, "Second page.")]}]

        done = builtin().extract(self.pdf(pages))

        self.assertEqual([(b.place.page, b.place.ordinal) for b in done.blocks], [(1, 1), (2, 2)])
        self.assertEqual([b.id for b in done.blocks], ["b0001", "b0002"])
        self.assertEqual(done.member, "spec.pdf")
        self.assertEqual(done.extractor, "pdf-text")

    def test_a_page_with_no_text_is_listed_by_number_and_is_not_an_empty_page_that_says_nothing(self):
        pages = [{"lines": [(72, 700, 11, "Text.")]}, {}, {}]

        done = builtin().extract(self.pdf(pages))

        entry = next(n for n in done.not_carried if "carry no text" in n.what)
        self.assertEqual(entry.count, 2)
        self.assertIn("2 of 3 page(s)", entry.what)
        self.assertIn("2, 3", entry.what)
        self.assertFalse(done.clean)

    def test_a_document_of_pages_that_are_all_pictures_is_all_of_it_unread(self):
        done = builtin().extract(self.pdf([{}, {}]))

        self.assertEqual(done.blocks, ())
        self.assertEqual([n.count for n in done.not_carried], [2])
        self.assertFalse(done.clean)

    def test_a_long_run_of_pages_is_listed_to_a_limit_and_counted_in_full(self):
        done = builtin().extract(self.pdf([{} for _ in range(20)]))

        entry = done.not_carried[0]
        self.assertEqual(entry.count, 20)
        self.assertTrue(entry.what.endswith("..."))
        self.assertLess(len(entry.what), 120)


@needs_pack
class WhatIsNotCarriedIsAlwaysSaid(Folder):
    def test_a_picture_is_counted_with_its_pages_and_not_placed(self):
        pages = [{"lines": [(72, 700, 11, "See below.")], "images": 2}, {"lines": [(72, 700, 11, "Next.")]}]

        done = builtin().extract(self.pdf(pages))

        entry = next(n for n in done.not_carried if "picture" in n.what)
        self.assertEqual(entry.count, 2)
        self.assertIn("page(s) 1", entry.what)
        self.assertEqual([b.kind for b in done.blocks], ["paragraph", "paragraph"])

    def test_the_layout_is_never_claimed_to_be_recovered(self):
        # A table comes out as lines. The extraction says so for every document with text, whether or
        # not this one has a table: it cannot tell, and a clean extraction would say it could.
        done = builtin().extract(self.pdf([{"lines": LAMP}]))

        self.assertFalse(done.clean)
        layout = [n for n in done.not_carried if "layout" in n.what]
        self.assertEqual(len(layout), 1)
        self.assertIn("table", layout[0].why)

    def test_a_table_read_as_lines_is_one_block_of_its_lines_which_is_what_is_said(self):
        lines = [(72, 700, 11, "state    lamp"), (72, 686, 11, "on       lit")]

        done = builtin().extract(self.pdf([{"lines": lines}]))

        self.assertEqual(len(done.blocks), 1)
        self.assertNotIn("table", [b.kind for b in done.blocks])
        self.assertTrue(any("tables" in n.what for n in done.not_carried))


@needs_pack
class WhatCannotBeReadIsRefusedWithItsNumbers(Folder):
    def test_a_file_that_begins_as_a_pdf_and_is_not_one_is_refused_naming_the_file(self):
        path = self.root / "broken.pdf"
        path.write_bytes(b"%PDF-1.4\nthis is not the rest of a pdf")

        with self.assertRaisesRegex(IngestError, "broken.pdf.*not a readable PDF"):
            builtin().extract(path)

    def test_a_file_past_the_size_limit_is_refused_and_not_cut(self):
        path = self.pdf([{"lines": LAMP}])

        with mock.patch.object(extractors_pdf, "MAX_BYTES", 100), self.assertRaisesRegex(
            IngestError, "past the 100.*not cut"
        ):
            builtin().extract(path)

    def test_more_pages_than_the_limit_is_refused_and_not_cut(self):
        path = self.pdf([{"lines": LAMP}, {"lines": LAMP}, {"lines": LAMP}])

        with mock.patch.object(extractors_pdf, "MAX_PAGES", 2), self.assertRaisesRegex(
            IngestError, "3 pages is past the 2"
        ):
            builtin().extract(path)

    def test_a_page_larger_than_any_sheet_is_refused_naming_it(self):
        path = self.pdf([{"lines": LAMP}, {"lines": [(72, 700, 11, "x")], "size": (20000, 20000)}])

        with self.assertRaisesRegex(IngestError, "page 2 is 20000 by 20000"):
            builtin().extract(path)

    def test_the_largest_page_that_is_allowed_is_read(self):
        path = self.pdf([{"lines": [(72, 700, 11, "Large.")], "size": (14400, 14400)}])

        self.assertEqual(builtin().extract(path).blocks[0].text, "Large.")

    def test_a_pdf_called_by_another_name_is_found_by_its_signature(self):
        path = self.pdf([{"lines": LAMP}], name="spec.dat")

        self.assertEqual(builtin().extract(path).extractor, "pdf-text")


if __name__ == "__main__":
    unittest.main()
