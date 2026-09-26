"""A gap in the specification says what the cases found there.

Every recorded guess -- `sce:assumed` in the document, `assumed` on a binding
rule -- comes back as refuted, held or UNTESTED. `verify` used to name only the
first, beside the failure it caused, and a guess no case compared read the
same as one that was checked.

Asserted here:

    a guess the cases contradict is refuted, with the case and both values
    a failure two guesses decide together refutes neither: both implicated
    a guess they agree with is held, by the cases that agreed
    a guess no case compares is untested        (the one a pass hides)
    a binding's guess behind a document's guess is credited too
    the report puts the refuted first and the held last
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.gaps import report
from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import Fixture

GUESSED = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="guessed">
  <datamodel>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          sce:assumed="lamp-threshold"
          sce:assumed-reason="no threshold is given; read as above zero"
          expr="count &gt; 0 ? 1 : 0"/>
    <data id="level" sce:type="int32" sce:direction="out"
          sce:assumed="level-scale"
          sce:assumed-reason="no scale is given; read as the count doubled"
          expr="count * 2"/>
    <data id="spare" sce:type="int32" sce:direction="out"
          sce:assumed="spare-default" sce:assumed-reason="nothing decides it"
          expr="0"/>
  </datamodel>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "guessed.scxml",
    "inputs": {"count": {"address": "Plant.Input.Count", "when_absent": 0}},
    "outputs": {
        "lamp": {"address": "Plant.Out.Lamp", "field": "Stat",
                 "map": {0: "OFF", 1: "ON"},
                 "assumed": "the lamp's symbols are the platform default"},
        "level": {"address": "Plant.Out.Lamp", "field": "Value", "passthrough": True},
        "spare": {"address": "Plant.Out.Unwritten", "field": "Stat",
                  "map": {0: "OFF"}},
    },
}

# The lamp is right and the level is not: the record says 3 where the guess
# doubles 2 to 4. Nothing reads the spare position.
CASE = {"name": "two counted",
        "given": {"Plant.Input.Count": 2}, "drove": ["Plant.Input.Count"],
        "expect": {"Plant.Out.Lamp.Stat": "ON", "Plant.Out.Lamp.Value": 3}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AGapSaysWhatTheCasesFoundThere(Fixture):
    def setUp(self):
        super().setUp()
        self.result = self.run_with(BINDING)
        self.ledger = self.result.assumptions

    def run_with(self, binding):
        (self.root / "guessed.scxml").write_text(GUESSED, encoding="utf-8")
        path = self.root / "guessed.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "cases": [CASE]}), encoding="utf-8")
        result = verify(self.pack(), path)
        self.assertTrue(result.ran, result.refusal)
        return result

    def test_a_failure_two_guesses_decide_together_refutes_neither(self):
        """⚠ The discriminator for `implicated`. With the binding ALSO
        guessing about the level, the wrong value rests on two guesses; the
        case says one of them is wrong and cannot say which, so neither is
        reported refuted -- each is implicated, naming the other."""
        rules = {**BINDING, "outputs": {**BINDING["outputs"], "level": {
            **BINDING["outputs"]["level"], "assumed": "the level is passed through"}}}
        found = self.run_with(rules).assumptions
        by_doc, by_rule = found["document:level"], found["binding:output:level"]
        self.assertEqual(("implicated", "implicated"), (by_doc.status, by_rule.status))
        self.assertEqual([("two counted", "Plant.Out.Lamp.Value", 3, 4, ["output level"])],
                         by_doc.implicated_by)
        self.assertEqual(["level-scale"], by_rule.implicated_by[0][4])

    def test_a_contradicted_guess_is_refuted_with_its_evidence(self):
        level = self.ledger["document:level"]
        self.assertEqual("refuted", level.status)
        self.assertEqual([("two counted", "Plant.Out.Lamp.Value", 3, 4)],
                         level.refuted_by)

    def test_an_agreeing_guess_is_held(self):
        lamp = self.ledger["document:lamp"]
        self.assertEqual(("held", ["two counted"]), (lamp.status, lamp.held_in))

    def test_a_guess_no_case_compares_is_untested(self):
        """⚠ The discriminator for the report's reason to exist: the run
        passed nothing about `spare` and failed nothing about it either."""
        spare = self.ledger["document:spare"]
        self.assertEqual("untested", spare.status)
        self.assertEqual(["Plant.Out.Unwritten.Stat"], spare.positions)

    def test_a_binding_guess_behind_a_document_guess_is_credited_too(self):
        """Two decisions about one output -- which value, which symbol. The
        failure message names the nearer; the ledger credits both."""
        rule = self.ledger["binding:output:lamp"]
        self.assertEqual(("held", ["Plant.Out.Lamp.Stat"]), (rule.status, rule.positions))

    def test_the_report_is_most_urgent_first(self):
        kinds = [g.kind for g in report(self.result, self.pack())]
        self.assertEqual(["refuted", "untested", "held", "held"], kinds)


if __name__ == "__main__":
    unittest.main()
