"""A picture the reader lost is said beside the specification text in the brief, not only in the reader's notes.

`ingest` reports what it could not carry as notes, and the brief prints each one on a line of its own
directly under the file it came from ("NOT CARRIED FROM THIS FILE"). A reviewer drove the real MCP with
three small Word documents, each attaching a workbook whose middle cell holds a picture, and found that
a workbook whose picture could not be read was handed to the writer as an empty cell with nothing said.
The reader now says it; these cases hold that the SAYING REACHES THE PAGE the writer reads, because a
note that stops at the reader helps nobody who only sees the brief.

Asserted here, for the three documents the review used:

  intact      the picture is placed on its cell and the page says how many pictures sit on cells
  missing     the sheet names a drawing the file lacks: the page says so by sheet name
  unreadable  the sheet's links to its drawing are not XML: the page says so by sheet name
  the table   in every case the table itself is still on the page, so nothing readable is withheld
  no wolf     a workbook with no drawing at all adds no picture line
"""

from __future__ import annotations

import io
import pathlib
import tempfile
import unittest
import zipfile

from sce_author.brief import assemble
from sce_author.ingest import ingest
from sce_author.pack import load_pack
from sce_author.prose import load_prose
from tests.test_a_picture_on_an_enclosed_workbook_cell_is_not_lost_quietly import (
    drawing_xml, picture_on, row, sheet_xml, workbook)

HERE = pathlib.Path(__file__).resolve().parent
PACK = HERE / "fixtures" / "crossing"

BODY = ('<?xml version="1.0" encoding="UTF-8"?><w:document '
        'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r>'
        '<w:t>IN_MainsPower is described by the attached table.</w:t></w:r></w:p></w:body></w:document>')


def actions_sheet() -> str:
    """A table of marks whose middle column is the datum: 'does this row have an image'."""
    return sheet_xml(row(1, A="Condition", B="Picture", C="Signal"), row(2, A="SupplyOn", C="ON"))


def intact() -> bytes:
    return workbook([("Actions", actions_sheet(), drawing_xml(picture_on(1, 1)))])


def without(blob: bytes, drop) -> bytes:
    out = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(blob)) as src, zipfile.ZipFile(out, "w") as dst:
        for item in src.infolist():
            data = drop(item.filename, src.read(item.filename))
            if data is not None:
                dst.writestr(item, data)
    return out.getvalue()


def missing_drawing() -> bytes:
    return without(intact(), lambda name, data: None if name.startswith("xl/drawings/") else data)


def unreadable_links() -> bytes:
    return without(intact(), lambda name, data: b"<Relationships"
                   if name == "xl/worksheets/_rels/sheet1.xml.rels" else data)


class ALostPictureIsSaidInTheBrief(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        self.pack = load_pack(PACK)

    def page(self, blob: bytes) -> str:
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", BODY)
            z.writestr("word/embeddings/Book.xlsx", blob)
        return assemble(load_prose([path]), self.pack)

    def not_carried(self, blob: bytes) -> list[str]:
        return [l for l in self.page(blob).splitlines() if l.startswith("> NOT CARRIED FROM THIS FILE:")]

    # -------------------------------------------------------------- intact

    def test_an_intact_picture_is_placed_and_counted_on_the_page(self):
        page = self.page(intact())
        self.assertIn("| SupplyOn | [picture] | ON |", page)
        said = " ".join(self.not_carried(intact()))
        self.assertIn("1 picture(s) sit on 1 cell(s)", said)
        self.assertNotIn("could not be read", said)

    # ------------------------------------------------------------- missing

    def test_a_missing_drawing_is_named_by_sheet_beside_the_text(self):
        said = " ".join(self.not_carried(missing_drawing()))
        self.assertIn('1 drawing(s) referred to by sheet(s) "Actions"', said)
        self.assertIn("a cell that reads as empty there may have held one", said)

    def test_the_table_is_still_there_when_its_picture_is_not(self):
        page = self.page(missing_drawing())
        self.assertIn("| SupplyOn |  | ON |", page)
        self.assertIn("--- sheet: Actions", page)
        self.assertNotIn("[picture]", page)

    # ---------------------------------------------------------- unreadable

    def test_unreadable_links_are_named_by_sheet_beside_the_text(self):
        said = " ".join(self.not_carried(unreadable_links()))
        self.assertIn('referred to by sheet(s) "Actions"', said)

    def test_the_table_is_still_there_when_its_links_are_not(self):
        self.assertIn("| SupplyOn |  | ON |", self.page(unreadable_links()))

    # -------------------------------------------------------------- no wolf

    def test_a_workbook_with_no_drawing_adds_no_picture_line(self):
        plain = workbook([("Actions", actions_sheet(), None)])
        for line in self.not_carried(plain):
            self.assertNotIn("picture", line)
            self.assertNotIn("drawing(s) referred to", line)

    def test_a_loss_is_told_apart_from_an_intact_picture_by_what_stands_beside_the_text(self):
        """What a reviewer could not do before: the intact page and the two lossy pages differ.

        The two losses say the SAME thing, on purpose. A drawing the file lacks and links that
        cannot be read are one fact to a reader of the page -- 'the pictures on this sheet were not
        seen' -- and splitting the sentence by cause would hand the writer a distinction it cannot
        act on. The sentence names every cause it covers."""
        notes = {name: " ".join(self.not_carried(blob))
                 for name, blob in (("intact", intact()), ("missing", missing_drawing()),
                                    ("unreadable", unreadable_links()))}
        self.assertNotEqual(notes["intact"], notes["missing"])
        self.assertNotEqual(notes["intact"], notes["unreadable"])
        self.assertEqual(notes["missing"], notes["unreadable"])
        self.assertIn("1 picture(s) sit on 1 cell(s)", notes["intact"])
        self.assertNotIn("could not be read", notes["intact"])
        for cause in ("is missing from the file", "cannot be parsed", "cannot be read"):
            self.assertIn(cause, notes["missing"])

    def test_the_reader_and_the_page_agree(self):
        """What `ingest` says is what the page prints: the page adds nothing and drops nothing."""
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", BODY)
            z.writestr("word/embeddings/Book.xlsx", missing_drawing())
        ingested = ingest(path)
        page = assemble(load_prose([path]), self.pack)
        for note in ingested.notes:
            self.assertIn(f"> NOT CARRIED FROM THIS FILE: {note}", page)


if __name__ == "__main__":
    unittest.main()
