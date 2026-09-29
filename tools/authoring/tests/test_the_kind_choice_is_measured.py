"""The measurement of kind choice is itself held to what it claims.

`eval/kind_choice.py` scores a client by what the product read in the
document the client wrote. A scorer that misread a manifest, or a corpus
naming a kind the product does not have, would report a number about
nothing -- so the corpus is checked against the product's kinds, and each
outcome is produced here from a document whose reading is known.
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import sys
import tempfile
import unittest

from sce_author.verify import _default_codegen

HERE = pathlib.Path(__file__).resolve().parent
EVAL = HERE.parent / "eval" / "kind_choice.py"
SCHEMA = HERE.parents[2] / "schemas" / "sce-kind-catalog.v1.schema.json"

spec = importlib.util.spec_from_file_location("kind_choice", EVAL)
kind_choice = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kind_choice)

# The product's kinds as its published schema lists them, so the corpus is
# checked without a built generator.
KINDS = set(json.loads(SCHEMA.read_text(encoding="utf-8"))["definitions"]["kindName"]["enum"])

# A kind the corpus leaves out, and why. A worker is judged only with the
# link it imports, as a document set, and the client writes one document.
NOT_IN_THE_CORPUS = {"worker"}

TRANSFORM = (
    '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
    'xmlns:sce="http://sce.dev/ext" sce:kind="transform" name="t">\n'
    '  <datamodel>\n'
    '    <data id="a" sce:type="int32" sce:direction="in"/>\n'
    '    <data id="b" sce:type="int32" sce:direction="out" expr="a + 1"/>\n'
    '  </datamodel>\n'
    '</scxml>\n')
OPEN_TRANSFORM = TRANSFORM.replace(
    '  <datamodel>\n',
    '  <sce:kind-basis sce:unresolved="kind" '
    'sce:unresolved-reason="the text never says what an unlisted input gives" '
    'sce:unresolved-candidates="lookup">\n'
    '    <sce:evidence>each listed input has one setting</sce:evidence>\n'
    '  </sce:kind-basis>\n'
    '  <datamodel>\n', 1)
UNDECLARED_STATECHART = (
    '<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" '
    'initial="idle">\n  <state id="idle"/>\n</scxml>\n')


class TheCorpus(unittest.TestCase):
    def test_every_case_names_kinds_the_product_has(self):
        cases = kind_choice.load_cases(kind_choice.CASES, KINDS)
        self.assertGreaterEqual(len(cases), 20)

    def test_every_kind_is_measured_or_its_absence_explained(self):
        cases = kind_choice.load_cases(kind_choice.CASES, KINDS)
        measured = {c["kind"] for c in cases if "kind" in c}
        self.assertEqual(KINDS - NOT_IN_THE_CORPUS, measured)
        self.assertTrue(any("undetermined" in c for c in cases),
                        "no case measures whether a client asks")

    def test_a_malformed_corpus_is_refused_whole(self):
        bad = [
            [{"id": "a", "prose": "x", "kind": "transform"},
             {"id": "a", "prose": "y", "kind": "lookup"}],
            [{"id": "a", "prose": "x", "kind": "transform", "undetermined": ["lookup", "enum"]}],
            [{"id": "a", "prose": "x", "kind": "state-machine"}],
            [{"id": "a", "prose": "x", "undetermined": ["lookup"]}],
            [{"id": "a", "prose": "", "kind": "lookup"}],
        ]
        for cases in bad:
            with self.subTest(cases=cases), tempfile.TemporaryDirectory() as tmp:
                path = pathlib.Path(tmp) / "cases.json"
                path.write_text(json.dumps({"cases": cases}), encoding="utf-8")
                with self.assertRaises(kind_choice.CaseError):
                    kind_choice.load_cases(path, KINDS)


class TheScore(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def test_no_document_is_asking_only_where_the_text_is_open(self):
        absent = self.dir / "draft.scxml"
        self.assertEqual("no_document",
                         kind_choice.score({"id": "d", "kind": "lookup"}, absent)["outcome"])
        self.assertEqual("asked", kind_choice.score(
            {"id": "u", "undetermined": ["lookup", "interpolation"]}, absent)["outcome"])

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_the_outcome_is_the_products_reading(self):
        draft = self.dir / "draft.scxml"
        cases = [
            ({"id": "c", "kind": "transform"}, TRANSFORM, "correct"),
            ({"id": "w", "kind": "lookup"}, TRANSFORM, "wrong_kind"),
            ({"id": "s", "kind": "statechart"}, UNDECLARED_STATECHART, "correct_by_default"),
            ({"id": "r", "kind": "transform"}, "<scxml/>", "refused"),
            ({"id": "u", "undetermined": ["transform", "lookup"]}, TRANSFORM, "decided"),
            # The kind marked open in the document is the question asked.
            ({"id": "o", "undetermined": ["transform", "lookup"]}, OPEN_TRANSFORM,
             "left_open"),
            ({"id": "h", "kind": "transform"}, OPEN_TRANSFORM, "left_open"),
        ]
        for case, text, outcome in cases:
            with self.subTest(case=case["id"]):
                draft.write_text(text, encoding="utf-8")
                self.assertEqual(outcome, kind_choice.score(case, draft)["outcome"])

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_a_client_is_run_in_its_own_directory_and_scored(self):
        # A stand-in client that writes the document a real one would.
        client = [sys.executable, "-c",
                  "import pathlib, sys; "
                  f"pathlib.Path('draft.scxml').write_text({TRANSFORM!r}); "
                  "print(sys.argv[1:])", "{prompt}"]
        result = kind_choice.run_case({"id": "adc", "kind": "transform", "prose": "p"},
                                      client, self.dir, timeout=60, model=None)
        self.assertEqual("correct", result["outcome"])
        self.assertEqual("p\n", (self.dir / "adc" / "spec.md").read_text())
        self.assertIn(kind_choice.REQUEST,
                      (self.dir / "adc" / "transcript.txt").read_text())

    def test_the_summary_counts_each_outcome(self):
        summary = kind_choice.summarise([
            {"expected": "lookup", "outcome": "correct"},
            {"expected": "lookup", "outcome": "wrong_kind"},
            {"expected": None, "outcome": "asked"},
        ])
        self.assertEqual((2, 1, 1), (summary["determined"]["total"],
                                     summary["determined"]["correct"],
                                     summary["undetermined"]["asked"]))


if __name__ == "__main__":
    unittest.main()
