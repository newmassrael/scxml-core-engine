"""A key the standalone target does not implement is refused, never dropped.

A library that quietly ignored `hysteresis` would pass its own replay and ship without it, so
every thing it does not build is a refusal that names it. The platform's numbers are the same
kind of thing: the interface lists symbols, and a table of numbers in its place, or a symbol
handed to a document as a number, is refused rather than carried.

Every refusal is paired with the nearest input that must not be refused.
"""

from __future__ import annotations

import unittest

import yaml

from sce_targets import standalone

from tests.support import STANDALONE_TARGET, Working


class TheInterfaceIsRead(Working):
    def refusal(self, edit, target: dict | None = None) -> str:
        split_dir = self.split()
        path = split_dir / "logical" / "pump.interface.yaml"
        interface = yaml.safe_load(path.read_text(encoding="utf-8"))
        edit(interface)
        path.write_text(yaml.safe_dump(interface), encoding="utf-8")
        target_path = STANDALONE_TARGET
        if target is not None:
            target_path = self.root / "target.yaml"
            target_path.write_text(yaml.safe_dump(target), encoding="utf-8")
        done = self.run_module("sce_targets.standalone", path, target_path, self.root / "nocodegen",
                               self.root / "lib")
        self.assertEqual(3, done.returncode, done.stdout + done.stderr)
        return done.stderr

    def test_a_table_of_numbers_for_a_signals_symbols(self):
        said = self.refusal(
            lambda i: i["signals"]["IN_Mode"].update(values={"AUTO": 0, "MANUAL": 1, "OFF": 2}))
        self.assertIn("IN_Mode", said)
        self.assertIn("list of symbols", said)

    def test_a_target_that_is_not_on_change(self):
        said = self.refusal(lambda i: None,
                            target={**yaml.safe_load(STANDALONE_TARGET.read_text()), "activation": "periodic"})
        self.assertIn("only on-change", said)

    def test_a_round_the_target_does_not_know(self):
        said = self.refusal(lambda i: None,
                            target={**yaml.safe_load(STANDALONE_TARGET.read_text()), "round": "burst"})
        self.assertIn("batch or per-signal", said)


class TheInputsAreRead(unittest.TestCase):
    SIGNALS = {"IN_Mode": {"values": ["AUTO", "MANUAL"]}, "IN_Level": {"type": "number"}}

    def expression(self, rule: dict, ptype: str = "std::int32_t"):
        return standalone.input_expression("level", rule, ptype, {"IN_Mode": 0, "IN_Level": 1},
                                           self.SIGNALS, {})

    def test_a_number_signal_is_handed_to_a_number(self):
        """The discriminator: the refusal below is about the symbols, not about the type."""
        body = self.expression({"signal": "IN_Level"})
        self.assertTrue(any("static_cast<std::int32_t>(number)" in line for line in body), body)

    def test_a_closed_set_is_not_handed_to_a_number(self):
        with self.assertRaises(standalone.Refused) as caught:
            self.expression({"signal": "IN_Mode"})
        self.assertIn("closed set of symbols", str(caught.exception))
        self.assertIn("by name", str(caught.exception))

    def test_a_closed_set_is_compared_by_its_symbols(self):
        body = self.expression({"signal": "IN_Mode", "equals": "AUTO"}, ptype="bool")
        self.assertEqual(["return (mSig0 && *mSig0 == INMode::Auto);"], body)

    def test_a_comparison_with_a_symbol_the_signal_does_not_have(self):
        with self.assertRaises(standalone.Refused) as caught:
            self.expression({"signal": "IN_Mode", "equals": "BOGUS"}, ptype="bool")
        self.assertIn("BOGUS", str(caught.exception))


class TheTransformIsPlanned(unittest.TestCase):
    def plan(self, input_rule: dict):
        interface = {"component": "c", "signals": {"IN_A": {"type": "number"}},
                     "inputs": {"a": {"signal": "IN_A", **input_rule}}, "outputs": {}}
        return standalone.plan_transform(interface, "")

    def test_an_input_key_it_builds(self):
        self.plan({"when_absent": 0})

    def test_an_input_key_it_does_not_build(self):
        with self.assertRaises(standalone.Refused) as caught:
            self.plan({"hysteresis": 3})
        self.assertIn("hysteresis", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
