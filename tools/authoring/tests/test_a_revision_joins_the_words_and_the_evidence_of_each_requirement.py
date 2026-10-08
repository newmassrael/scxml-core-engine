"""A revision's words and its evidence are joined by requirement, and the join says whether it stayed in reach.

`scxml_requirement_set` says what happened to each requirement's WORDS (carried, changed, new,
retired) and `scxml_acceptance_delta` what happened to the design's EVIDENCE for it (unchanged,
changed, new, dropped). Neither alone says whether a revision is sound: a design that moved where
the words did not, or one that still cites what the specification dropped, is a violation, and a
requirement whose words changed and whose design did not is a place to look. These tests hold the
join (`docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`):

* every row of the table comes out of the join and out of no other input;
* only a violation makes the verdict `outside-reach`;
* what is not a delta of the right shape is refused, not read as "nothing moved";
* the page folds what carries over into one line, lists the rest with its places, prints a
  sentence only when given one, and says what `carries over` means;
* the tools reach the product's `acceptance-delta` with the arguments they were given.
"""

from __future__ import annotations

import io
import json
import pathlib
import subprocess
import sys
import unittest
from unittest import mock

from sce_author import mcp, process, revision, verify


def words(carried=(), changed=(), new=(), retired=(), **extra):
    return {"requirements": {"carried": list(carried),
                             "changed": [{"id": i, "how": "near-match"} for i in changed],
                             "new": list(new), "retired": list(retired)}, **extra}


def evidence(*lines, added=(), gone=0):
    return {"version": 1, "summary": None, "requirements": list(lines),
            "unclaimed": {"added": list(added), "gone": gone}}


def line(requirement, kind, **more):
    return {"requirement": requirement, "evidence": kind, **more}


def row(result, requirement):
    return next(r for r in result["requirements"] if r["requirement"] == requirement)


class EveryRowOfTheTableComesOutOfTheJoin(unittest.TestCase):
    CASES = [
        # words, evidence line, kind, severity
        ("carried", "unchanged", "carries-over", "ok"),
        ("carried", "changed", "moved-without-reason", "violation"),
        ("carried", "dropped", "moved-without-reason", "violation"),
        ("carried", "new", "newly-cited", "look"),
        ("carried", None, "uncited", "uncovered"),
        ("changed", "changed", "revised", "ok"),
        ("changed", "new", "revised", "ok"),
        ("changed", "unchanged", "words-changed-design-same", "look"),
        ("changed", "dropped", "changed-but-uncited", "look"),
        ("changed", None, "changed-but-uncited", "look"),
        ("new", "new", "implemented-new", "ok"),
        ("new", "changed", "implemented-new", "ok"),
        ("new", "unchanged", "unimplemented-new", "look"),
        ("new", "dropped", "unimplemented-new", "look"),
        ("new", None, "unimplemented-new", "look"),
        ("retired", "dropped", "retired-cleanly", "ok"),
        ("retired", None, "retired-cleanly", "ok"),
        ("retired", "unchanged", "retired-still-cited", "violation"),
        ("retired", "changed", "retired-still-cited", "violation"),
        ("retired", "new", "retired-still-cited", "violation"),
    ]

    def test_each_pair_of_words_and_evidence_is_read_as_the_table_says(self):
        for w, e, kind, severity in self.CASES:
            with self.subTest(words=w, evidence=e):
                delta = words(**{{"carried": "carried", "changed": "changed", "new": "new",
                                  "retired": "retired"}[w]: ["R1"]})
                lines = [line("R1", e)] if e else []
                found = row(revision.join(delta, evidence(*lines)), "R1")
                self.assertEqual((kind, severity), (found["kind"], found["severity"]))

    def test_the_table_has_no_pair_the_cases_leave_out(self):
        # Every (words, evidence) pair the join can meet is one of the cases above.
        pairs = {(w, e) for w, e, _, _ in self.CASES}
        for w in revision.WORDS:
            for e in (*revision.EVIDENCE, None):
                self.assertIn((w, e), pairs, f"nothing says what {w} with evidence {e} is")

    def test_a_requirement_the_words_do_not_list_is_unlisted_whatever_the_design_says(self):
        for e in revision.EVIDENCE:
            found = row(revision.join(words(carried=["R9"]), evidence(line("R1", e))), "R1")
            self.assertEqual(("unlisted", "look", "unlisted"),
                             (found["kind"], found["severity"], found["words"]))


class ARequirementNoNodeCitesWasNotCompared(unittest.TestCase):
    """Found in review (2026-10-08): a carried requirement that no node cites, in the record or now,
    was called `carries-over` and folded into the line that says the design was checked against it.
    There was no evidence on either side, so nothing was checked."""

    def test_it_is_uncited_and_counted_apart_and_never_carried_over(self):
        result = revision.join(words(carried=["R1", "R2"]), evidence(line("R1", "unchanged")))
        self.assertEqual(("carries-over", "ok"), (row(result, "R1")["kind"], row(result, "R1")["severity"]))
        self.assertEqual(("uncited", "uncovered"), (row(result, "R2")["kind"], row(result, "R2")["severity"]))
        self.assertEqual((1, 1, 1), (result["summary"]["uncovered"], result["summary"]["seen"],
                                     result["summary"]["ok"]))
        # It is not a finding against the revision.
        self.assertEqual("within-reach", result["verdict"])

    def test_the_page_lists_it_apart_and_does_not_fold_it_into_what_carries_over(self):
        page = revision.render(revision.join(words(carried=["R1", "R2"]),
                                             evidence(line("R1", "unchanged"))))
        self.assertIn("## Not covered by this check (1)", page)
        self.assertIn("- R2 -- uncited", page)
        self.assertIn("## Carries over (1)", page)
        carries = page.split("## Carries over")[1]
        self.assertIn("R1", carries)
        self.assertNotIn("R2", carries)
        self.assertIn("cited by no node", page)

    def test_a_check_that_saw_no_evidence_at_all_says_it_compared_nothing(self):
        # A kind of document with nowhere to cite a requirement, or a design that cites none.
        result = revision.join(words(carried=["R1", "R2", "R3"]), evidence())
        self.assertEqual(0, result["summary"]["seen"])
        self.assertEqual(3, result["summary"]["uncovered"])
        self.assertEqual("within-reach", result["verdict"], "the verdict is unchanged: it is the page that must say")
        page = revision.render(result)
        self.assertIn("saw no evidence for any requirement", page)
        self.assertIn("## Carries over (0)", page)

    def test_no_such_warning_when_the_check_saw_something(self):
        page = revision.render(revision.join(words(carried=["R1"]), evidence(line("R1", "unchanged"))))
        self.assertNotIn("saw no evidence", page)
        self.assertNotIn("Not covered", page)

    def test_a_revision_that_lists_no_requirement_is_not_called_blind(self):
        # Nothing listed, nothing to see: the empty revision is not "the check saw nothing".
        page = revision.render(revision.join(words(), evidence()))
        self.assertNotIn("saw no evidence", page)


class OnlyAViolationMakesTheVerdictOutsideReach(unittest.TestCase):
    def test_a_revision_with_looks_but_no_violation_is_within_reach(self):
        result = revision.join(words(carried=["R1"], changed=["R2"], new=["R3"]),
                               evidence(line("R1", "unchanged"), line("R2", "unchanged")))
        self.assertEqual("within-reach", result["verdict"])
        self.assertEqual(0, result["summary"]["violations"])
        self.assertEqual(2, result["summary"]["look"], result["summary"])

    def test_one_violation_makes_it_outside_reach(self):
        result = revision.join(words(carried=["R1", "R2"]),
                               evidence(line("R1", "unchanged"), line("R2", "changed", moved=["a"], gone=1)))
        self.assertEqual("outside-reach", result["verdict"])
        self.assertEqual(1, result["summary"]["violations"])
        self.assertEqual(["a"], row(result, "R2")["moved"])
        self.assertEqual(1, row(result, "R2")["gone"])

    def test_new_rows_that_claim_no_requirement_are_a_look_and_never_a_violation(self):
        result = revision.join(words(carried=["R1"]),
                               evidence(line("R1", "unchanged"), added=["states.spare"], gone=0))
        self.assertEqual("within-reach", result["verdict"])
        self.assertEqual(["states.spare"], result["unclaimed"]["added"])
        self.assertEqual(1, result["summary"]["look"])

    def test_a_revision_that_changed_nothing_carries_everything_over(self):
        result = revision.join(words(carried=["R1", "R2"]),
                               evidence(line("R1", "unchanged"), line("R2", "unchanged")))
        self.assertEqual({"carries-over": 2}, result["summary"]["kinds"])
        self.assertEqual(2, result["summary"]["ok"])


class ACarriedRequirementThatMovedOnlyWhereAChangedOneStandsIsALookNotAViolation(unittest.TestCase):
    """A node is cited by several requirements at once. Changing it for the one the specification
    asked to change moves the evidence of the others too (a real trial, 2026-10-08: a pop-up state
    cited by R7 and R8, R8's sound changed, and R7 read as a violation). Such a requirement is a
    `look` that names the neighbour, and only when nothing else about it moved."""

    PLACE = "states.alarm.on_entry_blocks[0][0]"

    def joined(self, neighbour_words, neighbour_line, carried_line=None):
        delta = words(carried=["R1"], **neighbour_words)
        return revision.join(delta, evidence(carried_line or line("R1", "changed", moved=[self.PLACE], gone=1),
                                             neighbour_line))

    def test_a_neighbour_whose_words_are_new_explains_it_and_is_named(self):
        result = self.joined({"new": ["R2"]}, line("R2", "new", at=[self.PLACE, "states.alarm"]))
        explained = row(result, "R1")
        self.assertEqual(("moved-with-a-changed-neighbour", "look"), (explained["kind"], explained["severity"]))
        self.assertEqual(["R2"], explained["shared_with"])
        self.assertEqual("within-reach", result["verdict"])
        self.assertEqual(0, result["summary"]["violations"])

    def test_a_neighbour_whose_words_changed_explains_it_by_the_places_it_moved(self):
        result = self.joined({"changed": ["R2"]}, line("R2", "changed", moved=[self.PLACE], gone=1))
        self.assertEqual("moved-with-a-changed-neighbour", row(result, "R1")["kind"])
        self.assertEqual("revised", row(result, "R2")["kind"])

    def test_every_place_it_moved_has_to_be_one_the_neighbour_stands_on(self):
        for what, line_ in (("a place the neighbour does not stand on",
                             line("R1", "changed", moved=[self.PLACE, "states.elsewhere"], gone=1)),
                            ("a neighbour that shares none of it",
                             line("R1", "changed", moved=["states.elsewhere"], gone=1))):
            with self.subTest(what):
                result = self.joined({"new": ["R2"]}, line("R2", "new", at=[self.PLACE]), line_)
                self.assertEqual(("moved-without-reason", "violation"),
                                 (row(result, "R1")["kind"], row(result, "R1")["severity"]))
                self.assertEqual("outside-reach", result["verdict"])

    def test_more_recorded_rows_gone_than_places_moved_is_a_loss_no_neighbour_accounts_for(self):
        result = self.joined({"new": ["R2"]}, line("R2", "new", at=[self.PLACE]),
                             line("R1", "changed", moved=[self.PLACE], gone=2))
        self.assertEqual("moved-without-reason", row(result, "R1")["kind"])

    def test_a_neighbour_whose_words_did_not_change_explains_nothing(self):
        delta = words(carried=["R1", "R2"])
        result = revision.join(delta, evidence(line("R1", "changed", moved=[self.PLACE], gone=1),
                                               line("R2", "changed", moved=[self.PLACE], gone=1)))
        self.assertEqual(["moved-without-reason"] * 2, [r["kind"] for r in result["requirements"]])
        self.assertEqual("outside-reach", result["verdict"])

    def test_a_neighbour_that_is_retired_or_dropped_explains_nothing(self):
        result = self.joined({"retired": ["R2"]}, line("R2", "dropped", gone=1))
        self.assertEqual("moved-without-reason", row(result, "R1")["kind"])

    def test_a_carried_requirement_whose_citations_all_vanished_is_not_explained(self):
        result = self.joined({"new": ["R2"]}, line("R2", "new", at=[self.PLACE]), line("R1", "dropped", gone=1))
        self.assertEqual("moved-without-reason", row(result, "R1")["kind"])

    def test_all_the_neighbours_that_share_a_place_are_named(self):
        delta = words(carried=["R1"], new=["R2"], changed=["R3"])
        result = revision.join(delta, evidence(line("R1", "changed", moved=["a", "b"], gone=2),
                                               line("R2", "new", at=["a"]),
                                               line("R3", "changed", moved=["b"], gone=1)))
        self.assertEqual(["R2", "R3"], row(result, "R1")["shared_with"])

    def test_the_page_lists_it_under_look_again_with_the_neighbour(self):
        result = self.joined({"new": ["R2"]}, line("R2", "new", at=[self.PLACE]))
        page = revision.render(result)
        self.assertIn("## Look again", page)
        self.assertIn("- R1 -- moved-with-a-changed-neighbour", page)
        self.assertIn("shared with R2", page)
        self.assertNotIn("## Outside the revision's reach", page)
        self.assertIn("Verdict: within-reach", page)


class WhatIsNotADeltaIsRefusedAndNotReadAsNothingMoved(unittest.TestCase):
    def test_a_words_delta_that_is_not_the_one_the_tool_returned(self):
        for bad in (None, [], {}, {"requirements": []}, {"requirements": {"carried": []}},
                    {"requirements": {"carried": [], "changed": [], "new": []}},
                    words(carried=["R1"], new=["R1"]), words(carried=[""]),
                    {"requirements": {"carried": [1], "changed": [], "new": [], "retired": []}}):
            with self.subTest(bad=bad), self.assertRaises(revision.RevisionError):
                revision.join(bad, evidence())

    def test_an_evidence_delta_that_is_not_the_one_the_tool_returned(self):
        for bad in (None, {}, {"requirements": {}}, {"requirements": [{"requirement": "R1"}]},
                    {"requirements": [{"requirement": "R1", "evidence": "gone"}]},
                    {"requirements": [line("R1", "changed"), line("R1", "unchanged")]},
                    {"requirements": [], "unclaimed": {"added": "x"}}):
            with self.subTest(bad=bad), self.assertRaises(revision.RevisionError):
                revision.join(words(carried=["R1"]), bad)


class ThePageShowsOnlyWhatToLookAtAgain(unittest.TestCase):
    RESULT = revision.join(
        words(carried=["R1", "R2"], changed=["R3"], new=["R4"], retired=["R5"]),
        evidence(line("R1", "unchanged"), line("R2", "unchanged"),
                 line("R3", "changed", moved=["states.alarm.on_entry_blocks[0][0]"], gone=1),
                 line("R5", "unchanged"), added=["states.spare"]))

    def test_what_carries_over_is_one_line_and_the_rest_is_listed_with_its_places(self):
        page = revision.render(self.RESULT)
        self.assertIn("## Carries over (2)", page)
        self.assertIn("R1, R2", page)
        self.assertNotIn("- R1 --", page, "a requirement that carries over was listed as a finding")
        self.assertIn("- R5 -- retired-still-cited", page)
        self.assertIn("now at states.alarm.on_entry_blocks[0][0]", page)
        self.assertIn("1 recorded row(s) gone", page)
        self.assertIn("states.spare", page)
        self.assertIn("## Outside the revision's reach (1)", page)

    def test_the_page_says_what_carries_over_does_not_mean(self):
        page = revision.render(self.RESULT)
        self.assertIn("does not say the requirement is met", page)
        self.assertIn("still lapses by its bytes", page)

    def test_a_sentence_is_printed_only_when_given_and_the_page_then_says_so(self):
        plain = revision.render(self.RESULT)
        self.assertNotIn("local artefact", plain)
        self.assertNotIn("\"", plain, "a page with no sentences quoted one")
        with_words = revision.render(self.RESULT, {"R3": "The alarm sounds for five seconds.",
                                                    "R1": "A sentence of a requirement that carries over."})
        self.assertIn("The alarm sounds for five seconds.", with_words)
        self.assertIn("local artefact", with_words)
        self.assertNotIn("A sentence of a requirement that carries over.", with_words,
                         "a requirement that carries over was quoted")

    def test_a_clean_revision_says_so(self):
        clean = revision.join(words(carried=["R1"]), evidence(line("R1", "unchanged")))
        page = revision.render(clean, title="revision 1 to 2")
        self.assertIn("# Revision report: revision 1 to 2", page)
        self.assertIn("Verdict: within-reach", page)
        self.assertNotIn("## Look again", page)


def call(name: str, **arguments) -> dict:
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}
    out = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), out)
    return json.loads(out.getvalue())["result"]


class TheToolsReachTheProductsDelta(unittest.TestCase):
    PRODUCT = "\n".join(json.dumps(l) for l in (
        {"v": 1, "kind": "acceptance-delta", "requirement": "R1", "evidence": "unchanged"},
        {"v": 1, "kind": "acceptance-delta", "requirement": "R2", "evidence": "changed",
         "moved": ["states.a.transitions[1]"], "gone": 1},
        {"v": 1, "kind": "acceptance-delta-unclaimed", "added": [], "gone": 0},
        {"v": 1, "kind": "acceptance-delta-summary", "record": "r", "requirements": 2,
         "unchanged": 1, "changed": 1, "new": 0, "dropped": 0})) + "\n"

    def run_tool(self, name: str, **arguments):
        seen = {}

        def fake_run(argv, **kwargs):
            seen["argv"] = argv
            return subprocess.CompletedProcess(argv, 0, stdout=self.PRODUCT, stderr="")

        with mock.patch.object(process.subprocess, "run", fake_run), \
             mock.patch.object(verify, "_default_codegen", lambda: pathlib.Path(sys.executable)):
            result = call(name, **arguments)
        return result, seen.get("argv")

    def test_the_check_asks_the_product_and_joins_what_comes_back(self):
        result, argv = self.run_tool("scxml_revision_check", delta=words(carried=["R1", "R2"]),
                                     record="acceptance.json", root=".")
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        self.assertEqual("acceptance-delta", argv[argv.index("acceptance-delta")])
        self.assertIn("--root", argv)
        self.assertNotIn("--design", argv)
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("outside-reach", answer["verdict"])
        self.assertEqual("moved-without-reason", row(answer, "R2")["kind"])

    def test_a_named_draft_reaches_the_product_as_design(self):
        _, argv = self.run_tool("scxml_revision_check", delta=words(carried=["R1", "R2"]),
                                record="acceptance.json", root=".", design="draft.scxml")
        self.assertEqual(str(pathlib.Path("draft.scxml").resolve()), argv[argv.index("--design") + 1])

    def test_the_report_carries_the_page_and_a_sentence_only_when_a_sidecar_is_given(self):
        delta = words(carried=["R1"], changed=["R2"], from_rev="1", rev="2")
        plain, _ = self.run_tool("scxml_revision_report", delta=delta, record="a.json", root=".")
        page = json.loads(plain["content"][0]["text"])["page"]
        self.assertIn("# Revision report: revision 1 to 2", page)
        self.assertNotIn("local artefact", page)
        sidecar = json.dumps({"doc_id": "d", "rev": "2", "text": {"R2": "The lamp turns off."}})
        given, _ = self.run_tool("scxml_revision_report", delta=delta, record="a.json", root=".",
                                 sidecar_text=sidecar)
        text = json.loads(given["content"][0]["text"])["page"]
        self.assertIn("The lamp turns off.", text)
        self.assertIn("local artefact", text)

    def test_a_delta_that_is_not_one_is_an_argument_error_the_client_can_read(self):
        for arguments in ({"record": "a.json", "root": "."},
                          {"delta": "not an object", "record": "a.json", "root": "."},
                          {"delta": {"requirements": []}, "record": "a.json", "root": "."}):
            with self.subTest(arguments=sorted(arguments)):
                result, _ = self.run_tool("scxml_revision_check", **arguments)
                self.assertTrue(result.get("isError"))

    def test_a_sidecar_that_is_not_one_is_refused(self):
        result, _ = self.run_tool("scxml_revision_report", delta=words(carried=["R1"]),
                                  record="a.json", root=".", sidecar_text="[]")
        self.assertTrue(result.get("isError"))

    def test_both_tools_are_offered_with_the_arguments_they_need(self):
        for name in ("scxml_revision_check", "scxml_revision_report"):
            tool = next(t for t in mcp.TOOLS if t["name"] == name)
            self.assertEqual(["delta", "record", "root"], tool["inputSchema"]["required"])
        report = next(t for t in mcp.TOOLS if t["name"] == "scxml_revision_report")
        self.assertIn("sidecar_text", report["inputSchema"]["properties"])


if __name__ == "__main__":
    unittest.main()
