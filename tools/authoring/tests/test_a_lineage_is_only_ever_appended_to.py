"""A lineage is only ever appended to, and is the lineage of the list beside it.

A work keeps the lineage a requirement list was built against, in the same record as the list
(`docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md`), and the core
refuses to lose it. What the core does not know is what a lineage MEANS, so two more questions
are the authoring package's, asked before a list is saved:

* `belongs_to_manifest`: is this the lineage of THIS manifest? A list and its lineage are one
  fact, and a lineage whose last revision names another list would be stored beside this one;
* `extends`: does it continue the lineage the work holds? Nothing a lineage said is taken back: an
  id once issued is still there with the words it had, a retired id stays retired, revisions are
  the same revisions with more after them, and ids issued since are numbered from where the last
  left off, so that none is issued twice.

Each rule has a case that breaks only it, and a control that does not.
"""

from __future__ import annotations

import copy
import json
import unittest

from sce_author import requirement_lineage as rl, requirement_set as rs

LAMP = ("The lamp starts off. Pressing the switch turns it on; pressing it again turns it off. "
        "After 30 seconds on, it turns itself off. Nothing else changes it.")
QUOTES = ["The lamp starts off.", "Pressing the switch turns it on", "pressing it again turns it off",
          "After 30 seconds on, it turns itself off.", "Nothing else changes it."]


def items(quotes, statement="s"):
    return [{"quote": q, "statement": statement} for q in quotes]


def build(prose, quotes, previous=None, statement="s"):
    kw = {}
    if previous is not None:
        kw = {"lineage_text": rl.render(previous.lineage),
              "previous_sidecar_text": json.dumps(previous.sidecar)}
    built = rs.build(prose, items(quotes, statement), doc_id="lamp", **kw)
    assert not built.refused, built.refused
    return built


def manifest_of(built):
    return rs.render_manifest(built.manifest)


class Histories(unittest.TestCase):
    """Three revisions: the last sentence is dropped, then a new one is added."""

    def setUp(self):
        self.first = build(LAMP, QUOTES)
        self.second = build(LAMP.replace(" Nothing else changes it.", ""), QUOTES[:4], self.first)
        self.third = build(LAMP.replace(" Nothing else changes it.", " A reset key clears it."),
                           QUOTES[:4] + ["A reset key clears it."], self.second)

    def broken(self, built, edit):
        lineage = copy.deepcopy(built.lineage)
        edit(lineage)
        return lineage


class ALineageThatContinuesTheWorksIsAccepted(Histories):
    def test_each_step_extends_the_one_before_it(self):
        rl.extends(self.first.lineage, self.second.lineage)
        rl.extends(self.second.lineage, self.third.lineage)

    def test_a_later_lineage_extends_an_earlier_one_that_is_not_the_last_before_it(self):
        rl.extends(self.first.lineage, self.third.lineage)

    def test_a_lineage_extends_itself(self):
        rl.extends(self.second.lineage, copy.deepcopy(self.second.lineage))

    def test_the_same_text_read_into_another_list_is_the_same_revision_with_another_manifest(self):
        # The last sentence is not taken as a requirement this time: the same text, another list,
        # and an id retired within the revision it was issued in.
        again = build(LAMP, QUOTES[:4], self.first)
        self.assertEqual(self.first.lineage["revisions"][-1]["rev"],
                         again.lineage["revisions"][-1]["rev"])
        self.assertNotEqual(self.first.lineage["revisions"][-1]["manifest_sha256"],
                            again.lineage["revisions"][-1]["manifest_sha256"])
        rl.extends(self.first.lineage, again.lineage)

    def test_a_requirement_requoted_within_the_last_revision_may_have_its_last_words_replaced(self):
        again = build(LAMP, QUOTES[:4] + ["Nothing else changes it"], self.first)
        rl.extends(self.first.lineage, again.lineage)


class ALineageThatTakesSomethingBackIsRefused(Histories):
    def refused(self, previous, following, says):
        with self.assertRaises(rl.LineageError) as raised:
            rl.extends(previous, following)
        self.assertIn(says, str(raised.exception))

    def test_another_specification(self):
        other = self.broken(self.second, lambda l: l.update(doc_id="other"))
        self.refused(self.first.lineage, other, "one lineage follows one specification")

    def test_a_next_that_is_behind(self):
        behind = self.broken(self.third, lambda l: l.update(next=self.first.lineage["next"] - 1))
        # `_checked` refuses a next that is not past every id; lower the ids with it is not the
        # point, so judge it against the work's lineage directly.
        self.refused(self.third.lineage, behind, "is behind the work's")

    def test_fewer_revisions(self):
        fewer = self.broken(self.third, lambda l: l["revisions"].pop())
        self.refused(self.third.lineage, fewer, "revisions are never taken back")

    def test_an_earlier_revision_rewritten(self):
        def rewrite(lineage):
            lineage["revisions"][0]["spec_sha256"] = "0" * 64
        self.refused(self.first.lineage, self.broken(self.second, rewrite), "is not rewritten")

    def test_the_last_revision_rewritten_when_more_follow_it(self):
        def rewrite(lineage):
            lineage["revisions"][0]["manifest_sha256"] = "0" * 64
        # Unlike the same text read again, a revision that has another after it is final.
        self.refused(self.first.lineage, self.broken(self.second, rewrite), "is not rewritten")

    def test_an_id_forgotten(self):
        def forget(lineage):
            lineage["requirements"] = [r for r in lineage["requirements"] if r["id"] != "R5"]
        self.refused(self.second.lineage, self.broken(self.third, forget), "R5 is in the work's lineage")

    def test_an_id_first_issued_in_another_revision(self):
        def move(lineage):
            lineage["requirements"][0]["first_rev"] = "2"
        self.refused(self.second.lineage, self.broken(self.third, move), "was first issued in revision 1")

    def test_a_retired_id_made_live_again(self):
        retired = next(r for r in self.second.lineage["requirements"] if r["retired_rev"])
        def revive(lineage):
            next(r for r in lineage["requirements"] if r["id"] == retired["id"])["retired_rev"] = None
        self.refused(self.second.lineage, self.broken(self.third, revive),
                     "a retired id is never issued again")

    def test_a_requirements_history_rewritten(self):
        def rewrite(lineage):
            row = next(r for r in lineage["requirements"] if r["id"] == "R4")
            row["quotes"] = [{"rev": "1", "sha256": "0" * 64}] + row["quotes"][1:]
        built = build(LAMP.replace("30 seconds", "45 seconds"), QUOTES[:3] + [
            "After 45 seconds on, it turns itself off."] + QUOTES[4:], self.first)
        self.assertGreater(len(next(r for r in built.lineage["requirements"]
                                    if r["id"] == "R4")["quotes"]), 1)
        self.refused(built.lineage, self.broken(build(LAMP.replace("30 seconds", "45 seconds").replace(
            "Nothing else changes it.", "Nothing else changes it. A reset key clears it."),
            QUOTES[:3] + ["After 45 seconds on, it turns itself off.", QUOTES[4],
                          "A reset key clears it."], built), rewrite), "history is only added to")

    def test_a_new_id_numbered_below_the_works_next(self):
        def add(lineage):
            lineage["requirements"].append({"id": "R1x", "first_rev": "2", "retired_rev": None,
                                            "quotes": [{"rev": "2", "sha256": "0" * 64}]})
        self.refused(self.second.lineage, self.broken(self.third, add), "is not an id issued from")

    def test_a_new_id_that_was_already_issued_by_number(self):
        def add(lineage):
            lineage["requirements"].append({"id": "R0", "first_rev": "3", "retired_rev": None,
                                            "quotes": [{"rev": "3", "sha256": "0" * 64}]})
        self.refused(self.second.lineage, self.broken(self.third, add), "would be issued twice")


def kinds(delta):
    wanted = delta["requirements"]
    return {"carried": wanted["carried"], "changed": [c["id"] for c in wanted["changed"]],
            "new": wanted["new"], "retired": wanted["retired"]}


class TheWordsBetweenTwoStatesOfAWorkAreDerivedFromTheirLineages(Histories):
    """The words delta of a revision, derived from the lineage of the list that was accepted and the
    lineage of the list the work has now: any two states, however many revisions lie between."""

    def test_a_state_between_itself_carries_everything_over(self):
        self.assertEqual({"carried": ["R1", "R2", "R3", "R4", "R5"], "changed": [], "new": [],
                          "retired": []}, kinds(rl.between(self.first.lineage, self.first.lineage)))

    def test_one_step_says_what_it_dropped(self):
        delta = rl.between(self.first.lineage, self.second.lineage)
        self.assertEqual({"carried": ["R1", "R2", "R3", "R4"], "changed": [], "new": [],
                          "retired": ["R5"]}, kinds(delta))

    def test_two_steps_at_once_are_what_each_alone_would_say_put_together(self):
        # R5 was dropped in the second revision and the third added R6 (a new id, not R5 again).
        delta = rl.between(self.first.lineage, self.third.lineage)
        self.assertEqual({"carried": ["R1", "R2", "R3", "R4"], "changed": [], "new": ["R6"],
                          "retired": ["R5"]}, kinds(delta))

    def test_what_is_issued_and_retired_between_the_two_is_not_listed(self):
        gone = build(LAMP.replace(" Nothing else changes it.", ""), QUOTES[:4], self.first)
        later = build(LAMP.replace(" Nothing else changes it.", " A reset key clears it."),
                      QUOTES[:4] + ["A reset key clears it."], gone)
        again = build(LAMP.replace(" Nothing else changes it.", ""), QUOTES[:4], later)
        listed = kinds(rl.between(self.first.lineage, again.lineage))
        self.assertEqual(["R5"], listed["retired"])
        self.assertNotIn("R6", listed["new"] + listed["carried"] + listed["retired"])

    def test_words_that_changed_are_changed_and_keep_their_id(self):
        reworded = build(LAMP.replace("30 seconds", "45 seconds"), QUOTES[:3] + [
            "After 45 seconds on, it turns itself off."] + QUOTES[4:], self.first)
        delta = rl.between(self.first.lineage, reworded.lineage)
        self.assertEqual(["R4"], kinds(delta)["changed"])
        self.assertEqual(["R1", "R2", "R3", "R5"], kinds(delta)["carried"])

    def test_words_reworded_and_reworded_back_are_carried(self):
        away = build(LAMP.replace("30 seconds", "45 seconds"), QUOTES[:3] + [
            "After 45 seconds on, it turns itself off."] + QUOTES[4:], self.first)
        back = build(LAMP, QUOTES, away)
        self.assertEqual([], kinds(rl.between(self.first.lineage, back.lineage))["changed"])

    def test_the_delta_names_the_list_it_starts_from_and_the_revisions(self):
        delta = rl.between(self.first.lineage, self.third.lineage)
        self.assertEqual(("lamp", "1", "3"), (delta["doc_id"], delta["from_rev"], delta["rev"]))
        self.assertEqual(self.first.lineage["revisions"][-1]["manifest_sha256"],
                         delta["from_manifest_sha256"])
        self.assertTrue(delta["specification_changed"])
        self.assertFalse(rl.between(self.first.lineage, self.first.lineage)["specification_changed"])

    def test_the_delta_has_the_shape_the_join_reads(self):
        from sce_author import revision
        record = {"manifest": {"doc_id": "lamp", "rev": "1",
                               "sha256": self.first.lineage["revisions"][-1]["manifest_sha256"]}}
        delta = rl.between(self.first.lineage, self.third.lineage)
        revision.belongs_to(delta, record)
        revision.join(delta, {"requirements": [], "unclaimed": {"added": [], "gone": 0}})

    def test_a_lineage_that_does_not_continue_the_other_says_nothing_about_it(self):
        other = rs.build(LAMP, items(QUOTES), doc_id="lamp")
        for older, newer in ((self.third.lineage, self.first.lineage),
                             (self.first.lineage, self.broken(other, lambda l: l.update(doc_id="x")))):
            with self.subTest(), self.assertRaises(rl.LineageError):
                rl.between(older, newer)

    def test_a_lineage_adopted_from_a_list_that_predates_lineages_is_a_beginning_like_any(self):
        manifest_text = rs.render_manifest(self.first.manifest)
        adopted = rl.adopt("lamp", self.first.manifest,
                           {i: rs.normalise(t) for i, t in self.first.sidecar["text"].items()},
                           manifest_sha256=rl.sha256_text(manifest_text))
        built = rs.build(LAMP.replace(" Nothing else changes it.", ""), items(QUOTES[:4]), doc_id="lamp",
                         previous_manifest_text=manifest_text,
                         previous_sidecar_text=json.dumps(self.first.sidecar))
        delta = rl.between(adopted, built.lineage)
        self.assertEqual(["R5"], kinds(delta)["retired"])
        self.assertEqual(rl.sha256_text(manifest_text), delta["from_manifest_sha256"])


class ALineageIsTheLineageOfTheManifestBesideIt(Histories):
    def test_the_lineage_of_a_call_is_the_lineage_of_its_manifest(self):
        rl.belongs_to_manifest(self.third.lineage, manifest_of(self.third))

    def refused(self, lineage, manifest_text, says):
        with self.assertRaises(rl.LineageError) as raised:
            rl.belongs_to_manifest(lineage, manifest_text)
        self.assertIn(says, str(raised.exception))

    def test_the_manifest_of_another_revision(self):
        self.refused(self.third.lineage, manifest_of(self.second), "last revision is")

    def test_the_manifest_of_another_specification(self):
        other = json.loads(manifest_of(self.third))
        other["doc_id"] = "other"
        self.refused(self.third.lineage, json.dumps(other, indent=2) + "\n", "is of 'lamp'")

    def test_a_manifest_that_is_the_same_list_with_another_spelling(self):
        # The digest is of the bytes a caller saves, which an acceptance pins: a list
        # re-serialised is another list as far as everything downstream is concerned.
        text = json.dumps(json.loads(manifest_of(self.third)), indent=4) + "\n"
        self.refused(self.third.lineage, text, "another manifest than this one")

    def test_a_lineage_that_does_not_say_which_manifest_it_was_written_as(self):
        for what, lineage in (
                ("a null digest", self.broken(self.third, lambda l: l["revisions"][-1].update(manifest_sha256=None))),
                ("no digest at all", self.broken(self.third, lambda l: l["revisions"][-1].pop("manifest_sha256")))):
            with self.subTest(what):
                self.refused(lineage, manifest_of(self.third), "does not say which manifest")

    def test_a_manifest_that_is_not_json(self):
        self.refused(self.third.lineage, "not json", "is not JSON")
        self.refused(self.third.lineage, "[]", "is not a JSON object")


if __name__ == "__main__":
    unittest.main()
