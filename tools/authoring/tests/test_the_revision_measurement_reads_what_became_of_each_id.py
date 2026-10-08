"""The revision measurement says what became of every requirement id, and can be trusted to.

`eval/revision_identity.py` applies an owner's edit to a specification and to
its requirement list by rule, builds both lists as `scxml_requirement_set`
does, and reports for each id of the first list whether it still names the
same requirement. These tests hold the measurement honest, not the scheme:

* the corpus is the owner's own words and every edit applies, so a figure is
  about a revision and not about a quote that never matched;
* an edit that changes no word (`reflow`) is read as changing no id, which is
  the control without which a zero elsewhere would prove nothing;
* the two ways an id goes wrong are told apart: it names another requirement
  (silent, the dangerous one) or it is absent (loud, which the product reports
  as dangling);
* every id of the first list is accounted for exactly once.

The pinned numbers are the BASELINE `docs/adr/0006-a-requirement-keeps-its-id-
across-a-revision.md` cites, measured on the ids `requirement_set.build` gives
today (reading order). A change to how that function makes ids or `rev` moves
them, and this file and the ADR's table change in the same commit.
"""

from __future__ import annotations

import pathlib
import sys
import unittest

EVAL = pathlib.Path(__file__).resolve().parents[1] / "eval"
sys.path.insert(0, str(EVAL))

import revision_identity as ri  # noqa: E402


def row(case: str, edit: str) -> dict:
    found = [r for r in ri.measure() if r["case"] == case and r["edit"] == edit]
    assert len(found) == 1, (case, edit, found)
    return found[0]


class TheCorpusIsTheOwnersWordsAndEveryEditApplies(unittest.TestCase):
    def test_every_edit_of_every_case_builds_a_list_for_each_revision(self):
        rows = ri.measure()
        self.assertEqual(13, len(rows))
        self.assertEqual({"vending-controller", "door-with-auto-close", "connection-keeper"},
                         {r["case"] for r in rows})

    def test_every_id_of_the_first_list_is_accounted_for_exactly_once(self):
        for r in ri.measure():
            self.assertEqual(r["requirements"],
                             r["holds"] + r["silent_wrong"] + r["lost"] + r["retired"], r)

    def test_a_quote_that_is_not_the_specifications_words_is_refused_not_measured(self):
        case = {k: v for k, v in ri.load_cases()[0].items()}
        case["requirements"] = case["requirements"][:1] + [
            {"quote": "The controller starts without any credit", "statement": "a rewording"}]
        with self.assertRaises(ri.CaseError):
            ri.measure_edit(case, case["edits"][0])


class TheControlChangesNoWordAndSoNoId(unittest.TestCase):
    def test_a_reflow_leaves_every_id_naming_what_it_named(self):
        reflows = [r for r in ri.measure() if r["edit"] == "reflow"]
        self.assertEqual(3, len(reflows))
        for r in reflows:
            self.assertEqual(r["requirements"], r["holds"], r)
            self.assertEqual((0, 0, 0, 0), (r["silent_wrong"], r["lost"], r["retired"], r["added"]), r)

    def test_an_edit_at_the_end_or_of_a_number_leaves_every_id_in_place(self):
        for case, edit in (("vending-controller", "append"), ("vending-controller", "reword-a-number"),
                           ("door-with-auto-close", "reword-a-number"),
                           ("connection-keeper", "reword-a-number")):
            r = row(case, edit)
            self.assertEqual(r["requirements"], r["holds"], r)


class TheBaselineIdsAreReadingOrderAndASentenceInsertedEarlyRepointsThem(unittest.TestCase):
    def test_a_sentence_inserted_after_the_first_repoints_every_later_id(self):
        r = row("vending-controller", "insert-early")
        self.assertEqual((11, 1, 10, 0, 0), (r["requirements"], r["holds"], r["silent_wrong"],
                                             r["lost"], r["retired"]), r)
        self.assertEqual(10, r["sections_moved"])

    def test_two_sentences_that_swap_places_swap_their_ids(self):
        r = row("vending-controller", "swap-two-sentences")
        self.assertEqual((9, 2, 0), (r["holds"], r["silent_wrong"], r["lost"]), r)

    def test_a_deleted_sentence_hands_its_id_to_the_next_and_loses_the_last(self):
        r = row("vending-controller", "delete")
        self.assertEqual((3, 7, 1, 0), (r["holds"], r["silent_wrong"], r["lost"], r["retired"]), r)
        r = row("door-with-auto-close", "delete")
        self.assertEqual((4, 1, 1, 0), (r["holds"], r["silent_wrong"], r["lost"], r["retired"]), r)

    def test_the_revision_note_cannot_tell_two_revisions_apart(self):
        # The tool leaves `rev` at "1" for every list it builds, so a document
        # citing `spec@1` is never told its manifest is another revision's.
        moved = [r for r in ri.measure() if r["edit"] != "reflow"]
        self.assertTrue(moved)
        for r in moved:
            self.assertEqual(["1", "1"], r["rev"], r)
        self.assertTrue(any(r["same_rev_told_apart_by_nothing"] for r in moved))
        control = [r for r in ri.measure() if r["edit"] == "reflow"]
        self.assertFalse(any(r["same_rev_told_apart_by_nothing"] for r in control))


if __name__ == "__main__":
    unittest.main()
