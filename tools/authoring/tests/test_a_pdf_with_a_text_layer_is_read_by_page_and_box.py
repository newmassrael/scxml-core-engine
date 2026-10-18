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
