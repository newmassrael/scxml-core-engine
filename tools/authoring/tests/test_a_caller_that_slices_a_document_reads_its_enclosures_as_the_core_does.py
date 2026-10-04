"""A caller that slices a document by clause reads what it encloses the way the core reads it whole.

The core reads a word document top to bottom and carries each enclosed workbook below the body.
A caller that cuts a long specification into one component's clauses cannot take that text and
cut it again -- the cut needs the document's own structure -- so it walks the body itself. When
it kept only its own paragraph and table text it lost every attached sheet, and said nothing: the
sentence "the limits are in the attached sheet" survived and the sheet did not.

The reading is therefore public and shared, so a slicing caller asks the core instead of
writing a second reader. Asserted here:

  the objects   `read_enclosed` says which relationship id names which enclosed file
  the rows      a workbook that opens is carried as its rows, under its name
  the rest      an object that is not a workbook or deck is named as unopened, never dropped
  the mark      `enclosed_marks` is the one spelling of the mark, in the order the objects sit
  the files     `enclosed_objects` says which files an element refers to, without reading a mark back
  the lines     `carried` is heading then rows, and a name that did not open is refused
  the same      the lines a slicing caller would carry are the lines the core's own reading carries
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile

from sce_author.ingest import Enclosed, enclosed_marks, enclosed_objects, ingest, read_enclosed
from tests.test_a_document_is_read_with_what_it_encloses import deck, workbook
from tests.test_a_document_says_where_an_enclosed_file_was_attached import PKG, R, W, obj, para

BOOK_HEADING = "--- enclosed: Book.xlsx"


class ACallerThatSlicesADocumentReadsItsEnclosuresAsTheCoreDoes(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def document(self, body: str, relationships: dict[str, str] | None, **enclosed: bytes) -> pathlib.Path:
        xml = (f'<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="{W}" '
               f'xmlns:r="{R}" xmlns:o="urn:schemas-microsoft-com:office:office" '
               f'xmlns:v="urn:schemas-microsoft-com:vml"><w:body>{body}</w:body></w:document>')
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", xml)
            if relationships is not None:
                rels = "".join(f'<Relationship Id="{rid}" Type="{R}/oleObject" Target="{target}"/>'
                               for rid, target in relationships.items())
                z.writestr("word/_rels/document.xml.rels", f'<Relationships xmlns="{PKG}">{rels}</Relationships>')
            for name, blob in enclosed.items():
                z.writestr(f"word/embeddings/{name}", blob)
        return path

    def enclosed(self, body: str, relationships: dict[str, str] | None, **files: bytes) -> Enclosed:
        with zipfile.ZipFile(self.document(body, relationships, **files)) as zf:
            return read_enclosed(zf)

    def paragraph(self, text: str, *objects: str):
        root = ET.fromstring(
            f'<w:body xmlns:w="{W}" xmlns:r="{R}" xmlns:o="urn:schemas-microsoft-com:office:office" '
            f'xmlns:v="urn:schemas-microsoft-com:vml">{para(text, *objects)}</w:body>')
        return root[0]

    # ------------------------------------------------------------ the objects

    def test_each_relationship_id_names_the_file_it_refers_to(self):
        got = self.enclosed(para("", obj("rId1")) + para("", obj("rId2", "PowerPoint.Show.12")),
                            {"rId1": "embeddings/Book.xlsx", "rId2": "embeddings/Slides.pptx"},
                            **{"Book.xlsx": workbook(), "Slides.pptx": deck()})
        self.assertEqual({"rId1": "Book.xlsx", "rId2": "Slides.pptx"}, got.attached)

    def test_a_reference_to_something_that_is_not_enclosed_is_not_listed(self):
        got = self.enclosed(para("", obj("rId7")), {"rId7": "media/image1.png"}, **{"Book.xlsx": workbook()})
        self.assertEqual({}, got.attached)

    def test_a_document_with_no_relationships_part_still_opens_what_it_encloses(self):
        got = self.enclosed(para("Plain."), None, **{"Book.xlsx": workbook()})
        self.assertEqual({}, got.attached)
        self.assertEqual(["Book.xlsx"], list(got.rows))

    # --------------------------------------------------------------- the rows

    def test_a_workbook_that_opens_is_carried_as_its_rows_under_its_name(self):
        got = self.enclosed(para("", obj("rId1")), {"rId1": "embeddings/Book.xlsx"}, **{"Book.xlsx": workbook()})
        self.assertEqual(["Book.xlsx"], list(got.rows))
        self.assertIn("| CONDITION | SHOWS |", got.rows["Book.xlsx"])
        self.assertEqual([], got.unopened)
        self.assertEqual([], got.notes)

    def test_a_deck_that_opens_is_carried_too(self):
        got = self.enclosed(para("", obj("rId1", "PowerPoint.Show.12")), {"rId1": "embeddings/Slides.pptx"},
                            **{"Slides.pptx": deck()})
        self.assertEqual(["Slides.pptx"], list(got.rows))
        self.assertTrue(got.rows["Slides.pptx"])

    def test_files_are_carried_in_name_order(self):
        got = self.enclosed(para(""), {}, **{"B.xlsx": workbook(), "A.xlsx": workbook()})
        self.assertEqual(["A.xlsx", "B.xlsx"], list(got.rows))

    # --------------------------------------------------------------- the rest

    def test_an_object_that_is_not_a_workbook_or_deck_is_named_as_unopened(self):
        got = self.enclosed(para("", obj("rId1", "Equation.3")), {"rId1": "embeddings/oleObject1.bin"},
                            **{"oleObject1.bin": b"\x01not a zip", "Book.xlsx": workbook()})
        self.assertEqual(["oleObject1.bin"], got.unopened)
        self.assertEqual(["Book.xlsx"], list(got.rows))

    def test_a_workbook_that_cannot_be_opened_says_why_and_is_named_as_unopened(self):
        got = self.enclosed(para(""), {}, **{"Broken.xlsx": b"\x01not a zip"})
        self.assertEqual(["Broken.xlsx"], got.unopened)
        self.assertEqual({}, got.rows)
        self.assertTrue(any(n.startswith("Broken.xlsx: ") for n in got.notes), got.notes)

    # --------------------------------------------------------------- the mark

    def test_the_mark_is_spelled_once_with_the_kind_in_the_order_the_objects_sit(self):
        marks = enclosed_marks(self.paragraph("See", obj("rId1"), obj("rId2", "PowerPoint.Show.12")),
                               {"rId1": "Book.xlsx", "rId2": "Slides.pptx"})
        self.assertEqual(["[enclosed object: Book.xlsx (Excel.Sheet.12)]",
                          "[enclosed object: Slides.pptx (PowerPoint.Show.12)]"], marks)

    def test_an_object_that_names_no_kind_is_marked_without_one(self):
        marks = enclosed_marks(self.paragraph("", obj("rId1", prog=None)), {"rId1": "Book.xlsx"})
        self.assertEqual(["[enclosed object: Book.xlsx]"], marks)

    def test_an_object_that_names_no_enclosed_file_is_not_marked(self):
        self.assertEqual([], enclosed_marks(self.paragraph("", obj("rId99")), {"rId1": "Book.xlsx"}))
        self.assertEqual([], enclosed_marks(self.paragraph("", obj(None, "Equation.3")), {"rId1": "Book.xlsx"}))

    def test_the_files_an_element_refers_to_are_listed_with_their_kind_in_order(self):
        refs = enclosed_objects(self.paragraph("See", obj("rId1"), obj("rId2", None)),
                                {"rId1": "Book.xlsx", "rId2": "Slides.pptx"})
        self.assertEqual([("Book.xlsx", "Excel.Sheet.12"), ("Slides.pptx", None)], refs)

    def test_a_reference_to_no_enclosed_file_is_not_listed(self):
        self.assertEqual([], enclosed_objects(self.paragraph("", obj("rId99")), {"rId1": "Book.xlsx"}))

    def test_an_element_that_holds_paragraphs_lists_the_files_of_all_of_them(self):
        """A table cell is not a paragraph: a caller asks about the cell and gets its paragraphs' files."""
        cell = ET.fromstring(f'<w:tc xmlns:w="{W}" xmlns:r="{R}" xmlns:o="urn:schemas-microsoft-com:office:office" '
                             f'xmlns:v="urn:schemas-microsoft-com:vml">{para("", obj("rId1"))}{para("", obj("rId2"))}</w:tc>')
        self.assertEqual(["Book.xlsx", "Slides.pptx"],
                         [name for name, _kind in enclosed_objects(cell, {"rId1": "Book.xlsx", "rId2": "Slides.pptx"})])

    # --------------------------------------------------------------- the lines

    def test_carried_is_a_heading_then_the_rows(self):
        got = self.enclosed(para(""), {}, **{"Book.xlsx": workbook()})
        self.assertEqual([BOOK_HEADING, *got.rows["Book.xlsx"]], got.carried(["Book.xlsx"]))

    def test_carried_with_no_names_carries_every_file_that_opened(self):
        got = self.enclosed(para(""), {}, **{"B.xlsx": workbook(), "A.xlsx": workbook()})
        self.assertEqual(got.carried(["A.xlsx", "B.xlsx"]), got.carried())

    def test_a_name_that_did_not_open_is_refused_not_skipped(self):
        got = self.enclosed(para(""), {}, **{"oleObject1.bin": b"\x01not a zip", "Book.xlsx": workbook()})
        with self.assertRaises(KeyError) as raised:
            got.carried(["oleObject1.bin"])
        self.assertIn("Book.xlsx", str(raised.exception))

    # --------------------------------------------------------------- the same

    def test_the_lines_a_slicing_caller_carries_are_the_lines_the_core_carries_whole(self):
        body = para("The limits are in the attached sheet.", obj("rId1")) + para("After.")
        rels = {"rId1": "embeddings/Book.xlsx"}
        path = self.document(body, rels, **{"Book.xlsx": workbook()})
        with zipfile.ZipFile(path) as zf:
            got = read_enclosed(zf)
        whole = ingest(path).text.splitlines()
        carried = got.carried()
        at = whole.index(BOOK_HEADING)
        self.assertEqual(carried, whole[at:at + len(carried)])
        self.assertIn("The limits are in the attached sheet. [enclosed object: Book.xlsx (Excel.Sheet.12)]", whole)


if __name__ == "__main__":
    unittest.main()
