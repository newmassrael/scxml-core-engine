"""A statechart can be told that a signal is not reporting.

A signal that times out is not a value of its address: the platform flags it and the
component's callback runs with nothing to read. A transform reads that with `absent` and
`when_absent`; a statechart is handed only events, and its input rule read `event`,
`address`, `becomes` and `carries` and nothing else, so `check` refused both keys and
`verify` refused to judge a case that drives the silence unless the rule said what it reads
as (`when_absent`). No binding could satisfy both, and every case that drives a timeout at a
statechart was left unjudged: 32 cases of one component, the timeout case of two more.

Two shapes, both asserted here:

  `absent: true`            the rule's event is sent when the address is not reporting; the
                            rules that read a value are not told of the silence
  `carries` + `when_absent` the event's data takes the value `when_absent` names when the
                            address is not reporting, as a transform's input would read it

  check     `when_absent` without `carries` and `absent` beside a value key are refused,
            `absent: true` with an event and `carries` with `when_absent` are not
  verify    a case that drives the absence token reaches the machine as the event the
            binding names, and as the value it names, and no other rule fires for it
  control   without the rule that reads the silence the case is not judged a pass, so the
            passes above mean the silence arrived
"""

from __future__ import annotations

import copy
import pathlib
import tempfile
import unittest

import yaml

from sce_author.check import check
from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, verify

LEVEL_SCHEMA = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_level"
       sce:event-name="level.set">
  <datamodel>
    <data id="value" sce:type="uint16" sce:direction="in"/>
  </datamodel>
</scxml>
"""

WATCH = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="sce-static" initial="running" sce:kind="statechart"
       name="watch">
  <sce:import kind="event-schema" src="schema_level.scxml" as="Level"/>
  <datamodel>
    <data id="total" sce:type="uint16" sce:direction="out" expr="0"/>
    <data id="lost" sce:type="uint8" sce:direction="out" expr="0"/>
  </datamodel>
  <state id="running">
    <transition event="level.set" type="internal">
      <assign location="total" expr="_event.data.value"/>
    </transition>
    <transition event="link.lost" type="internal">
      <assign location="lost" expr="1"/>
    </transition>
    <transition event="link.up" type="internal">
      <assign location="lost" expr="0"/>
    </transition>
  </state>
</scxml>
"""

MODEL = {
    "version": 1,
    "entries": [
        {"address": "plant/in/level", "role": "input", "names": ["IN_Level"],
         "type": "integer", "range": {"minimum": 0, "maximum": 99}},
        {"address": "plant/in/link", "role": "input", "names": ["IN_Link"],
         "values": {"DOWN": 0, "UP": 1}},
        {"address": "plant/out/total", "role": "output", "names": ["OUT_Total"],
         "fields": {"value": {"type": "integer", "range": {"minimum": 0}}}},
        {"address": "plant/out/lost", "role": "output", "names": ["OUT_Lost"],
         "fields": {"value": {"type": "integer", "range": {"minimum": 0}}}},
    ],
}

CONVENTIONS = {
    "version": 1,
    "absence_tokens": ["timeout"],
    "name_classes": [
        {"pattern": r"\bIN_[A-Za-z0-9_]+\b", "role": "supplied"},
        {"pattern": r"\bOUT_[A-Za-z0-9_]+\b", "role": "own_output"},
    ],
}

EXAMPLES = {
    "version": 1,
    "origin": "written for this test",
    "independent_cases": True,
    "ordered": True,
    "cases": [
        {"name": "a level of 7",
         "given": {"plant/in/level": 7},
         "drove": ["plant/in/level"],
         "expect": {"plant/out/total.value": 7}},
        {"name": "the level times out",
         "given": {"plant/in/level": "timeout"},
         "drove": ["plant/in/level"],
         "expect": {"plant/out/total.value": 255}},
        {"name": "the link times out",
         "given": {"plant/in/link": "timeout"},
         "drove": ["plant/in/link"],
         "expect": {"plant/out/lost.value": 1}},
        {"name": "the link is up again",
         "given": {"plant/in/link": "UP"},
         "drove": ["plant/in/link"],
         "expect": {"plant/out/lost.value": 0}},
    ],
}

BINDING = {
    "version": 1,
    "document": "watch.scxml",
    "inputs": {
        "level": {"address": "plant/in/level", "event": "level.set",
                  "carries": "value", "when_absent": 255},
        "linkSilent": {"address": "plant/in/link", "event": "link.lost", "absent": True},
        "linkUp": {"address": "plant/in/link", "event": "link.up", "becomes": "UP"},
    },
    "outputs": {
        "total": {"address": "plant/out/total", "field": "value", "passthrough": True},
        "lost": {"address": "plant/out/lost", "field": "value", "passthrough": True},
    },
}


def codegen_is_built() -> bool:
    return _default_codegen().exists()


class Fixture(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        (self.tmp / "schema_level.scxml").write_text(LEVEL_SCHEMA, encoding="utf-8")
        (self.tmp / "watch.scxml").write_text(WATCH, encoding="utf-8")
        for name, body in (("interface-model.yaml", MODEL),
                           ("conventions.yaml", CONVENTIONS),
                           ("examples.yaml", EXAMPLES)):
            (self.tmp / name).write_text(yaml.safe_dump(body), encoding="utf-8")

    def binding_path(self, binding) -> pathlib.Path:
        path = self.tmp / "watch.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return path

    def findings(self, binding) -> list[str]:
        pack = load_pack(self.tmp)
        return [f"{f.where}: {f.message}" if hasattr(f, "message") else str(f)
                for f in check(pack, self.binding_path(binding))]


class CheckReadsTheSilenceKeys(Fixture):
    def test_a_rule_that_reads_the_silence_is_not_refused(self):
        found = [f for f in self.findings(BINDING)
                 if "when_absent" in f or "absent" in f]
        self.assertEqual([], found, self.findings(BINDING))

    def test_when_absent_without_carries_is_refused(self):
        binding = copy.deepcopy(BINDING)
        del binding["inputs"]["level"]["carries"]
        found = self.findings(binding)
        self.assertTrue(any("`when_absent`" in f and "`carries`" in f for f in found), found)

    def test_absent_beside_a_value_key_is_refused(self):
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["linkSilent"]["becomes"] = "DOWN"
        found = self.findings(binding)
        self.assertTrue(any("`absent: true`" in f and "`becomes`" in f for f in found), found)

    def test_absent_that_is_not_true_is_refused(self):
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["linkSilent"]["absent"] = False
        found = self.findings(binding)
        self.assertTrue(any("`absent`" in f and "`true`" in f for f in found), found)

    def test_a_key_without_a_driver_is_still_refused(self):
        """The control for the keys having been added to the ones the driver reads."""
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["level"]["equals"] = "7"
        found = self.findings(binding)
        self.assertTrue(any("`equals` computes a value" in f for f in found), found)


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class TheSilenceReachesTheMachine(Fixture):
    def verified(self, binding):
        return verify(load_pack(self.tmp), self.binding_path(binding))

    def outcome(self, result):
        return [(r.name, bool(r.failures), bool(r.refusal)) for r in result.results]

    def test_both_shapes_reach_the_machine(self):
        result = self.verified(BINDING)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((4, 0, 0), (result.passed, result.failed, result.unjudged),
                         self.outcome(result))

    def test_without_the_rule_that_reads_the_silence_the_link_case_is_not_a_pass(self):
        """The control. The only rule left for the link reads a value, and a silent address
        is not one: no event reaches the machine and the case cannot pass."""
        binding = copy.deepcopy(BINDING)
        del binding["inputs"]["linkSilent"]
        result = self.verified(binding)
        names = {r.name: r for r in result.results}
        silent = names.get("the link times out")
        self.assertIsNotNone(silent, self.outcome(result))
        self.assertFalse(silent.passed, self.outcome(result))

    def test_without_when_absent_the_level_case_is_not_judged(self):
        """The control for the other shape: the carried value has no reading for silence."""
        binding = copy.deepcopy(BINDING)
        del binding["inputs"]["level"]["when_absent"]
        result = self.verified(binding)
        names = {r.name: r for r in result.results}
        silent = names.get("the level times out")
        self.assertIsNotNone(silent, self.outcome(result))
        self.assertFalse(silent.passed, self.outcome(result))

    def test_a_value_does_not_fire_the_rule_that_reads_the_silence(self):
        """`UP` is a value: the silence rule stays quiet, so `lost` goes back to 0."""
        result = self.verified(BINDING)
        names = {r.name: r for r in result.results}
        self.assertTrue(names["the link is up again"].passed, self.outcome(result))


if __name__ == "__main__":
    unittest.main()
