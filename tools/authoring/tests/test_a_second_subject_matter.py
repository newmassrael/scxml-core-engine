"""The whole pipeline, on a subject matter that shares nothing with the first.

Every other test here uses a fixture built to exercise one check. This one
uses a complete pack for a different discipline -- other name prefixes, other
value idioms, comparisons written in words, addresses with no dotted
hierarchy, a different gate-off cascade -- and runs brief, questions and check
end to end against it.

⚠ It is a KNOWN-ANSWER test, and that is what makes it evidence rather than a
demonstration. Four gaps were put into the specification on purpose and are
listed below. The tool has to find those four, and it has to find nothing
else. Writing the specification and the pack by the same hand would otherwise
prove only that the hand is consistent.

Both halves are asserted, because either alone is worthless: a tool that names
what is missing but produces no document is a critic, and a tool that produces
a document without naming what is missing is a guess.

What this test has already caught, in the hour it was written:

  * a value space keyed by a YAML boolean crashed the loader instead of being
    refused (bare ON and OFF)
  * an output named in words rather than by identifier was reported as
    undecided -- seven false findings out of eleven
  * a document with a double hyphen in a comment raised a traceback out of
    `check` instead of an answer

None of the three was reachable from the first subject matter, because its
specifications write identifiers and its packs happened to avoid the words.
"""

import json
import pathlib
import unittest

from sce_author.brief import assemble
from sce_author.check import check
from sce_author.pack import load_pack
from sce_author.prose import load_prose
from sce_author.questions import ask

HERE = pathlib.Path(__file__).resolve().parent
PACK = HERE / "fixtures" / "crossing"
SPEC = PACK / "specification.md"
BINDING = PACK / "controller.binding.yaml"

# Put into the specification deliberately. Each is a different KIND of silence,
# so between them they exercise every route by which a gap can be noticed.
PLANTED = {
    # a value the platform cannot take (the detector says DETECTED)
    "value-not-in-space": "IN_ObstacleDetector is BLOCKED",
    # driven by the examples, never named in the prose
    "example-drives-unnamed-signal": "plant/in/mains-power",
    # read back by the examples, never decided by the prose
    "example-expects-undecided-output": "plant/out/train-signal.value",
    # a duration with nothing able to observe time
    "no-time-input": "8 seconds",
    # two records drive every input identically and the bell does two
    # different things, so the installation remembers something the prose
    # never states. ⚠ No other class here can reach this one: the model says
    # only what exists, the prose describes each situation correctly, and a
    # document written for it would be a syntactically pure computation.
    "example-shows-memory": "plant/out/bell.value",
}


class ASecondSubjectMatter(unittest.TestCase):
    def setUp(self):
        self.pack = load_pack(PACK)
        self.prose = load_prose([SPEC])
        self.found = ask(self.prose, self.pack.model, self.pack.conventions,
                         self.pack.examples)

    def test_every_planted_gap_is_found(self):
        for kind, subject in PLANTED.items():
            hits = [q for q in self.found if q.kind == kind and q.subject == subject]
            self.assertEqual(
                1, len(hits),
                f"the specification hides a {kind} at {subject!r} and the tool "
                f"found {len(hits)}",
            )

    def test_nothing_else_is_reported_as_a_defect(self):
        """The half that keeps this honest.

        Finding the planted gaps is easy for a tool that reports everything.
        What has to hold is that the outputs the specification DOES decide are
        left alone.
        """
        loud = [q for q in self.found if q.severity == "error"]
        unexpected = [q for q in loud
                      if PLANTED.get(q.kind) != q.subject]
        self.assertEqual(
            [], [f"{q.kind} {q.subject}" for q in unexpected],
            "a specification that decides an output must not be told it does not",
        )

    def test_the_specification_is_not_called_silent_where_it_speaks(self):
        """It names its outputs in words, not identifiers. A reader that only
        matches identifiers calls four decided outputs undecided."""
        undecided = {q.subject for q in self.found if q.kind == "no-decision-logic"}
        for decided in ("plant/out/road-signal", "plant/out/bell", "plant/out/barrier"):
            self.assertNotIn(decided, undecided)

    def test_the_pack_and_not_the_core_supplies_the_reading(self):
        """`is` comparisons, `IN_`/`OUT_` prefixes and a DARK-first cascade all
        come from this pack. If any of them had leaked into the core as a
        default, the first subject matter would still pass and this would not."""
        conv = self.pack.conventions
        self.assertIsNotNone(conv.comparison_pattern)
        self.assertEqual({"supplied", "own_internal", "own_output"},
                         {c.role for c in conv.name_classes})
        self.assertEqual("DARK", conv.gate_off[0]["use"])

    def test_a_document_comes_out_and_reaches_the_platform(self):
        """The other half. Questions without a document are a critic."""
        findings = check(self.pack, BINDING)
        self.assertEqual([], [str(f) for f in findings])

    def test_the_undecided_output_is_marked_rather_than_guessed(self):
        """What an author does with an answer nobody has.

        A value invented here would compile, reach the platform, and be wrong
        in a way nothing downstream could see. The marker is the only honest
        thing to write, and it has to survive in the document.
        """
        document = (PACK / "controller.scxml").read_text(encoding="utf-8")
        self.assertIn("sce:unresolved", document)
        self.assertIn("trainSignal", document)

    def test_the_brief_carries_the_whole_specification(self):
        page = assemble(self.prose, self.pack)
        self.assertIn("Level crossing controller", page)
        self.assertIn("plant/out/road-signal", page)


if __name__ == "__main__":
    unittest.main()
