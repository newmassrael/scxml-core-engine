"""A guess is a decision variable, because that is what a case can contradict.

`verify` blames a guess through the values that read it and `gaps --counterfactual` runs the
alternatives in its place, and both walk the document's `<data>`. A mark on a state, a region or
the root has no value behind it: it is recorded and never tried. Measured 2026-10-10, three of five
writers given one specification and its picture wrote the reading of that picture on such an
element, and nothing said that no case could ever contradict it.

Asserted here:

    a guess on a state, a datamodel or the root is refused, and the refusal says what to write
    the same guess on a `<data>` is not                                  (the discriminator)
    a mark that cites an answer the owner gave may sit on any element
    given nothing, no mark is a citation: it is refused, and told how to say it is one
    the ids the owner answered are the record's, and the product's mark for a house rule
    the CLI and the tool take the record and the profile
"""

from __future__ import annotations

import contextlib
import io
import json
import unittest

import yaml

from sce_author.__main__ import main
from sce_author.check import check, read_document, unplaced_guesses
from sce_author.decisions import cited_by_binding, cited_markers
from sce_author.verify import _default_codegen
from tests.test_refusals_actually_fire import BINDING, DOCUMENT, Fixture

ON_DATA = ('sce:assumed="LAMP" sce:assumed-reason="no threshold is given" '
           'sce:assumed-candidates="1 0"')


def on_the_datamodel(marker: str, document: str = DOCUMENT) -> str:
    return document.replace(
        "<datamodel>",
        f'<datamodel sce:assumed="{marker}" sce:assumed-reason="read from a drawing" '
        f'sce:assumed-candidates="a b">')


class AGuessIsWrittenWhereACaseCanReachIt(Fixture):
    def findings(self, document: str, cited=frozenset()) -> list:
        (self.root / "fixture.scxml").write_text(document, encoding="utf-8")
        path = self.root / "fixture.binding.yaml"
        path.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        return [f for f in check(self.pack(), path, None, cited) if "holds no value" in f.detail]

    def test_a_guess_on_the_datamodel_is_refused_with_what_to_write_instead(self):
        found = self.findings(on_the_datamodel("READING"))
        self.assertEqual(["datamodel"], [f.where for f in found])
        self.assertIn("`<data>`", found[0].detail)
        self.assertIn("`true false`", found[0].detail)
        self.assertIn("--decisions", found[0].detail)

    def test_a_guess_on_the_root_is_refused_too(self):
        root = DOCUMENT.replace('name="fixture"', 'name="fixture" sce:assumed="ROOT"')
        self.assertEqual(["scxml"], [f.where for f in self.findings(root)])

    def test_the_same_guess_on_a_data_element_is_not(self):
        placed = DOCUMENT.replace('sce:direction="out" expr', f'sce:direction="out" {ON_DATA} expr')
        self.assertEqual([], self.findings(placed))

    def test_a_mark_citing_an_answer_the_owner_gave_may_sit_on_any_element(self):
        document = on_the_datamodel("D1")
        self.assertEqual(["datamodel"], [f.where for f in self.findings(document)])
        self.assertEqual([], self.findings(document, cited=frozenset({"D1"})))

    def test_a_citation_the_owner_did_not_make_is_not_excused_by_another(self):
        self.assertEqual(["datamodel"],
                         [f.where for f in self.findings(on_the_datamodel("D2"),
                                                         cited=frozenset({"D1"}))])


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class WhatTheOwnerHasAnswered(Fixture):
    CHART = ('<?xml version="1.0" encoding="UTF-8"?>\n'
             '<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"\n'
             '       version="1.0" name="door" initial="closed" datamodel="ecmascript">\n'
             '  <state id="closed">\n'
             '    <transition event="open" target="open" sce:assumed="D1"/>\n'
             '    <transition event="lock" target="closed" sce:assumed="H1"/>\n'
             '    <transition event="kick" target="closed" sce:assumed="GUESS"/>\n'
             '  </state>\n'
             '  <state id="open"/>\n'
             '</scxml>\n')
    RECORD = {"record": "sce-decision-record", "v": 1, "specification": {"doc_id": "door-spec"},
              "decisions": [{"id": "D1", "question": "Does open restart the timer?",
                             "answer": "Yes."}]}
    PROFILE = {"record": "sce-authoring-profile", "v": 1,
               "house_rules": [{"id": "H1", "rule": "An event a state does not mention is ignored."}]}

    def setUp(self):
        super().setUp()
        self.document = self.root / "door.scxml"
        self.document.write_text(self.CHART, encoding="utf-8")
        self.record = self.root / "decisions.json"
        self.record.write_text(json.dumps(self.RECORD), encoding="utf-8")
        self.profile = self.root / "profile.json"
        self.profile.write_text(json.dumps(self.PROFILE), encoding="utf-8")

    def test_the_ids_are_the_records_and_the_products_mark_for_a_house_rule(self):
        cited, refused = cited_markers(self.document, self.record, self.profile)
        self.assertEqual(("", {"D1", "H1"}), (refused, set(cited)))

    def test_with_only_the_record_a_house_rule_is_not_yet_a_citation(self):
        cited, _ = cited_markers(self.document, self.record, None)
        self.assertEqual({"D1"}, set(cited))
        said = [f.detail.split('"')[1] for f in unplaced_guesses(read_document(self.document), cited)]
        self.assertEqual(["H1", "GUESS"], said)

    def test_given_both_only_the_guess_is_refused(self):
        cited, _ = cited_markers(self.document, self.record, self.profile)
        left = [f.detail.split('"')[1] for f in unplaced_guesses(read_document(self.document), cited)]
        self.assertEqual(["GUESS"], left)

    def test_given_nothing_every_mark_is_a_guess(self):
        cited, _ = cited_markers(self.document, None, None)
        self.assertEqual(frozenset(), cited)
        said = [f.detail.split('"')[1] for f in unplaced_guesses(read_document(self.document), cited)]
        self.assertEqual(["D1", "H1", "GUESS"], said)

    def test_the_binding_names_the_document_the_ids_are_read_from(self):
        binding = self.root / "door.binding.yaml"
        binding.write_text(yaml.safe_dump({"version": 1, "document": "door.scxml"}),
                           encoding="utf-8")
        cited, refused = cited_by_binding(binding, self.record, self.profile)
        self.assertEqual(("", {"D1", "H1"}), (refused, set(cited)))

    def test_the_command_takes_the_record_and_the_profile(self):
        (self.root / "fixture.scxml").write_text(on_the_datamodel("D1"), encoding="utf-8")
        binding = self.root / "fixture.binding.yaml"
        binding.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        record = self.root / "answers.json"
        record.write_text(json.dumps({**self.RECORD, "decisions": [
            {"id": "D1", "question": "q", "answer": "a"}]}), encoding="utf-8")
        ran = []
        for extra in ([], ["--decisions", str(record)]):
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                ran.append(main(["check", "--pack", str(self.pack_dir), "--binding", str(binding),
                                 *extra]))
        self.assertEqual([1, 0], ran)


if __name__ == "__main__":
    unittest.main()
