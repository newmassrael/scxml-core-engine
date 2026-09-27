"""What a round writes, and whether there is one, as the platform has it.

Two places where the run and the platform parted on 2026-09-27, both found by
setting `verify`'s verdicts beside the platform's own, case by case:

    a case whose drives reach no input the binding reads runs nothing on an
    `on-change` host, so a host that states its writes announces nothing and
    the case fails -- where it passed, computed anyway     (the discriminator)
    such a failure is at the positions the case waited on, so the guess
    behind them is blamed                               (the discriminator)
    without the host stated, the run is as it was
    a drive that does reach a bound input is a round as before
    a restatement the record says the platform DELIVERED is a round
                                                        (the discriminator)
    a delivery of something the step did not drive is refused

    `check`: two `when` values that land the same value write different
    fields of one record, so one shown after the other carries its fields
                                                        (the discriminator)
    every such value writing every field is not refused
    a value going off that writes fewer fields than the ways of showing is
    not compared with them
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.errors import PackError
from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import CONVENTIONS, MODEL, Fixture

CONSTANT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="constant">
  <datamodel>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out" expr="2"/>
  </datamodel>
</scxml>
"""

CONSTANT_BINDING = {
    "version": 1, "document": "constant.scxml",
    "inputs": {"count": {"address": "Plant.Input.Count", "when_absent": 0}},
    "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Stat",
                         "map": {2: "ON"}}},
}

HOST = {"activation": "on-change", "writes": "every-round"}


def case(name, drove, value):
    return {"name": name, "given": {drove: value}, "drove": [drove],
            "expect": {"Plant.Out.Lamp.Stat": "ON"}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class ARoundIsWhatTheHostRuns(Fixture):
    def run_cases(self, cases, host, document=CONSTANT):
        conventions = {**CONVENTIONS, "host": host} if host else CONVENTIONS
        self.write_pack(MODEL, conventions, {
            "version": 1, "origin": "written for this test", "ordered": True,
            "independent_cases": False, "cases": cases})
        (self.root / "constant.scxml").write_text(document, encoding="utf-8")
        path = self.root / "constant.binding.yaml"
        path.write_text(yaml.safe_dump(CONSTANT_BINDING), encoding="utf-8")
        result = verify(self.pack(), path)
        self.assertTrue(result.ran, result.refusal)
        return result

    def test_a_drive_that_reaches_no_bound_input_is_no_round(self):
        """⚠ The discriminator. The supply mode is an input of this
        component, and the binding reads only the count: changing the mode
        runs nothing on this host. The document's constant was computed and
        passed anyway; the platform announced nothing and failed it."""
        result = self.run_cases([
            case("counted", "Plant.Input.Count", 3),
            case("supply changed", "Plant.Input.SupplyMode", "HIGH")], HOST)
        verdicts = {r.name: r for r in result.results}
        self.assertTrue(verdicts["counted"].passed)
        failed = verdicts["supply changed"]
        self.assertFalse(failed.passed)
        self.assertEqual("Plant.Out.Lamp.Stat", failed.failures[0][0])
        self.assertIn("no input the binding reads changed", failed.failures[0][2])

    def test_a_round_that_never_ran_is_blamed_on_the_guess_behind_the_position(self):
        """⚠ The discriminator for attribution. The author wrote the lamp as
        a constant because the text never decides it; a case changing only
        the input the platform decides it by is failed by that choice. It
        used to fail at a pseudo-position no guess stands behind, and the
        guess came back as one nothing compared."""
        guessed = CONSTANT.replace(
            'expr="2"', 'expr="2" sce:assumed="lamp-constant" '
                        'sce:assumed-reason="the text never decides the lamp"')
        result = self.run_cases([
            case("counted", "Plant.Input.Count", 3),
            case("supply changed", "Plant.Input.SupplyMode", "HIGH")], HOST, guessed)
        guess = result.assumptions["document:lamp"]
        self.assertEqual("refuted", guess.status)
        self.assertEqual("supply changed", guess.refuted_by[0][0])

    def test_a_restated_bound_input_is_no_round_either(self):
        result = self.run_cases([
            case("counted", "Plant.Input.Count", 3),
            case("counted again", "Plant.Input.Count", 3)], HOST)
        self.assertFalse({r.name: r for r in result.results}["counted again"].passed)

    def test_a_restatement_the_platform_delivered_is_a_round(self):
        """⚠ The discriminator for `delivered`. The same symbol twice is no
        change to the core -- but the platform decides on the reading it
        carries, which the record knows and the value space does not."""
        again = case("counted again", "Plant.Input.Count", 3)
        again["delivered"] = ["Plant.Input.Count"]
        result = self.run_cases([case("counted", "Plant.Input.Count", 3), again], HOST)
        self.assertTrue({r.name: r for r in result.results}["counted again"].passed)

    def test_a_delivery_of_something_not_driven_is_refused(self):
        stray = case("counted", "Plant.Input.Count", 3)
        stray["delivered"] = ["Plant.Input.SupplyMode"]
        self.write_pack(MODEL, {**CONVENTIONS, "host": HOST}, {
            "version": 1, "origin": "written for this test", "ordered": True,
            "cases": [stray]})
        with self.assertRaises(PackError) as caught:
            self.pack()
        self.assertIn("a delivery is of a drive", str(caught.exception))

    def test_without_the_host_stated_the_run_is_as_it_was(self):
        result = self.run_cases([
            case("counted", "Plant.Input.Count", 3),
            case("supply changed", "Plant.Input.SupplyMode", "HIGH")], None)
        self.assertTrue(all(r.passed for r in result.results),
                        [(r.name, r.failures) for r in result.results])


# A record address: an event's status, its identifier and its sound.
EVENT_MODEL = {**MODEL, "entries": MODEL["entries"] + [{
    "address": "Plant.Out.Event", "role": "output", "names": ["Fig_Event"],
    "fields": {"Stat": {"values": {"OFF": 1, "ON": 2}}, "ID": {"type": "text"},
               "Count": {"type": "number"}, "Duration": {"type": "number"}}}]}

EVENT_DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="fixture">
  <datamodel>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="event" sce:type="int32" sce:direction="out"
          expr="count &gt; 10 ? 2 : (count &gt; 0 ? 1 : 0)"/>
  </datamodel>
</scxml>
"""


def event_binding(when):
    return {"version": 1, "document": "fixture.scxml",
            "inputs": {"count": {"address": "Plant.Input.Count", "when_absent": 0}},
            "outputs": {"event": {"address": "Plant.Out.Event", "field": "Stat",
                                  "map": {0: "OFF", 1: "ON", 2: "ON"}, "when": when}}}


class ARecordIsWrittenWhole(Fixture):
    def found(self, when):
        self.write_pack(EVENT_MODEL, CONVENTIONS)
        (self.root / "fixture.scxml").write_text(EVENT_DOCUMENT, encoding="utf-8")
        return [str(f) for f in self.bind(event_binding(when))]

    def test_two_ways_of_showing_that_write_different_fields_are_refused(self):
        """⚠ The discriminator. Shown as 1 after 2, the event would carry 2's
        duration: 1 does not write one."""
        found = self.found({1: {"ID": "A", "Count": 1}, 2: {"ID": "B", "Duration": 10}})
        said = [f for f in found if "write different fields" in f]
        self.assertEqual(1, len(said), found)
        self.assertIn("1 leaves Duration", said[0])
        self.assertIn("2 leaves Count", said[0])

    def test_every_way_of_showing_writing_every_field_is_not_refused(self):
        found = self.found({1: {"ID": "A", "Count": 1, "Duration": 0},
                            2: {"ID": "B", "Count": 0, "Duration": 10}})
        self.assertFalse([f for f in found if "write different fields" in f], found)

    def test_going_off_is_not_compared_with_showing(self):
        """The identifier says what is turning off; the sound is not
        rewritten. That is a different value of the event's status, and not
        a second story about the same one."""
        found = self.found({0: {"ID": "A"},
                            1: {"ID": "A", "Count": 1, "Duration": 0},
                            2: {"ID": "B", "Count": 0, "Duration": 10}})
        self.assertFalse([f for f in found if "write different fields" in f], found)


if __name__ == "__main__":
    unittest.main()
