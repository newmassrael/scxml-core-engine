"""A boolean output is mapped whichever way its binding spells the two cases.

A document output declared `bool` arrives as True or False. A binding writes
the two cases of its `map` -- and of its `when` -- as `0`/`1`, as
`true`/`false`, or as YAML's own booleans, and the schema admits all three.
The verifier matched only `0`/`1`, so the other two spellings made every case
unjudged: "the binding's map has no entry", a verdict about the verifier
reported as one about the binding. Measured 2026-09-22 when two authors
writing from a brief both chose `true`/`false` and `check` had accepted both.
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.verify import output_values

RULE = {"address": "Plant.Out.Lamp", "field": "Stat"}


class ATruthValueMapsUnderEverySpelling(unittest.TestCase):
    def mapped(self, keys_yaml: str, computed):
        rule = {**RULE, "map": yaml.safe_load(keys_yaml)}
        return output_values("lamp", rule, computed)["Plant.Out.Lamp.Stat"]

    # ⚠ The written symbols are quoted: YAML reads a bare ON and OFF as
    # booleans themselves, which is a different defect the README warns of.
    def test_numbers(self):
        self.assertEqual("ON", self.mapped("{1: 'ON', 0: 'OFF'}", True))
        self.assertEqual("OFF", self.mapped("{1: 'ON', 0: 'OFF'}", False))

    def test_yamls_own_booleans(self):
        self.assertEqual("ON", self.mapped("{true: 'ON', false: 'OFF'}", True))
        self.assertEqual("OFF", self.mapped("{true: 'ON', false: 'OFF'}", False))

    def test_the_words_as_strings(self):
        self.assertEqual("ON", self.mapped("{'true': 'ON', 'false': 'OFF'}", True))
        self.assertEqual("OFF", self.mapped("{'True': 'ON', 'FALSE': 'OFF'}", False))

    def test_when_reads_the_same_way(self):
        rule = {**RULE, "map": {0: "OFF", 1: "ON"},
                "when": {True: {"Blink": "YES"}}}
        self.assertEqual("YES", output_values("lamp", rule, True)["Plant.Out.Lamp.Blink"])
        self.assertNotIn("Plant.Out.Lamp.Blink", output_values("lamp", rule, False))

    def test_a_truth_value_names_no_number(self):
        """The discriminator. Reading `true` as `1` is a rule about BOOLEAN
        outputs; a number output of 1 is not named by a truth-value key."""
        rule = {**RULE, "map": {True: "ON"}}
        from sce_author.verify import VerifyError
        with self.assertRaises(VerifyError):
            output_values("lamp", rule, 1)


if __name__ == "__main__":
    unittest.main()
