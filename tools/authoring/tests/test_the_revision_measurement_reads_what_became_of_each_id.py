"""The revision measurement says what became of every requirement id, and can be trusted to.

`eval/revision_identity.py` applies an owner's edit to a specification and to
its requirement list by rule, builds both lists as `scxml_requirement_set`
does, and reports for each id of the first list whether it still names the
same requirement. These tests hold the measurement honest first, then pin the
figures `docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md` cites:

* the corpus is the owner's own words and every edit applies, so a figure is
  about a revision and not about a quote that never matched;
* an edit that changes no word (`reflow`) is read as changing no id under every
  scheme, which is the control without which a zero elsewhere would prove nothing;
* the ways an id goes wrong are told apart: it names another requirement and
  nothing says so (silent, the dangerous one), it names another and the list
  says it changed (miscarried), or it is absent (loud, reported as dangling);
* every id of the first list is accounted for exactly once.

The pinned numbers are measured, not wished for. The reading-order scheme is
the baseline the ADR started from (the list built with no lineage); the lineage
schemes are what it proposed. A change to how ids or `rev` are made moves them,
and this file and the ADR's tables change in the same commit.
"""

from __future__ import annotations

import pathlib
import sys
import unittest

EVAL = pathlib.Path(__file__).resolve().parents[1] / "eval"
sys.path.insert(0, str(EVAL))

import revision_identity as ri  # noqa: E402


def row(case: str, edit: str, scheme: str = ri.TODAY) -> dict:
    found = [r for r in ri.measure(scheme=scheme) if r["case"] == case and r["edit"] == edit]
    assert len(found) == 1, (case, edit, found)
    return found[0]


class TheCorpusIsTheOwnersWordsAndEveryEditApplies(unittest.TestCase):
    def test_every_edit_of_every_case_builds_a_list_for_each_revision_under_each_scheme(self):
        for scheme in ri.SCHEMES:
            rows = ri.measure(scheme=scheme)
            self.assertEqual(16, len(rows), scheme)
            self.assertEqual({"vending-controller", "door-with-auto-close", "connection-keeper"},
                             {r["case"] for r in rows})

    def test_every_id_of_the_first_list_is_accounted_for_exactly_once(self):
        for scheme in ri.SCHEMES:
            for r in ri.measure(scheme=scheme):
                self.assertEqual(
                    r["requirements"],
                    r["holds"] + r["silent_wrong"] + r["miscarried"] + r["lost"] + r["retired"], r)

    def test_a_quote_that_is_not_the_specifications_words_is_refused_not_measured(self):
        case = {k: v for k, v in ri.load_cases()[0].items()}
        case["requirements"] = case["requirements"][:1] + [
            {"quote": "The controller starts without any credit", "statement": "a rewording"}]
        with self.assertRaises(ri.CaseError):
            ri.measure_edit(case, case["edits"][0])


class TheControlChangesNoWordAndSoNoId(unittest.TestCase):
    def test_a_reflow_leaves_every_id_naming_what_it_named_under_every_scheme(self):
        for scheme in ri.SCHEMES:
            reflows = [r for r in ri.measure(scheme=scheme) if r["edit"] == "reflow"]
            self.assertEqual(3, len(reflows))
            for r in reflows:
                self.assertEqual(r["requirements"], r["holds"], r)
                self.assertEqual((0, 0, 0, 0, 0), (r["silent_wrong"], r["miscarried"], r["lost"],
                                                   r["retired"], r["added"]), r)

    def test_an_edit_at_the_end_or_of_a_number_leaves_every_id_in_place_under_reading_order(self):
        for case, edit in (("vending-controller", "append"), ("vending-controller", "reword-a-number"),
                           ("door-with-auto-close", "reword-a-number"),
                           ("connection-keeper", "reword-a-number")):
            r = row(case, edit)
            self.assertEqual(r["requirements"], r["holds"], r)


class TheBaselineIdsAreReadingOrderAndASentenceInsertedEarlyRepointsThem(unittest.TestCase):
    def test_a_sentence_inserted_after_the_first_repoints_every_later_id(self):
        r = row("vending-controller", "insert-early")
        self.assertEqual((11, 1, 10, 0, 0, 0), (r["requirements"], r["holds"], r["silent_wrong"],
                                                r["miscarried"], r["lost"], r["retired"]), r)
        self.assertEqual(10, r["sections_moved"])

    def test_two_sentences_that_swap_places_swap_their_ids(self):
        r = row("vending-controller", "swap-two-sentences")
        self.assertEqual((9, 2, 0), (r["holds"], r["silent_wrong"], r["lost"]), r)

    def test_a_deleted_sentence_hands_its_id_to_the_next_and_loses_the_last(self):
        r = row("vending-controller", "delete")
        self.assertEqual((3, 7, 1, 0), (r["holds"], r["silent_wrong"], r["lost"], r["retired"]), r)
        r = row("door-with-auto-close", "delete")
        self.assertEqual((4, 1, 1, 0), (r["holds"], r["silent_wrong"], r["lost"], r["retired"]), r)

    def test_a_replaced_sentence_gives_its_id_to_the_sentence_that_stands_in_its_place(self):
        for case, edit in (("vending-controller", "replace-a-sentence-unrelated"),
                           ("door-with-auto-close", "replace-a-sentence-similar-wording"),
                           ("connection-keeper", "replace-a-sentence-unrelated")):
            r = row(case, edit)
            self.assertEqual((r["requirements"] - 1, 1, 0), (r["holds"], r["silent_wrong"], r["retired"]), r)

    def test_the_revision_note_cannot_tell_two_revisions_apart(self):
        # The tool left `rev` at "1" for every list it built, so a document
        # citing `spec@1` was never told its manifest is another revision's.
        moved = [r for r in ri.measure() if r["edit"] != "reflow"]
        self.assertTrue(moved)
        for r in moved:
            self.assertEqual(["1", "1"], r["rev"], r)
        self.assertTrue(any(r["same_rev_told_apart_by_nothing"] for r in moved))
        control = [r for r in ri.measure() if r["edit"] == "reflow"]
        self.assertFalse(any(r["same_rev_told_apart_by_nothing"] for r in control))

    def test_the_baseline_totals(self):
        t = ri.totals(ri.measure())
        self.assertEqual((127, 93, 32, 0, 2, 0), (t["requirements"], t["holds"], t["silent_wrong"],
                                                  t["miscarried"], t["lost"], t["retired"]), t)


class ALineageStopsAnIdFromNamingAnotherRequirementUnnoticed(unittest.TestCase):
    def test_no_edit_of_the_corpus_leaves_an_id_naming_another_requirement_unflagged(self):
        for scheme in (ri.WITH_WORDS, ri.HASHES_ONLY):
            for r in ri.measure(scheme=scheme):
                self.assertEqual(0, r["silent_wrong"], r)

    def test_an_insert_a_delete_and_a_swap_hold_every_id_of_a_requirement_that_is_still_there(self):
        for scheme in (ri.WITH_WORDS, ri.HASHES_ONLY):
            for case, edit in (("vending-controller", "insert-early"), ("door-with-auto-close", "insert-early"),
                               ("connection-keeper", "insert-early"), ("vending-controller", "append"),
                               ("vending-controller", "swap-two-sentences")):
                r = row(case, edit, scheme)
                self.assertEqual(r["requirements"], r["holds"], r)
            for case, edit in (("vending-controller", "delete"), ("door-with-auto-close", "delete")):
                r = row(case, edit, scheme)
                self.assertEqual((r["requirements"] - 1, 1), (r["holds"], r["retired"]), r)

    def test_the_revision_follows_the_text_and_not_the_whitespace(self):
        for scheme in (ri.WITH_WORDS, ri.HASHES_ONLY):
            for r in ri.measure(scheme=scheme):
                expect = ["1", "1"] if r["edit"] == "reflow" else ["1", "2"]
                self.assertEqual(expect, r["rev"], r)

    def test_a_reworded_requirement_keeps_its_id_and_is_called_changed_when_its_words_are_given(self):
        rows = ri.measure(scheme=ri.WITH_WORDS)
        rewords = [r for r in rows if r["edit"] == "reword-a-number"]
        self.assertEqual(3, len(rewords))
        for r in rewords:
            self.assertEqual(r["requirements"], r["holds"], r)
        t = ri.totals(rows)
        self.assertEqual(t["reworded"], t["changed_flagged"], t)
        self.assertEqual(4, t["reworded"])
        self.assertEqual(0, t["lost"])

    def test_without_the_words_a_reworded_requirement_loses_its_id_loudly(self):
        rows = ri.measure(scheme=ri.HASHES_ONLY)
        t = ri.totals(rows)
        self.assertEqual((4, 0), (t["lost"], t["silent_wrong"]), t)
        self.assertEqual({"reword-a-number"}, {r["edit"] for r in rows if r["lost"]})

    def test_the_price_of_carrying_an_id_through_a_rewording_is_measured(self):
        # Of three requirements the owner REPLACED, the two unrelated ones are not carried; the
        # one whose new sentence differs by a single word ("closed" -> "locked") is, and the list
        # calls it changed so a person reads it. Wording alone cannot tell an edit from a
        # replacement; what makes this safe is that "same" needs equal words.
        replaced = [r for r in ri.measure(scheme=ri.WITH_WORDS) if r["replaced"]]
        self.assertEqual(3, len(replaced))
        self.assertEqual(1, sum(r["miscarried"] for r in replaced))
        self.assertEqual("replace-a-sentence-similar-wording",
                         next(r["edit"] for r in replaced if r["miscarried"]))
        self.assertEqual(0, sum(r["miscarried"] for r in ri.measure(scheme=ri.HASHES_ONLY)))

    def test_the_lineage_totals(self):
        words = ri.totals(ri.measure(scheme=ri.WITH_WORDS))
        self.assertEqual((127, 122, 0, 1, 0, 4), (words["requirements"], words["holds"], words["silent_wrong"],
                                                  words["miscarried"], words["lost"], words["retired"]), words)
        hashes = ri.totals(ri.measure(scheme=ri.HASHES_ONLY))
        self.assertEqual((127, 118, 0, 0, 4, 5), (hashes["requirements"], hashes["holds"], hashes["silent_wrong"],
                                                  hashes["miscarried"], hashes["lost"], hashes["retired"]), hashes)


if __name__ == "__main__":
    unittest.main()
