"""A guess sits where a case can reach it, and `check` says how to write it there.

A guess is worth recording because a case can contradict it. A case reaches one two ways: through the
value that reads it (a `<data>` the logic reads) or through its alternatives (a DECISION REGION whose
every candidate the logic reads with `In()`). A mark anywhere else is recorded and never tried.

⚠ The first rule written for this asked for a `<data>` only. A host that forbids a script engine
cannot give one, and five writers barred from it moved the mark into a comment, which nothing reads.
This rule asks for what every host can give, and the refusal shows the region written from the mark's
own candidates, so the way out is a rewrite of a few lines.

Asserted here:

    a guess on a state, a region or the root is refused, and the refusal shows a region
    the same guess on a `<data>` is not                                       (the discriminator)
    the same guess as a region whose every candidate is read is not
    a region some candidate of which nothing reads is refused and names that candidate
    a mark that cites an answer the owner gave may sit on any element
    given nothing, no mark is a citation, and the refusal says how to say it is one
    a `<datamodel>` mark (an event schema's payload choice) is not this rule's
    the command and the tool take the record and the profile
"""

from __future__ import annotations

import contextlib
import io
import json
import unittest

import yaml

from sce_author.__main__ import main
from sce_author.check import check, read_document
from sce_author.decisions import cited_by_binding
from sce_author.mcp import call_tool
from sce_author.verify import _default_codegen
from tests.test_refusals_actually_fire import BINDING, DOCUMENT, Fixture

ON_DATA = ('sce:assumed="LAMP" sce:assumed-reason="no threshold is given" '
           'sce:assumed-candidates="1 0"')


def on_the_root(marker: str, candidates: str = "a b") -> str:
    return DOCUMENT.replace(
        'name="fixture"',
        f'name="fixture" sce:assumed="{marker}" sce:assumed-reason="read from a drawing" '
        f'sce:assumed-candidates="{candidates}"')


def with_region(read=("a", "b"), marker="PAIR_RULE") -> str:
    """The same document with a decision region and conditions reading `read` of its candidates."""
    guards = "".join(f'<state id="g{c}"><transition event="go" cond="In(\'{c}\')" target="g{c}"/></state>'
                     for c in read)
    return DOCUMENT.replace(
        "</scxml>",
        f'<state id="rule" initial="a" sce:assumed="{marker}" sce:assumed-reason="read from a drawing" '
        f'sce:assumed-candidates="a b"><state id="a"/><state id="b"/></state>{guards}\n</scxml>')


class AGuessIsWrittenWhereACaseCanReachIt(Fixture):
    def findings(self, document: str, cited=frozenset()) -> list:
        (self.root / "fixture.scxml").write_text(document, encoding="utf-8")
        path = self.root / "fixture.binding.yaml"
        path.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        return [f for f in check(self.pack(), path, None, cited)
                if "no case can ever try it" in f.detail or "decision region" in f.detail]

    def test_a_guess_on_the_root_is_refused_and_a_region_is_shown_from_its_candidates(self):
        found = self.findings(on_the_root("READING"))
        self.assertEqual(["scxml"], [f.where for f in found])
        detail = found[0].detail
        self.assertIn('<state id="READING_choice" initial="a"', detail)
        self.assertIn('sce:assumed-candidates="a b"', detail)
        self.assertIn("<state id=\"a\"/>", detail)
        self.assertIn("cond=\"In('b')\"", detail)
        self.assertIn("--decisions", detail)

    def test_candidates_that_cannot_be_ids_get_the_generic_shape_not_a_broken_one(self):
        found = self.findings(on_the_root("READING", "1 2"))
        self.assertIn('initial="A"', found[0].detail)
        self.assertNotIn('<state id="1"', found[0].detail)

    def test_the_same_guess_on_a_data_element_is_not_refused(self):
        placed = DOCUMENT.replace('sce:direction="out" expr', f'sce:direction="out" {ON_DATA} expr')
        self.assertEqual([], self.findings(placed))

    def test_a_decision_region_whose_every_candidate_is_read_is_not_refused(self):
        self.assertEqual([], self.findings(with_region()))

    def test_a_region_some_candidate_of_which_nothing_reads_is_refused_and_names_it(self):
        """The discriminator against a region that decides nothing: `b` is a label, and choosing
        it would switch the rule off rather than run the other reading."""
        found = self.findings(with_region(read=("a",)))
        self.assertEqual(["state rule"], [f.where for f in found])
        self.assertIn("'b'", found[0].detail)
        self.assertNotIn("'a'", found[0].detail.split("with In()")[0])

    def test_a_mark_citing_an_answer_the_owner_gave_may_sit_on_any_element(self):
        document = on_the_root("D1")
        self.assertEqual(["scxml"], [f.where for f in self.findings(document)])
        self.assertEqual([], self.findings(document, cited=frozenset({"D1"})))

    def test_a_citation_the_owner_did_not_make_is_not_excused_by_another(self):
        self.assertEqual(["scxml"], [f.where for f in self.findings(on_the_root("D2"),
                                                                    cited=frozenset({"D1"}))])

    def test_a_datamodel_mark_is_an_event_schemas_business(self):
        document = DOCUMENT.replace(
            "<datamodel>", '<datamodel sce:assumed="PAYLOAD" sce:assumed-reason="chosen">')
        self.assertEqual([], self.findings(document))

    def test_the_document_lists_the_conditions_a_region_is_read_through(self):
        (self.root / "fixture.scxml").write_text(with_region(), encoding="utf-8")
        document = read_document(self.root / "fixture.scxml")
        self.assertEqual(["In('a')", "In('b')"], list(document.conditions))
        self.assertTrue(document.region_reads_every_candidate(document.marks[0]))


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class WhatTheOwnerHasAnswered(Fixture):
    def setUp(self):
        super().setUp()
        (self.root / "fixture.scxml").write_text(on_the_root("D1"), encoding="utf-8")
        self.binding = self.root / "fixture.binding.yaml"
        self.binding.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        self.record = self.root / "decisions.json"
        self.record.write_text(json.dumps({
            "record": "sce-decision-record", "v": 1, "specification": {"doc_id": "s"},
            "decisions": [{"id": "D1", "question": "q", "answer": "a"}]}), encoding="utf-8")

    def test_the_command_takes_the_record(self):
        ran = []
        for extra in ([], ["--decisions", str(self.record)]):
            with contextlib.redirect_stdout(io.StringIO()):
                ran.append(main(["check", "--pack", str(self.pack_dir), "--binding",
                                 str(self.binding), *extra]))
        self.assertEqual([1, 0], ran)

    def test_the_tool_takes_the_record(self):
        refused = call_tool("check", {"pack": str(self.pack_dir), "binding": str(self.binding)})
        self.assertTrue(refused.get("isError"), refused)
        accepted = call_tool("check", {"pack": str(self.pack_dir), "binding": str(self.binding),
                                       "decisions": str(self.record)})
        self.assertFalse(accepted.get("isError"), accepted)

    def test_the_ids_are_the_records(self):
        cited, refused = cited_by_binding(self.binding, self.record, None)
        self.assertEqual(("", {"D1"}), (refused, set(cited)))


if __name__ == "__main__":
    unittest.main()
