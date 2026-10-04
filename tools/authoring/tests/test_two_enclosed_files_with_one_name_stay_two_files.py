"""Two enclosed files that share a NAME and not a path are two files, and each is read as itself.

A document may carry `word/embeddings/first/Book.xlsx` and `word/embeddings/second/Book.xlsx`: different
workbooks, each referred to by its own object in the body. The reader keyed what it had read by the file's
base name, so the second `Book.xlsx` replaced the first, both marks in the text read `Book.xlsx`, and the
authoring tools answered without an error and without the first table (a review, 2026-10-05). On a
specification whose decision logic is in the attachment, that is the loss that looks most like success.

Asserted here, from what is read outward:

  distinct     two files with one name are both carried, each under a name that tells them apart
  marks        each object in the body is marked with the name of ITS file
  placed       the file a mark names holds the rows of the workbook that object refers to
  unique       a name that is unique is still the plain base name, so nothing else changes
  failures     one of the two that cannot be opened is named by the same name its mark carries
  slice        `Enclosed.carried` hands back each by that name, and a name it does not know is refused
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile

from sce_author.ingest import enclosed_marks, ingest, read_enclosed
from tests.test_a_document_says_where_an_enclosed_file_was_attached import PKG, R, W, obj, para
from tests.test_a_document_is_read_with_what_it_encloses import workbook
from tests.test_a_picture_on_an_enclosed_workbook_cell_is_not_lost_quietly import row, sheet_xml

FIRST = workbook(sheet_xml(row(1, A="FIRST_ONLY", B="11")))
SECOND = workbook(sheet_xml(row(1, A="SECOND_ONLY", B="22")))
TWIN_A, TWIN_B = "first/Book.xlsx", "second/Book.xlsx"


class TwoEnclosedFilesWithOneNameStayTwoFiles(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def document(self, files: dict[str, bytes], body: str, rels: dict[str, str]) -> pathlib.Path:
        xml = (f'<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="{W}" xmlns:r="{R}" '
               f'xmlns:o="urn:schemas-microsoft-com:office:office" xmlns:v="urn:schemas-microsoft-com:vml">'
               f"<w:body>{body}</w:body></w:document>")
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", xml)
            links = "".join(f'<Relationship Id="{i}" Type="{R}/oleObject" Target="{t}"/>' for i, t in rels.items())
            z.writestr("word/_rels/document.xml.rels", f'<Relationships xmlns="{PKG}">{links}</Relationships>')
            for name, blob in files.items():
                z.writestr(f"word/embeddings/{name}", blob)
        return path

    def twins(self) -> pathlib.Path:
        return self.document({TWIN_A: FIRST, TWIN_B: SECOND},
                             para("Before.", obj("rId1")) + para("Then.", obj("rId2")),
                             {"rId1": f"embeddings/{TWIN_A}", "rId2": f"embeddings/{TWIN_B}"})

    def read(self, path: pathlib.Path):
        with zipfile.ZipFile(path) as zf:
            return read_enclosed(zf)

    # -------------------------------------------------------------- distinct

    def test_both_files_are_carried(self):
        got = self.read(self.twins())
        self.assertEqual(2, len(got.rows))
        self.assertEqual(2, len(set(got.rows)), "the two must not share one name")

    def test_each_name_still_says_which_file_it_is(self):
        got = self.read(self.twins())
        self.assertEqual({TWIN_A, TWIN_B}, set(got.rows))

    # ----------------------------------------------------------------- marks

    def test_each_object_is_marked_with_the_name_of_its_own_file(self):
        got = self.read(self.twins())
        self.assertEqual({"rId1": TWIN_A, "rId2": TWIN_B}, got.attached)

    def test_the_marks_in_the_text_differ(self):
        text = ingest(self.twins()).text
        self.assertIn(f"[enclosed object: {TWIN_A}", text)
        self.assertIn(f"[enclosed object: {TWIN_B}", text)

    # ---------------------------------------------------------------- placed

    def test_the_file_a_mark_names_holds_that_objects_workbook(self):
        got = self.read(self.twins())
        self.assertTrue(any("FIRST_ONLY" in line for line in got.rows[TWIN_A]))
        self.assertFalse(any("SECOND_ONLY" in line for line in got.rows[TWIN_A]))
        self.assertTrue(any("SECOND_ONLY" in line for line in got.rows[TWIN_B]))
        self.assertFalse(any("FIRST_ONLY" in line for line in got.rows[TWIN_B]))

    def test_the_whole_document_reading_carries_both_tables_under_their_names(self):
        text = ingest(self.twins()).text
        self.assertIn(f"--- enclosed: {TWIN_A}", text)
        self.assertIn(f"--- enclosed: {TWIN_B}", text)
        self.assertIn("FIRST_ONLY", text)
        self.assertIn("SECOND_ONLY", text)

    def test_a_slice_asking_for_each_gets_its_own_rows(self):
        got = self.read(self.twins())
        first = "\n".join(got.carried([TWIN_A]))
        second = "\n".join(got.carried([TWIN_B]))
        self.assertIn("FIRST_ONLY", first)
        self.assertNotIn("SECOND_ONLY", first)
        self.assertIn("SECOND_ONLY", second)
        self.assertNotIn("FIRST_ONLY", second)

    def test_a_name_that_is_not_a_file_is_refused(self):
        got = self.read(self.twins())
        with self.assertRaises(KeyError):
            got.carried(["Book.xlsx"])

    # ---------------------------------------------------------------- unique

    def test_a_name_that_is_unique_is_the_plain_base_name(self):
        path = self.document({"first/Book.xlsx": FIRST, "second/Other.xlsx": SECOND},
                             para("", obj("rId1")) + para("", obj("rId2")),
                             {"rId1": "embeddings/first/Book.xlsx", "rId2": "embeddings/second/Other.xlsx"})
        self.assertEqual({"rId1": "Book.xlsx", "rId2": "Other.xlsx"}, self.read(path).attached)

    def test_files_in_the_embeddings_folder_itself_keep_their_plain_names(self):
        path = self.document({"A.xlsx": FIRST, "B.xlsx": SECOND}, para("", obj("rId1")),
                             {"rId1": "embeddings/A.xlsx"})
        self.assertEqual(["A.xlsx", "B.xlsx"], list(self.read(path).rows))

    def test_a_clash_renames_every_file_that_shares_the_name_not_only_the_later_one(self):
        path = self.document({"a/Book.xlsx": FIRST, "b/Book.xlsx": SECOND, "c/Book.xlsx": FIRST, "d/Alone.xlsx": SECOND},
                             para(""), {})
        names = set(self.read(path).rows)
        self.assertEqual({"a/Book.xlsx", "b/Book.xlsx", "c/Book.xlsx", "Alone.xlsx"}, names)

    def test_a_folder_entry_in_the_zip_is_not_a_file(self):
        """Some writers list `word/embeddings/first/` itself; it holds nothing to read and names nothing."""
        path = self.twins()
        with zipfile.ZipFile(path, "a") as z:
            z.writestr("word/embeddings/first/", b"")
        got = self.read(path)
        self.assertEqual({TWIN_A, TWIN_B}, set(got.rows))
        self.assertEqual([], got.unopened)

    # -------------------------------------------------------------- failures

    def test_one_of_two_that_cannot_be_opened_is_named_as_its_mark_names_it(self):
        path = self.document({TWIN_A: b"\x01not a zip", TWIN_B: SECOND},
                             para("", obj("rId1")) + para("", obj("rId2")),
                             {"rId1": f"embeddings/{TWIN_A}", "rId2": f"embeddings/{TWIN_B}"})
        got = self.read(path)
        self.assertEqual([TWIN_A], got.unopened)
        self.assertEqual([TWIN_B], list(got.rows))
        self.assertTrue(any(n.startswith(f"{TWIN_A}: ") for n in got.notes), got.notes)
        ingested = ingest(path)
        self.assertTrue(any(TWIN_A in n and "could not be opened" in n for n in ingested.notes), ingested.notes)

    # ------------------------------------------------------------------ mark

    def test_the_public_mark_spelling_uses_the_same_names(self):
        body = ET.fromstring(
            f'<w:body xmlns:w="{W}" xmlns:r="{R}" xmlns:o="urn:schemas-microsoft-com:office:office" '
            f'xmlns:v="urn:schemas-microsoft-com:vml">{para("", obj("rId2"))}</w:body>')[0]
        got = self.read(self.twins())
        self.assertEqual([f"[enclosed object: {TWIN_B} (Excel.Sheet.12)]"], enclosed_marks(body, got.attached))


if __name__ == "__main__":
    unittest.main()
