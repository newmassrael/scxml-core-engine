"""Which lines of a document are headings is the file's to say where it can say it.

An outline divides a document by its headings (ADR 0039), so a line wrongly read as one splits the document
where it should not be split, and a heading read as a paragraph leaves a part that cannot be chosen. Text that
is Markdown says its headings by `#`; an HTML page by `<h1>` to `<h6>`; a Word file by its styles, and for
those two the text a reader writes is not where the structure lives. Each case that must find a heading is
paired with the nearest line that begins as one and must not be.
"""

from __future__ import annotations

import io
import pathlib
import tempfile
import unittest
import zipfile

from sce_author.extractors import blocks_of_text, builtin
from sce_author.scope import outline

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"


class Folder(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.root = pathlib.Path(self._temp.name)

    def extract(self, name: str, data: bytes | str):
        path = self.root / name
        path.write_bytes(data if isinstance(data, bytes) else data.encode("utf-8"))
        return builtin().extract(path)


class HtmlSaysItsHeadings(Folder):
    def test_h1_to_h6_are_headings_at_their_levels_and_the_rest_is_paragraphs(self):
        page = (
            "<html><body><h1>Lamp</h1><p>On while supplied.</p><h2>Timing</h2><p>Off after 30 seconds.</p>"
            "<h3>Delay</h3><p>Five seconds.</p></body></html>"
        )

        done = self.extract("spec.html", page)

        self.assertEqual([(b.kind, b.level, b.text) for b in done.blocks], [
            ("heading", 1, "Lamp"),
            ("paragraph", None, "On while supplied."),
            ("heading", 2, "Timing"),
            ("paragraph", None, "Off after 30 seconds."),
            ("heading", 3, "Delay"),
            ("paragraph", None, "Five seconds."),
        ])

    def test_a_heading_with_markup_inside_it_and_attributes_on_it_says_its_words(self):
        page = '<h2 id="t" class="x">Core <em>Constructs</em>\n  and more</h2><p>Body.</p>'

        done = self.extract("spec.html", page)

        self.assertEqual((done.blocks[0].kind, done.blocks[0].text), ("heading", "Core Constructs / and more"))

    def test_an_empty_heading_is_nothing_and_not_an_empty_part(self):
        done = self.extract("spec.html", "<h1> </h1><p>Words.</p>")

        self.assertEqual([b.kind for b in done.blocks], ["paragraph"])

    def test_a_line_of_a_page_that_begins_with_a_hash_is_not_a_heading(self):
        # A code sample in a page (`# comment`) is a line that begins as a Markdown heading and is not one.
        page = "<h1>Algorithm</h1><pre># Here we handle events\n# and more</pre><p>Words.</p>"

        done = self.extract("spec.html", page)

        self.assertEqual([b.kind for b in done.blocks if b.kind == "heading"], ["heading"])
        self.assertEqual(done.blocks[0].text, "Algorithm")
        self.assertTrue(any(b.text.startswith("# Here") for b in done.blocks))

    def test_the_page_is_divided_by_them(self):
        done = self.extract("spec.html", "<h1>Lamp</h1><p>On.</p><h1>Fan</h1><p>Runs.</p>")

        way, sections = outline(done)

        self.assertEqual((way, [s.title for s in sections]), ("headings", ["Lamp", "Fan"]))

    def test_a_page_with_no_heading_has_none_and_is_not_divided(self):
        done = self.extract("spec.html", "<p>One.</p><p>Two.</p>")

        self.assertEqual(outline(done)[0], "whole")


class AWordParagraphThatBeginsWithAHashIsNotAHeading(Folder):
    def docx(self, *paragraphs: str) -> bytes:
        body = "".join(f"<w:p><w:r><w:t>{text}</w:t></w:r></w:p>" for text in paragraphs)
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as archive:
            archive.writestr(
                "word/document.xml",
                f'<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="{W}"><w:body>{body}</w:body></w:document>',
            )
        return buffer.getvalue()

    def test_a_comment_line_in_a_word_file_is_a_paragraph(self):
        done = self.extract("spec.docx", self.docx("# a comment in a code sample", "The lamp is on."))

        self.assertEqual([(b.kind, b.text) for b in done.blocks], [
            ("paragraph", "# a comment in a code sample"),
            ("paragraph", "The lamp is on."),
        ])


class MarkdownKeepsItsFencesApart(Folder):
    def kinds(self, text: str):
        blocks, _ = blocks_of_text(text, "spec.md", line_is_block=False, delimiter_row=True)
        return [(b.kind, b.text) for b in blocks]

    def test_a_hash_line_in_a_fenced_block_is_code_and_not_a_heading(self):
        text = "# Algorithm\n\n```python\n# set up\nx = 1\n```\n\nAfter the code.\n"

        found = self.kinds(text)

        self.assertEqual([k for k, _ in found], ["heading", "paragraph", "paragraph"])
        self.assertEqual(found[0][1], "Algorithm")
        self.assertIn("# set up", found[1][1])
        self.assertEqual(found[2][1], "After the code.")

    def test_a_list_marker_and_a_table_row_in_a_fence_are_code_too(self):
        text = "```\n- not an item\n| not | a table |\n```\n"

        found = self.kinds(text)

        self.assertEqual([k for k, _ in found], ["paragraph"])

    def test_a_fence_closes_with_its_own_mark_and_not_a_shorter_or_other_one(self):
        # ``` is not closed by ~~~ or by ``, so the heading after them is still inside the code.
        text = "```\ncode\n~~~\n``\n# still code\n```\n# real heading\n"

        found = self.kinds(text)

        self.assertEqual([k for k, _ in found], ["paragraph", "heading"])
        self.assertEqual(found[1][1], "real heading")

    def test_markdown_outside_a_fence_is_read_as_it_always_was(self):
        found = self.kinds("# One\n\n- item\n\n## Two\n\n| a | b |\n| - | - |\n| 1 | 2 |\n")

        self.assertEqual([k for k, _ in found], ["heading", "list_item", "heading", "table"])


if __name__ == "__main__":
    unittest.main()
