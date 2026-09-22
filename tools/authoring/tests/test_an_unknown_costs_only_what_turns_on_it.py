"""Declaring an unknown costs only the positions that turn on it.

A binding may say it does not know which address feeds an input, and that is
the honest answer when nobody has said. It used to stop the whole run: no case
of the component was judged. Writing a plausible address instead cost nothing
and passed -- and a plausible WRONG address is invisible, which is the reason
the key exists. Measured 2026-09-22 across thirty documents written from prose,
the key was used zero times.

So the run no longer refuses and still never invents a value. It asks the one
question that needs no value: does the answer change with it? An unresolved
boolean input has two values, every case runs under both, and only a position
that comes out different is withheld. Asserted here:

    an unknown the answer ignores     is no obstacle at all
    a position that turns on it       is withheld, and the case not passed
    constants alone                   never pass a case     (the first version did)
    a wrong answer that was settled   is still a failure
    an unknown NUMBER                 still refuses -- it has no two values
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import Fixture

HEADER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="guarded">
  <datamodel>
    <data id="mode" sce:type="bool" sce:direction="in"/>
    <data id="guard" sce:type="bool" sce:direction="in"/>
"""

# The guard is declared and never read: nothing can turn on it.
IGNORES = HEADER + """    <data id="lamp" sce:type="int32" sce:direction="out" expr="mode ? 1 : 0"/>
  </datamodel>
</scxml>
"""

# The guard matters only while the supply is HIGH. At LOW the lamp is off
# whatever it is -- which is what lets one case be judged and not the other.
DEPENDS = HEADER + """    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="(mode &amp;&amp; guard) ? 1 : 0"/>
  </datamodel>
</scxml>
"""

UNKNOWN = "nobody has said which signal carries the guard"

BINDING = {
    "version": 1,
    "document": "guarded.scxml",
    "inputs": {
        "mode": {"address": "Plant.Input.SupplyMode", "equals": "HIGH"},
        "guard": {"unresolved": UNKNOWN},
    },
    "outputs": {
        "lamp": {
            "address": "Plant.Out.Lamp",
            "field": "Stat",
            "map": {0: "OFF", 1: "ON"},
            # A constant the binding always writes. It comes out the same
            # under every value of the guard, which is exactly what let the
            # first version pass a case on it alone.
            "also": {"Value": 7},
        }
    },
}

HIGH = {"name": "supply high",
        "given": {"Plant.Input.SupplyMode": "HIGH"},
        "expect": {"Plant.Out.Lamp.Stat": "ON", "Plant.Out.Lamp.Value": 7}}
LOW = {"name": "supply low",
       "given": {"Plant.Input.SupplyMode": "LOW"},
       "expect": {"Plant.Out.Lamp.Stat": "OFF", "Plant.Out.Lamp.Value": 7}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AnUnknownCostsOnlyWhatTurnsOnIt(Fixture):
    def run_with(self, document, *cases, binding=BINDING):
        (self.root / "guarded.scxml").write_text(document, encoding="utf-8")
        path = self.root / "guarded.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "cases": list(cases)}), encoding="utf-8")
        return verify(self.pack(), path)

    def by_name(self, result):
        return {c.name: c for c in result.results}

    def test_an_unknown_the_answer_ignores_is_no_obstacle(self):
        """The discriminator for everything below: without it, a verifier
        that withheld every position would pass the other tests."""
        result = self.run_with(IGNORES, HIGH, LOW)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((2, 0, 0),
                         (result.passed, result.failed, result.unjudged),
                         [(c.name, c.refusal) for c in result.results])
        self.assertEqual(0, result.undetermined)
        self.assertEqual({"guard": UNKNOWN}, result.unresolved,
                         "declared even when it cost nothing -- the reason is "
                         "still a question somebody owes an answer to")

    def test_a_position_that_turns_on_the_unknown_is_withheld(self):
        result = self.run_with(DEPENDS, HIGH, LOW)
        self.assertTrue(result.ran, result.refusal)
        cases = self.by_name(result)
        self.assertTrue(cases["supply low"].passed,
                        "off whatever the guard is: judged, and right")
        self.assertEqual(["Plant.Out.Lamp.Stat"],
                         cases["supply high"].undetermined)
        self.assertFalse(cases["supply high"].judged)
        self.assertEqual((1, 0, 1),
                         (result.passed, result.failed, result.unjudged))

    def test_constants_alone_never_pass_a_case(self):
        """⚠ The loophole the first version of this change left open.

        It withheld a case only when EVERY position was withheld. A case that
        expects an event also expects the constants the binding writes beside
        it, those agree under any input, and so a case whose whole point
        turned on the unknown still counted as passed. On the first real probe
        that read 4 passed where 3 of the 4 had their status withheld.
        """
        result = self.run_with(DEPENDS, HIGH)
        case = self.by_name(result)["supply high"]
        self.assertNotIn("Plant.Out.Lamp.Value", case.undetermined,
                         "the constant WAS settled; the point is that it is "
                         "not enough")
        self.assertFalse(case.passed)
        self.assertEqual(0, result.passed)

    def test_a_wrong_answer_that_was_settled_is_still_a_failure(self):
        """Withholding the unknown does not excuse what was known."""
        wrong = {**LOW, "name": "supply low, wrongly expected on",
                 "expect": {"Plant.Out.Lamp.Stat": "ON"}}
        result = self.run_with(DEPENDS, wrong)
        self.assertEqual((0, 1, 0),
                         (result.passed, result.failed, result.unjudged))

    def test_an_unknown_number_still_refuses(self):
        """A number has no two values to try; any one chosen is invented."""
        binding = {**BINDING, "inputs": {
            **BINDING["inputs"],
            "guard": {"unresolved": UNKNOWN, "number": True, "when_absent": 0}}}
        result = self.run_with(DEPENDS, HIGH, binding=binding)
        self.assertFalse(result.ran)
        self.assertIn("number", result.refusal)
        self.assertIn(UNKNOWN, result.refusal)


if __name__ == "__main__":
    unittest.main()
