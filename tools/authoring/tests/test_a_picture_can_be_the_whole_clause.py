"""Saying WHERE the unread pictures are, not just how many.

A reader of text cannot say what a picture shows. That is a real limit and it
is not going away: this core makes no model calls, on purpose, because its
answers have to be the same answer twice.

⚠ BUT "71 pictures were not read" IS NOT AN ANSWER EITHER. It leaves a person
holding two piles they cannot tell apart -- a screenshot sitting beside a
paragraph that already says the rule, and a diagram the clause hands its whole
content to. The first is layout. The second is the requirement, and every
command downstream will run clean without it.

So the reader answers the part it CAN answer mechanically: whether the clause
around the picture says anything at all. That is not a reading of the picture
and does not pretend to be one -- it is the question a person is owed, with
the clause number attached so they can go and look.

⚠⚠ A cheaper version of this was built first and measured wrong; the note in
`_clauses_carried_by_a_picture` records it. These cases exist to keep the
discriminator honest: the silent clause and the talkative one are in the SAME
document, so a check that simply reported every picture would fail here.
"""

from __future__ import annotations

import io
import pathlib
import tempfile
import unittest
import zipfile

from sce_author.ingest import ingest

W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'


def paragraph(text: str = "", drawn: bool = False) -> str:
    run = f"<w:r><w:t>{text}</w:t></w:r>" if text else ""
    art = "<w:r><w:drawing><w:inline/></w:drawing></w:r>" if drawn else ""
    return f"<w:p>{run}{art}</w:p>"


def document(*paragraphs: str) -> str:
    return (f'<?xml version="1.0" encoding="UTF-8"?>'
            f"<w:document {W}><w:body>{''.join(paragraphs)}</w:body>"
            f"</w:document>")


class APictureCanBeTheWholeClause(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def spec(self, body: str, media: bool = True) -> pathlib.Path:
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml", body)
            if media:
                z.writestr("word/media/image1.png", b"\x89PNG not really")
        return path

    def note(self, path: pathlib.Path) -> str:
        return " ".join(n for n in ingest(path).notes if "picture" in n)

    # ------------------------------------------------- the two piles, apart

    def test_a_clause_that_says_nothing_but_shows_a_picture_is_named(self):
        said = self.note(self.spec(document(
            paragraph("3.1 The lamp"),
            paragraph("The lamp is on while the door is open."),
            paragraph(drawn=True),
            paragraph("3.2 The diagnostic update sequence"),
            paragraph(drawn=True),
        )))
        self.assertIn("3.2", said)
        self.assertNotIn("3.1", said,
                         "3.1 states its rule in text; naming it is noise")

    def test_a_document_whose_pictures_are_all_illustrations_says_so(self):
        """⚠ The discriminator. Without this, a note that always named
        something would pass the case above while meaning nothing.
        """
        said = self.note(self.spec(document(
            paragraph("3.1 The lamp"),
            paragraph("The lamp is on while the door is open."),
            paragraph(drawn=True),
        )))
        self.assertIn("No numbered clause", said)

    # --------------------------------------------- telling none from unknown

    def test_a_document_without_numbering_does_not_claim_none(self):
        """"No clause hands its content to a picture" is trivially true of a
        document that has no clauses, and would be read as reassurance.
        """
        said = self.note(self.spec(document(
            paragraph("The lamp is on while the door is open."),
            paragraph(drawn=True),
        )))
        self.assertIn("no numbered clauses", said)
        self.assertNotIn("No numbered clause hands", said)

    def test_pictures_before_the_first_clause_are_counted_as_unplaced(self):
        said = self.note(self.spec(document(
            paragraph(drawn=True),
            paragraph("3.1 The lamp"),
            paragraph("The lamp is on while the door is open."),
        )))
        self.assertIn("sit outside the numbering", said)

    # ------------------------------------------------------- the limit held

    def test_the_reader_never_says_what_a_picture_shows(self):
        """The line this whole file sits on. The report names a place; it
        does not describe, summarise or interpret the image.
        """
        got = ingest(self.spec(document(
            paragraph("3.2 The diagnostic update sequence"),
            paragraph(drawn=True),
        )))
        said = " ".join(got.notes)
        for invented in ("sequence", "diagram", "shows a", "appears to"):
            self.assertNotIn(invented, said.replace("3.2 ", ""),
                             f"the reader described the image: {said}")

    def test_a_document_with_no_pictures_says_nothing_about_them(self):
        got = ingest(self.spec(document(
            paragraph("3.1 The lamp"),
            paragraph("The lamp is on while the door is open."),
        ), media=False))
        self.assertEqual([], [n for n in got.notes if "picture" in n])


if __name__ == "__main__":
    unittest.main()
