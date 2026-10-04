"""`verify --explain` shows what the document did in a case, not only whether it was right.

A failure says what was expected and what was written at one position, and cannot say WHY: the why
is in the values between the inputs and the position, which an author cannot see from outside the
document. Measured 2026-10-04 while tracing one failing case by hand: the case drove a single input
and expected one event; the document wrote another. Reading the specification and the document's
expressions side by side did not say why, and only running the document on that case and reading
its variables did -- every event condition was false in that round, so the identifier kept the
first branch of a fallback chain. An author has no way to ask that question.

Asserted here, with a document that is really run:

  asked        a case named in `explain` carries what the binding handed the document, what it
               returned, and what it kept
  not asked    every other case carries nothing, so the report does not grow with the suite
  unchanged    asking changes no verdict
  by text      a name matches by substring, several can be given, and one that matches nothing
               shows nothing and is not an error
  honest       a pure computation shows its outputs and says it kept nothing
"""

from __future__ import annotations

import pathlib
import unittest

from sce_author.pack import load_pack
from sce_author.verify import verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
BINDING = CROSSING / "controller_resolved.binding.yaml"
FAILING = "the same train, the same everything, and the bell is silent"
PASSING = "a train approaches on a healthy installation"


def codegen_is_built() -> bool:
    from sce_author.verify import _default_codegen
    return _default_codegen().exists()


@unittest.skipUnless(codegen_is_built(),
                     "the product's code generator is not built in this tree")
class AVerificationCanShowACase(unittest.TestCase):
    def setUp(self):
        self.pack = load_pack(CROSSING)

    def run_with(self, *explain):
        result = verify(self.pack, BINDING, None, "python", tuple(explain))
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        return {case.name: case for case in result.results}

    # ---------------------------------------------------------------- asked

    def test_a_case_that_was_asked_for_carries_its_inputs_and_what_the_document_returned(self):
        shown = self.run_with(PASSING)[PASSING].shown
        self.assertEqual({"inputs", "returned", "kept"}, set(shown))
        self.assertTrue(shown["inputs"], "the inputs the binding handed the document are shown")
        self.assertTrue(shown["returned"], "what the document returned is shown")

    def test_a_failing_case_that_was_asked_for_is_shown_beside_its_failure(self):
        case = self.run_with(FAILING)[FAILING]
        self.assertTrue(case.failures, "it still fails")
        self.assertTrue(case.shown, "and it is shown")

    def test_what_is_shown_is_what_the_judge_compared(self):
        """The outputs shown are the document's own values, from which the failure was judged."""
        case = self.run_with(FAILING)[FAILING]
        self.assertIn("bell", case.shown["returned"])
        self.assertEqual(1, len(case.failures))

    # ------------------------------------------------------------ not asked

    def test_a_case_nobody_asked_about_carries_nothing(self):
        cases = self.run_with(PASSING)
        for name, case in cases.items():
            if name != PASSING:
                self.assertEqual({}, case.shown, name)

    def test_without_explain_nothing_is_shown_anywhere(self):
        for case in self.run_with().values():
            self.assertEqual({}, case.shown)

    # ------------------------------------------------------------ unchanged

    def test_asking_changes_no_verdict(self):
        plain = verify(self.pack, BINDING)
        asked = verify(self.pack, BINDING, None, "python", (PASSING, FAILING))
        self.assertEqual((plain.passed, plain.failed), (asked.passed, asked.failed))
        self.assertEqual([c.failures for c in plain.results], [c.failures for c in asked.results])

    # -------------------------------------------------------------- by text

    def test_a_name_matches_by_substring(self):
        shown = [n for n, c in self.run_with("healthy installation").items() if c.shown]
        self.assertEqual([PASSING], shown)

    def test_several_names_show_several_cases(self):
        shown = {n for n, c in self.run_with(PASSING, "bell is silent").items() if c.shown}
        self.assertEqual({PASSING, FAILING}, shown)

    def test_a_name_that_matches_nothing_shows_nothing_and_is_not_an_error(self):
        cases = self.run_with("no such case anywhere")
        self.assertEqual(5, len(cases))
        self.assertTrue(all(not c.shown for c in cases.values()))

    # --------------------------------------------------------------- honest

    def test_a_pure_computation_says_it_kept_nothing(self):
        """The fixture's document reads no previous(), so there is no holder: `kept` is empty on
        purpose, and the report says so rather than leaving it to be inferred."""
        shown = self.run_with(PASSING)[PASSING].shown
        self.assertEqual({}, shown["kept"])


if __name__ == "__main__":
    unittest.main()
