"""The form of a conversion, its schema, and the `convert` and `check-conversion` commands.

The check itself is held in `test_a_conversion_is_checked_against_its_original_by_sce_and_not_by_its_maker`.
What is held here is that the form refuses what it must (a field nobody defined, a self-declared
completeness, a conversion of nothing), that the schema file and the code say the same, and that a real
document goes through the whole route a person's import takes without a model: extract, convert, check,
and the Markdown is the words of the document.
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import pathlib
import tempfile
import unittest

import jsonschema

from sce_author import conversion as conversion_module
from sce_author.__main__ import main
from sce_author.conversion import Conversion, Converter, MarkdownBlock
from sce_author.errors import IngestError
from sce_author.extraction import Extraction
from sce_author.extractors import builtin

SCHEMA = json.loads(
    (pathlib.Path(__file__).resolve().parents[1] / "schema" / "conversion.v1.schema.json").read_text(encoding="utf-8")
)
REVISION = "a" * 64


def run(*argv: str) -> tuple[int, str, str]:
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = main(list(argv))
    return code, out.getvalue(), err.getvalue()


def made(*blocks: MarkdownBlock, kind: str = "model") -> Conversion:
    return Conversion(Converter(kind, "a-model", "1"), REVISION, tuple(blocks))


class TheFormIsKeptAndReadBackExactly(unittest.TestCase):
    def test_every_field_survives_the_wire_form_and_the_name_is_the_hash_of_the_text(self):
        whole = made(MarkdownBlock("m1", "# A", ("b1",)), MarkdownBlock("m2", "Invented.", ()))

        again = Conversion.from_json(whole.to_json())

        self.assertEqual(again, whole)
        self.assertEqual(whole.revision(), hashlib.sha256(whole.to_json().encode("utf-8")).hexdigest())

    def test_the_markdown_is_the_blocks_in_order_one_to_a_paragraph(self):
        self.assertEqual(made(MarkdownBlock("m1", "# A"), MarkdownBlock("m2", "B")).markdown(), "# A\n\nB\n")

    def test_what_the_code_writes_the_schema_accepts_and_the_other_way_round(self):
        jsonschema.Draft202012Validator.check_schema(SCHEMA)
        whole = made(MarkdownBlock("m1", "# A", ("b1", "b2")), MarkdownBlock("m2", "x", ()))

        jsonschema.validate(whole.to_dict(), SCHEMA)
        self.assertEqual(SCHEMA["x-sce-schema-status"], conversion_module.SCHEMA_STATUS)
        self.assertEqual(SCHEMA["properties"]["v"]["const"], conversion_module.SCHEMA_VERSION)
        self.assertEqual(tuple(SCHEMA["properties"]["converter"]["properties"]["kind"]["enum"]), conversion_module.CONVERTER_KINDS)


class TheFormRefusesWhatItMust(unittest.TestCase):
    def wire(self, **change) -> dict:
        wire = made(MarkdownBlock("m1", "x", ("b1",))).to_dict()
        wire.update(change)
        return wire

    def refused(self, wire: dict) -> bool:
        try:
            Conversion.from_dict(wire)
        except IngestError:
            schema_says_valid = True
            try:
                jsonschema.validate(wire, SCHEMA)
            except jsonschema.ValidationError:
                schema_says_valid = False
            # What the code refuses on the form of the wire, the schema refuses too (the one rule
            # the schema cannot say, a repeated id, is held on its own below).
            return not schema_says_valid or "id" in json.dumps(wire)
        return False

    def test_a_field_nobody_defined_is_refused_so_a_maker_cannot_declare_itself_complete(self):
        for extra in ({"complete": True}, {"verified": "yes"}):
            wire = self.wire(**extra)
            with self.subTest(extra=extra), self.assertRaises(IngestError):
                Conversion.from_dict(wire)
            with self.assertRaises(jsonschema.ValidationError):
                jsonschema.validate(wire, SCHEMA)

    def test_the_extraction_it_is_of_is_named_by_a_revision_and_not_by_anything_else(self):
        for bad in ("", "abc", "A" * 64, "g" * 64, "a" * 63):
            with self.subTest(bad=bad), self.assertRaises(IngestError):
                Conversion.from_dict(self.wire(extraction=bad))
            with self.assertRaises(jsonschema.ValidationError):
                jsonschema.validate(self.wire(extraction=bad), SCHEMA)

    def test_a_converter_of_a_kind_nobody_defined_or_with_no_name_is_refused(self):
        for converter in (
            {"kind": "magic", "name": "x", "version": "1"},
            {"kind": "model", "name": "", "version": "1"},
            {"kind": "model", "name": "x", "version": ""},
            {"kind": "model", "name": "x"},
        ):
            with self.subTest(converter=converter), self.assertRaises(IngestError):
                Conversion.from_dict(self.wire(converter=converter))

    def test_a_block_cites_a_block_once_and_a_repeated_id_is_refused(self):
        with self.assertRaises(IngestError):
            MarkdownBlock("m1", "x", ("b1", "b1"))
        with self.assertRaises(IngestError):
            made(MarkdownBlock("m1", "x"), MarkdownBlock("m1", "y"))
        with self.assertRaises(IngestError):
            MarkdownBlock("", "x")

    def test_what_is_not_json_or_not_an_object_or_of_another_version_is_refused(self):
        for text in ("not json", "[]", "7", "null", json.dumps(self.wire(v=2))):
            with self.subTest(text=text), self.assertRaises(IngestError):
                Conversion.from_json(text)

    def test_the_wrong_type_of_a_field_is_refused(self):
        for change in (
            {"blocks": "none"},
            {"blocks": [{"id": "m1", "markdown": 5, "cites": []}]},
            {"blocks": [{"id": "m1", "markdown": "x", "cites": "b1"}]},
            {"blocks": [{"id": "m1", "markdown": "x", "cites": [1]}]},
            {"blocks": [{"id": "m1", "markdown": "x", "cites": [], "extra": 1}]},
        ):
            with self.subTest(change=change), self.assertRaises(IngestError):
                Conversion.from_dict(self.wire(**change))


class Folder(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.root = pathlib.Path(self._temp.name)

    def write(self, name: str, text: str) -> pathlib.Path:
        path = self.root / name
        path.write_text(text, encoding="utf-8")
        return path


DOCUMENT = (
    "# Lamp controller\n\n"
    "The lamp is on while the supply is present.\nIt turns off when the supply is lost.\n\n"
    "- first item\n- second item\n\n"
    "| state | lamp |\n| on | lit |\n| off | dark |\n"
)


class ADocumentGoesThroughTheWholeRouteWithNoModel(Folder):
    def route(self, source: pathlib.Path) -> tuple[Extraction, Conversion, str]:
        extraction_file = self.root / "e.json"
        conversion_file = self.root / "c.json"
        markdown_file = self.root / "out.md"
        self.assertEqual(run("extract", str(source), "--out", str(extraction_file))[0], 0)
        code, said, _ = run(
            "convert", "--extraction", str(extraction_file), "--out", str(conversion_file), "--markdown", str(markdown_file)
        )
        self.assertEqual(code, 0, said)
        return (
            Extraction.from_json(extraction_file.read_text(encoding="utf-8")),
            Conversion.from_json(conversion_file.read_text(encoding="utf-8")),
            markdown_file.read_text(encoding="utf-8"),
        )

    def test_markdown_is_extracted_converted_and_found_complete(self):
        extraction, conversion, markdown = self.route(self.write("spec.md", DOCUMENT))

        self.assertEqual(conversion.extraction, extraction.revision())
        self.assertEqual(markdown, conversion.markdown())
        code, said, _ = run(
            "check-conversion", "--extraction", str(self.root / "e.json"), "--conversion", str(self.root / "c.json")
        )
        self.assertEqual(code, 0)
        self.assertIn("complete: 5 matched", said)

    def test_the_markdown_made_is_read_back_as_the_same_words(self):
        # What the converter writes is the document again: extracting it gives the same words.
        extraction, _, markdown = self.route(self.write("spec.md", DOCUMENT))
        again = builtin().extract(self.write("again.md", markdown))

        # A table is its cells: the text of the block keeps the delimiter row the converter writes.
        words = lambda e: [  # noqa: E731
            (b.kind, b.cells if b.cells is not None else " ".join(b.text.split())) for b in e.blocks
        ]
        self.assertEqual(words(again), words(extraction))

    def test_a_html_document_with_a_table_goes_the_same_way(self):
        source = self.write(
            "spec.html", "<p>Intro</p><table><tr><td>a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table>"
        )

        extraction, conversion, markdown = self.route(source)

        self.assertIn("| a | b |", markdown)
        self.assertEqual(len(conversion.blocks), len(extraction.blocks))

    def test_the_conversion_file_hashes_to_its_revision(self):
        _, conversion, _ = self.route(self.write("spec.md", DOCUMENT))

        self.assertEqual(hashlib.sha256((self.root / "c.json").read_bytes()).hexdigest(), conversion.revision())


class TheCheckCommandSaysWhatSceCounts(Folder):
    def setUp(self) -> None:
        super().setUp()
        source = self.write("spec.md", DOCUMENT)
        run("extract", str(source), "--out", str(self.root / "e.json"))
        run("convert", "--extraction", str(self.root / "e.json"), "--out", str(self.root / "good.json"))
        self.good = Conversion.from_json((self.root / "good.json").read_text(encoding="utf-8"))

    def check(self, conversion: Conversion, *flags: str) -> tuple[int, str, str]:
        path = self.root / "under-check.json"
        path.write_text(conversion.to_json(), encoding="utf-8")
        return run("check-conversion", "--extraction", str(self.root / "e.json"), "--conversion", str(path), *flags)

    def test_a_damaged_copy_is_not_complete_and_the_exit_code_says_so(self):
        blocks = list(self.good.blocks)
        blocks[1] = MarkdownBlock(blocks[1].id, "The lamp is off while the supply is present.", blocks[1].cites)
        damaged = Conversion(self.good.converter, self.good.extraction, tuple(blocks[:-1]))

        code, said, _ = self.check(damaged)

        self.assertEqual(code, 1)
        self.assertIn("not complete", said)
        self.assertIn("ask: check m0002", said)
        self.assertIn("ask: missing b0005", said)

    def test_the_json_form_is_counts_and_ids_and_never_the_words_of_the_document(self):
        damaged = Conversion(self.good.converter, self.good.extraction, self.good.blocks[:-1])

        code, said, _ = self.check(damaged, "--json")

        self.assertEqual(code, 1)
        report = json.loads(said)
        self.assertEqual(report["complete"], False)
        self.assertEqual(report["missing"], ["b0005"])
        self.assertNotIn("lamp", said)

    def test_a_conversion_of_another_extraction_is_refused_and_counts_nothing(self):
        other = Conversion(self.good.converter, "b" * 64, self.good.blocks)

        code, said, err = self.check(other)

        self.assertEqual(code, 2)
        self.assertEqual(said, "")
        self.assertIn("not of this one", err)

    def test_a_file_that_is_not_an_extraction_or_a_conversion_is_the_tools_usual_refusal(self):
        bad = self.write("bad.json", "not json")

        code, _, err = run("check-conversion", "--extraction", str(bad), "--conversion", str(self.root / "good.json"))
        again, _, err2 = run("check-conversion", "--extraction", str(self.root / "e.json"), "--conversion", str(bad))
        gone, _, err3 = run(
            "check-conversion", "--extraction", str(self.root / "nowhere.json"), "--conversion", str(bad)
        )

        self.assertEqual((code, again, gone), (2, 2, 2))
        for said in (err, err2, err3):
            self.assertIn("refused:", said)

    def test_a_convert_of_an_extraction_with_a_table_it_has_no_cells_for_is_refused(self):
        wire = {
            "v": 1,
            "extractor": "x",
            "extractor_version": "1",
            "member": "m",
            "blocks": [
                {"id": "b1", "kind": "table", "text": "| a |", "place": {"member": "m", "precision": "none"}, "how_read": "text-layer"}
            ],
        }
        path = self.write("nocells.json", json.dumps(wire))

        code, _, err = run("convert", "--extraction", str(path), "--out", str(self.root / "c.json"))

        self.assertEqual(code, 2)
        self.assertIn("no grid of cells", err)
        self.assertFalse((self.root / "c.json").exists())


if __name__ == "__main__":
    unittest.main()
