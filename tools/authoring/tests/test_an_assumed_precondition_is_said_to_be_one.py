"""A precondition read by assumption is reported as one, everywhere it reaches.

A specification states a precondition as a phrase, and the pack's table says
what expression the phrase becomes. Some of those readings are not facts the
platform publishes but assumptions about it -- most sharply, a phrase read as
the constant `true`, because the platform has no input that observes it and
the condition always holds while anything is running at all.

Such a reading is often right. What made it dangerous was that nothing said
so. Measured on a copy of the fixture pack before this existed:

  * replacing a phrase's reading with a constant changed one line of the brief
    and NOTHING in `questions`
  * deleting a phrase the prose writes from the table changed nothing either
  * `verify` never read the table at all

So a condition could leave the document by either route -- read as a constant,
or decided by whoever wrote the document because the table was silent -- and a
green run looked the same. A constant is the sharper of the two: no case can
exercise a condition that is not in the document, so a right reading and a
wrong one pass identically.

What is asserted here, in the order a reader meets it:

  the pack      a constant reading without a stated reason is refused
  questions     every phrase the prose writes is looked up: one not in the
                table is a warning, one read by assumption is a note with its
                reason and the place it is written
  brief         an assumed reading is printed AS one
  verify        the verdict carries what the pass rests on
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.brief import assemble
from sce_author.errors import PackError
from sce_author.pack import load_pack, reads_no_input
from sce_author.prose import load_prose
from sce_author.questions import ask
from sce_author.verify import _default_codegen, assumed_preconditions_of

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "crossing"

REASON = "the controller only runs while the installation is energised"


class APackUnderTest:
    """A copy of the fixture pack whose precondition table a test rewrites."""

    def __init__(self, tmp: pathlib.Path):
        self.root = tmp / "pack"
        shutil.copytree(FIXTURE, self.root)
        self.path = self.root / "conventions.yaml"
        self.doc = yaml.safe_load(self.path.read_text(encoding="utf-8"))

    def preconditions(self, **changes):
        pre = self.doc.setdefault("preconditions", {})
        for key, value in changes.items():
            if value is None:
                pre.pop(key, None)
            else:
                pre[key] = value
        self.path.write_text(yaml.safe_dump(self.doc, sort_keys=False),
                             encoding="utf-8")
        return self

    def load(self):
        return load_pack(self.root)

    def prose(self, text: str):
        spec = self.root / "specification.md"
        spec.write_text(text, encoding="utf-8")
        return load_prose([spec])


def kinds(questions, *wanted):
    return [q for q in questions if q.kind in wanted]


class AConstantReadingNeedsAReason(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.pack = APackUnderTest(pathlib.Path(self._tmp.name))

    def tearDown(self):
        self._tmp.cleanup()

    def test_a_constant_without_a_reason_is_refused(self):
        """⚠ The refusal is the point. A reason written once at the pack is
        what every later report repeats; without it there is nothing to
        repeat, and the constant reads as a fact from then on."""
        self.pack.preconditions(phrases={"powered": "true"})
        with self.assertRaises(PackError) as caught:
            self.pack.load()
        message = str(caught.exception)
        self.assertIn("'powered'", message)
        self.assertIn("assumed", message)

    def test_every_spelling_of_a_constant_is_one(self):
        for constant in ("true", "false", "!true", "TRUE", "false && true"):
            with self.subTest(constant=constant):
                self.assertTrue(reads_no_input(constant))
        for reading in ("poweredUp", "!poweredUp", "poweredUp && true"):
            with self.subTest(reading=reading):
                self.assertFalse(reads_no_input(reading))

    def test_a_reading_that_names_an_input_needs_no_reason(self):
        """The fixture's own table: a plain reading, over a declared input."""
        conv = self.pack.load().conventions
        self.assertEqual("poweredUp", conv.precondition_phrases["powered"])
        self.assertEqual({}, conv.precondition_assumed)

    def test_the_object_form_carries_the_reason(self):
        self.pack.preconditions(phrases={
            "powered": {"expression": "true", "assumed": REASON}})
        conv = self.pack.load().conventions
        self.assertEqual("true", conv.precondition_phrases["powered"])
        self.assertEqual(REASON, conv.precondition_assumed["powered"])

    def test_a_pattern_without_its_group_is_refused(self):
        """Without `phrase` the core cannot say which part of a match to look
        up, and guessing the whole match would look up the sentence."""
        self.pack.preconditions(pattern=r"while the installation is \w+")
        with self.assertRaises(PackError) as caught:
            self.pack.load()
        self.assertIn("phrase", str(caught.exception))


class TheQuestionsLookEveryPreconditionUp(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.pack = APackUnderTest(pathlib.Path(self._tmp.name))

    def tearDown(self):
        self._tmp.cleanup()

    def ask(self, text):
        pack = self.pack.load()
        return ask(self.pack.prose(text), pack.model, pack.conventions,
                   pack.examples)

    def test_a_phrase_read_by_assumption_is_reported_where_it_is_written(self):
        self.pack.preconditions(phrases={
            "powered": {"expression": "true", "assumed": REASON}})
        found = kinds(self.ask(
            "Intro.\n"
            "While the installation is powered, the road signal is DARK.\n"
            "While the installation is powered, the bell is SILENT.\n"),
            "precondition-assumed")
        self.assertEqual(1, len(found), "one question per phrase, not per use")
        q = found[0]
        self.assertEqual("powered", q.subject)
        self.assertEqual(2, q.line, "the FIRST place the prose relies on it")
        self.assertIn("written 2 time(s)", q.detail)
        self.assertIn("no case can exercise", q.detail)
        self.assertIn(REASON, q.detail)
        self.assertEqual("note", q.severity)

    def test_an_assumed_reading_over_an_input_says_what_a_case_does_test(self):
        """Not every assumption is a constant. One that names an input is
        exercised -- but what a case then tests is the pack's expression, not
        the phrase, and the report says which."""
        self.pack.preconditions(phrases={
            "powered": {"expression": "poweredUp", "assumed": REASON}})
        found = kinds(self.ask("While the installation is powered, go.\n"),
                      "precondition-assumed")
        self.assertEqual(1, len(found))
        self.assertIn("a case exercises that expression", found[0].detail)
        self.assertNotIn("no case can exercise", found[0].detail)

    def test_a_phrase_the_table_does_not_read_is_a_warning(self):
        """⚠ The route that was completely silent: the prose writes a
        precondition, the table has nothing for it, and whoever writes the
        document decides what it means."""
        found = kinds(self.ask(
            "While the installation is powered, the road signal is DARK.\n"
            "While the installation is on standby, the bell is SILENT.\n"),
            "precondition-not-in-table")
        self.assertEqual(["on standby"], [q.subject for q in found])
        self.assertEqual(2, found[0].line)
        self.assertEqual("warning", found[0].severity)

    def test_a_plain_reading_raises_nothing(self):
        """The fixture as shipped: the one phrase its prose writes is read
        plainly, over a declared input. Nothing to confirm, nothing missing."""
        found = kinds(self.ask(
            "While the installation is powered, the road signal is DARK.\n"),
            "precondition-assumed", "precondition-not-in-table",
            "precondition-check-declined")
        self.assertEqual([], found)

    def test_spelling_drift_is_normalised_before_the_lookup(self):
        """A table keyed by one spelling must not report the other as
        missing -- that is what `normalise` is for, and a lookup that skipped
        it would turn drift into a warning."""
        self.pack.preconditions(normalise=[{"from": r"^powered up$",
                                            "to": "powered"}])
        found = kinds(self.ask(
            "While the installation is powered up, the bell is SILENT.\n"),
            "precondition-not-in-table")
        self.assertEqual([], found)

    def test_without_a_pattern_the_check_says_it_did_not_run(self):
        """⚠ A table the core cannot match against the prose is the case that
        looked exactly like a clean answer. It is declined out loud, and the
        note counts the assumptions it could not place."""
        self.pack.preconditions(pattern=None, phrases={
            "powered": {"expression": "true", "assumed": REASON}})
        found = kinds(self.ask("While the installation is powered, go.\n"),
                      "precondition-check-declined",
                      "precondition-assumed", "precondition-not-in-table")
        self.assertEqual(["precondition-check-declined"],
                         [q.kind for q in found])
        self.assertIn("1 of its phrase(s) are read by assumption",
                      found[0].detail)

    def test_a_pack_with_no_table_is_not_asked(self):
        """No table, no claim about preconditions to check."""
        self.pack.preconditions(phrases=None, pattern=None)
        found = kinds(self.ask("While the installation is powered, go.\n"),
                      "precondition-check-declined",
                      "precondition-assumed", "precondition-not-in-table")
        self.assertEqual([], found)


class TheBriefPrintsAnAssumptionAsOne(unittest.TestCase):
    def test_the_reason_is_beside_the_reading(self):
        with tempfile.TemporaryDirectory() as tmp:
            pack = APackUnderTest(pathlib.Path(tmp)).preconditions(phrases={
                "powered": {"expression": "true", "assumed": REASON},
                "not powered": "!poweredUp"})
            loaded = pack.load()
            text = assemble(pack.prose("While the installation is powered.\n"),
                            loaded)
        self.assertIn(f"- `powered` -> `true` — ASSUMED by the pack: {REASON}",
                      text)
        self.assertIn("- `not powered` -> `!poweredUp`\n", text)


class TheVerdictSaysWhatItRestsOn(unittest.TestCase):
    def test_the_figure_is_the_pack_s_assumptions_with_their_reasons(self):
        with tempfile.TemporaryDirectory() as tmp:
            conv = APackUnderTest(pathlib.Path(tmp)).preconditions(phrases={
                "powered": {"expression": "true", "assumed": REASON},
                "not powered": "!poweredUp"}).load().conventions
        self.assertEqual(
            {"powered": {"expression": "true", "reason": REASON}},
            assumed_preconditions_of(conv))

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_a_run_carries_it_beside_the_counts(self):
        """End to end: the same run, the same counts, and now the thing the
        counts are conditional on."""
        from sce_author.verify import verify

        with tempfile.TemporaryDirectory() as tmp:
            pack = APackUnderTest(pathlib.Path(tmp)).preconditions(phrases={
                "powered": {"expression": "true", "assumed": REASON}})
            result = verify(pack.load(),
                            pack.root / "controller_resolved.binding.yaml")
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(
            {"powered": {"expression": "true", "reason": REASON}},
            result.assumed_preconditions)


if __name__ == "__main__":
    unittest.main()
