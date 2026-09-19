"""A specification that hands a requirement to an attachment.

An office document can ENCLOSE other documents: a table pasted from a
spreadsheet is stored as the whole spreadsheet, and the body text keeps only a
reference to it. A reader that opens the document part alone sees a complete,
readable specification with a hole in it -- and succeeds.

⚠ THIS IS THE LOSS THAT LOOKS MOST LIKE SUCCESS, and it was measured before it
was fixed: on one 22,669-line specification, eleven enclosed files held 732
spreadsheet rows and 14 slides, the body handed its requirements to them, and
nothing had ever opened one. Every command downstream ran clean on the
remainder.

Nothing about reading them needs guessing: `.xlsx` and `.pptx` are zips of
XML, so the rows come out mechanically. What needs care is the GRID, which is
why these cases exist.
"""

from __future__ import annotations

import io
import pathlib
import tempfile
import unittest
import zipfile

from sce_author.ingest import IngestError, ingest

DOCUMENT_XML = """<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>The limits are in the attached sheet.</w:t></w:r></w:p></w:body>
</w:document>"""

SHARED = """<?xml version="1.0" encoding="UTF-8"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <si><t>CONDITION</t></si><si><t>SHOWS</t></si>
  <si><t>over</t></si><si><t>RED</t></si>
  <si><t>under</t></si><si><t>GREEN</t></si>
</sst>"""

# ⚠ Row 3 omits its first cell, which is what a sparse sheet looks like on
# disk. A reader that places cells in the order the file lists them puts
# `GREEN` in column A and silently reports a different rule.
SHEET = """<?xml version="1.0" encoding="UTF-8"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row>
    <row r="2"><c r="A2" t="s"><v>2</v></c><c r="B2" t="s"><v>3</v></c></row>
    <row r="3"><c r="B3" t="s"><v>5</v></c></row>
  </sheetData>
</worksheet>"""

SLIDE = """<?xml version="1.0" encoding="UTF-8"?>
<sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:p><a:r><a:t>the lamp follows the sensor</a:t></a:r></a:p>
</sld>"""


def workbook(sheet: str = SHEET) -> bytes:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("xl/sharedStrings.xml", SHARED)
        z.writestr("xl/worksheets/sheet1.xml", sheet)
    return buf.getvalue()


def deck() -> bytes:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("ppt/slides/slide1.xml", SLIDE)
    return buf.getvalue()


class ADocumentIsReadWithWhatItEncloses(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def document(self, **enclosed: bytes) -> pathlib.Path:
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", DOCUMENT_XML)
            for name, blob in enclosed.items():
                z.writestr(f"word/embeddings/{name}", blob)
        return path

    def test_an_enclosed_workbook_is_carried_not_merely_named(self):
        got = ingest(self.document(**{"Sheet.xlsx": workbook()}))
        self.assertIn("CONDITION", got.text)
        self.assertIn("RED", got.text)
        self.assertIn("The limits are in the attached sheet.", got.text)
        self.assertTrue(any("WERE opened" in n for n in got.notes))

    def test_a_sparse_row_keeps_its_columns(self):
        """⚠ The defect this reader exists to avoid.

        A row that omits an empty cell reads one column short if cells are
        placed in file order, so every value after the gap shifts left and
        `under -> GREEN` becomes `GREEN -> (nothing)`. The grid IS the logic
        in these documents: shifting it produces a different rule with no sign
        that anything happened.
        """
        got = ingest(self.document(**{"Sheet.xlsx": workbook()}))
        rows = [l for l in got.text.splitlines() if l.startswith("|")]
        self.assertIn("|  | GREEN |", rows,
                      f"the empty first column was dropped: {rows}")

    def test_an_enclosed_deck_is_carried_in_slide_order(self):
        got = ingest(self.document(**{"Deck.pptx": deck()}))
        self.assertIn("the lamp follows the sensor", got.text)

    def test_merged_cells_are_counted_rather_than_unmerged(self):
        """⚠ Reported, never repaired. Which rows a merge was meant to cover
        is exactly the guess this reader refuses: the value sits in the first
        cell of the range and the rest are empty, and inventing the repeats
        would manufacture rules nobody wrote.
        """
        merged = SHEET.replace(
            "</sheetData>",
            "</sheetData><mergeCells count=\"1\">"
            "<mergeCell ref=\"A1:A2\"/></mergeCells>")
        got = ingest(self.document(**{"Sheet.xlsx": workbook(merged)}))
        self.assertTrue(any("merged cell range" in n for n in got.notes))

    def test_what_cannot_be_opened_is_still_named(self):
        """An OLE blob is not a zip. It is reported by name, not skipped."""
        got = ingest(self.document(**{"oleObject1.bin": b"\x01\x02not a zip"}))
        said = " ".join(got.notes)
        self.assertIn("oleObject1.bin", said)
        self.assertIn("could not be opened", said)

    def test_a_broken_workbook_reports_rather_than_raising(self):
        got = ingest(self.document(**{"Sheet.xlsx": b"PK\x03\x04 truncated"}))
        self.assertTrue(got.text.strip(), "the body must still be readable")
        self.assertTrue(any("Sheet.xlsx" in n for n in got.notes))

    def test_a_document_that_encloses_nothing_says_nothing_about_it(self):
        got = ingest(self.document())
        self.assertFalse([n for n in got.notes if "enclosed" in n])


if __name__ == "__main__":
    unittest.main()
