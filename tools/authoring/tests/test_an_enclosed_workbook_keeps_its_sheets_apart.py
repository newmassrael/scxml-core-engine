"""An enclosed workbook's sheets are told apart by name and read in the workbook's order.

A specification that hands a requirement to an attached spreadsheet hands it to a
sheet of it, and which sheet applies to the product being specified is a question
for a person, who can only be asked about a sheet by its name. A reader that
concatenated every sheet's rows under the stored file names gave that person
nothing to point at: nine grids ran together, ordered `sheet1`, `sheet10`,
`sheet2` -- the order of the characters in the file names, not of the sheets.

Asserted here, in the order a reader meets it:

  the name    every sheet that carries rows is headed by the name the workbook gives it
  the order   sheets come in the order the workbook lists them, not the file names'
  hidden      a sheet the workbook hides is carried, and said to be hidden
  fallback    a workbook whose list cannot be read, or a sheet it does not list, is read
              as it always was and named by its stored file
  empty       a sheet with no rows is not announced
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
  <w:body><w:p><w:r><w:t>The limits are in the attached sheet.</w:t></w:r></w:p></w:body>
</w:document>"""

MAIN = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
PKG = "http://schemas.openxmlformats.org/package/2006/relationships"


def sheet_xml(*texts: str, merged: str | None = None) -> str:
    """One row per text, in column A, as inline strings (no shared table to keep in step)."""
    rows = "".join(
        f'<row r="{n}"><c r="A{n}" t="inlineStr"><is><t>{text}</t></is></c></row>'
        for n, text in enumerate(texts, start=1))
    merge = f'<mergeCells count="1"><mergeCell ref="{merged}"/></mergeCells>' if merged else ""
    return (f'<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="{MAIN}">'
            f"<sheetData>{rows}</sheetData>{merge}</worksheet>")


def workbook(sheets: dict[str, str], listed: list[tuple[str, str, str]] | None = None,
             target_prefix: str = "worksheets/") -> bytes:
    """A workbook of `sheets` (stored part -> xml). `listed` is the workbook's own list as
    (name, part, state), or None for a package with no workbook part at all."""
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        for part, xml in sheets.items():
            z.writestr(f"xl/worksheets/{part}", xml)
        if listed is not None:
            entries = "".join(
                f'<sheet name="{name}" sheetId="{n}" r:id="rId{n}"'
                + (f' state="{state}"' if state else "") + "/>"
                for n, (name, _part, state) in enumerate(listed, start=1))
            z.writestr("xl/workbook.xml",
                       f'<workbook xmlns="{MAIN}" xmlns:r="{REL}"><sheets>{entries}</sheets></workbook>')
            rels = "".join(
                f'<Relationship Id="rId{n}" Type="{REL}/worksheet" '
                f'Target="{target_prefix}{part}"/>'
                for n, (_name, part, _state) in enumerate(listed, start=1))
            z.writestr("xl/_rels/workbook.xml.rels", f'<Relationships xmlns="{PKG}">{rels}</Relationships>')
    return buf.getvalue()


class AnEnclosedWorkbookKeepsItsSheetsApart(unittest.TestCase):
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

    def lines(self, blob: bytes) -> list[str]:
        return self.read(blob).text.splitlines()

    def three(self, **over):
        return workbook(
            {"sheet1.xml": sheet_xml("in alpha"),
             "sheet2.xml": sheet_xml("in beta"),
             "sheet10.xml": sheet_xml("in gamma")},
            over.get("listed", [("Beta", "sheet2.xml", ""), ("Gamma", "sheet10.xml", ""),
                                ("Alpha", "sheet1.xml", "")]))

    def test_every_sheet_with_rows_is_headed_by_the_name_the_workbook_gives_it(self):
        lines = self.lines(self.three())
        for name in ("Alpha", "Beta", "Gamma"):
            self.assertIn(f"--- sheet: {name}", lines)

    def test_a_row_sits_under_the_heading_of_its_own_sheet(self):
        lines = self.lines(self.three())
        for name, row in (("Alpha", "| in alpha |"), ("Beta", "| in beta |"),
                          ("Gamma", "| in gamma |")):
            at = lines.index(f"--- sheet: {name}")
            self.assertEqual(row, lines[at + 1], name)

    def test_the_sheets_come_in_the_order_the_workbook_lists_them(self):
        """Stored as sheet1, sheet10, sheet2; listed Beta (sheet2), Gamma (sheet10), Alpha
        (sheet1). The file names' order is Alpha, Gamma, Beta -- the order a reader that
        sorted the files would have produced."""
        lines = self.lines(self.three())
        headings = [l for l in lines if l.startswith("--- sheet:")]
        self.assertEqual(["--- sheet: Beta", "--- sheet: Gamma", "--- sheet: Alpha"], headings)

    def test_a_target_written_from_the_package_root_resolves_too(self):
        blob = workbook({"sheet1.xml": sheet_xml("in alpha")},
                        [("Alpha", "sheet1.xml", "")], target_prefix="/xl/worksheets/")
        self.assertIn("--- sheet: Alpha", self.lines(blob))

    def test_a_hidden_sheet_is_carried_and_said_to_be_hidden(self):
        blob = workbook({"sheet1.xml": sheet_xml("seen"), "sheet2.xml": sheet_xml("unseen")},
                        [("Visible", "sheet1.xml", ""), ("Secret", "sheet2.xml", "hidden")])
        lines = self.lines(blob)
        self.assertIn("--- sheet: Secret (hidden)", lines)
        self.assertIn("| unseen |", lines)
        self.assertIn("--- sheet: Visible", lines)

    def test_a_very_hidden_sheet_is_said_to_be_hidden_too(self):
        blob = workbook({"sheet1.xml": sheet_xml("x")}, [("Deep", "sheet1.xml", "veryHidden")])
        self.assertIn("--- sheet: Deep (hidden)", self.lines(blob))

    def test_a_sheet_with_no_rows_is_not_announced(self):
        blob = workbook({"sheet1.xml": sheet_xml(), "sheet2.xml": sheet_xml("has a row")},
                        [("Blank", "sheet1.xml", ""), ("Full", "sheet2.xml", "")])
        headings = [l for l in self.lines(blob) if l.startswith("--- sheet:")]
        self.assertEqual(["--- sheet: Full"], headings)

    def test_a_workbook_with_no_list_is_read_by_its_stored_files(self):
        blob = workbook({"sheet1.xml": sheet_xml("one"), "sheet2.xml": sheet_xml("two")}, None)
        headings = [l for l in self.lines(blob) if l.startswith("--- sheet:")]
        self.assertEqual(["--- sheet: sheet1.xml", "--- sheet: sheet2.xml"], headings)

    def test_a_sheet_the_workbook_does_not_list_is_still_read_after_the_listed_ones(self):
        blob = workbook({"sheet1.xml": sheet_xml("listed"), "sheet2.xml": sheet_xml("unlisted")},
                        [("Named", "sheet1.xml", "")])
        lines = self.lines(blob)
        self.assertEqual(["--- sheet: Named", "--- sheet: sheet2.xml"],
                         [l for l in lines if l.startswith("--- sheet:")])
        self.assertIn("| unlisted |", lines)

    def test_a_list_that_points_at_a_part_that_is_not_there_does_not_hide_the_stored_sheet(self):
        blob = workbook({"sheet1.xml": sheet_xml("stored")},
                        [("Ghost", "sheet9.xml", ""), ("Real", "sheet1.xml", "")])
        self.assertEqual(["--- sheet: Real"],
                         [l for l in self.lines(blob) if l.startswith("--- sheet:")])

    def test_a_workbook_part_that_cannot_be_parsed_falls_back_rather_than_raising(self):
        buf = io.BytesIO()
        with zipfile.ZipFile(buf, "w") as z:
            z.writestr("xl/worksheets/sheet1.xml", sheet_xml("still here"))
            z.writestr("xl/workbook.xml", "<workbook><sheets>")
        got = self.read(buf.getvalue())
        self.assertIn("| still here |", got.text)
        self.assertIn("--- sheet: sheet1.xml", got.text)

    def test_a_merged_range_is_reported_against_the_sheet_by_name(self):
        blob = workbook({"sheet1.xml": sheet_xml("a", "b", merged="A1:A2")},
                        [("Limits", "sheet1.xml", "")])
        said = " ".join(self.read(blob).notes)
        self.assertIn('sheet "Limits"', said)

    def test_a_name_that_is_not_ascii_is_kept_as_written(self):
        blob = workbook({"sheet1.xml": sheet_xml("x")}, [("시트 하나 (예시)", "sheet1.xml", "")])
        self.assertIn("--- sheet: 시트 하나 (예시)", self.lines(blob))


if __name__ == "__main__":
    unittest.main()
