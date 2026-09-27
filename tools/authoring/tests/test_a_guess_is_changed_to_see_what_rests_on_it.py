"""A failure several guesses decide is settled by changing each guess.

`verify` can only say that a failure at a position several recorded guesses
decide together implicates every one of them. `gaps --counterfactual` runs
the cases again with each binding guess changed to every alternative it has,
and reports what moved.

Asserted here, over one lamp a document decides and a binding lands:

    a guess whose every alternative leaves the failure as it was is cleared,
    and the guess beside it is then the only recorded one left
                                                        (the discriminator)
    an alternative that repairs every failure and breaks nothing is the
    answer, and the guess is reported refuted with it
    two failures repaired by two DIFFERENT values are not a witness: no
    single value is right
    a number's alternatives are a sample and never clear a guess
    a run budget that runs out says what it did not try
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.counterfactual import explore, lines
from sce_author.gaps import report
from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import CONVENTIONS, MODEL, Fixture

# A switch the platform reports as a truth value: its absence has exactly two
# readings, so both can be run.
SWITCH = "Plant.Input.Switch"
SWITCHED = {**MODEL, "entries": MODEL["entries"] + [
    {"address": SWITCH, "role": "input", "names": ["In_Switch"], "type": "boolean"}]}


def document(on_type: str, test: str) -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="changed">
  <datamodel>
    <data id="on" sce:type="{on_type}" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          sce:assumed="lamp-rule"
          sce:assumed-reason="the text does not say which way round the lamp reads"
          expr="{test} ? 1 : 2"/>
  </datamodel>
</scxml>
"""


LAMP = {"address": "Plant.Out.Lamp", "field": "Stat", "map": {1: "OFF", 2: "ON"}}


def case(name, expect, at=SWITCH, value="true"):
    return {"name": name, "given": {at: value},
            "drove": [at], "expect": {"Plant.Out.Lamp.Stat": expect}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AGuessIsChangedToSeeWhatRestsOnIt(Fixture):
    def run_with(self, inputs, lamp, cases, max_runs=64, text=None):
        self.write_pack(SWITCHED, CONVENTIONS)
        (self.root / "changed.scxml").write_text(text or document("bool", "on"),
                                                 encoding="utf-8")
        path = self.root / "changed.binding.yaml"
        path.write_text(yaml.safe_dump({"version": 1, "document": "changed.scxml",
                                        "inputs": inputs, "outputs": {"lamp": lamp}}),
                        encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "cases": cases}), encoding="utf-8")
        pack = self.pack()
        base = verify(pack, path)
        self.assertTrue(base.ran, base.refusal)
        # Every case here is a judged failure: what is asserted is what
        # changing a guess does to one, and a refused case changes nothing.
        self.assertTrue(all(r.judged and r.failures for r in base.results),
                        [(r.name, r.refusal, r.failures, r.compared, r.unwritten)
                         for r in base.results])
        found = explore(pack, path, base, lambda variant: verify(pack, variant), max_runs)
        return base, found, {g.subject: g for g in report(base, pack, counterfactuals=found)}

    def test_a_guess_no_alternative_moves_is_cleared_and_leaves_one(self):
        """⚠ The discriminator. The lamp is wrong, and two recorded guesses
        decide it: the document's reading of the lamp, and the binding's
        reading of a count that is absent. The count is present in the case,
        so neither truth value for its absence changes anything -- both are
        run, the guess is cleared, and the document's guess is the only
        recorded one left. Without the runs, both were only implicated."""
        base, found, gaps = self.run_with(
            {"on": {"address": SWITCH, "when_absent": False,
                    "assumed": "an absent switch reads as off"}},
            LAMP, [case("counted", "ON")])
        self.assertEqual("implicated", base.assumptions["binding:input:on"].status)
        cf = found["binding:input:on"]
        self.assertEqual(("cleared", True), (cf.verdict, cf.exhaustive))
        self.assertEqual("cleared", gaps["input on"].kind)
        self.assertEqual("implicated", gaps["lamp"].kind)
        self.assertEqual([("counted", "Plant.Out.Lamp.Stat")], gaps["lamp"].sole)
        self.assertTrue(any("the only one left" in line for line in lines(gaps["lamp"])))

    def test_an_alternative_that_repairs_everything_is_the_answer(self):
        base, found, gaps = self.run_with(
            {"on": {"address": SWITCH, "when_absent": False}},
            {**LAMP, "assumed": "the platform's symbols for the lamp"},
            [case("counted", "ON")])
        cf = found["binding:output:lamp"]
        self.assertEqual("witness", cf.verdict)
        repairing = [(f.decision, f.value) for f in cf.flips if f.fixed and not f.broken]
        self.assertEqual([("map[1]", "ON")], repairing)
        self.assertEqual("refuted", gaps["output lamp"].kind)
        self.assertTrue(any("what the tests expect" in line
                            for line in lines(gaps["output lamp"])))

    def test_two_failures_wanting_two_values_have_no_witness(self):
        """One document value lands in two cases the tests tell apart: each
        alternative repairs one of them, and none is the answer."""
        base, found, gaps = self.run_with(
            {"on": {"address": SWITCH, "when_absent": False}},
            {**LAMP, "assumed": "the platform's symbols for the lamp"},
            [case("counted", "ON"), case("counted again", "MAX")])
        cf = found["binding:output:lamp"]
        self.assertEqual("moves", cf.verdict)
        self.assertEqual("implicated", gaps["output lamp"].kind)

    def test_a_number_is_sampled_and_never_cleared(self):
        base, found, gaps = self.run_with(
            {"on": {"address": "Plant.Input.Count", "when_absent": 0,
                    "assumed": "an absent count reads as zero"}},
            LAMP, [case("counted", "ON", "Plant.Input.Count", 1)],
            text=document("int32", "on &gt; 0"))
        cf = found["binding:input:on"]
        self.assertEqual(("unmoved", False), (cf.verdict, cf.exhaustive))
        self.assertEqual("implicated", gaps["input on"].kind)
        self.assertEqual([], gaps["lamp"].sole)

    def test_a_spent_budget_says_what_it_did_not_try(self):
        base, found, gaps = self.run_with(
            {"on": {"address": SWITCH, "when_absent": False,
                    "assumed": "an absent switch reads as off"}},
            LAMP, [case("counted", "ON")], max_runs=0)
        cf = found["binding:input:on"]
        self.assertEqual(("untried", False), (cf.verdict, cf.exhaustive))
        self.assertTrue(any("run budget" in u for u in cf.untried))
        self.assertEqual("implicated", gaps["input on"].kind)


if __name__ == "__main__":
    unittest.main()
