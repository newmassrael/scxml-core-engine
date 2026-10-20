"""An image a PDF page holds is a figure block, named by its page and place, and is drawn as PNG when read.

ADR 0036 reads a picture a figure block marks; ADR 0032 names a picture by the hash of the member that holds
it. A PDF holds an image object on a page and no file name, so the name is the page and the place among that
page's images, the hash is of the bytes the document stores for it, and what is handed to a client is a
rendition drawn from the decoded samples. The PDFs are written byte by byte by `build_pdf`, so the expected
pixels are the fixture's own, and the PNG is decoded here with the standard library and not by the encoder's
own code.

The library is a pack (`pypdfium2`); without it the cases are skipped and a lane that sets
`SCE_REQUIRE_PDF_PACK` fails (see the text-layer test, which holds that).
"""

from __future__ import annotations

import struct
import unittest
import zlib
from unittest import mock

from sce_author import extractors_pdf, pdf_pictures
from sce_author.extractors import builtin
from sce_author.ingest import PictureTooLarge, picture_names_held, read_pictures
from sce_author.reading import (
    OTHER_PICTURE,
    PICTURE_NOT_FOUND,
    PICTURE_NOT_SHOWABLE,
    PICTURE_TOO_LARGE,
    units_for,
)

from tests.test_a_pdf_with_a_text_layer_is_read_by_page_and_box import Folder, needs_pack

RED, GREEN, BLUE, WHITE = (255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 255)


def rgb(*pixels: tuple[int, int, int]) -> bytes:
    return b"".join(bytes(pixel) for pixel in pixels)


def picture(at: tuple[float, float, float, float], pixels: tuple[int, int], data: bytes, space: str = "DeviceRGB"):
    return {"at": at, "pixels": pixels, "space": space, "data": data}


# Two pictures on one page: a 2 by 2 colour picture and, below it, a 1 by 2 gray one.
COLOURS = picture((100, 600, 80, 80), (2, 2), rgb(RED, GREEN, BLUE, WHITE))
GRAYS = picture((100, 300, 40, 80), (1, 2), b"\x00\xff", "DeviceGray")


def decoded(png: bytes) -> tuple[int, int, int, list[bytes]]:
    """A PNG read with the standard library: width, height, colour type and its scan lines without filters."""
    assert png[:8] == b"\x89PNG\r\n\x1a\n"
    chunks, at = {}, 8
    while at < len(png):
        (length,) = struct.unpack(">I", png[at : at + 4])
        tag, body = png[at + 4 : at + 8], png[at + 8 : at + 8 + length]
        (crc,) = struct.unpack(">I", png[at + 8 + length : at + 12 + length])
        assert crc == zlib.crc32(tag + body), f"the checksum of {tag!r} is wrong"
        chunks[tag] = body
        at += 12 + length
    width, height, depth, colour = struct.unpack(">IIBB", chunks[b"IHDR"][:10])
    assert depth == 8
    samples = {0: 1, 2: 3, 6: 4}[colour]
    raw = zlib.decompress(chunks[b"IDAT"])
    stride = 1 + width * samples
    rows = []
    for y in range(height):
        assert raw[y * stride] == 0, "a filtered line is not what this encoder writes"
        rows.append(raw[y * stride + 1 : (y + 1) * stride])
    return width, height, colour, rows


@needs_pack
class AnImageIsAFigureAtItsBox(Folder):
    def extract(self, pages):
        return builtin().extract(self.pdf(pages))

    def test_each_image_is_a_figure_named_by_its_page_and_place_and_placed_by_its_box(self):
        done = self.extract([{"lines": [(72, 720, 11, "Above.")], "images": [COLOURS, GRAYS]}])

        figures = [b for b in done.blocks if b.kind == "figure"]
        self.assertEqual([b.text for b in figures], ["[picture: page-0001-image-01.png]", "[picture: page-0001-image-02.png]"])
        self.assertEqual([b.place.page for b in figures], [1, 1])
        # The page is 792 points high and the first image reaches from 600 to 680 up from the bottom.
        self.assertEqual(figures[0].place.box, (100.0, 112.0, 180.0, 192.0))
        self.assertEqual(figures[0].place.precision, "exact")
        self.assertEqual(figures[0].how_read, "text-layer")

    def test_a_figure_sits_among_the_paragraphs_by_where_it_is_on_the_page(self):
        lines = [(72, 720, 11, "First."), (72, 400, 11, "Second."), (72, 100, 11, "Third.")]
        # One image between the first two paragraphs and one under the last.
        between = picture((100, 500, 40, 40), (1, 1), b"\x10", "DeviceGray")
        under = picture((100, 40, 40, 40), (1, 1), b"\x20", "DeviceGray")

        done = self.extract([{"lines": lines, "images": [under, between]}])

        self.assertEqual(
            [b.text for b in done.blocks],
            ["First.", "[picture: page-0001-image-02.png]", "Second.", "Third.", "[picture: page-0001-image-01.png]"],
        )
        self.assertEqual([b.id for b in done.blocks], [f"b{n:04d}" for n in range(1, 6)])

    def test_a_page_that_is_only_an_image_is_a_figure_and_is_still_listed_as_carrying_no_text(self):
        done = self.extract([{"images": [picture((0, 0, 612, 792), (1, 1), b"\x80", "DeviceGray")]}])

        self.assertEqual([b.kind for b in done.blocks], ["figure"])
        entry = next(n for n in done.not_carried if "carry no text" in n.what)
        self.assertEqual(entry.count, 1)

    def test_the_figure_names_the_bytes_the_document_stores_and_how_to_read_them(self):
        same = picture((100, 600, 80, 80), (2, 2), rgb(RED, GREEN, BLUE, WHITE))
        wider = picture((100, 600, 80, 80), (4, 1), rgb(RED, GREEN, BLUE, WHITE))
        other = picture((100, 600, 80, 80), (2, 2), rgb(WHITE, BLUE, GREEN, RED))

        done = self.extract([{"images": [COLOURS, same, wider, other]}])

        hashes = [b.picture for b in done.blocks]
        self.assertTrue(all(hashes))
        self.assertEqual(hashes[0], hashes[1], "the same samples at the same size are the same picture")
        self.assertNotEqual(hashes[0], hashes[2], "the same samples at another width are another picture")
        self.assertNotEqual(hashes[0], hashes[3], "other samples are another picture")

    def test_a_page_past_the_limit_places_what_it_can_and_counts_the_rest(self):
        spec = {"images": [picture((100, 40 + 30 * i, 20, 20), (1, 1), bytes([i]), "DeviceGray") for i in range(5)]}

        with mock.patch.object(extractors_pdf, "MAX_PICTURES_PER_PAGE", 3):
            done = self.extract([spec])

        self.assertEqual([b.kind for b in done.blocks], ["figure"] * 3)
        entry = next(n for n in done.not_carried if "past the 3" in n.what)
        self.assertEqual(entry.count, 2)
        self.assertIn("page(s) 1", entry.what)

    def test_a_document_with_text_says_a_drawing_of_strokes_is_not_placed(self):
        done = self.extract([{"lines": [(72, 700, 11, "Text.")], "images": [COLOURS]}])

        self.assertTrue(any("drawings" in n.what for n in done.not_carried))
        self.assertFalse(any("counted and not placed" in n.why for n in done.not_carried))


@needs_pack
class AnImageIsDrawnWhenItIsRead(Folder):
    def test_the_colour_picture_is_drawn_with_the_samples_the_document_holds(self):
        path = self.pdf([{"images": [COLOURS, GRAYS]}])

        (found,) = read_pictures([path], "page-0001-image-01.png")

        self.assertEqual(found.mime, "image/png")
        width, height, colour, rows = decoded(found.data)
        self.assertEqual((width, height, colour), (2, 2, 2))
        self.assertEqual(rows, [rgb(RED, GREEN), rgb(BLUE, WHITE)])

    def test_the_gray_picture_is_drawn_as_gray(self):
        path = self.pdf([{"images": [COLOURS, GRAYS]}])

        (found,) = read_pictures([path], "page-0001-image-02.png")

        width, height, colour, rows = decoded(found.data)
        self.assertEqual((width, height, colour), (1, 2, 0))
        self.assertEqual(rows, [b"\x00", b"\xff"])

    def test_what_is_drawn_is_named_by_the_hash_the_extraction_gave_and_is_the_same_every_time(self):
        path = self.pdf([{"lines": [(72, 720, 11, "Text.")], "images": [COLOURS]}])
        figure = next(b for b in builtin().extract(path).blocks if b.kind == "figure")

        (first,) = read_pictures([path], "page-0001-image-01.png")
        (again,) = read_pictures([path], "page-0001-image-01.png")

        self.assertEqual(first.identity, figure.picture)
        self.assertEqual(first.data, again.data)

    def test_a_name_the_document_does_not_hold_is_no_picture(self):
        path = self.pdf([{"images": [COLOURS]}, {}])

        for name in ("page-0001-image-02.png", "page-0002-image-01.png", "page-0009-image-01.png", "image1.png",
                     "page-0000-image-01.png", "page-0001-image-00.png"):
            with self.subTest(name=name):
                self.assertEqual(read_pictures([path], name), [])

    def test_the_names_a_document_could_have_been_asked_for_are_listed(self):
        path = self.pdf([{"images": [COLOURS, GRAYS]}, {}, {"images": [GRAYS]}])

        self.assertEqual(
            picture_names_held([path]),
            {path: ["page-0001-image-01.png", "page-0001-image-02.png", "page-0003-image-01.png"]},
        )

    def test_a_picture_heavier_than_the_limit_is_refused_by_name_and_size_and_not_cut(self):
        path = self.pdf([{"images": [COLOURS]}])

        with self.assertRaisesRegex(PictureTooLarge, "page-0001-image-01.png is .* bytes drawn"):
            read_pictures([path], "page-0001-image-01.png", limit=10)

    def test_a_picture_with_more_pixels_than_a_reader_holds_is_named_and_not_shown(self):
        path = self.pdf([{"images": [COLOURS]}])

        with mock.patch.object(pdf_pictures, "MAX_PIXELS", 3):
            (found,) = read_pictures([path], "page-0001-image-01.png")

        self.assertIsNone(found.mime)
        self.assertEqual(found.data, b"")
        self.assertIsNotNone(found.identity)


@needs_pack
class AFigureOfAPdfGoesToAReader(Folder):
    def test_the_picture_a_figure_marks_is_handed_over_as_a_unit_and_nothing_is_refused(self):
        path = self.pdf([{"lines": [(72, 720, 11, "Text.")], "images": [COLOURS, GRAYS]}])
        extraction = builtin().extract(path)
        ids = [b.id for b in extraction.blocks if b.kind == "figure"]

        units, refused = units_for(extraction, ids, path)

        self.assertEqual(refused, [])
        self.assertEqual([h.unit.picture.name for h in units], ["page-0001-image-01.png", "page-0001-image-02.png"])
        self.assertEqual([h.unit.picture.media_type for h in units], ["image/png", "image/png"])
        self.assertEqual(decoded(units[0].data)[3], [rgb(RED, GREEN), rgb(BLUE, WHITE)])

    def test_a_document_whose_picture_of_that_name_is_another_one_is_not_the_one_the_extraction_read(self):
        path = self.pdf([{"images": [COLOURS]}])
        extraction = builtin().extract(path)
        other = self.pdf([{"images": [GRAYS]}], name="other.pdf")

        units, refused = units_for(extraction, [extraction.blocks[0].id], other)

        self.assertEqual(units, [])
        self.assertEqual([r.code for r in refused], [OTHER_PICTURE])

    def test_a_picture_that_cannot_be_drawn_is_refused_as_one_that_cannot_be_shown(self):
        path = self.pdf([{"images": [COLOURS]}])
        extraction = builtin().extract(path)

        with mock.patch.object(pdf_pictures, "MAX_PIXELS", 3):
            units, refused = units_for(extraction, [extraction.blocks[0].id], path)

        self.assertEqual(units, [])
        self.assertEqual([r.code for r in refused], [PICTURE_NOT_SHOWABLE])

    def test_a_picture_too_heavy_to_hand_over_is_refused_alone(self):
        path = self.pdf([{"images": [COLOURS, GRAYS]}])
        extraction = builtin().extract(path)
        ids = [b.id for b in extraction.blocks]

        units, refused = units_for(extraction, ids, path, limit=70)

        self.assertEqual([r.code for r in refused], [PICTURE_TOO_LARGE])
        self.assertEqual([h.unit.picture.name for h in units], ["page-0001-image-02.png"])

    def test_a_figure_that_names_a_picture_the_document_has_lost_is_not_found(self):
        path = self.pdf([{"images": [COLOURS]}])
        extraction = builtin().extract(path)
        fewer = self.pdf([{}], name="fewer.pdf")

        units, refused = units_for(extraction, [extraction.blocks[0].id], fewer)

        self.assertEqual(units, [])
        self.assertEqual([r.code for r in refused], [PICTURE_NOT_FOUND])


if __name__ == "__main__":
    unittest.main()
