"""A document says, at the place an enclosed file was attached, which file it is.

A specification that hands a requirement to an attached spreadsheet keeps a placeholder
picture and a reference where the attachment sits, and the spreadsheet's rows are carried
below the body, under the file's name. Read as text, the sentence "the limits are in the
attached sheet" and the sheet it means were therefore apart: with several enclosed files,
nothing said which one a clause handed its requirement to, and a reader sizing a slice of the
document (one component's clauses out of a long specification) could not tell which of the
files belonged to it.

Asserted here, in the order a reader meets it:

  the place    the paragraph that carries an object says `[enclosed object: NAME (KIND)]`,
               in the text where the object sits, by the name the file is carried under
  the table    an object in a table cell says it inside that cell, in the row
  no file      an object that names no enclosed file -- a picture of an equation, an
               undeclared reference -- is not marked, and nothing raises
  the rest     the file is still carried once, under its own name, and every other line
               reads as it did
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest
import zipfile

from sce_author.ingest import ingest
from tests.test_a_document_is_read_with_what_it_encloses import deck, workbook

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
PKG = "http://schemas.openxmlformats.org/package/2006/relationships"


def para(text: str = "", *objects: str) -> str:
    run = f"<w:r><w:t>{text}</w:t></w:r>" if text else ""
    return f"<w:p>{run}{''.join(f'<w:r>{o}</w:r>' for o in objects)}</w:p>"


def obj(rid: str | None, prog: str | None = "Excel.Sheet.12") -> str:
    """An embedded object as Word stores one: a placeholder shape and the OLE reference."""
    kind = f' ProgID="{prog}"' if prog else ""
    link = f' r:id="{rid}"' if rid else ""
    return (f'<w:object><v:shape id="s"/><o:OLEObject Type="Embed"{kind} '
            f'ShapeID="_x0000_i1025" DrawAspect="Icon"{link}/></w:object>')


def table(*rows: list[str]) -> str:
    """A cell given as plain text becomes a paragraph of it; one that is already markup is kept."""
    def cell(content: str) -> str:
        return content if content.startswith("<") else para(content)

    return "<w:tbl>" + "".join(
        "<w:tr>" + "".join(f"<w:tc>{cell(c)}</w:tc>" for c in row) + "</w:tr>" for row in rows
    ) + "</w:tbl>"


class ADocumentSaysWhereAnEnclosedFileWasAttached(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def read(self, body: str, relationships: dict[str, str] | None = None, **enclosed: bytes):
        """`relationships` maps an id to its target inside word/ (e.g. embeddings/Book.xlsx);
        None writes no relationships part at all."""
        document = (f'<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="{W}" '
                    f'xmlns:r="{R}" xmlns:o="urn:schemas-microsoft-com:office:office" '
                    f'xmlns:v="urn:schemas-microsoft-com:vml"><w:body>{body}</w:body></w:document>')
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", document)
            if relationships is not None:
                rels = "".join(f'<Relationship Id="{rid}" Type="{R}/oleObject" Target="{target}"/>'
                               for rid, target in relationships.items())
                z.writestr("word/_rels/document.xml.rels",
                           f'<Relationships xmlns="{PKG}">{rels}</Relationships>')
            for name, blob in enclosed.items():
                z.writestr(f"word/embeddings/{name}", blob)
        return ingest(path)

    def lines(self, *args, **kw) -> list[str]:
        return self.read(*args, **kw).text.splitlines()

    # ------------------------------------------------------------ the place

    def test_the_mark_sits_in_the_text_where_the_object_is(self):
        lines = self.lines(para("Before.") + para("", obj("rId5")) + para("After."),
                           {"rId5": "embeddings/Book.xlsx"}, **{"Book.xlsx": workbook()})
        at = lines.index("[enclosed object: Book.xlsx (Excel.Sheet.12)]")
        self.assertEqual("Before.", lines[at - 1])
        self.assertEqual("After.", lines[at + 1])

    def test_the_mark_is_on_the_paragraph_that_also_has_text(self):
        lines = self.lines(para("The limits are in the attached sheet.", obj("rId5")),
                           {"rId5": "embeddings/Book.xlsx"}, **{"Book.xlsx": workbook()})
        self.assertIn("The limits are in the attached sheet. "
                      "[enclosed object: Book.xlsx (Excel.Sheet.12)]", lines)

    def test_an_object_that_says_no_kind_is_marked_without_one(self):
        lines = self.lines(para("", obj("rId5", prog=None)),
                           {"rId5": "embeddings/Book.xlsx"}, **{"Book.xlsx": workbook()})
        self.assertIn("[enclosed object: Book.xlsx]", lines)

    def test_two_objects_in_one_paragraph_are_marked_in_order(self):
        lines = self.lines(para("See", obj("rId1"), obj("rId2", "PowerPoint.Show.12")),
                           {"rId1": "embeddings/A.xlsx", "rId2": "embeddings/B.pptx"},
                           **{"A.xlsx": workbook(), "B.pptx": deck()})
        self.assertIn("See [enclosed object: A.xlsx (Excel.Sheet.12)] "
                      "[enclosed object: B.pptx (PowerPoint.Show.12)]", lines)

    def test_two_references_to_one_file_are_both_marked_and_the_file_is_carried_once(self):
        got = self.read(para("", obj("rId1")) + para("", obj("rId2")),
                        {"rId1": "embeddings/Book.xlsx", "rId2": "embeddings/Book.xlsx"},
                        **{"Book.xlsx": workbook()})
        lines = got.text.splitlines()
        self.assertEqual(2, lines.count("[enclosed object: Book.xlsx (Excel.Sheet.12)]"))
        self.assertEqual(1, lines.count("--- enclosed: Book.xlsx"))

    def test_an_object_the_reader_cannot_open_is_marked_by_name_too(self):
        """The mark points at the file; whether it could be opened is said where it always was."""
        got = self.read(para("", obj("rId1", "Equation.3")),
                        {"rId1": "embeddings/oleObject1.bin"}, **{"oleObject1.bin": b"\x01not a zip"})
        self.assertIn("[enclosed object: oleObject1.bin (Equation.3)]", got.text.splitlines())
        self.assertTrue(any("oleObject1.bin" in n and "could not be opened" in n for n in got.notes))

    # ------------------------------------------------------------ the table

    def test_an_object_in_a_table_cell_is_marked_inside_its_row(self):
        body = table(["Limit", para("", obj("rId5"))], ["Other", para("x")])
        lines = self.lines(body, {"rId5": "embeddings/Book.xlsx"}, **{"Book.xlsx": workbook()})
        self.assertIn("| Limit | [enclosed object: Book.xlsx (Excel.Sheet.12)] |", lines)
        self.assertIn("| Other | x |", lines)

    # -------------------------------------------------------------- no file

    def test_an_object_with_no_reference_is_not_marked(self):
        lines = self.lines(para("Equation:", obj(None, "Equation.3")), {"rId5": "embeddings/Book.xlsx"},
                           **{"Book.xlsx": workbook()})
        self.assertIn("Equation:", lines)
        self.assertEqual([], [l for l in lines if l.startswith("[enclosed object")])

    def test_a_reference_that_is_not_declared_is_not_marked_and_does_not_raise(self):
        lines = self.lines(para("", obj("rId99")), {"rId5": "embeddings/Book.xlsx"},
                           **{"Book.xlsx": workbook()})
        self.assertEqual([], [l for l in lines if l.startswith("[enclosed object")])

    def test_a_reference_to_something_that_is_not_an_enclosed_file_is_not_marked(self):
        lines = self.lines(para("", obj("rId7")), {"rId7": "media/image1.png"},
                           **{"Book.xlsx": workbook()})
        self.assertEqual([], [l for l in lines if l.startswith("[enclosed object")])

    def test_a_document_with_no_relationships_part_reads_as_before(self):
        got = self.read(para("Plain.") + para("", obj("rId5")), None, **{"Book.xlsx": workbook()})
        self.assertIn("Plain.", got.text)
        self.assertNotIn("[enclosed object", got.text)
        self.assertIn("--- enclosed: Book.xlsx", got.text)

    # --------------------------------------------------------------- the rest

    def test_the_file_is_still_carried_below_under_its_own_name(self):
        lines = self.lines(para("", obj("rId5")), {"rId5": "embeddings/Book.xlsx"},
                           **{"Book.xlsx": workbook()})
        at = lines.index("--- enclosed: Book.xlsx")
        self.assertGreater(at, lines.index("[enclosed object: Book.xlsx (Excel.Sheet.12)]"))
        self.assertIn("| CONDITION | SHOWS |", lines[at:])

    def test_a_document_that_attaches_nothing_reads_exactly_as_before(self):
        lines = self.lines(para("One.") + table(["a", "b"]) + para("Two."), {})
        self.assertEqual(["One.", "| a | b |", "a", "b", "Two."], lines)


if __name__ == "__main__":
    unittest.main()
