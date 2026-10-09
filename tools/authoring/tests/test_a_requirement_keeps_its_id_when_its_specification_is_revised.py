"""A requirement keeps its id when its specification is revised.

`requirement_set.build` gave ids by reading order, so a sentence inserted early
renumbered everything after it and an id could name another requirement with
nothing to say so (`eval/revision_identity.py` measured 29 of 105 ids doing it).
The lineage a list is built against remembers which id was issued for which
requirement. These tests hold what that promises:

* the first list of a specification is the list it always was;
* a later revision keeps the id of every requirement it still has, closes the
  id of one it lost and never issues that id again;
* "same" is claimed on equal words only: a requirement whose words changed is
  `changed`, and keeps its id only when the previous sidecar shows the words it
  had, or the caller says so;
* when a near match is not clearly the only one, nothing is carried;
* the lineage holds no word of the specification;
* what is not a lineage, or is another specification's, is refused.
"""

from __future__ import annotations

import hashlib
import io
import json
import pathlib
import unittest

import jsonschema

from sce_author import mcp, requirement_lineage as rl, requirement_set as rs

SCHEMA = pathlib.Path(__file__).resolve().parents[1] / "schema" / "requirement-lineage.v1.schema.json"

LAMP = ("The lamp starts off. Pressing the switch turns it on; pressing it again turns it off. "
        "After 30 seconds on, it turns itself off. Nothing else changes it.")
LAMP_QUOTES = ["The lamp starts off.", "Pressing the switch turns it on", "pressing it again turns it off",
               "After 30 seconds on, it turns itself off.", "Nothing else changes it."]


def items(quotes):
    return [{"quote": q, "statement": "s"} for q in quotes]


def build(prose, quotes, **kw):
    built = rs.build(prose, items(quotes), **kw)
    assert not built.refused, built.refused
    return built


def revise(first, prose, quotes, **kw):
    """The next revision of `first`, with the words behind its ids unless told not to."""
    kw.setdefault("previous_sidecar_text", json.dumps(first.sidecar))
    return build(prose, quotes, lineage_text=rl.render(first.lineage), **kw)


def id_of(built, quote):
    return next(r.id for r in built.requirements if r.quote == quote)


def call(name, **arguments):
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}
    out = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), out)
    return json.loads(out.getvalue())["result"]


class TheFirstListIsTheListItAlwaysWas(unittest.TestCase):
    def test_ids_are_in_reading_order_and_the_revision_is_one(self):
        built = build(LAMP, LAMP_QUOTES)
        self.assertEqual(["R1", "R2", "R3", "R4", "R5"], [r.id for r in built.requirements])
        self.assertEqual("1", built.manifest["rev"])
        self.assertIsNone(built.delta)
        self.assertEqual(6, built.lineage["next"])

    def test_a_first_answer_has_no_status_and_no_delta_and_carries_the_lineage(self):
        answer = rs.answer(build(LAMP, LAMP_QUOTES))
        self.assertNotIn("delta", answer)
        self.assertTrue(all("status" not in r for r in answer["requirements"]))
        self.assertEqual("1", answer["rev"])
        self.assertEqual(5, len(json.loads(answer["lineage_text"])["requirements"]))

    def test_a_call_with_no_lineage_says_it_cannot_tell_whether_the_specification_was_built_before(self):
        first = rs.answer(build(LAMP, LAMP_QUOTES))
        self.assertIn("first list of this specification as far as this call can tell", first["next"])
        self.assertIn("previous_sidecar", first["next"])
        revision = rs.answer(revise(build(LAMP, LAMP_QUOTES), LAMP + " A fuse protects it.",
                                    LAMP_QUOTES + ["A fuse protects it."]))
        self.assertNotIn("first list of this specification", revision["next"])
        self.assertIn("a revision", revision["next"])

    def test_the_lineage_holds_no_word_of_the_specification(self):
        built = build(LAMP, LAMP_QUOTES)
        text = rl.render(built.lineage)
        for quote in LAMP_QUOTES:
            self.assertNotIn(quote, text)
        for word in ("lamp", "switch", "seconds", "Pressing"):
            self.assertNotIn(word, text)

    def test_a_lineage_validates_against_its_schema(self):
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, LAMP.replace("30 seconds", "45 seconds"),
                        [q.replace("30 seconds", "45 seconds") for q in LAMP_QUOTES])
        for built in (first, second):
            jsonschema.validate(built.lineage, schema)


class ARevisionKeepsTheIdOfEveryRequirementItStillHas(unittest.TestCase):
    def test_a_sentence_inserted_early_takes_a_new_id_and_moves_no_other(self):
        first = build(LAMP, LAMP_QUOTES)
        prose = LAMP.replace("The lamp starts off.", "The lamp starts off. The lamp has a green cover.")
        second = revise(first, prose, ["The lamp has a green cover."] + LAMP_QUOTES)
        for quote in LAMP_QUOTES:
            self.assertEqual(id_of(first, quote), id_of(second, quote), quote)
        self.assertEqual("R6", id_of(second, "The lamp has a green cover."))
        self.assertEqual("2", second.manifest["rev"])
        requirements = second.delta["requirements"]
        self.assertEqual((["R1", "R2", "R3", "R4", "R5"], ["R6"], [], []),
                         (requirements["carried"], requirements["new"], requirements["retired"],
                          requirements["changed"]))
        self.assertEqual({"kept": 4, "added": ["S2"], "removed": []}, second.delta["sentences"])

    def test_the_order_of_the_sentences_is_not_what_an_id_follows(self):
        first = build(LAMP, LAMP_QUOTES)
        prose = ("Nothing else changes it. The lamp starts off. Pressing the switch turns it on; pressing it "
                 "again turns it off. After 30 seconds on, it turns itself off.")
        second = revise(first, prose, [LAMP_QUOTES[4]] + LAMP_QUOTES[:4])
        for quote in LAMP_QUOTES:
            self.assertEqual(id_of(first, quote), id_of(second, quote), quote)

    def test_a_deleted_requirement_is_retired_and_its_id_is_never_issued_again(self):
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, LAMP.replace(" Nothing else changes it.", ""), LAMP_QUOTES[:4])
        self.assertEqual(["R5"], second.delta["requirements"]["retired"])
        row = next(r for r in second.lineage["requirements"] if r["id"] == "R5")
        self.assertEqual("2", row["retired_rev"])
        third = revise(second, LAMP.replace(" Nothing else changes it.", "") + " A fuse protects it.",
                       LAMP_QUOTES[:4] + ["A fuse protects it."])
        self.assertEqual("R6", id_of(third, "A fuse protects it."))
        self.assertNotIn("R5", [r.id for r in third.requirements])

    def test_a_reflow_is_not_a_revision(self):
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, LAMP.replace(". ", ".\n"), LAMP_QUOTES)
        self.assertEqual("1", second.manifest["rev"])
        self.assertFalse(second.delta["specification_changed"])
        self.assertEqual([], second.delta["requirements"]["new"] + second.delta["requirements"]["retired"])
        self.assertEqual(first.lineage, second.lineage)

    def test_the_ids_do_not_depend_on_the_order_the_client_listed_the_quotes_in(self):
        first = build(LAMP, LAMP_QUOTES)
        prose = LAMP + " A reset key clears it."
        forward = revise(first, prose, LAMP_QUOTES + ["A reset key clears it."])
        backward = revise(first, prose, list(reversed(LAMP_QUOTES + ["A reset key clears it."])))
        self.assertEqual(forward.manifest, backward.manifest)
        self.assertEqual(forward.lineage, backward.lineage)

    def test_three_revisions_keep_the_first_ones_ids(self):
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, LAMP + " A reset key clears it.", LAMP_QUOTES + ["A reset key clears it."])
        third = revise(second, "A fuse protects it. " + LAMP + " A reset key clears it.",
                       ["A fuse protects it."] + LAMP_QUOTES + ["A reset key clears it."])
        self.assertEqual("3", third.manifest["rev"])
        self.assertEqual(["R1", "R2", "R3", "R4", "R5", "R6"],
                         [id_of(third, q) for q in LAMP_QUOTES + ["A reset key clears it."]])
        self.assertEqual("R7", id_of(third, "A fuse protects it."))


class TheLineageNamesTheListOfEachRevisionByItsManifestsDigest(unittest.TestCase):
    """A name and a revision number are carried by every copy of a specification; the digest of the
    exact manifest text a caller saves, which an acceptance pins, is what says it is THIS list."""

    def sha(self, text):
        return hashlib.sha256(text.encode("utf-8")).hexdigest()

    def test_each_revision_row_holds_the_digest_of_the_manifest_text_its_list_was_written_as(self):
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, LAMP + " A reset key clears it.", LAMP_QUOTES + ["A reset key clears it."])
        self.assertEqual(self.sha(rs.answer(first)["manifest_text"]),
                         first.lineage["revisions"][-1]["manifest_sha256"])
        self.assertEqual([self.sha(rs.answer(first)["manifest_text"]), self.sha(rs.answer(second)["manifest_text"])],
                         [row["manifest_sha256"] for row in second.lineage["revisions"]])

    def test_the_delta_starts_from_the_list_the_previous_revision_was_written_as(self):
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, LAMP + " A reset key clears it.", LAMP_QUOTES + ["A reset key clears it."])
        self.assertEqual(self.sha(rs.answer(first)["manifest_text"]), second.delta["from_manifest_sha256"])

    def test_a_list_written_again_for_the_same_text_is_pinned_again(self):
        first = build(LAMP, LAMP_QUOTES)
        again = revise(first, LAMP, LAMP_QUOTES[:4] + ["Nothing else changes it"])
        self.assertEqual(first.lineage["revisions"][-1]["rev"], again.lineage["revisions"][-1]["rev"])
        self.assertEqual(self.sha(rs.answer(again)["manifest_text"]),
                         again.lineage["revisions"][-1]["manifest_sha256"])
        self.assertEqual(self.sha(rs.answer(first)["manifest_text"]), again.delta["from_manifest_sha256"])

    def test_a_list_that_predates_lineages_is_adopted_with_the_digest_of_the_manifest_it_was_given(self):
        old = build(LAMP, LAMP_QUOTES)
        manifest_text, sidecar_text = rs.answer(old)["manifest_text"], rs.answer(old)["sidecar_text"]
        second = build(LAMP + " A reset key clears it.", LAMP_QUOTES + ["A reset key clears it."],
                       previous_manifest_text=manifest_text, previous_sidecar_text=sidecar_text)
        self.assertEqual(self.sha(manifest_text), second.delta["from_manifest_sha256"])

    def test_a_lineage_made_before_the_digest_was_kept_still_reads_and_says_it_is_unknown(self):
        first = build(LAMP, LAMP_QUOTES)
        old = json.loads(rl.render(first.lineage))
        for row in old["revisions"]:
            del row["manifest_sha256"]
        second = rs.build(LAMP + " A reset key clears it.", items(LAMP_QUOTES + ["A reset key clears it."]),
                          lineage_text=json.dumps(old), previous_sidecar_text=json.dumps(first.sidecar))
        self.assertIsNone(second.delta["from_manifest_sha256"])
        self.assertEqual(self.sha(rs.answer(second)["manifest_text"]),
                         second.lineage["revisions"][-1]["manifest_sha256"])


class TheDeltaNamesTheSpecificationAndTheStepItDescribes(unittest.TestCase):
    def test_it_carries_the_doc_id_and_the_revision_it_starts_from(self):
        # `revision.belongs_to` compares these with the acceptance record's manifest, so a delta of
        # another specification, or of another step, is not joined to the wrong design.
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, LAMP + " A reset key clears it.", LAMP_QUOTES + ["A reset key clears it."])
        self.assertEqual(("spec", "1", "2"),
                         (second.delta["doc_id"], second.delta["from_rev"], second.delta["rev"]))


class SameIsClaimedOnEqualWordsOnly(unittest.TestCase):
    REWORDED = LAMP.replace("30 seconds", "45 seconds")
    REWORDED_QUOTES = LAMP_QUOTES[:3] + ["After 45 seconds on, it turns itself off."] + LAMP_QUOTES[4:]

    def test_a_reworded_requirement_keeps_its_id_when_the_words_it_had_are_given(self):
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, self.REWORDED, self.REWORDED_QUOTES)
        self.assertEqual("R4", id_of(second, self.REWORDED_QUOTES[3]))
        self.assertEqual("changed", next(r.status for r in second.requirements if r.id == "R4"))
        self.assertEqual([{"id": "R4", "how": "near-match"}], second.delta["requirements"]["changed"])
        row = next(r for r in second.lineage["requirements"] if r["id"] == "R4")
        self.assertEqual(["1", "2"], [q["rev"] for q in row["quotes"]])

    def test_without_the_words_it_had_the_requirement_is_a_new_one_and_the_old_id_is_closed(self):
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, self.REWORDED, self.REWORDED_QUOTES, previous_sidecar_text=None)
        self.assertEqual("R6", id_of(second, self.REWORDED_QUOTES[3]))
        self.assertEqual(["R4"], second.delta["requirements"]["retired"])
        self.assertTrue(any("previous_sidecar" in note for note in second.notes))

    def test_an_unrelated_sentence_in_its_place_is_not_carried(self):
        first = build(LAMP, LAMP_QUOTES)
        prose = LAMP.replace("After 30 seconds on, it turns itself off.", "A fuse protects the circuit.")
        second = revise(first, prose, LAMP_QUOTES[:3] + ["A fuse protects the circuit."] + LAMP_QUOTES[4:])
        self.assertEqual("R6", id_of(second, "A fuse protects the circuit."))
        self.assertEqual(["R4"], second.delta["requirements"]["retired"])

    def test_a_near_match_that_is_not_the_only_one_carries_nothing(self):
        before = ("The pump starts when the tank is empty and the gate is open. "
                  "The pump starts when the tank is empty and the gate is shut.")
        quotes = ["The pump starts when the tank is empty and the gate is open.",
                  "The pump starts when the tank is empty and the gate is shut."]
        first = build(before, quotes)
        after = "The pump starts when the tank is empty and the gate is closed."
        second = revise(first, after, [after])
        self.assertEqual("R3", id_of(second, after))
        self.assertEqual(["R1", "R2"], second.delta["requirements"]["retired"])
        self.assertEqual([], second.delta["requirements"]["changed"])

    def test_the_caller_can_state_a_continuation_the_words_do_not_show(self):
        first = build("The door closes after 20 seconds.", ["The door closes after 20 seconds."])
        after = "A door that stays open for twenty seconds is shut by the controller."
        silent = revise(first, after, [after])
        self.assertEqual("R2", id_of(silent, after))
        stated = revise(first, after, [after], continues={after: "R1"})
        self.assertEqual("R1", id_of(stated, after))
        self.assertEqual([{"id": "R1", "how": "client"}], stated.delta["requirements"]["changed"])

    def test_a_continuation_that_does_not_fit_is_refused_and_never_invented(self):
        first = build(LAMP, LAMP_QUOTES)
        second = revise(first, LAMP.replace(" Nothing else changes it.", ""), LAMP_QUOTES[:4])
        more = LAMP + " A reset key clears it. A fuse protects it."
        extra = LAMP_QUOTES + ["A reset key clears it.", "A fuse protects it."]
        reasons = {
            "a quote that is not in the list": (LAMP, LAMP_QUOTES, {"nowhere": "R1"}),
            "an id never issued": (LAMP, LAMP_QUOTES, {LAMP_QUOTES[0]: "R99"}),
            "a quote carried by its own words": (LAMP, LAMP_QUOTES, {LAMP_QUOTES[0]: "R2"}),
            "an id another quote already carries": (more, extra, {"A reset key clears it.": "R1"}),
            "one id continued by two quotes": (
                more, LAMP_QUOTES[:3] + ["A reset key clears it.", "A fuse protects it."],
                {"A reset key clears it.": "R4", "A fuse protects it.": "R4"}),
        }
        for what, (prose, quotes, continues) in reasons.items():
            with self.subTest(what), self.assertRaises(rs.RequirementSetError):
                revise(first, prose, quotes, continues=continues)
        prose = LAMP.replace(" Nothing else changes it.", " A reset key clears it.")
        with self.assertRaises(rs.RequirementSetError):
            # R5 was retired in the second revision: it is never issued again.
            revise(second, prose, LAMP_QUOTES[:4] + ["A reset key clears it."],
                   continues={"A reset key clears it.": "R5"})


class ASingleCharacterEditedInAShortQuoteIsStillTheSameRequirementReworded(unittest.TestCase):
    """A quote is kept as short as the requirement allows, and a short quote is often one identifier. The
    similarity counts the character trigrams of its words, so changing one character of an identifier
    moves a few trigrams of many, where counting whole words said two quotes of three words shared two
    and a bare identifier shared none (a trial on a real component, 2026-10-08)."""

    PROSE = "The cluster plays beep_fuel_low_1. The cluster shows the fuel icon."
    QUOTES = ["beep_fuel_low_1", "The cluster shows the fuel icon."]

    def revised(self, old, new):
        first = build(self.PROSE.replace("beep_fuel_low_1", old), [old, self.QUOTES[1]])
        prose = self.PROSE.replace("beep_fuel_low_1", new)
        return revise(first, prose, [new, self.QUOTES[1]])

    def test_a_bare_identifier_with_one_character_changed_keeps_its_id(self):
        second = self.revised("beep_fuel_low_1", "beep_fuel_low_2")
        self.assertEqual("R1", id_of(second, "beep_fuel_low_2"))
        self.assertEqual([{"id": "R1", "how": "near-match"}], second.delta["requirements"]["changed"])
        self.assertEqual([], second.delta["requirements"]["retired"])

    def test_the_same_edit_inside_a_longer_quote_keeps_its_id(self):
        old, new = "The cluster plays beep_fuel_low_1", "The cluster plays beep_fuel_low_2"
        second = self.revised(old, new)
        self.assertEqual("R1", id_of(second, new))
        self.assertEqual("changed", next(r.status for r in second.requirements if r.id == "R1"))

    def test_another_identifier_is_not_the_same_requirement(self):
        second = self.revised("beep_fuel_low_1", "beep_fuel_hi_1")
        self.assertEqual("R3", id_of(second, "beep_fuel_hi_1"))
        self.assertEqual(["R1"], second.delta["requirements"]["retired"])

    def test_an_unrelated_sentence_is_not_the_same_requirement(self):
        second = self.revised("beep_fuel_low_1", "A chime follows the icon.")
        self.assertEqual("R3", id_of(second, "A chime follows the icon."))
        self.assertEqual(["R1"], second.delta["requirements"]["retired"])

    def test_a_change_of_capitals_or_punctuation_alone_keeps_its_id(self):
        # The similarity reads the words, not how they are written: neither a title-cased sentence nor a
        # list that lost its commas is another requirement.
        for what, old, new in (("capitals", "The lamp starts off.", "The Lamp Starts Off."),
                               ("punctuation", "fuel, low, warn", "fuel low warn")):
            with self.subTest(what):
                first = build(f"A. {old} B.", [old])
                second = revise(first, f"A. {new} B.", [new])
                self.assertEqual("R1", id_of(second, new))
                self.assertEqual([{"id": "R1", "how": "near-match"}], second.delta["requirements"]["changed"])

    def test_a_very_short_identifier_falls_short_and_is_said_so_by_a_retirement(self):
        # beep_low_1 -> 2 scores 0.75 against 0.8: the edit is one character of ten. This is the limit of
        # any similarity, and it fails loudly (a retired id and a new one), never as a wrong succession.
        second = self.revised("beep_low_1", "beep_low_2")
        self.assertEqual("R3", id_of(second, "beep_low_2"))
        self.assertEqual(["R1"], second.delta["requirements"]["retired"])


class TheWordsBehindTheIdsAreOnlyUsedWhenTheLineageVouchesForThem(unittest.TestCase):
    def test_a_sidecar_that_is_not_the_one_the_lineage_saw_is_set_aside_and_said(self):
        first = build(LAMP, LAMP_QUOTES)
        stale = json.loads(json.dumps(first.sidecar))
        stale["text"]["R4"] = "After 10 seconds on, it turns itself off."
        reworded = LAMP.replace("30 seconds", "45 seconds")
        second = revise(first, reworded, LAMP_QUOTES[:3] + ["After 45 seconds on, it turns itself off."]
                        + LAMP_QUOTES[4:], previous_sidecar_text=json.dumps(stale))
        self.assertEqual("R6", id_of(second, "After 45 seconds on, it turns itself off."))
        self.assertTrue(any("R4" in note and "not the words" in note for note in second.notes))


class AListThatPredatesLineagesIsAdopted(unittest.TestCase):
    def test_its_ids_are_kept_and_its_revision_is_followed(self):
        old = build(LAMP, LAMP_QUOTES)
        manifest, sidecar = json.dumps(old.manifest), json.dumps(old.sidecar)
        prose = LAMP.replace("The lamp starts off.", "The lamp starts off. The lamp has a green cover.")
        second = build(prose, ["The lamp has a green cover."] + LAMP_QUOTES,
                       previous_manifest_text=manifest, previous_sidecar_text=sidecar)
        for quote in LAMP_QUOTES:
            self.assertEqual(id_of(old, quote), id_of(second, quote), quote)
        self.assertEqual("R6", id_of(second, "The lamp has a green cover."))
        self.assertEqual("2", second.manifest["rev"])
        self.assertIsNone(second.delta["sentences"])
        self.assertTrue(any("predates lineages" in note for note in second.notes))

    def test_what_cannot_be_adopted_is_refused(self):
        old = build(LAMP, LAMP_QUOTES)
        manifest, sidecar = json.dumps(old.manifest), json.dumps(old.sidecar)
        native = json.loads(manifest)
        native["extraction"]["ids"] = "native"
        cases = {
            "a manifest without its sidecar": dict(previous_manifest_text=manifest),
            "the source's own ids": dict(previous_manifest_text=json.dumps(native),
                                         previous_sidecar_text=sidecar),
            "a lineage and a manifest": dict(previous_manifest_text=manifest, previous_sidecar_text=sidecar,
                                            lineage_text=rl.render(old.lineage)),
            "a sidecar with nothing to revise": dict(previous_sidecar_text=sidecar),
        }
        for what, kw in cases.items():
            with self.subTest(what), self.assertRaises(rs.RequirementSetError):
                rs.build(LAMP, items(LAMP_QUOTES), **kw)


class AnythingThatIsNotThisSpecificationsLineageIsRefused(unittest.TestCase):
    def test_another_specifications_lineage(self):
        other = build(LAMP, LAMP_QUOTES, doc_id="lamp")
        with self.assertRaises(rs.RequirementSetError):
            rs.build(LAMP, items(LAMP_QUOTES), doc_id="fan", lineage_text=rl.render(other.lineage))

    def test_text_that_is_not_a_lineage(self):
        for text in ("not json", "[]", json.dumps({"lineage": "something-else", "v": 1})):
            with self.subTest(text), self.assertRaises(rs.RequirementSetError):
                rs.build(LAMP, items(LAMP_QUOTES), lineage_text=text)

    def test_a_lineage_that_would_issue_an_id_twice(self):
        lineage = json.loads(rl.render(build(LAMP, LAMP_QUOTES).lineage))
        lineage["next"] = 3
        with self.assertRaises(rs.RequirementSetError):
            rs.build(LAMP, items(LAMP_QUOTES), lineage_text=json.dumps(lineage))
        lineage["next"] = 6
        lineage["requirements"].append(dict(lineage["requirements"][0]))
        with self.assertRaises(rs.RequirementSetError):
            rs.build(LAMP, items(LAMP_QUOTES), lineage_text=json.dumps(lineage))

    def test_a_lineage_that_names_a_revision_it_has_no_row_for(self):
        lineage = json.loads(rl.render(build(LAMP, LAMP_QUOTES).lineage))
        lineage["requirements"][0]["retired_rev"] = "9"
        with self.assertRaises(rs.RequirementSetError):
            rs.build(LAMP, items(LAMP_QUOTES), lineage_text=json.dumps(lineage))


class EveryIdCanBeIssuedAfreshWhenTheOwnerAsks(unittest.TestCase):
    """The way to start the numbering over without ever reusing a number: nothing is carried, every
    id the lineage had live is retired, and every requirement is issued a new id from `next`. A
    lineage that was built wrongly (a succession nobody should have believed) cannot be reset to
    R1, which would let `R3` name a requirement the owner once accepted under another meaning; it
    is closed, and the numbering goes on."""

    def fresh(self, prose=LAMP, quotes=LAMP_QUOTES):
        first = build(LAMP, LAMP_QUOTES)
        return first, revise(first, prose, quotes, fresh=True)

    def test_every_requirement_gets_a_new_id_numbered_on_from_next_even_when_its_words_are_the_same(self):
        first, again = self.fresh()
        self.assertEqual(["R6", "R7", "R8", "R9", "R10"], [r.id for r in again.requirements])
        self.assertEqual({"new"}, {r.status for r in again.requirements})
        self.assertEqual(11, again.lineage["next"])

    def test_the_delta_says_every_old_id_was_retired_and_nothing_was_carried(self):
        _, again = self.fresh()
        moved = again.delta["requirements"]
        self.assertEqual([], moved["carried"])
        self.assertEqual([], moved["changed"])
        self.assertEqual(["R1", "R2", "R3", "R4", "R5"], moved["retired"])
        self.assertEqual(["R6", "R7", "R8", "R9", "R10"], moved["new"])

    def test_it_is_a_revision_of_its_own_though_the_text_is_the_same(self):
        # The old ids were retired IN a revision, and the new ones issued in it: one row on which
        # an id is both issued and retired would say nothing about when either happened.
        first, again = self.fresh()
        self.assertEqual("2", again.rev)
        self.assertEqual(["1", "2"], [row["rev"] for row in again.lineage["revisions"]])
        old = {row["id"]: row for row in again.lineage["requirements"]}
        self.assertEqual({"2"}, {old[f"R{n}"]["retired_rev"] for n in range(1, 6)})
        self.assertEqual({"2"}, {old[f"R{n}"]["first_rev"] for n in range(6, 11)})
        self.assertEqual({None}, {old[f"R{n}"]["retired_rev"] for n in range(6, 11)})

    def test_what_it_makes_continues_the_lineage_it_was_made_from(self):
        # The store holds a list to this: a lineage that does not continue the one it keeps is
        # refused, so a way to renumber that is not an extension would be a way nobody can save.
        first, again = self.fresh()
        rl.extends(first.lineage, again.lineage)

    def test_what_the_work_says_of_it_is_that_every_old_id_was_retired_and_every_new_one_is_new(self):
        # The acceptance the owner made keeps pinning the list it was taken of; the words side of
        # the revision report is `between` of that lineage and this one. It must say what happened
        # to the ids and not claim a word carried.
        first, again = self.fresh()
        said = rl.between(first.lineage, again.lineage)["requirements"]
        self.assertEqual([], said["carried"])
        self.assertEqual([], said["changed"])
        self.assertEqual(["R1", "R2", "R3", "R4", "R5"], said["retired"])
        self.assertEqual(["R10", "R6", "R7", "R8", "R9"], sorted(said["new"]))

    def test_a_later_ordinary_revision_keeps_the_new_ids_and_never_the_old_ones(self):
        first, again = self.fresh()
        later = revise(again, LAMP, LAMP_QUOTES)
        self.assertEqual(["R6", "R7", "R8", "R9", "R10"], [r.id for r in later.requirements])
        self.assertEqual({"carried"}, {r.status for r in later.requirements})
        rl.extends(again.lineage, later.lineage)

    def test_it_is_a_revision_of_the_words_too(self):
        prose = LAMP.replace("30 seconds", "45 seconds")
        _, again = self.fresh(prose, [q.replace("30 seconds", "45 seconds") for q in LAMP_QUOTES])
        self.assertEqual(["R6", "R7", "R8", "R9", "R10"], [r.id for r in again.requirements])
        self.assertTrue(again.delta["specification_changed"])

    def test_the_lineage_it_makes_validates_against_its_schema(self):
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        _, again = self.fresh()
        jsonschema.validate(again.lineage, schema)

    def test_it_cannot_be_asked_together_with_a_continuation_that_it_would_make_meaningless(self):
        first = build(LAMP, LAMP_QUOTES)
        with self.assertRaises(rs.RequirementSetError) as raised:
            revise(first, LAMP, LAMP_QUOTES, fresh=True, continues={LAMP_QUOTES[0]: "R1"})
        self.assertIn("'fresh'", str(raised.exception))
        self.assertIn("'continues'", str(raised.exception))

    def test_a_first_list_has_nothing_to_renumber_and_says_so(self):
        with self.assertRaises(rs.RequirementSetError) as raised:
            build(LAMP, LAMP_QUOTES, fresh=True)
        self.assertIn("'fresh' belongs to a revision", str(raised.exception))

    def test_a_list_that_predates_lineages_can_be_renumbered_too(self):
        first = build(LAMP, LAMP_QUOTES)
        again = build(LAMP, LAMP_QUOTES, previous_manifest_text=rs.render_manifest(first.manifest),
                      previous_sidecar_text=json.dumps(first.sidecar), fresh=True)
        self.assertEqual(["R6", "R7", "R8", "R9", "R10"], [r.id for r in again.requirements])


class TheNumberingCanBeToldWhereToStart(unittest.TestCase):
    """`next_at_least`: a work that holds a lineage this build cannot read refuses a list that
    numbers from an id its lists already carried (`lineage-numbers-reused`), and says from where to
    number. A client cannot choose that any other way: `next` comes from the lineage it builds
    against, and there is none it can read."""

    def test_a_first_list_numbers_from_the_floor(self):
        built = build(LAMP, LAMP_QUOTES, next_at_least=9)
        self.assertEqual(["R9", "R10", "R11", "R12", "R13"], [r.id for r in built.requirements])
        self.assertEqual(14, built.lineage["next"])

    def test_a_revision_issues_its_new_ids_from_the_floor_when_it_is_past_next(self):
        first = build(LAMP, LAMP_QUOTES)
        prose = LAMP.replace("The lamp starts off.", "The lamp starts off. The lamp has a green cover.")
        again = revise(first, prose, ["The lamp has a green cover."] + LAMP_QUOTES, next_at_least=20)
        self.assertEqual("R20", id_of(again, "The lamp has a green cover."))
        # What was carried keeps its id: a floor decides where NEW ids start, not which exist.
        self.assertEqual(["R1", "R2", "R3", "R4", "R5"], [id_of(again, q) for q in LAMP_QUOTES])
        self.assertEqual(21, again.lineage["next"])

    def test_a_floor_below_next_changes_nothing(self):
        first = build(LAMP, LAMP_QUOTES)
        prose = LAMP.replace("The lamp starts off.", "The lamp starts off. The lamp has a green cover.")
        again = revise(first, prose, ["The lamp has a green cover."] + LAMP_QUOTES, next_at_least=2)
        self.assertEqual("R6", id_of(again, "The lamp has a green cover."))

    def test_it_goes_with_fresh_so_that_the_ids_started_over_start_past_the_floor(self):
        first = build(LAMP, LAMP_QUOTES)
        again = revise(first, LAMP, LAMP_QUOTES, fresh=True, next_at_least=30)
        self.assertEqual(["R30", "R31", "R32", "R33", "R34"], [r.id for r in again.requirements])
        rl.extends(first.lineage, again.lineage)

    def test_what_it_makes_continues_the_lineage_it_was_made_from(self):
        first = build(LAMP, LAMP_QUOTES)
        again = revise(first, LAMP.replace("30 seconds", "45 seconds"),
                       [q.replace("30 seconds", "45 seconds") for q in LAMP_QUOTES], next_at_least=40)
        rl.extends(first.lineage, again.lineage)
        jsonschema.validate(again.lineage, json.loads(SCHEMA.read_text(encoding="utf-8")))

    def test_what_is_not_a_whole_number_of_one_or_more_is_refused(self):
        for bad in (0, -3, 2.5, "9", True):
            with self.subTest(bad=bad):
                with self.assertRaises(rs.RequirementSetError) as raised:
                    build(LAMP, LAMP_QUOTES, next_at_least=bad)
                self.assertIn("'next_at_least'", str(raised.exception))


class TheToolHandsTheLineageBackAndTakesItAgain(unittest.TestCase):
    def test_the_tool_offers_next_at_least_as_a_whole_number(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "scxml_requirement_set")
        property_ = tool["inputSchema"]["properties"]["next_at_least"]
        self.assertEqual("integer", property_["type"])
        self.assertEqual(1, property_["minimum"])

    def test_a_list_numbered_from_a_floor_through_the_tool(self):
        result = call("scxml_requirement_set", specification_text=LAMP, requirements=items(LAMP_QUOTES),
                      doc_id="lamp", next_at_least=7)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual(["R10", "R11", "R7", "R8", "R9"], sorted(r["id"] for r in answer["requirements"]))

    def test_a_floor_that_is_not_a_number_is_an_argument_error_the_client_can_read(self):
        result = call("scxml_requirement_set", specification_text=LAMP, requirements=items(LAMP_QUOTES),
                      next_at_least="seven")
        self.assertTrue(result.get("isError"))
        self.assertIn("next_at_least", result["content"][0]["text"])

    def test_a_fresh_revision_through_the_tool(self):
        first = call("scxml_requirement_set", specification_text=LAMP, requirements=items(LAMP_QUOTES),
                     doc_id="lamp")
        first = json.loads(first["content"][0]["text"])
        again = call("scxml_requirement_set", specification_text=LAMP, requirements=items(LAMP_QUOTES),
                     doc_id="lamp", lineage_text=first["lineage_text"],
                     previous_sidecar_text=first["sidecar_text"], fresh=True)
        self.assertFalse(again.get("isError"), again["content"][0]["text"])
        answer = json.loads(again["content"][0]["text"])
        self.assertEqual(["R6", "R7", "R8", "R9", "R10"], sorted(
            (r["id"] for r in answer["requirements"]), key=lambda i: int(i[1:])))
        self.assertEqual(["R1", "R2", "R3", "R4", "R5"], answer["delta"]["requirements"]["retired"])

    def test_the_tool_offers_fresh_and_says_it_never_reuses_a_number(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "scxml_requirement_set")
        self.assertIn("fresh", tool["inputSchema"]["properties"])
        self.assertEqual("boolean", tool["inputSchema"]["properties"]["fresh"]["type"])
        self.assertIn("never reused", tool["description"])

    def test_a_revision_through_the_tool(self):
        first = call("scxml_requirement_set", specification_text=LAMP, requirements=items(LAMP_QUOTES),
                     doc_id="lamp")
        first = json.loads(first["content"][0]["text"])
        prose = LAMP.replace("The lamp starts off.", "The lamp starts off. The lamp has a green cover.")
        second = call("scxml_requirement_set", specification_text=prose,
                      requirements=items(["The lamp has a green cover."] + LAMP_QUOTES), doc_id="lamp",
                      lineage_text=first["lineage_text"], previous_sidecar_text=first["sidecar_text"])
        self.assertFalse(second.get("isError"), second["content"][0]["text"])
        answer = json.loads(second["content"][0]["text"])
        self.assertEqual("2", answer["rev"])
        self.assertEqual(["R6"], answer["delta"]["requirements"]["new"])
        self.assertEqual(["R1", "R2", "R3", "R4", "R5", "R6"],
                         sorted(r["id"] for r in answer["requirements"]))
        self.assertEqual({"R6": "new"}, {r["id"]: r["status"] for r in answer["requirements"]
                                         if r["status"] == "new"})

    def test_a_lineage_that_does_not_fit_is_an_argument_error_the_client_can_read(self):
        result = call("scxml_requirement_set", specification_text=LAMP, requirements=items(LAMP_QUOTES),
                      lineage_text="not json")
        self.assertTrue(result.get("isError"))
        self.assertIn("lineage", result["content"][0]["text"])

    def test_the_tool_offers_the_forms_of_the_lineage_and_of_the_words(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "scxml_requirement_set")
        properties = tool["inputSchema"]["properties"]
        for key in ("lineage", "lineage_text", "previous_sidecar", "previous_sidecar_text",
                    "previous_manifest", "previous_manifest_text", "continues"):
            self.assertIn(key, properties)
        self.assertEqual(["requirements"], tool["inputSchema"]["required"])


if __name__ == "__main__":
    unittest.main()
