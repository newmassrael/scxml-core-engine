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
