"""A sheet that names a drawing its links do not list says so, rather than reading as having none.

A sheet carries two records of its pictures: its own XML names the drawing (`<drawing r:id>`), and
its relationships say where that drawing is. The reader walked the relationships only, so a
drawing the sheet named and the relationships did not list left nothing to walk: the relationship
file was missing, or it was there with no entry for that id. Both read as a sheet with no
pictures, and the cells that held them as empty, with no word said (a review, 2026-10-05). On a
workbook where the picture IS the datum, that is the loss that looks most like success.

Asserted here, by what the sheet names against what its links list:

  not listed     no relationship file / a file with no entry for the id / an entry of another kind
                 each say so by sheet name, once per drawing named, and the other cells still read
  listed         a drawing the links list reads as before, with its `[picture]` marks and no note
  plain          a sheet that names no drawing and has no links file says nothing
  once           links that cannot be read say it once, not once more for each drawing named
  where          the note reaches the brief, beside the specification it is missing from
"""

from __future__ import annotations

import io
import pathlib
import shutil
import zipfile

from sce_author.brief import assemble
from sce_author.pack import load_pack
from sce_author.prose import load_prose
from tests.test_a_picture_on_an_enclosed_workbook_cell_is_not_lost_quietly import (
    MAIN, PKG, REL, Reads, drawing_xml, picture_on, row)


def sheet_naming(*rows: str, ids: tuple[str, ...] = ("rId1",)) -> str:
    named = "".join(f'<drawing r:id="{i}"/>' for i in ids)
    return (f'<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="{MAIN}" xmlns:r="{REL}">'
            f'<sheetData>{"".join(rows)}</sheetData>{named}</worksheet>')


def book(sheet: str, links: str | None = None, drawing: str | None = None) -> bytes:
    """One sheet named Marks. `links` is the inside of its relationships file (None: no such file)."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("xl/worksheets/sheet1.xml", sheet)
        z.writestr("xl/workbook.xml", f'<workbook xmlns="{MAIN}" xmlns:r="{REL}"><sheets>'
                                      f'<sheet name="Marks" sheetId="1" r:id="rId1"/></sheets></workbook>')
        z.writestr("xl/_rels/workbook.xml.rels",
                   f'<Relationships xmlns="{PKG}"><Relationship Id="rId1" Type="{REL}/worksheet" '
                   f'Target="worksheets/sheet1.xml"/></Relationships>')
        if links is not None:
            z.writestr("xl/worksheets/_rels/sheet1.xml.rels", links)
        if drawing is not None:
            z.writestr("xl/drawings/drawing1.xml", drawing)
    return buf.getvalue()


def links(*entries: str) -> str:
    return f'<Relationships xmlns="{PKG}">{"".join(entries)}</Relationships>'


def entry(rid: str, kind: str, target: str) -> str:
    return f'<Relationship Id="{rid}" Type="{REL}/{kind}" Target="{target}"/>'


CELLS = (row(1, A="x"), row(2, A="y"))
FIXTURE = pathlib.Path(__file__).resolve().parent / "fixtures" / "crossing"


class ASheetThatNamesADrawingItsLinksDoNotList(Reads):
    def said(self, blob: bytes) -> str:
        return " ".join(self.notes_about_pictures(blob))

    # ------------------------------------------------------------ not listed

    def test_no_relationship_file_at_all_says_so_by_sheet_name(self):
        blob = book(sheet_naming(*CELLS))
        self.assertIn('1 drawing(s) referred to by sheet(s) "Marks"', self.said(blob))

    def test_a_relationship_file_with_no_entry_for_the_id_says_so(self):
        blob = book(sheet_naming(*CELLS), links(entry("rId9", "hyperlink", "https://example.invalid/")))
        self.assertIn('1 drawing(s) referred to by sheet(s) "Marks"', self.said(blob))

    def test_an_entry_of_another_kind_under_the_id_says_so(self):
        blob = book(sheet_naming(*CELLS), links(entry("rId1", "image", "../media/image1.png")))
        self.assertIn('1 drawing(s) referred to by sheet(s) "Marks"', self.said(blob))

    def test_each_drawing_named_is_counted(self):
        blob = book(sheet_naming(*CELLS, ids=("rId1", "rId2")))
        self.assertIn('2 drawing(s) referred to by sheet(s) "Marks"', self.said(blob))

    def test_the_cells_are_still_read_and_no_picture_is_invented(self):
        blob = book(sheet_naming(*CELLS))
        got = self.read(blob)
        self.assertIn("| x |", got.text)
        self.assertIn("| y |", got.text)
        self.assertNotIn("[picture]", got.text)

    def test_it_says_what_the_loss_means_for_a_reader(self):
        said = self.said(book(sheet_naming(*CELLS)))
        self.assertIn("a cell that reads as empty there may have held one", said)

    # ---------------------------------------------------------------- listed

    def test_a_drawing_the_links_list_reads_as_before(self):
        blob = book(sheet_naming(*CELLS), links(entry("rId1", "drawing", "../drawings/drawing1.xml")),
                    drawing_xml(picture_on(1, 0)))
        got = self.read(blob)
        self.assertIn("[picture]", got.text)
        self.assertEqual([], [n for n in got.notes if "drawing(s) referred to" in n])

    def test_a_listed_drawing_among_unlisted_ones_counts_only_the_unlisted(self):
        blob = book(sheet_naming(*CELLS, ids=("rId1", "rId2")),
                    links(entry("rId1", "drawing", "../drawings/drawing1.xml")), drawing_xml(picture_on(1, 0)))
        self.assertIn('1 drawing(s) referred to by sheet(s) "Marks"', self.said(blob))
        self.assertIn("[picture]", self.read(blob).text)

    # ----------------------------------------------------------------- plain

    def test_a_plain_sheet_with_no_links_file_says_nothing(self):
        blob = book(sheet_naming(*CELLS, ids=()))
        self.assertEqual([], self.notes_about_pictures(blob))

    def test_a_plain_sheet_with_a_links_file_for_something_else_says_nothing(self):
        blob = book(sheet_naming(*CELLS, ids=()), links(entry("rId1", "hyperlink", "https://example.invalid/")))
        self.assertEqual([], self.notes_about_pictures(blob))

    # ------------------------------------------------------------------ once

    def test_links_that_cannot_be_read_say_it_once_however_many_drawings_are_named(self):
        blob = book(sheet_naming(*CELLS, ids=("rId1", "rId2")), "<Relationships")
        said = self.said(blob)
        self.assertIn('1 drawing(s) referred to by sheet(s) "Marks"', said)
        self.assertNotIn("3 drawing(s)", said)

    # ----------------------------------------------------------------- where

    def test_the_note_reaches_the_brief_beside_the_specification(self):
        blob = book(sheet_naming(*CELLS))
        self.read(blob)
        spec = self.tmp / "spec.docx"
        pack_dir = self.tmp / "pack"
        pack_dir.mkdir()
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(FIXTURE / name, pack_dir / name)
        brief = assemble(load_prose([spec]), load_pack(pack_dir))
        self.assertIn("NOT CARRIED FROM THIS FILE", brief)
        self.assertIn('sheet(s) "Marks"', brief)


if __name__ == "__main__":
    import unittest

    unittest.main()
