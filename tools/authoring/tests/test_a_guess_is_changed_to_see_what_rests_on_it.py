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
    a number the document only compares is tried once per interval its
    thresholds cut, and cleared                         (the discriminator)
    the model's range drops an interval no reading can land in
    a number it computes with is only sampled, and never cleared
    a document's guess is run with each of its `sce:assumed-candidates`
                                                        (the discriminator)
    candidates whose place in the expression is not known are refused
    a guess without candidates is not changed
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
    def run_with(self, inputs, lamp, cases, max_runs=64, text=None, model=SWITCHED):
        self.write_pack(model, CONVENTIONS)
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

    def test_a_number_only_compared_is_tried_per_interval_and_cleared(self):
        """⚠ The discriminator for numbers. The document only asks whether
        the count is above 0, so it cannot tell apart two counts on the same
        side of 0: the threshold and one value either side are every
        behaviour it has. All ran, none moved the failure -- cleared, where a
        sample could only have said "unmoved"."""
        base, found, gaps = self.run_with(
            {"on": {"address": "Plant.Input.Count", "when_absent": 0,
                    "assumed": "an absent count reads as zero"}},
            LAMP, [case("counted", "ON", "Plant.Input.Count", 1)],
            text=document("int32", "on &gt; 0"))
        cf = found["binding:input:on"]
        self.assertEqual(("cleared", True), (cf.verdict, cf.exhaustive))
        self.assertEqual([-1, 1], sorted(f.value for f in cf.flips))
        self.assertEqual("cleared", gaps["input on"].kind)
        self.assertEqual([("counted", "Plant.Out.Lamp.Stat")], gaps["lamp"].sole)

    def test_the_models_range_bounds_the_intervals(self):
        """A count cannot go below 0 on this platform: the interval below the
        threshold is not tried."""
        model = {**SWITCHED, "entries": [
            {**e, "range": {"minimum": 0}} if e["address"] == "Plant.Input.Count" else e
            for e in SWITCHED["entries"]]}
        base, found, _ = self.run_with(
            {"on": {"address": "Plant.Input.Count", "when_absent": 0,
                    "assumed": "an absent count reads as zero"}},
            LAMP, [case("counted", "ON", "Plant.Input.Count", 1)],
            text=document("int32", "on &gt; 0"), model=model)
        self.assertEqual([1], [f.value for f in found["binding:input:on"].flips])

    def test_a_number_the_document_computes_with_is_only_sampled(self):
        base, found, gaps = self.run_with(
            {"on": {"address": "Plant.Input.Count", "when_absent": 0,
                    "assumed": "an absent count reads as zero"}},
            LAMP, [case("counted", "ON", "Plant.Input.Count", 1)],
            text=document("int32", "(on + 1) &gt; 1"))
        cf = found["binding:input:on"]
        self.assertEqual(("unmoved", False), (cf.verdict, cf.exhaustive))
        self.assertTrue(any("beyond comparing" in u for u in cf.untried))
        self.assertEqual("implicated", gaps["input on"].kind)
        self.assertEqual([], gaps["lamp"].sole)

    def test_a_documents_guess_is_changed_to_its_candidates(self):
        """⚠ The discriminator for a document's own guess. The author chose 1
        for a switched-on lamp and wrote down 2 as the other reading: with 2
        in its place the case passes, and that is the answer."""
        text = document("bool", "on").replace(
            'expr="on ? 1 : 2"', 'sce:assumed-candidates="1 2" expr="on ? 1 : 3"')
        base, found, gaps = self.run_with(
            {"on": {"address": SWITCH, "when_absent": False}},
            {**LAMP, "map": {1: "OFF", 2: "ON", 3: "MAX"}},
            [case("counted", "ON")], text=text)
        cf = found["document:lamp"]
        self.assertEqual(("witness", True), (cf.verdict, cf.exhaustive))
        self.assertEqual([("candidate", "2")],
                         [(f.decision, f.value) for f in cf.flips if f.fixed])
        self.assertEqual("refuted", gaps["lamp"].kind)

    def test_candidates_whose_place_is_not_known_are_refused_by_check(self):
        """1 appears twice: which of them the author chose is not written
        down, and choosing for them would change a different document."""
        self.write_pack(SWITCHED, CONVENTIONS)
        (self.root / "changed.scxml").write_text(document("bool", "on").replace(
            'expr="on ? 1 : 2"', 'sce:assumed-candidates="1 2" expr="on ? 1 : 1"'),
            encoding="utf-8")
        found = [str(f) for f in self.bind(
            {"version": 1, "document": "changed.scxml",
             "inputs": {"on": {"address": SWITCH, "when_absent": False}},
             "outputs": {"lamp": {**LAMP, "map": {1: "OFF"}}}})]
        self.assertTrue(any("1 ×2" in f and "assumed-candidates" in f for f in found), found)

    def test_a_guess_without_candidates_is_not_changed(self):
        base, found, _ = self.run_with(
            {"on": {"address": SWITCH, "when_absent": False}},
            LAMP, [case("counted", "ON")])
        cf = found["document:lamp"]
        self.assertEqual("untried", cf.verdict)
        self.assertTrue(any("nobody wrote down" in u for u in cf.untried))

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
