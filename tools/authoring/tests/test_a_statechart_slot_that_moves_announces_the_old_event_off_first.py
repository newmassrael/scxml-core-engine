"""A statechart's event slot that moves announces the old event off first.

A component that `announces_old_off` moves a slot from one event to another by first publishing
the OLD event off. A case read as its first announcement (`observed: first`) therefore expects
`(old ID, OFF)` in the round that moves the slot, not the state the round ends in. The pure path
read it that way; the statechart path judged the end state for every case, so a document that
behaved as the component does failed each such case (measured 2026-10-14: 20 of 270 cases of one
component, all in one position).

Asserted here, through `verify`, against a lamp that shows E1 on an approaching train and E2 on
an occupied crossing:

    a round that moves the slot while the old event was shown has the old event off as its
    first announcement                                               (the discriminator)
    what the slot held may have been left by a SETUP step, not only by a judged case
    a case read as `any` keeps the state the round ends in
    a slot that was already off announces nothing before the new event
    a component that does not announce the old event off is read as the round ends
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import load_pack
from sce_author.verify import verify
from tests.test_a_statechart_is_driven_not_called import CROSSING, codegen_is_built

APPROACH = "plant/in/train-approach"
LAMP = "plant/out/lamp"

MODEL = """\
version: 1
entries:
  - address: plant/in/train-approach
    role: input
    names: [IN_TrainApproach]
    values: {{CLEAR: 0, APPROACHING: 1, OCCUPIED: 2}}

  - address: plant/out/lamp
    role: output
    {announces}
    names: [OUT_Lamp]
    fields:
      ID: {{type: text}}
      Stat: {{values: {{NONE: 0, "OFF": 1, "ON": 2}}}}
"""

# E1 while a train approaches, E2 once it occupies the crossing, off when it clears. The lamp's
# identifier stays what it was when the event goes off, as the product's slots do.
DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="idle"
       sce:kind="statechart">
  <datamodel>
    <data id="lampId" sce:type="string" sce:direction="out" expr="''"/>
    <data id="lampStat" sce:type="string" sce:direction="out" expr="'off'"/>
  </datamodel>
  <state id="idle">
    <transition event="train.approaching" target="approaching"/>
    <transition event="train.occupied" target="occupied"/>
  </state>
  <state id="approaching">
    <onentry><assign location="lampId" expr="'E1'"/><assign location="lampStat" expr="'on'"/></onentry>
    <transition event="train.occupied" target="occupied"/>
    <transition event="train.cleared" target="idle"><assign location="lampStat" expr="'off'"/></transition>
  </state>
  <state id="occupied">
    <onentry><assign location="lampId" expr="'E2'"/><assign location="lampStat" expr="'on'"/></onentry>
    <transition event="train.cleared" target="idle"><assign location="lampStat" expr="'off'"/></transition>
  </state>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "lamp.scxml",
    "activation": "on-change",
    "inputs": {
        "approaching": {"address": APPROACH, "becomes": "APPROACHING", "event": "train.approaching"},
        "occupied": {"address": APPROACH, "becomes": "OCCUPIED", "event": "train.occupied"},
        "cleared": {"address": APPROACH, "becomes": "CLEAR", "event": "train.cleared"},
    },
    "outputs": {
        "lampId": {"address": LAMP, "field": "ID", "passthrough": True},
        "lampStat": {"address": LAMP, "field": "Stat", "map": {"off": "OFF", "on": "ON"}},
    },
}


def step(value: str) -> dict:
    return {"given": {APPROACH: value}, "drove": [APPROACH]}


def case(name: str, value: str, expect_id: str, expect_stat: str, **extra) -> dict:
    return {"name": name, **step(value),
            "expect": {f"{LAMP}.ID": expect_id, f"{LAMP}.Stat": expect_stat}, **extra}


@unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
class AStatechartSlotThatMovesAnnouncesTheOldEventOffFirst(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        shutil.copy(CROSSING / "conventions.yaml", self.tmp / "conventions.yaml")
        conventions = yaml.safe_load((self.tmp / "conventions.yaml").read_text(encoding="utf-8"))
        conventions["host"] = {"writes": "every-round"}
        (self.tmp / "conventions.yaml").write_text(yaml.safe_dump(conventions), encoding="utf-8")
        (self.tmp / "lamp.scxml").write_text(DOCUMENT, encoding="utf-8")
        self.announces = "announces_old_off: true"

    def run_cases(self, cases):
        (self.tmp / "interface-model.yaml").write_text(
            MODEL.format(announces=self.announces), encoding="utf-8")
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "ordered": True, "cases": cases}), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        result = verify(load_pack(self.tmp), path)
        self.assertTrue(result.ran, result.refusal)
        return result

    @staticmethod
    def counts(result):
        return result.passed, result.failed, result.unjudged

    def test_the_round_that_moves_a_shown_slot_has_the_old_event_off_as_its_first_announcement(self):
        result = self.run_cases([
            case("approaching", "APPROACHING", "E1", "ON", observed="first"),
            case("occupied", "OCCUPIED", "E1", "OFF", observed="first"),
        ])

        self.assertEqual((2, 0, 0), self.counts(result),
                         [(r.name, r.failures) for r in result.results])

    def test_the_end_state_is_not_the_first_announcement_of_such_a_round(self):
        result = self.run_cases([
            case("approaching", "APPROACHING", "E1", "ON", observed="first"),
            case("occupied", "OCCUPIED", "E2", "ON", observed="first"),
        ])

        self.assertEqual((1, 1, 0), self.counts(result))
        self.assertEqual("occupied", [r.name for r in result.results if r.failures][0])

    def test_a_setup_step_leaves_the_event_the_judged_round_moves_the_slot_from(self):
        moved = case("occupied", "OCCUPIED", "E1", "OFF", observed="first",
                     before=[step("APPROACHING")])

        self.assertEqual((1, 0, 0), self.counts(self.run_cases([moved])))

    def test_a_case_read_as_any_keeps_the_state_the_round_ends_in(self):
        result = self.run_cases([
            case("approaching", "APPROACHING", "E1", "ON"),
            case("occupied", "OCCUPIED", "E2", "ON"),
        ])

        self.assertEqual((2, 0, 0), self.counts(result))

    def test_a_slot_that_was_already_off_announces_nothing_before_the_new_event(self):
        result = self.run_cases([
            case("approaching", "APPROACHING", "E1", "ON", observed="first"),
            case("cleared", "CLEAR", "E1", "OFF", observed="first"),
            case("occupied", "OCCUPIED", "E2", "ON", observed="first"),
        ])

        self.assertEqual((3, 0, 0), self.counts(result),
                         [(r.name, r.failures) for r in result.results])

    def test_a_component_that_does_not_announce_the_old_event_off_is_read_as_the_round_ends(self):
        self.announces = ""

        result = self.run_cases([
            case("approaching", "APPROACHING", "E1", "ON", observed="first"),
            case("occupied", "OCCUPIED", "E2", "ON", observed="first"),
        ])

        self.assertEqual((2, 0, 0), self.counts(result))


if __name__ == "__main__":
    unittest.main()
