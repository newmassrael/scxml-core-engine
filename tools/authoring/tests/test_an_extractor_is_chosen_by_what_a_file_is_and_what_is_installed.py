"""Which extractor reads a file, and what is said when none does.

ADR 0036 makes a format a seam: an extractor declares what it reads and what it needs, and a registry
picks one. Each case builds a file the way it arrives (a wrong name, a missing pack, a format nobody
reads) and asserts the answer is typed and says what to do, and pairs it with the nearest file that
must be read: a registry that refuses everything passes the first half and is worthless.
"""

from __future__ import annotations

import io
import pathlib
import tempfile
import unittest
import zipfile

from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place
from sce_author.extractors import (
    FORMAT_UNREAD,
    PACK_MISSING,
    SNIFF_MISMATCH,
    Declaration,
    Registry,
    Unreadable,
    blocks_of_text,
    builtin,
)

W = "http://schemas.openxmlformats.org/wordprocessingml/2006/main"


def docx(body: str) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        archive.writestr(
            "word/document.xml",
            f'<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="{W}"><w:body>{body}</w:body></w:document>',
        )
    return buffer.getvalue()


def paragraph(text: str) -> str:
    return f"<w:p><w:r><w:t>{text}</w:t></w:r></w:p>"


class Fake:
    """An extractor of a format of its own: it says what it is given and reads nothing."""

    def __init__(self, declaration: Declaration, text: str = "read") -> None:
        self.declaration = declaration
        self.text = text

    def extract(self, path: pathlib.Path) -> Extraction:
        block = Block("b1", "paragraph", self.text, Place(path.name, ordinal=1))
        return Extraction(self.declaration.name, self.declaration.version, path.name, (block,))


class Folder(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.root = pathlib.Path(self._temp.name)

    def write(self, name: str, data: bytes | str) -> pathlib.Path:
        path = self.root / name
        path.write_bytes(data if isinstance(data, bytes) else data.encode("utf-8"))
        return path


class TheReadersOfIngestAreTheFirstExtractors(Folder):
    def test_markdown_is_headings_list_items_paragraphs_and_a_table_with_its_grid(self):
        path = self.write(
            "spec.md",
            "# Lamp\n\nThe lamp is on\nwhile the supply is present.\n\n- first\n- second\n\n"
            "| state | lamp |\n| on | lit |\n| off | dark |\n\n[picture: image1.png]\n",
        )

        done = builtin().extract(path)

        kinds = [b.kind for b in done.blocks]
        self.assertEqual(kinds, ["heading", "paragraph", "list_item", "list_item", "table", "figure"])
        self.assertEqual(done.blocks[0].level, 1)
        self.assertEqual(done.blocks[0].text, "Lamp")
        # A wrapped paragraph is one block.
        self.assertEqual(done.blocks[1].text, "The lamp is on while the supply is present.")
        self.assertEqual(done.blocks[4].cells, (("state", "lamp"), ("on", "lit"), ("off", "dark")))
        self.assertEqual(done.blocks[5].text, "[picture: image1.png]")
        self.assertEqual([b.place.ordinal for b in done.blocks], [1, 2, 3, 4, 5, 6])
        self.assertEqual(done.member, "spec.md")
        self.assertEqual(done.extractor, "plain")

    def test_ids_are_stable_for_the_same_file_and_the_extraction_is_named_by_its_content(self):
        path = self.write("spec.md", "One.\n\nTwo.\n")

        first = builtin().extract(path)
        second = builtin().extract(path)

        self.assertEqual([b.id for b in first.blocks], ["b0001", "b0002"])
        self.assertEqual(first.revision(), second.revision())

    def test_html_rows_are_a_table_and_a_short_row_is_counted_as_not_exact(self):
        path = self.write(
            "spec.html",
            "<p>Intro</p><table><tr><td>a</td><td>b</td></tr><tr><td>c</td></tr></table><p>After</p>",
        )

        done = builtin().extract(path)

        table = next(b for b in done.blocks if b.kind == "table")
        self.assertEqual(table.cells, (("a", "b"), ("c", "")))
        short = [n for n in done.not_carried if "shorter" in n.what]
        self.assertEqual([n.count for n in short], [1])

    def test_a_word_document_gives_a_block_to_a_paragraph_and_a_merged_cell_in_each_column_it_covers(self):
        merged = (
            "<w:tbl><w:tr>"
            '<w:tc><w:tcPr><w:gridSpan w:val="2"/></w:tcPr><w:p><w:r><w:t>Header</w:t></w:r></w:p></w:tc>'
            "</w:tr><w:tr>"
            "<w:tc><w:p><w:r><w:t>left</w:t></w:r></w:p></w:tc>"
            "<w:tc><w:p><w:r><w:t>right</w:t></w:r></w:p></w:tc>"
            "</w:tr></w:tbl>"
        )
        path = self.write("spec.docx", docx(paragraph("First clause.") + paragraph("Second clause.") + merged))

        done = builtin().extract(path)

        self.assertEqual([b.kind for b in done.blocks], ["paragraph", "paragraph", "table"])
        self.assertEqual(done.blocks[0].text, "First clause.")
        self.assertEqual(done.blocks[2].cells, (("Header", "Header"), ("left", "right")))

    def test_what_the_reader_could_not_carry_is_kept_with_the_blocks(self):
        # A picture the reader cannot show is a note of the reader, and it must reach the extraction.
        path = self.write("spec.docx", docx(paragraph("Clause.")))
        with zipfile.ZipFile(path, "a") as archive:
            archive.writestr("word/media/image1.png", b"\x89PNG\r\n\x1a\n")

        done = builtin().extract(path)

        self.assertFalse(done.clean)
        self.assertTrue(any("picture" in n.what for n in done.not_carried))

    def test_an_empty_text_makes_no_blocks_for_the_reader_to_have_said_nothing_of(self):
        blocks, notes = blocks_of_text("", "x.md", line_is_block=False)

        self.assertEqual((blocks, notes), ([], []))


class ASuffixIsAClaimAndTheBytesDecide(Folder):
    def test_a_file_called_docx_that_is_not_one_is_not_handed_to_the_reader(self):
        path = self.write("spec.docx", "just some text, not a zip")

        chosen = builtin().choose(path)

        self.assertIsInstance(chosen, Unreadable)
        self.assertEqual(chosen.code, SNIFF_MISMATCH)

    def test_the_same_refusal_is_the_tools_usual_one_for_a_caller_that_wants_that(self):
        path = self.write("spec.docx", "just some text")

        with self.assertRaisesRegex(IngestError, "spec.docx.*does not begin as a docx file"):
            builtin().extract(path)

    def test_a_real_word_document_is_read(self):
        path = self.write("spec.docx", docx(paragraph("Clause.")))

        self.assertEqual(builtin().extract(path).blocks[0].text, "Clause.")

    def test_a_signature_finds_a_file_whose_name_says_nothing(self):
        pdf = Declaration("pdf-test", "1", (".pdf",), magic=(b"%PDF-",))
        registry = Registry([Fake(pdf, "from a pdf")])
        path = self.write("spec.dat", b"%PDF-1.7\n...")

        self.assertEqual(registry.extract(path).extractor, "pdf-test")

    def test_a_signature_that_other_formats_share_never_finds_a_file(self):
        # A zip is a word document, a workbook and an archive: only the name can say which.
        path = self.write("archive.zip", docx(paragraph("x")))

        chosen = builtin().choose(path)

        self.assertIsInstance(chosen, Unreadable)
        self.assertEqual(chosen.code, FORMAT_UNREAD)

    def test_a_pdf_called_by_another_name_is_checked_against_its_signature_too(self):
        pdf = Declaration("pdf-test", "1", (".pdf",), magic=(b"%PDF-",))
        path = self.write("spec.pdf", "this is a text file")

        chosen = Registry([Fake(pdf)]).choose(path)

        self.assertEqual(chosen.code, SNIFF_MISMATCH)


class WhatNobodyReadsIsAnAnswerAndNotAnError(Folder):
    def test_a_format_nobody_reads_is_unread_and_names_what_to_do_when_the_tool_knows(self):
        path = self.write("spec.vsdx", b"PK\x03\x04")

        chosen = builtin().choose(path)

        self.assertEqual(chosen.code, FORMAT_UNREAD)
        self.assertIn("diagram", chosen.why)

    def test_a_format_nobody_has_heard_of_says_so_without_a_hint(self):
        chosen = builtin().choose(self.write("spec.xyz", "x"))

        self.assertEqual(chosen.code, FORMAT_UNREAD)
        self.assertIn(".xyz", chosen.why)

    def test_a_file_with_no_suffix_is_named_as_such(self):
        chosen = builtin().choose(self.write("README", "x"))

        self.assertIn("no suffix", chosen.why)

    def test_try_extract_hands_the_answer_back_and_extract_raises_it(self):
        path = self.write("spec.xyz", "x")

        self.assertIsInstance(builtin().try_extract(path), Unreadable)
        with self.assertRaises(IngestError):
            builtin().extract(path)

    def test_a_path_that_is_not_a_file_is_the_tools_usual_refusal_and_not_an_answer(self):
        with self.assertRaises(IngestError):
            builtin().choose(self.root / "nothing-here.md")
        with self.assertRaises(IngestError):
            builtin().choose(self.root)


class WhatAnExtractorNeedsIsAPack(Folder):
    PDF = dict(suffixes=(".pdf",), magic=(b"%PDF-",))
    ABSENT = "sce_author_test_module_that_is_not_installed"

    def test_an_extractor_whose_pack_is_not_installed_is_reported_with_the_pack_to_install(self):
        needing = Declaration("pdf-pack", "1", needs=(self.ABSENT,), pack="pdf-text", **self.PDF)
        path = self.write("spec.pdf", b"%PDF-1.7")

        chosen = Registry([Fake(needing)]).choose(path)

        self.assertIsInstance(chosen, Unreadable)
        self.assertEqual(chosen.code, PACK_MISSING)
        self.assertEqual(chosen.pack, "pdf-text")
        self.assertIn(self.ABSENT, chosen.why)

    def test_the_next_extractor_of_the_format_is_used_when_the_first_one_cannot_run(self):
        needing = Declaration("pdf-pack", "1", needs=(self.ABSENT,), pack="pdf-text", **self.PDF)
        plain = Declaration("pdf-plain", "1", **self.PDF)
        path = self.write("spec.pdf", b"%PDF-1.7")

        done = Registry([Fake(needing, "rich"), Fake(plain, "plain")]).extract(path)

        self.assertEqual(done.extractor, "pdf-plain")

    def test_the_preferred_extractor_is_used_when_it_can_run(self):
        needing = Declaration("pdf-pack", "1", needs=("json",), pack="pdf-text", **self.PDF)
        plain = Declaration("pdf-plain", "1", **self.PDF)
        path = self.write("spec.pdf", b"%PDF-1.7")

        done = Registry([Fake(needing, "rich"), Fake(plain, "plain")]).extract(path)

        self.assertEqual(done.extractor, "pdf-pack")

    def test_the_pack_missing_words_name_what_is_missing_and_not_what_is_present(self):
        needing = Declaration("pdf-pack", "1", needs=("json", self.ABSENT), pack="pdf-text", **self.PDF)

        chosen = Registry([Fake(needing)]).choose(self.write("spec.pdf", b"%PDF-1.7"))

        self.assertIn(self.ABSENT, chosen.why)
        self.assertNotIn("json", chosen.why)


class ADeclarationAndARegistryRefuseWhatCannotWork(unittest.TestCase):
    def test_a_name_is_registered_once(self):
        registry = Registry([Fake(Declaration("one", "1", (".a",)))])

        with self.assertRaises(ValueError):
            registry.register(Fake(Declaration("one", "2", (".b",))))

    def test_a_suffix_is_lower_case_with_its_dot(self):
        for bad in ("md", ".MD"):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                Declaration("x", "1", (bad,))

    def test_an_extractor_reads_something_and_says_what_it_is(self):
        with self.assertRaises(ValueError):
            Declaration("x", "1")
        with self.assertRaises(ValueError):
            Declaration("", "1", (".a",))

    def test_an_extractor_that_needs_modules_names_the_pack_that_carries_them(self):
        with self.assertRaises(ValueError):
            Declaration("x", "1", (".a",), needs=("somewhere",))


if __name__ == "__main__":
    unittest.main()
