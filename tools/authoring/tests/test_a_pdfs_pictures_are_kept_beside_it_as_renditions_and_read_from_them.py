"""The pictures of a PDF are drawn once to be kept beside it, and a reading takes the kept ones.

ADR 0032: what is derived from a member is a member of its own, a rendition, that says what it was made from
and what drew it. A bundle that keeps the renditions of a PDF's images keeps what the model saw, and a machine
without the PDF library can still be shown them. `render-pictures` draws them; a picture that is a file of the
renditions folder beside the PDF is handed over as it is, and the PDF is not opened for it.

The PDFs are written by `build_pdf` of the text-layer test, and the PNGs are decoded with the standard library.
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import unittest
from unittest import mock

import pathlib

from sce_author import pdf_pictures
from sce_author.__main__ import main
from sce_author.errors import IngestError
from sce_author.extractors import builtin
from sce_author.ingest import PictureTooLarge, picture_names_held, read_pictures
from sce_author.reading import PICTURE_NOT_FOUND, units_for

from tests.test_a_pdf_with_a_text_layer_is_read_by_page_and_box import Folder, needs_pack
from tests.test_a_picture_in_a_pdf_is_a_figure_and_is_drawn_when_it_is_read import (
    COLOURS,
    GRAYS,
    RED,
    GREEN,
    BLUE,
    WHITE,
    decoded,
    rgb,
)


class Renditions(Folder):
    def render(self, pages, *extra: str) -> tuple[int, str]:
        document = self.pdf(pages)
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = main(["render-pictures", "--document", str(document), "--out-dir", str(self.root / "r"), *extra])
        return code, out.getvalue()

    def listing(self) -> dict:
        return json.loads((self.root / "r" / "renditions.json").read_text(encoding="utf-8"))

    def keep(self, pages) -> pathlib.Path:
        """A PDF with its renditions kept beside it, as a bundle holds them."""
        code, _ = self.render(pages)
        self.assertEqual(code, 0)
        kept = self.root / pdf_pictures.RENDITIONS
        (self.root / "r").rename(kept)
        (kept / "renditions.json").unlink()
        return self.root / "spec.pdf"


@needs_pack
class TheCommandDrawsEveryPicture(Renditions):
    def test_each_picture_is_a_png_file_and_the_listing_names_it_by_the_hash_of_those_bytes(self):
        code, said = self.render([{"images": [COLOURS, GRAYS]}, {"images": [GRAYS]}])

        self.assertEqual(code, 0, said)
        listing = self.listing()
        self.assertEqual(
            [p["name"] for p in listing["pictures"]],
            ["page-0001-image-01.png", "page-0001-image-02.png", "page-0002-image-01.png"],
        )
        for picture in listing["pictures"]:
            data = (self.root / "r" / picture["name"]).read_bytes()
            self.assertEqual(picture["sha256"], hashlib.sha256(data).hexdigest())
            self.assertEqual(picture["bytes"], len(data))
        first = decoded((self.root / "r" / "page-0001-image-01.png").read_bytes())
        self.assertEqual(first[3], [rgb(RED, GREEN), rgb(BLUE, WHITE)])
        self.assertEqual(listing["document"], "spec.pdf")
        self.assertRegex(listing["renderer"], r"^pdfium .*pypdfium2 ")
        self.assertLessEqual(len(listing["renderer"].encode()), 64)
        self.assertEqual(listing["skipped"], [])

    def test_a_picture_that_cannot_be_drawn_or_is_too_heavy_is_said_and_not_an_error(self):
        with mock.patch.object(pdf_pictures, "MAX_PIXELS", 3):
            code, _ = self.render([{"images": [COLOURS, GRAYS]}])

        self.assertEqual(code, 0)
        listing = self.listing()
        self.assertEqual([p["name"] for p in listing["pictures"]], ["page-0001-image-02.png"])
        self.assertEqual([s["name"] for s in listing["skipped"]], ["page-0001-image-01.png"])
        self.assertIn("cannot be drawn", listing["skipped"][0]["why"])

    def test_past_the_most_pictures_a_document_is_rendered_for_the_rest_are_said(self):
        with mock.patch.object(pdf_pictures, "MAX_RENDERED", 1):
            code, _ = self.render([{"images": [COLOURS, GRAYS]}])

        self.assertEqual(code, 0)
        self.assertEqual(len(self.listing()["pictures"]), 1)
        self.assertEqual([s["name"] for s in self.listing()["skipped"]], ["page-0001-image-02.png"])

    def test_a_document_with_no_picture_has_a_listing_that_names_no_renderer(self):
        code, _ = self.render([{}])

        self.assertEqual(code, 0)
        self.assertEqual(self.listing()["pictures"], [])
        self.assertIsNone(self.listing()["renderer"])

    def test_a_file_that_is_not_a_pdf_has_no_renditions_made(self):
        path = self.root / "note.md"
        path.write_text("# a\n", encoding="utf-8")
        err = io.StringIO()

        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            code = main(["render-pictures", "--document", str(path), "--out-dir", str(self.root / "r")])

        self.assertNotEqual(code, 0)
        self.assertFalse((self.root / "r").exists())


@needs_pack
class AKeptRenditionIsWhatIsHandedOver(Renditions):
    def test_the_kept_file_is_what_is_read_and_the_pdf_is_not_opened_for_it(self):
        document = self.keep([{"images": [COLOURS]}])
        kept = (self.root / pdf_pictures.RENDITIONS / "page-0001-image-01.png").read_bytes()

        with mock.patch.object(pdf_pictures, "draw_named", side_effect=AssertionError("the PDF was opened")):
            (found,) = read_pictures([document], "page-0001-image-01.png")

        self.assertEqual(found.data, kept)
        self.assertEqual(found.mime, "image/png")
        self.assertTrue(found.derived)

    def test_it_is_read_without_the_pack_because_it_is_a_file_and_the_unkept_ones_say_the_pack_is_missing(self):
        document = self.keep([{"images": [COLOURS, GRAYS]}])
        (self.root / pdf_pictures.RENDITIONS / "page-0001-image-02.png").unlink()
        without_pack = IngestError(f"{document}: a picture in a PDF is drawn by the pack pdf-text")

        with mock.patch.object(pdf_pictures, "_open", side_effect=without_pack):
            (found,) = read_pictures([document], "page-0001-image-01.png")
            with self.assertRaisesRegex(IngestError, "pack pdf-text"):
                read_pictures([document], "page-0001-image-02.png")
            held = picture_names_held([document])

        self.assertEqual(found.mime, "image/png")
        self.assertEqual(held, {document: ["page-0001-image-01.png"]})

    def test_a_figure_is_handed_over_from_the_rendition_with_no_identity_to_recompute(self):
        document = self.keep([{"lines": [(72, 720, 11, "Text.")], "images": [COLOURS]}])
        extraction = builtin().extract(document)
        figure = next(b for b in extraction.blocks if b.kind == "figure")

        with mock.patch.object(pdf_pictures, "_open", side_effect=IngestError("no pack")):
            units, refused = units_for(extraction, [figure.id], document)

        self.assertEqual(refused, [])
        self.assertEqual([h.unit.picture.name for h in units], ["page-0001-image-01.png"])
        self.assertEqual(
            units[0].unit.picture.sha256,
            hashlib.sha256((self.root / pdf_pictures.RENDITIONS / "page-0001-image-01.png").read_bytes()).hexdigest(),
        )

    def test_a_picture_that_is_not_kept_is_still_drawn_from_the_document(self):
        document = self.keep([{"images": [COLOURS, GRAYS]}])
        (self.root / pdf_pictures.RENDITIONS / "page-0001-image-02.png").unlink()

        (found,) = read_pictures([document], "page-0001-image-02.png")

        self.assertEqual(decoded(found.data)[:3], (1, 2, 0))
        self.assertFalse(found.derived)

    def test_a_kept_file_heavier_than_the_limit_is_refused_and_not_cut(self):
        document = self.keep([{"images": [COLOURS]}])

        with self.assertRaisesRegex(PictureTooLarge, "page-0001-image-01.png is .* bytes"):
            read_pictures([document], "page-0001-image-01.png", limit=10)

    def test_only_a_name_a_pdfs_pictures_are_called_by_is_looked_up_in_the_renditions_folder(self):
        document = self.keep([{"images": [COLOURS]}])
        other = self.root / pdf_pictures.RENDITIONS / "secret.png"
        other.write_bytes(b"not a rendition")

        self.assertEqual(read_pictures([document], "secret.png"), [])
        # A name with a folder in it is no name of a picture at all.
        for name in ("../spec.pdf", "renditions/secret.png"):
            with self.subTest(name=name), self.assertRaises(IngestError):
                read_pictures([document], name)

    def test_a_link_out_of_the_renditions_folder_is_not_followed(self):
        document = self.keep([{"images": [COLOURS]}])
        outside = self.root / "outside.png"
        outside.write_bytes(b"\x89PNG-outside")
        link = self.root / pdf_pictures.RENDITIONS / "page-0001-image-09.png"
        link.symlink_to(outside)

        self.assertEqual(read_pictures([document], "page-0001-image-09.png"), [])


@needs_pack
class TheReadingTakesTheKeptPictureWhateverTheExtractionHolds(Renditions):
    def test_a_reading_names_the_picture_it_was_handed_by_the_hash_of_the_kept_file(self):
        document = self.keep([{"lines": [(72, 720, 11, "Text.")], "images": [COLOURS]}])
        extraction = builtin().extract(document)
        figure = next(b for b in extraction.blocks if b.kind == "figure")
        kept = (self.root / pdf_pictures.RENDITIONS / "page-0001-image-01.png").read_bytes()

        units, _ = units_for(extraction, [figure.id], document)

        self.assertEqual(units[0].data, kept)
        self.assertEqual(units[0].unit.picture.media_type, "image/png")

    def test_a_name_the_bundle_does_not_keep_and_the_document_does_not_hold_is_not_found(self):
        document = self.keep([{"lines": [(72, 720, 11, "Text.")], "images": [COLOURS]}])
        extraction = builtin().extract(document)
        figure = next(b for b in extraction.blocks if b.kind == "figure")
        (self.root / pdf_pictures.RENDITIONS / "page-0001-image-01.png").unlink()
        fewer = self.pdf([{}], name="spec.pdf")

        units, refused = units_for(extraction, [figure.id], fewer)

        self.assertEqual(units, [])
        self.assertEqual([r.code for r in refused], [PICTURE_NOT_FOUND])


if __name__ == "__main__":
    unittest.main()
