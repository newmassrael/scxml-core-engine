"""A picture on a cell of an enclosed workbook is placed on its cell, and what could not be placed is said.

In a table of marks the datum is often whether a cell HOLDS a picture: its text says '-' or
'O', and the picture beside it says which row has an image. A reader that opened the cells
and skipped the drawing reported the table as complete -- on one specification the workbook
it opened held 2,188 pictures on its cells, and the report said "4 enclosed files WERE opened
and their contents carried". That is the loss that looks most like success, one level down:
the file was opened, so nothing about the picture inside it was ever said.

Only WHERE a picture sits is read. What it shows, and what the column it sits in means, is for
whoever can read it, so the core marks the cell and leaves the meaning alone.

Asserted here, in the order a reader meets it:

  the cell     a picture anchored on a cell marks that cell `[picture]`, beside its text if it
               has any, as `[N pictures]` when several sit there
  the row      a row that exists only because a picture sits on it still appears, in order
  the sheet    the mark is under the heading of its own sheet
  the account  how many pictures sit on how many cells is said once per workbook
  not placed   a floating or grouped picture, a chart or shape, and a drawing that cannot be
               parsed are each counted and said, never silently dropped
  absence      a workbook with no drawing says nothing about pictures and reads as before
"""

from __future__ import annotations

import io
import pathlib
import tempfile
import unittest
import zipfile

from sce_author.ingest import ingest

DOCUMENT_XML = """<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>The images are in the attached sheet.</w:t></w:r></w:p></w:body>
</w:document>"""

MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
PKG = "http://schemas.openxmlformats.org/package/2006/relationships"
XDR = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"


def row(number: int, **cells: str) -> str:
    """`row(2, A="x", C="y")` -> a row of inline-string cells, columns by letter."""
    body = "".join(f'<c r="{col}{number}" t="inlineStr"><is><t>{text}</t></is></c>'
                   for col, text in cells.items())
    return f'<row r="{number}">{body}</row>'


def sheet_xml(*rows: str) -> str:
    return (f'<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="{MAIN}">'
            f'<sheetData>{"".join(rows)}</sheetData></worksheet>')


def at(col: int, row_: int) -> str:
    """An `xdr:from` for the cell at zero-based (col, row)."""
    return (f"<xdr:from><xdr:col>{col}</xdr:col><xdr:colOff>0</xdr:colOff>"
            f"<xdr:row>{row_}</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>")


def picture_on(col: int, row_: int) -> str:
    """A picture anchored, as a spreadsheet stores one, from a cell to the next."""
    return (f"<xdr:twoCellAnchor>{at(col, row_)}{at(col + 1, row_ + 1).replace('from', 'to')}"
            f"<xdr:pic/><xdr:clientData/></xdr:twoCellAnchor>")


def drawing_xml(*anchors: str) -> str:
    return (f'<?xml version="1.0" encoding="UTF-8"?>'
            f'<xdr:wsDr xmlns:xdr="{XDR}">{"".join(anchors)}</xdr:wsDr>')


RICH = "http://schemas.microsoft.com/office/spreadsheetml/2017/richdata"


def rich_parts(structures: list[str], values: list[int], futures: list[int],
               held: list[int], kind: str = "XLRICHVALUE") -> dict[str, str]:
    """The parts a workbook needs to hold pictures in cells, as Excel writes them.

    `structures` are the type names (`_localImage` is a picture); `values[i]` is the structure
    of rich value i; `futures[k]` is the rich value that future-metadata entry k names; and
    `held[j]` is the future-metadata entry that a cell with `vm = j + 1` points at.
    """
    return {
        "xl/metadata.xml": (
            f'<metadata xmlns="{MAIN}" xmlns:xlrd="{RICH}">'
            f'<metadataTypes count="1"><metadataType name="{kind}"/></metadataTypes>'
            f'<futureMetadata name="XLRICHVALUE" count="{len(futures)}">'
            + "".join(f'<bk><extLst><ext uri="{{x}}"><xlrd:rvb i="{i}"/></ext></extLst></bk>'
                      for i in futures)
            + f'</futureMetadata><valueMetadata count="{len(held)}">'
            + "".join(f'<bk><rc t="1" v="{k}"/></bk>' for k in held)
            + "</valueMetadata></metadata>"),
        "xl/richData/rdrichvalue.xml": (
            f'<rvData xmlns="{RICH}" count="{len(values)}">'
            + "".join(f'<rv s="{s}"><v>0</v></rv>' for s in values) + "</rvData>"),
        "xl/richData/rdrichvaluestructure.xml": (
            f'<rvStructures xmlns="{RICH}" count="{len(structures)}">'
            + "".join(f'<s t="{t}"><k n="k" t="i"/></s>' for t in structures) + "</rvStructures>"),
    }


def in_cell(ref: str, vm: int) -> str:
    """A cell that holds a rich value: Excel stores an error as its placeholder text."""
    return f'<c r="{ref}" t="e" vm="{vm}"><v>#VALUE!</v></c>'


def workbook(sheets: list[tuple[str, str, str | None]],
             extra: dict[str, str] | None = None) -> bytes:
    """`sheets` is (name, sheet xml, drawing xml or None), stored as sheet1, sheet2 ...;
    `extra` is any further parts of the package."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        for part, xml in (extra or {}).items():
            z.writestr(part, xml)
        entries, rels = "", ""
        for n, (name, xml, drawing) in enumerate(sheets, start=1):
            z.writestr(f"xl/worksheets/sheet{n}.xml", xml)
            entries += f'<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>'
            rels += (f'<Relationship Id="rId{n}" Type="{REL}/worksheet" '
                     f'Target="worksheets/sheet{n}.xml"/>')
            if drawing is not None:
                z.writestr(f"xl/drawings/drawing{n}.xml", drawing)
                z.writestr(f"xl/worksheets/_rels/sheet{n}.xml.rels",
                           f'<Relationships xmlns="{PKG}"><Relationship Id="rId1" '
                           f'Type="{REL}/drawing" Target="../drawings/drawing{n}.xml"/>'
                           f"</Relationships>")
        z.writestr("xl/workbook.xml",
                   f'<workbook xmlns="{MAIN}" xmlns:r="{REL}"><sheets>{entries}</sheets></workbook>')
        z.writestr("xl/_rels/workbook.xml.rels", f'<Relationships xmlns="{PKG}">{rels}</Relationships>')
    return buf.getvalue()


class Reads(unittest.TestCase):
    """How both groups of cases read a workbook. It holds no case of its own."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def read(self, blob: bytes):
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", DOCUMENT_XML)
            z.writestr("word/embeddings/Book.xlsx", blob)
        return ingest(path)

    def grid(self, blob: bytes) -> list[str]:
        return [l for l in self.read(blob).text.splitlines() if l.startswith(("|", "--- sheet"))]

    def notes_about_pictures(self, blob: bytes) -> list[str]:
        return [n for n in self.read(blob).notes if "picture" in n or "drawn object" in n
                or "drawing(s) referred to" in n]


class APictureOnAnEnclosedCellIsNotLostQuietly(Reads):
    # ------------------------------------------------------------ the cell

    def test_a_picture_marks_the_cell_it_is_anchored_on(self):
        blob = workbook([("Marks", sheet_xml(row(1, A="-", B="O"), row(2, A="-", B="X")),
                          drawing_xml(picture_on(1, 0)))])           # zero-based (B, row 1)
        self.assertEqual(["--- sheet: Marks", "| - | O [picture] |", "| - | X |"],
                         self.grid(blob))

    def test_a_cell_with_no_text_is_just_the_mark(self):
        blob = workbook([("Marks", sheet_xml(row(1, A="x")), drawing_xml(picture_on(2, 0)))])
        self.assertEqual(["--- sheet: Marks", "| x |  | [picture] |"], self.grid(blob))

    def test_several_pictures_on_one_cell_are_counted(self):
        blob = workbook([("Marks", sheet_xml(row(1, A="x")),
                          drawing_xml(picture_on(0, 0), picture_on(0, 0), picture_on(0, 0)))])
        self.assertEqual(["--- sheet: Marks", "| x [3 pictures] |"], self.grid(blob))

    def test_a_one_cell_anchor_places_a_picture_too(self):
        anchor = f"<xdr:oneCellAnchor>{at(1, 0)}<xdr:pic/><xdr:clientData/></xdr:oneCellAnchor>"
        blob = workbook([("Marks", sheet_xml(row(1, A="x")), drawing_xml(anchor))])
        self.assertIn("| x | [picture] |", self.grid(blob))

    # ------------------------------------------------------------- the row

    def test_a_row_that_exists_only_for_its_picture_appears_in_order(self):
        """Row 2 has no cells at all, so the sheet part has no `<row>` for it. The picture is on
        it, and a reader that only walks the rows the file lists would never see it."""
        blob = workbook([("Marks", sheet_xml(row(1, A="first"), row(3, A="third")),
                          drawing_xml(picture_on(0, 1)))])
        self.assertEqual(["--- sheet: Marks", "| first |", "| [picture] |", "| third |"],
                         self.grid(blob))

    def test_a_sheet_that_has_only_pictures_is_still_carried(self):
        blob = workbook([("OnlyImages", sheet_xml(), drawing_xml(picture_on(0, 0)))])
        self.assertEqual(["--- sheet: OnlyImages", "| [picture] |"], self.grid(blob))

    # ------------------------------------------------------------ the sheet

    def test_each_mark_is_under_the_heading_of_its_own_sheet(self):
        blob = workbook([
            ("Before", sheet_xml(row(1, A="a")), drawing_xml(picture_on(0, 0))),
            ("After", sheet_xml(row(1, A="b")), None),
        ])
        self.assertEqual(["--- sheet: Before", "| a [picture] |",
                          "--- sheet: After", "| b |"], self.grid(blob))

    # ---------------------------------------------------------- the account

    def test_how_many_pictures_sit_on_how_many_cells_is_said_once(self):
        blob = workbook([
            ("One", sheet_xml(row(1, A="a")), drawing_xml(picture_on(0, 0), picture_on(0, 0))),
            ("Two", sheet_xml(row(1, A="b")), drawing_xml(picture_on(1, 0))),
        ])
        said = self.notes_about_pictures(blob)
        self.assertEqual(1, len(said), said)
        self.assertIn("3 picture(s) sit on 2 cell(s)", said[0])
        self.assertIn("what a picture shows is not read", said[0])
        self.assertTrue(said[0].startswith("Book.xlsx: "), said[0])

    # ----------------------------------------------------------- not placed

    def test_a_picture_floating_at_a_page_position_is_counted_not_placed(self):
        floating = ("<xdr:absoluteAnchor><xdr:pos x=\"0\" y=\"0\"/><xdr:ext cx=\"1\" cy=\"1\"/>"
                    "<xdr:pic/><xdr:clientData/></xdr:absoluteAnchor>")
        blob = workbook([("Marks", sheet_xml(row(1, A="x")), drawing_xml(floating))])
        said = " ".join(self.notes_about_pictures(blob))
        self.assertIn("1 picture(s) in this workbook are not anchored on a cell", said)
        self.assertEqual(["--- sheet: Marks", "| x |"], self.grid(blob))

    def test_pictures_inside_a_group_are_counted_not_placed(self):
        group = (f"<xdr:twoCellAnchor>{at(0, 0)}<xdr:grpSp><xdr:pic/><xdr:pic/></xdr:grpSp>"
                 f"<xdr:clientData/></xdr:twoCellAnchor>")
        blob = workbook([("Marks", sheet_xml(row(1, A="x")), drawing_xml(group))])
        said = " ".join(self.notes_about_pictures(blob))
        self.assertIn("2 picture(s) in this workbook are not anchored on a cell", said)
        self.assertIn("1 drawn object(s)", said)

    def test_a_chart_or_a_shape_is_counted_as_not_read(self):
        chart = f"<xdr:twoCellAnchor>{at(0, 0)}<xdr:graphicFrame/><xdr:clientData/></xdr:twoCellAnchor>"
        shape = f"<xdr:twoCellAnchor>{at(1, 0)}<xdr:sp/><xdr:clientData/></xdr:twoCellAnchor>"
        blob = workbook([("Marks", sheet_xml(row(1, A="x")), drawing_xml(chart, shape))])
        said = " ".join(self.notes_about_pictures(blob))
        self.assertIn("2 drawn object(s) in this workbook other than pictures", said)

    def test_a_drawing_that_cannot_be_parsed_is_said_and_the_cells_are_still_read(self):
        blob = workbook([("Marks", sheet_xml(row(1, A="x")), "<xdr:wsDr")])
        got = self.read(blob)
        self.assertIn("| x |", got.text)
        self.assertTrue(any('1 drawing(s) referred to by sheet(s) "Marks"' in n
                            for n in got.notes), got.notes)

    def test_an_anchor_with_no_readable_cell_is_counted_not_placed(self):
        broken = ("<xdr:twoCellAnchor><xdr:from><xdr:col>x</xdr:col><xdr:row>y</xdr:row></xdr:from>"
                  "<xdr:pic/><xdr:clientData/></xdr:twoCellAnchor>")
        blob = workbook([("Marks", sheet_xml(row(1, A="x")), drawing_xml(broken))])
        said = " ".join(self.notes_about_pictures(blob))
        self.assertIn("1 picture(s) in this workbook are not anchored on a cell", said)

    def test_a_sheet_that_refers_to_a_drawing_the_file_does_not_hold_says_so_by_sheet_name(self):
        """⚠ This case used to assert the OPPOSITE -- that such a reference is ignored -- and so
        pinned the defect: the sheet says it has a drawing, the file has none to give, the
        pictures it held are gone, and the cells they sat on read as empty with no word said."""
        blob = workbook([("Marks", sheet_xml(row(1, A="x"), row(2, A="y")), drawing_xml(picture_on(1, 0)))])
        # Remove the drawing part and keep the reference to it.
        kept = io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(blob)) as src, zipfile.ZipFile(kept, "w") as dst:
            for item in src.infolist():
                if not item.filename.startswith("xl/drawings/"):
                    dst.writestr(item, src.read(item.filename))
        got = self.read(kept.getvalue())
        self.assertIn("| x |", got.text)
        said = " ".join(self.notes_about_pictures(kept.getvalue()))
        self.assertIn('1 drawing(s) referred to by sheet(s) "Marks"', said)
        self.assertIn("a cell that reads as empty there may have held one", said)
        self.assertNotIn("[picture]", got.text)

    def test_a_sheet_whose_links_cannot_be_read_says_so_rather_than_reading_as_having_none(self):
        """The relationship file is there and is not XML. Its sheet may have a drawing; that is
        not known, and 'none' is not the same answer."""
        blob = workbook([("Marks", sheet_xml(row(1, A="x")), drawing_xml(picture_on(1, 0)))])
        broken = io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(blob)) as src, zipfile.ZipFile(broken, "w") as dst:
            for item in src.infolist():
                data = src.read(item.filename)
                if item.filename == "xl/worksheets/_rels/sheet1.xml.rels":
                    data = b"<Relationships"
                dst.writestr(item, data)
        got = self.read(broken.getvalue())
        self.assertIn("| x |", got.text)
        said = " ".join(self.notes_about_pictures(broken.getvalue()))
        self.assertIn('referred to by sheet(s) "Marks"', said)

    def test_a_sheet_with_no_links_file_at_all_says_nothing(self):
        """A relationship file that is simply absent means the sheet has no drawing. That is a
        fact, not a loss, and saying otherwise would cry wolf at every plain sheet."""
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w") as z:
            z.writestr("xl/worksheets/sheet1.xml", sheet_xml(row(1, A="x")))
        got = self.read(buf.getvalue())
        self.assertIn("| x |", got.text)
        self.assertEqual([], self.notes_about_pictures(buf.getvalue()))

    def test_only_the_sheet_that_lost_its_drawing_is_named(self):
        good = workbook([("Whole", sheet_xml(row(1, A="a")), drawing_xml(picture_on(1, 0))),
                         ("Lost", sheet_xml(row(1, A="b")), drawing_xml(picture_on(1, 0)))])
        trimmed = io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(good)) as src, zipfile.ZipFile(trimmed, "w") as dst:
            for item in src.infolist():
                if item.filename != "xl/drawings/drawing2.xml":
                    dst.writestr(item, src.read(item.filename))
        said = " ".join(self.notes_about_pictures(trimmed.getvalue()))
        self.assertIn('"Lost"', said)
        self.assertNotIn('"Whole"', said)
        # the sheet that still has its drawing keeps its mark
        self.assertIn("| a | [picture] |", self.read(trimmed.getvalue()).text)

    # -------------------------------------------------------------- absence

    def test_a_workbook_with_no_drawing_says_nothing_about_pictures(self):
        blob = workbook([("Plain", sheet_xml(row(1, A="x", B="y")), None)])
        got = self.read(blob)
        self.assertEqual([], self.notes_about_pictures(blob))
        self.assertIn("| x | y |", got.text)
        self.assertNotIn("[picture]", got.text)

    def test_a_cell_whose_own_text_looks_like_a_mark_is_not_mistaken_for_one(self):
        """The mark is a statement about the grid; a cell that happens to say the same thing is
        still just a cell, and no note is made about pictures that are not there."""
        blob = workbook([("Plain", sheet_xml(row(1, A="[picture]")), None)])
        self.assertEqual([], self.notes_about_pictures(blob))


PICTURE = rich_parts(["_localImage"], values=[0], futures=[0], held=[0])


class APictureHeldInACellIsPlacedToo(Reads):
    """The other way a spreadsheet stores a picture: in the cell, as a rich value, rather than
    over the grid as a drawing. Its stored text is a placeholder error, so a reader that took
    the cell at its word read `#VALUE!` where the cell showed an image."""

    def test_a_picture_held_in_a_cell_marks_the_cell_and_its_placeholder_is_not_read(self):
        sheet = sheet_xml('<row r="1"><c r="A1" t="inlineStr"><is><t>x</t></is></c>'
                          f'{in_cell("B1", 1)}</row>')
        blob = workbook([("Held", sheet, None)], extra=PICTURE)
        got = self.read(blob)
        self.assertEqual(["--- sheet: Held", "| x | [picture] |"], self.grid(blob))
        self.assertNotIn("#VALUE!", got.text)

    def test_each_cell_is_resolved_on_its_own_not_all_taken_for_pictures(self):
        """vm 1 names a picture and vm 2 a rich value of another kind. Taking any `vm` for a
        picture would mark both; only the first is one."""
        extra = rich_parts(["_localImage", "_richtype"], values=[0, 1], futures=[0, 1], held=[0, 1])
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}{in_cell("B1", 2)}</row>')
        blob = workbook([("Held", sheet, None)], extra=extra)
        self.assertIn("| [picture] | #VALUE! |", self.read(blob).text)
        self.assertIn("1 picture(s) sit on 1 cell(s)", " ".join(self.notes_about_pictures(blob)))

    def test_a_rich_value_that_is_not_a_picture_keeps_its_text_and_is_counted(self):
        extra = rich_parts(["_richtype"], values=[0], futures=[0], held=[0])
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}</row>')
        got = self.read(workbook([("Held", sheet, None)], extra=extra))
        self.assertIn("| #VALUE! |", got.text)
        self.assertTrue(any("1 cell(s) in this workbook hold a rich value that is not a picture"
                            in n for n in got.notes), got.notes)

    def test_a_workbook_with_no_rich_data_parts_counts_the_cell_instead_of_guessing(self):
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}</row>')
        got = self.read(workbook([("Held", sheet, None)]))
        self.assertNotIn("[picture]", got.text)
        self.assertTrue(any("hold a rich value that is not a picture" in n for n in got.notes))

    def test_a_chain_that_cannot_be_followed_is_not_a_picture_and_does_not_raise(self):
        """The cell points at future metadata 5, which does not exist."""
        extra = rich_parts(["_localImage"], values=[0], futures=[0], held=[5])
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}</row>')
        got = self.read(workbook([("Held", sheet, None)], extra=extra))
        self.assertNotIn("[picture]", got.text)
        self.assertTrue(any("rich value" in n for n in got.notes), got.notes)

    def test_one_broken_chain_does_not_discard_the_cells_that_can_be_followed(self):
        """vm 1 is a picture; vm 2 points at future metadata 5, which does not exist."""
        extra = rich_parts(["_localImage"], values=[0], futures=[0], held=[0, 5])
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}{in_cell("B1", 2)}</row>')
        got = self.read(workbook([("Held", sheet, None)], extra=extra))
        self.assertIn("| [picture] | #VALUE! |", got.text)

    def test_a_metadata_type_that_is_not_a_rich_value_is_not_followed(self):
        extra = rich_parts(["_localImage"], values=[0], futures=[0], held=[0], kind="SOMETHINGELSE")
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}</row>')
        got = self.read(workbook([("Held", sheet, None)], extra=extra))
        self.assertNotIn("[picture]", got.text)

    def test_a_structure_named_for_a_web_image_is_a_picture_too(self):
        extra = rich_parts(["_webimage"], values=[0], futures=[0], held=[0])
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}</row>')
        self.assertIn("| [picture] |",
                      self.read(workbook([("Held", sheet, None)], extra=extra)).text)

    def test_two_cells_naming_one_picture_are_both_marked(self):
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}{in_cell("C1", 1)}</row>')
        got = self.read(workbook([("Held", sheet, None)], extra=PICTURE))
        self.assertIn("| [picture] |  | [picture] |", got.text)

    def test_a_held_picture_and_a_drawn_one_are_counted_together(self):
        sheet = sheet_xml(f'<row r="1">{in_cell("A1", 1)}</row>')
        blob = workbook([("Both", sheet, drawing_xml(picture_on(1, 0)))], extra=PICTURE)
        self.assertIn("| [picture] | [picture] |", self.read(blob).text)
        self.assertIn("2 picture(s) sit on 2 cell(s)", " ".join(self.notes_about_pictures(blob)))


if __name__ == "__main__":
    unittest.main()
