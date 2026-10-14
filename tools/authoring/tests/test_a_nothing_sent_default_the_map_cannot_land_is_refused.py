"""The value a round writes when nothing was sent has to be one the output's map can land.

The host answers a round in which the document sent nothing with the rule's `when_nothing_sent`, and
that value goes through the map like any other. `check` asked this only of an output whose sends it
could list by name; an output that takes its value from a send's `param` has no list, so the default
was never asked about. Measured 2026-10-13: a Qwen draft bound `map: platform` -- whose keys are the
platform's NUMBERS -- beside `when_nothing_sent: "OFF"`, `check` said "no refusals", and `verify` then
left every case that sent nothing unjudged, "the binding's map has no entry".

`map: platform` has since been keyed by the symbols' NAMES (a closed set crosses the boundary by
name), so beside it the default that cannot land is a number and the one that can is a symbol; a
map a binding writes out is still held to its own keys, whatever they are.

Each refusal is paired with the nearest binding that must not be refused.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import check
from sce_author.landing import PLATFORM_MAP, unmapped_default
from sce_author.pack import load_pack

from tests.test_a_statechart_is_driven_not_called import BINDING, CROSSING, DELAYED

SEEN = "when_nothing_sent"


def road(**more) -> dict:
    return {"address": "plant/out/road-signal", "field": "value",
            "sent": {"processor": "x-sce-host", "param": "road"}, **more}


class AMapAndItsDefault(unittest.TestCase):
    """The pure question, beside the cases that must not be refused."""

    def test_a_default_the_map_has_no_entry_for_is_named(self):
        self.assertEqual(["OFF"], unmapped_default({"map": {0: "NONE", 1: "OFF"}, SEEN: "OFF"}))

    def test_a_default_the_map_keys_is_not(self):
        self.assertEqual([], unmapped_default({"map": {0: "NONE", 1: "OFF"}, SEEN: 1}))
        self.assertEqual([], unmapped_default({"map": {"signal.dark": "DARK"}, SEEN: "signal.dark"}))

    def test_nothing_is_asked_without_a_map_or_a_default(self):
        self.assertEqual([], unmapped_default({SEEN: "OFF", "passthrough": True}))
        self.assertEqual([], unmapped_default({"map": {0: "NONE"}}))

    def test_a_hold_last_default_may_be_left_out_on_purpose(self):
        rule = {"map": {0: "NONE"}, SEEN: "signal.none", "hold_last": True}
        self.assertEqual([], unmapped_default(rule))


class TheCheckRefuses(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(DELAYED, encoding="utf-8")

    def details(self, rule: dict) -> list:
        binding = {**BINDING, "outputs": {"roadSignal": rule}}
        (self.tmp / "b.yaml").write_text(yaml.safe_dump(binding), encoding="utf-8")
        return [f.detail for f in check(load_pack(self.tmp), self.tmp / "b.yaml")]

    def refusals(self, rule: dict) -> list:
        return [detail for detail in self.details(rule)
                if "is what the host writes in a round where nothing is sent" in detail]

    def test_a_symbol_beside_a_map_keyed_by_numbers(self):
        said = self.refusals(road(map={0: "DARK", 1: "FLASHING", 2: "STEADY"}, **{SEEN: "DARK"}))
        self.assertEqual(1, len(said), said)
        self.assertIn("'DARK'", said[0])
        self.assertIn("0, 1, 2", said[0])

    def test_the_same_with_the_platforms_own_table(self):
        """`map: platform` is keyed by the platform's symbols under their own NAMES, so the
        default it cannot land is the other spelling: a number, which names no symbol."""
        said = self.refusals(road(map=PLATFORM_MAP, **{SEEN: 0}))
        self.assertEqual(1, len(said), said)
        self.assertIn("`when_nothing_sent: 0`", said[0])
        self.assertIn("'DARK', 'FLASHING', 'STEADY'", said[0])

    def test_the_value_the_map_keys_is_accepted(self):
        self.assertEqual([], self.refusals(road(map={0: "DARK", 1: "FLASHING", 2: "STEADY"}, **{SEEN: 0})))
        self.assertEqual([], self.refusals(road(map=PLATFORM_MAP, **{SEEN: "DARK"})))

    def test_a_hold_last_output_may_leave_its_default_out_of_the_map(self):
        self.assertEqual([], self.refusals(road(map={1: "FLASHING"}, hold_last=True, **{SEEN: 0})))

    def test_a_map_keyed_by_numbers_lands_a_number_on_a_closed_set(self):
        """⚠ What a send's `param` carries has no declared type to judge, so the map says
        it: keyed by numbers, the document sends the platform's codes. The default fitting
        the map is beside the point -- it is the keys that are refused."""
        said = [d for d in self.details(road(map={0: "DARK", 1: "FLASHING", 2: "STEADY"},
                                             **{SEEN: 0})) if "closed set" in d]
        self.assertEqual(1, len(said), said)
        self.assertIn("plant/out/road-signal.value", said[0])
        self.assertIn("keyed by 0, 1, 2", said[0])

    def test_a_map_keyed_by_names_does_not(self):
        for table in (PLATFORM_MAP, {"dark": "DARK", "flashing": "FLASHING"}):
            with self.subTest(table=table):
                default = "DARK" if table == PLATFORM_MAP else "dark"
                self.assertEqual([], [d for d in self.details(road(map=table, **{SEEN: default}))
                                      if "closed set" in d])


if __name__ == "__main__":
    unittest.main()
