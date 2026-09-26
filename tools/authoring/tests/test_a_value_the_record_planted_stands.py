"""A value the record wrote into an output position stands until the document
writes there.

A product's own tests set output slots themselves -- an event's identifier,
planted before the round that turns that event off -- and the slot then holds
what the TEST wrote until the component writes it again. `verify` did not know
the test had written there. Measured 2026-09-27 over thirteen components: five
had records that drive an output position directly, 1,009 times between them,
and one document had cases judged against the identifier it wrote rounds
earlier, where the record had since planted another -- and every case whose
judged round planted a value was withheld outright, as a drive of "an address
this component does not receive".

Asserted here, on both paths:

    a planted value beats the older write a `hold_last` rule would replay
                                                        (the discriminator)
    the document writing the position in the same round replaces it
    a case whose agreeing positions were all planted judged nothing
    planting an output position is not a drive the component fails to receive
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, verify
from tests.test_a_binding_can_say_it_guessed import HELD, at, held_binding
from tests.test_a_statechart_is_driven_not_called import (
    CROSSING, EXAMPLES, HOLDING, SPEAKS_ON_CHANGE, codegen_is_built, track)
from tests.test_refusals_actually_fire import Fixture

SIGNAL = "plant/out/road-signal.value"
APPROACH = "plant/in/train-approach"


def details(result):
    return [(r.name, r.refusal, r.failures) for r in result.results]


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class AStatechartReadsWhatTheRecordPlanted(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(SPEAKS_ON_CHANGE, encoding="utf-8")

    def run_cases(self, cases):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump({**EXAMPLES, "cases": cases}), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(HOLDING), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    @staticmethod
    def plant(value: str, approach: str) -> dict:
        """A setup step that writes the signal slot itself."""
        return {"given": {APPROACH: approach, SIGNAL: value}, "drove": [SIGNAL]}

    def test_a_planted_value_beats_the_held_one(self):
        """⚠ The discriminator. The first case leaves FLASHING held; the
        second plants DARK over it and its judged round sends nothing. The
        slot holds DARK, so expecting FLASHING fails -- where replaying the
        held write passed it."""
        result = self.run_cases([
            track("a train approaches", "APPROACHING", "FLASHING"),
            track("occupied, after the record darkened the signal", "OCCUPIED",
                  "FLASHING", before=[self.plant("DARK", "APPROACHING")]),
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 1, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_agreeing_with_only_what_was_planted_judges_nothing(self):
        result = self.run_cases([
            track("a train approaches", "APPROACHING", "FLASHING"),
            track("occupied, after the record darkened the signal", "OCCUPIED",
                  "DARK", before=[self.plant("DARK", "APPROACHING")]),
        ])
        self.assertEqual((1, 0, 1), (result.passed, result.failed, result.unjudged),
                         details(result))
        self.assertIn("the record itself wrote", result.results[1].refusal)

    def test_the_document_writing_the_position_replaces_what_was_planted(self):
        """The plant lands first and the document's round after it, in the
        same step -- which is also a step driving an output position, and so
        must not be withheld as a drive this component never receives."""
        result = self.run_cases([
            {"name": "a train approaches over a planted dark signal",
             "given": {APPROACH: "APPROACHING", SIGNAL: "DARK"},
             "drove": [SIGNAL, APPROACH],
             "expect": {SIGNAL: "FLASHING"}},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         details(result))


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AComputationReadsWhatTheRecordPlanted(Fixture):
    def judge(self, *cases):
        (self.root / "counted.scxml").write_text(HELD, encoding="utf-8")
        path = self.root / "counted.binding.yaml"
        path.write_text(yaml.safe_dump(held_binding()), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "ordered": True,
            "cases": list(cases)}), encoding="utf-8")
        return verify(self.pack(), path)

    @staticmethod
    def planting(count, value, **expect) -> dict:
        """A round that drives the count and plants the lamp's value."""
        step = at(count, **expect)
        step["given"]["Plant.Out.Lamp.Value"] = value
        step["drove"] = ["Plant.Out.Lamp.Value", *step["drove"]]
        return step

    def test_a_planted_value_beats_the_held_one(self):
        """⚠ The discriminator. The setup round writes 22 and the judged one
        computes nothing mapped, so `hold_last` would replay 22 -- but the
        record planted 11 in that round, and 11 is what the slot holds."""
        result = self.judge({"name": "the record resets the lamp",
                             "before": [at(2)],
                             **self.planting(0, 11, **{"Plant.Out.Lamp.Value": 22})})
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 1, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_the_document_writing_the_position_replaces_what_was_planted(self):
        result = self.judge({"name": "the lamp is computed over a plant",
                             **self.planting(1, 22, **{"Plant.Out.Lamp.Value": 11})})
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_a_plant_does_not_outlive_its_case(self):
        """Across cases the order is a promise the examples need not have
        made, so what one case planted is not what the next one reads: the
        second case's round writes nothing, and nothing is there."""
        result = self.judge(
            {"name": "planted", **self.planting(0, 11)},
            {"name": "the next case", **at(0, **{"Plant.Out.Lamp.Value": 11})})
        case = result.results[-1]
        self.assertFalse(case.judged, details(result))
        self.assertEqual(["Plant.Out.Lamp.Value"], case.unwritten)


if __name__ == "__main__":
    unittest.main()
