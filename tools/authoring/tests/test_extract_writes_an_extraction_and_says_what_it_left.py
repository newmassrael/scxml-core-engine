"""The `extract` command: one file to an extraction, and what it could not carry, in words.

A host that holds an original calls it, and a person may. What it prints is counts and a revision and
never the document's words (a general tool does not decide they may be printed), and what it could not
carry is printed with every extraction: it is part of the answer and a caller that reads only the exit
code has to be able to see it. Each case is paired with the nearest one that must not be refused.
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.extraction import Extraction


def run(*argv: str) -> tuple[int, str, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = main(list(argv))
    return code, out.getvalue(), err.getvalue()


class Folder(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.root = pathlib.Path(self._temp.name)

    def write(self, name: str, text: str) -> pathlib.Path:
        path = self.root / name
        path.write_text(text, encoding="utf-8")
        return path


class ExtractWritesTheExtractionAndNamesIt(Folder):
    def test_the_file_written_is_the_extraction_and_the_revision_printed_is_its_name(self):
        source = self.write("spec.md", "# Lamp\n\nThe lamp is on.\n")
        out = self.root / "spec.extraction.json"

        code, said, _ = run("extract", str(source), "--out", str(out))

        self.assertEqual(code, 0)
        written = Extraction.from_json(out.read_text(encoding="utf-8"))
        self.assertEqual([b.kind for b in written.blocks], ["heading", "paragraph"])
        self.assertIn(f"revision {written.revision()}", said)
        self.assertIn("2 block(s)", said)

    def test_what_is_printed_is_counts_and_never_the_words_of_the_document(self):
        source = self.write("spec.md", "A sentence nobody should find on a terminal.\n")

        _, said, err = run("extract", str(source), "--out", str(self.root / "x.json"))

        self.assertNotIn("nobody should find", said + err)

    def test_what_the_extractor_could_not_carry_is_printed_with_it(self):
        source = self.write("spec.html", "<table><tr><td>a</td><td>b</td></tr><tr><td>c</td></tr></table>")

        code, said, _ = run("extract", str(source), "--out", str(self.root / "x.json"))

        self.assertEqual(code, 0)
        self.assertIn("not carried: rows shorter than their table", said)

    def test_a_clean_extraction_prints_nothing_not_carried(self):
        source = self.write("spec.md", "Plain.\n")

        _, said, _ = run("extract", str(source), "--out", str(self.root / "x.json"))

        self.assertNotIn("not carried", said)

    def test_the_same_file_is_named_the_same_each_time(self):
        source = self.write("spec.md", "Plain.\n")

        first = run("extract", str(source), "--out", str(self.root / "a.json"))[1]
        second = run("extract", str(source), "--out", str(self.root / "b.json"))[1]

        self.assertEqual(first.split("revision ")[1].split(" ")[0], second.split("revision ")[1].split(" ")[0])


class WhatIsNotReadIsSaidAndWritesNothing(Folder):
    def test_a_format_nobody_reads_is_a_typed_answer_on_standard_error_and_no_file(self):
        source = self.write("spec.xyz", "x")
        out = self.root / "x.json"

        code, said, err = run("extract", str(source), "--out", str(out))

        self.assertEqual(code, 1)
        self.assertIn("unread [format-unread]", err)
        self.assertEqual(said, "")
        self.assertFalse(out.exists())

    def test_a_file_whose_name_and_bytes_disagree_is_refused_by_its_code(self):
        source = self.write("spec.docx", "not a zip")

        code, _, err = run("extract", str(source), "--out", str(self.root / "x.json"))

        self.assertEqual(code, 1)
        self.assertIn("[sniff-mismatch]", err)

    def test_a_path_that_is_not_a_file_is_the_tools_usual_refusal(self):
        code, _, err = run("extract", str(self.root / "nothing.md"), "--out", str(self.root / "x.json"))

        self.assertEqual(code, 2)
        self.assertIn("refused:", err)

    def test_an_out_that_cannot_be_written_is_refused_and_not_a_traceback(self):
        source = self.write("spec.md", "Plain.\n")

        code, _, err = run("extract", str(source), "--out", str(self.root / "no-such-folder" / "x.json"))

        self.assertEqual(code, 2)
        self.assertIn("refused:", err)

    def test_the_extraction_file_is_json_a_reader_of_the_schema_can_parse(self):
        source = self.write("spec.md", "Plain.\n")
        out = self.root / "x.json"
        run("extract", str(source), "--out", str(out))

        self.assertEqual(json.loads(out.read_text(encoding="utf-8"))["v"], 1)


if __name__ == "__main__":
    unittest.main()
