"""A binding can say which of its decisions was a guess.

The document has had `sce:assumed` for this since before the binding existed:
the author writes down that a value was decided without the specification
saying so, and a case that fails on it is reported as that guess being
refuted rather than as the document being wrong. The binding had no such key.
A decision made THERE -- reading a number's absence as 0, choosing a symbol
for a platform default -- could only live in `note`, which nothing reads.
Measured 2026-09-22, an author reading a number's absence as 0 wrote that the
choice changes the outcome, and had nowhere to say so but a comment.

Asserted here:

    the schema admits `assumed` on an input and on an output rule
    a failure resting on an input's guess names that guess
    a failure on a position an output's guessed rule writes names that guess
    without the key, the same failure names nothing     (the discriminator)
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.check import check
from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import Fixture

COUNTED = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="counted">
  <datamodel>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="count &gt; 0 ? 1 : 0"/>
  </datamodel>
</scxml>
"""

GUESS = "nobody said what an absent count means; read as 0"


def binding(count_rule=None, lamp_rule=None):
    return {
        "version": 1,
        "document": "counted.scxml",
        "inputs": {"count": {"address": "Plant.Input.Count",
                             "when_absent": 0, **(count_rule or {})}},
        "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Stat",
                             "map": {0: "OFF", 1: "ON"}, **(lamp_rule or {})}},
    }


# The count is not reported, so `when_absent` decides -- and the record says
# the lamp was on. The guess about absence is the thing that is wrong.
ABSENT = {"name": "count not reported",
          "given": {"Plant.Input.SupplyMode": "HIGH"},
          "expect": {"Plant.Out.Lamp.Stat": "ON"}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class ABindingCanSayItGuessed(Fixture):
    def run_with(self, rules, *cases):
        (self.root / "counted.scxml").write_text(COUNTED, encoding="utf-8")
        path = self.root / "counted.binding.yaml"
        path.write_text(yaml.safe_dump(rules), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "cases": list(cases)}), encoding="utf-8")
        return path

    def test_the_schema_admits_the_key_on_both_sides(self):
        path = self.run_with(binding({"assumed": GUESS},
                                     {"assumed": "a platform default"}), ABSENT)
        said = "\n".join(str(f) for f in check(self.pack(), path))
        self.assertNotIn("Additional properties", said)

    def judge(self, rules, *cases):
        # ⚠ The examples are written BEFORE the pack is loaded. Passing both
        # as arguments of one call loads the pack first -- with the fixture's
        # own examples -- and every assertion below then holds or fails about
        # cases nobody here wrote.
        path = self.run_with(rules, *cases)
        return verify(self.pack(), path)

    def test_a_failure_resting_on_an_inputs_guess_names_it(self):
        result = self.judge(binding({"assumed": GUESS}), ABSENT)
        self.assertEqual(1, result.failed)
        self.assertIn(GUESS, result.refuted.get("Plant.Out.Lamp.Stat", ""))

    def test_a_failure_on_a_guessed_output_rule_names_it(self):
        result = self.judge(
            binding(lamp_rule={"assumed": "the lamp symbols are a default"}),
            ABSENT)
        self.assertEqual(1, result.failed)
        self.assertIn("the lamp symbols are a default",
                      result.refuted.get("Plant.Out.Lamp.Stat", ""))

    def test_without_the_key_the_same_failure_names_nothing(self):
        """The discriminator. A verifier that attributed every failure to
        some guess would pass the two tests above."""
        result = self.judge(binding(), ABSENT)
        self.assertEqual(1, result.failed, "the failure itself is unchanged")
        self.assertEqual({}, result.refuted)


# --------------------------------------------- what a position keeps: hold_last

HELD = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="counted">
  <datamodel>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <!-- Which alarm is active: 1 or 2, and 0 when neither is. -->
    <data id="which" sce:type="int32" sce:direction="out"
          expr="count === 1 ? 1 : (count === 2 ? 2 : 0)"/>
  </datamodel>
</scxml>
"""


def held_binding(hold=True):
    rule = {"address": "Plant.Out.Lamp", "field": "Value",
            "map": {1: 11, 2: 22}}
    if hold:
        rule["hold_last"] = True
    return {"version": 1, "document": "counted.scxml",
            "inputs": {"count": {"address": "Plant.Input.Count",
                                 "when_absent": 0}},
            "outputs": {"which": rule}}


def at(count, **expect):
    return {"given": {"Plant.Input.Count": count}, "drove": ["Plant.Input.Count"],
            **({"expect": expect} if expect else {})}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class APositionKeepsWhatItHeld(Fixture):
    """`hold_last`: a value with no map entry keeps the last one written.

    Carried over from the oracle the first packs were measured with, which put
    it in the binding on purpose -- the identifier beside an event says WHAT
    turns off, so it outlives the condition; that is the address's structure,
    not the specification's. The first authoring core never took it over, and
    three components written against that oracle had every case unjudged,
    because the document's "no alarm" value had no entry in the map.
    """

    def judge(self, rules, *cases, ordered=True):
        (self.root / "counted.scxml").write_text(HELD, encoding="utf-8")
        path = self.root / "counted.binding.yaml"
        path.write_text(yaml.safe_dump(rules), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "ordered": ordered,
            "cases": list(cases)}), encoding="utf-8")
        return verify(self.pack(), path)

    def test_an_unmapped_value_keeps_the_last_one_written(self):
        after_alarm = {"name": "the alarm clears", "before": [at(2)],
                       **at(0, **{"Plant.Out.Lamp.Value": 22})}
        result = self.judge(held_binding(), after_alarm)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0),
                         (result.passed, result.failed, result.unjudged),
                         [(c.name, c.refusal, c.failures) for c in result.results])

    def test_before_anything_was_held_the_position_is_not_written(self):
        """Not refused, and not given an invented value: nothing is there."""
        cold = {"name": "no alarm yet", **at(0, **{"Plant.Out.Lamp.Value": 22})}
        result = self.judge(held_binding(), cold)
        case = result.results[0]
        self.assertTrue(case.judged, case.refusal)
        self.assertEqual(["Plant.Out.Lamp.Value"], case.unchecked)

    def test_without_it_the_same_value_refuses_the_case(self):
        """The discriminator: without `hold_last` the unmapped value is a
        binding that cannot translate what the document produced."""
        after_alarm = {"name": "the alarm clears", "before": [at(2)],
                       **at(0, **{"Plant.Out.Lamp.Value": 22})}
        result = self.judge(held_binding(hold=False), after_alarm)
        self.assertEqual(1, result.unjudged)
        self.assertIn("no entry", result.results[0].refusal)

    def test_it_needs_an_order_to_have_a_last_value(self):
        result = self.judge(held_binding(), at(1, **{"Plant.Out.Lamp.Value": 11}),
                            ordered=False)
        self.assertFalse(result.ran)
        self.assertIn("which", result.refusal)


if __name__ == "__main__":
    unittest.main()
