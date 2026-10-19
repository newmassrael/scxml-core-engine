"""A Word document is divided into the parts an owner chooses between by what the file says is a heading.

ADR 0039 divides a document by its headings, and a Word file says which paragraphs are headings in its styles,
not in its text: a paragraph set in `Heading 1` reads as any other paragraph. The extractor's reader of Word
therefore writes those paragraphs as headings, and the prose reader (a brief for a model) is left as it was.
Each case that must find a heading is paired with the nearest paragraph that must not be one: a reader that
makes everything a heading divides nothing, and one that makes nothing a heading is the defect this fixes.
"""

from __future__ import annotations

import io
import pathlib
import tempfile
import unittest
import zipfile

from sce_author.extractors import builtin
from sce_author.ingest import ingest
from sce_author.scope import outline

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"


def paragraph(text: str, style: str | None = None, outline_level: int | None = None) -> str:
    props = ""
    if style is not None:
        props += f'<w:pStyle w:val="{style}"/>'
    if outline_level is not None:
        props += f'<w:outlineLvl w:val="{outline_level}"/>'
    return f"<w:p>{f'<w:pPr>{props}</w:pPr>' if props else ''}<w:r><w:t>{text}</w:t></w:r></w:p>"


def style(style_id: str, name: str, based_on: str | None = None, outline_level: int | None = None) -> str:
    inside = f'<w:name w:val="{name}"/>'
    if based_on is not None:
        inside += f'<w:basedOn w:val="{based_on}"/>'
    if outline_level is not None:
        inside += f'<w:pPr><w:outlineLvl w:val="{outline_level}"/></w:pPr>'
    return f'<w:style w:type="paragraph" w:styleId="{style_id}">{inside}</w:style>'


STYLES = "".join([
    style("Title", "Title"),
    style("Heading1", "heading 1"),
    style("Heading2", "heading 2", based_on="Normal"),
    # A style an owner made, based on a heading: it is a heading at that level.
    style("ClauseTitle", "Clause title", based_on="Heading2"),
    # A style that sets its own outline level, with no heading in its name or ancestry.
    style("Banner", "Banner", outline_level=0),
    # An outline level of 9 is body text, which is not a level.
    style("Plain", "Plain", outline_level=9),
    style("Normal", "Normal"),
])


def docx(body: str, styles: str | bytes | None = STYLES) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        archive.writestr(
            "word/document.xml",
            f'<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="{W}"><w:body>{body}</w:body></w:document>',
        )
        if styles is not None:
            data = styles if isinstance(styles, bytes) else (
                f'<?xml version="1.0" encoding="UTF-8"?><w:styles xmlns:w="{W}">{styles}</w:styles>'
            ).encode("utf-8")
            archive.writestr("word/styles.xml", data)
    return buffer.getvalue()


class Folder(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.root = pathlib.Path(self._temp.name)

    def write(self, body: str, styles: str | bytes | None = STYLES) -> pathlib.Path:
        path = self.root / "spec.docx"
        path.write_bytes(docx(body, styles))
        return path

    def blocks(self, body: str, styles: str | bytes | None = STYLES):
        return builtin().extract(self.write(body, styles)).blocks


class TheStylesOfAWordFileMakeItsHeadings(Folder):
    def test_the_built_in_headings_and_the_title_are_headings_at_their_levels(self):
        found = self.blocks(
            paragraph("Controller", "Title")
            + paragraph("Cover text.")
            + paragraph("Lamp", "Heading1")
            + paragraph("The lamp is on.")
            + paragraph("Timing", "Heading2")
            + paragraph("It turns off after 30 seconds.")
        )

        self.assertEqual([(b.kind, b.level, b.text) for b in found], [
            ("heading", 1, "Controller"),
            ("paragraph", None, "Cover text."),
            ("heading", 1, "Lamp"),
            ("paragraph", None, "The lamp is on."),
            ("heading", 2, "Timing"),
            ("paragraph", None, "It turns off after 30 seconds."),
        ])

    def test_a_style_based_on_a_heading_is_a_heading_at_that_level(self):
        found = self.blocks(paragraph("Delay", "ClauseTitle") + paragraph("Five seconds.", "Normal"))

        self.assertEqual([(b.kind, b.level) for b in found], [("heading", 2), ("paragraph", None)])

    def test_an_outline_level_in_a_style_or_in_the_paragraph_is_a_heading(self):
        found = self.blocks(
            paragraph("Banner text", "Banner")
            + paragraph("Set by hand", outline_level=2)
            + paragraph("An outline level of body text", "Plain")
            + paragraph("Set to body text by hand", outline_level=9)
        )

        self.assertEqual([(b.kind, b.level) for b in found], [
            ("heading", 1), ("heading", 3), ("paragraph", None), ("paragraph", None),
        ])

    def test_a_paragraph_in_no_heading_style_is_not_one_whatever_it_says(self):
        found = self.blocks(paragraph("Heading 1", "Normal") + paragraph("Lamp"))

        self.assertEqual([b.kind for b in found], ["paragraph", "paragraph"])

    def test_the_built_in_names_are_read_whatever_language_the_document_was_written_in(self):
        # Word writes `heading 1` in the file whatever its interface language is; the id is the author's.
        # The style id and the texts are Hangul, spelled as code points so this source stays ASCII:
        # a localized id (two syllables and a digit), a two-syllable title and a short sentence.
        local_id = chr(0xC81C) + chr(0xBAA9) + "1"
        title = chr(0xB7A8) + chr(0xD504)
        sentence = chr(0xCF1C) + chr(0xC838) + " " + chr(0xC788) + chr(0xB2E4) + "."
        styles = style(local_id, "heading 1")

        found = self.blocks(paragraph(title, local_id) + paragraph(sentence), styles)

        self.assertEqual([(b.kind, b.level, b.text) for b in found], [
            ("heading", 1, title), ("paragraph", None, sentence),
        ])

    def test_a_deep_heading_is_the_sixth_level_at_most(self):
        styles = style("Heading8", "heading 8")

        found = self.blocks(paragraph("Deep", "Heading8"), styles)

        self.assertEqual(found[0].level, 6)

    def test_a_loop_of_styles_is_not_followed_for_ever(self):
        styles = style("A", "A", based_on="B") + style("B", "B", based_on="A")

        found = self.blocks(paragraph("Looped", "A"), styles)

        self.assertEqual(found[0].kind, "paragraph")

    def test_a_document_with_no_styles_part_is_read_by_the_ids_of_the_built_in_headings(self):
        found = self.blocks(paragraph("Lamp", "Heading1") + paragraph("On."), styles=None)

        self.assertEqual([b.kind for b in found], ["heading", "paragraph"])

    def test_a_styles_part_that_is_not_xml_is_a_document_without_one_and_not_a_failure(self):
        found = self.blocks(paragraph("Lamp", "Heading1") + paragraph("On."), styles=b"not xml at all")

        self.assertEqual([b.kind for b in found], ["heading", "paragraph"])

    def test_an_empty_paragraph_in_a_heading_style_is_not_a_heading(self):
        found = self.blocks(paragraph("", "Heading1") + paragraph("Words."))

        self.assertEqual([b.kind for b in found], ["paragraph"])


class AWordDocumentIsDividedByThem(Folder):
    def test_the_outline_has_a_part_for_each_function_and_the_part_under_it(self):
        path = self.write(
            paragraph("Controller", "Title")
            + paragraph("Lamp", "Heading1")
            + paragraph("On while supplied.")
            + paragraph("Timing", "Heading2")
            + paragraph("Off after 30 seconds.")
            + paragraph("Fan", "Heading1")
            + paragraph("Runs above 40 degrees.")
        )

        way, sections = outline(builtin().extract(path))

        self.assertEqual(way, "headings")
        self.assertEqual([(s.title, s.parent) for s in sections], [
            ("Controller", None), ("Lamp", None), ("Timing", "s2"), ("Fan", None),
        ])

    def test_the_prose_reading_of_the_same_file_is_as_it_was(self):
        # A brief for a model reads the text of the document, and writes no heading marks of its own into it.
        path = self.write(paragraph("Lamp", "Heading1") + paragraph("The lamp is on."))

        text = ingest(path).text

        self.assertNotIn("#", text)
        self.assertEqual(text.split(), ["Lamp", "The", "lamp", "is", "on."])

    def test_a_document_converted_by_rule_keeps_its_headings_and_the_check_finds_it_complete(self):
        from sce_author.conversion import check_conversion, convert_by_rules

        extraction = builtin().extract(self.write(
            paragraph("Lamp", "Heading1") + paragraph("The lamp is on.") + paragraph("Timing", "Heading2")
            + paragraph("Off after 30 seconds.")
        ))

        made = convert_by_rules(extraction)

        self.assertTrue(check_conversion(extraction, made).complete)
        self.assertEqual(made.markdown().splitlines()[0::2], ["# Lamp", "The lamp is on.", "## Timing", "Off after 30 seconds."])


if __name__ == "__main__":
    unittest.main()
