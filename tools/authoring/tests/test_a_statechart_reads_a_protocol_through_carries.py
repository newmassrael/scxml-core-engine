"""A statechart reads what a pack's protocol says, as the data of the event its rule sends.

A pack defines a reading over several addresses -- a supply that is on when its on-counter
moved last -- and a computation reads it by naming the `protocol`. A statechart is handed
only events, so a rule of its own that named a `protocol` was refused as computing a value
nobody receives, and a component that has to wait AND read such a reading (a window counted
from the moment the supply came on) had no way to say so. A writer given exactly that
component spent a whole run between the refusals and never reached a document that passed.

A `protocol` that `carries` is read: the driver sends the rule's event whenever a case moves
one of the protocol's addresses, with the protocol's truth value -- what a computation reading
the same protocol would be handed -- as the event's data.

Asserted here:

  check     `protocol` beside `carries` is not refused; the field it fills must be `bool`; a
            `protocol` with no `carries` is refused as before, and the refusal says how to
            hand the machine the value
  verify    the value arrives as the protocol reads it: on after the on-counter moved, off
            after the off-counter did, on again after the on-counter moves again
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

SCHEMA = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_supply"
       sce:event-name="supply.set">
  <datamodel>
    <data id="value" sce:type="bool" sce:direction="in"/>
  </datamodel>
</scxml>
"""

LAMP = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="sce-static" initial="running" sce:kind="statechart"
       name="lamp">
  <sce:import kind="event-schema" src="schema_supply.scxml" as="Supply"/>
  <datamodel>
    <data id="lit" sce:type="int32" sce:direction="out" expr="1"/>
  </datamodel>
  <state id="running">
    <transition event="supply.set" type="internal">
      <assign location="lit" expr="_event.data.value ? 2 : 1"/>
    </transition>
  </state>
</scxml>
"""

MODEL = {
    "version": 1,
    "entries": [
        {"address": "plant/on", "role": "input", "names": ["IN_On"], "type": "integer",
         "range": {"minimum": 0, "maximum": 9}},
        {"address": "plant/off", "role": "input", "names": ["IN_Off"], "type": "integer",
         "range": {"minimum": 0, "maximum": 9}},
        {"address": "plant/out/lamp", "role": "output", "names": ["OUT_Lamp"],
         "fields": {"value": {"type": "integer", "range": {"minimum": 0}}}},
    ],
}

CONVENTIONS = {
    "version": 1,
    "name_classes": [
        {"pattern": r"\bIN_[A-Za-z0-9_]+\b", "role": "supplied"},
        {"pattern": r"\bOUT_[A-Za-z0-9_]+\b", "role": "own_output"},
    ],
    "protocols": {
        "last-incremented": {
            "parameters": ["on_counter", "off_counter"],
            "latch": {"set_when_changed": "on_counter", "clear_when_changed": "off_counter",
                      "both": "last", "initial": "clear"},
        },
    },
}

EXAMPLES = {
    "version": 1,
    "origin": "written for this test",
    "independent_cases": True,
    "ordered": True,
    "cases": [
        {"name": "the supply comes on",
         "given": {"plant/on": 1, "plant/off": 0},
         "drove": ["plant/on"],
         "expect": {"plant/out/lamp.value": 2}},
        {"name": "the supply goes off",
         "given": {"plant/on": 1, "plant/off": 1},
         "drove": ["plant/off"],
         "expect": {"plant/out/lamp.value": 1}},
        {"name": "the supply comes on again",
         "given": {"plant/on": 2, "plant/off": 1},
         "drove": ["plant/on"],
         "expect": {"plant/out/lamp.value": 2}},
    ],
}

BINDING = {
    "version": 1,
    "document": "lamp.scxml",
    "inputs": {
        "supply": {"protocol": "last-incremented",
                   "parameters": {"on_counter": "plant/on", "off_counter": "plant/off"},
                   "event": "supply.set", "carries": "value"},
    },
    "outputs": {
        "lit": {"address": "plant/out/lamp", "field": "value", "passthrough": True},
    },
}


class Fixture(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        (self.tmp / "schema_supply.scxml").write_text(SCHEMA, encoding="utf-8")
        (self.tmp / "lamp.scxml").write_text(LAMP, encoding="utf-8")
        for name, body in (("interface-model.yaml", MODEL),
                           ("conventions.yaml", CONVENTIONS),
                           ("examples.yaml", EXAMPLES)):
            (self.tmp / name).write_text(yaml.safe_dump(body), encoding="utf-8")

    def binding_path(self, binding) -> pathlib.Path:
        path = self.tmp / "lamp.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return path

    def findings(self, binding) -> list[str]:
        return [str(f) for f in check(load_pack(self.tmp), self.binding_path(binding))]


class CheckReadsAProtocolThatCarries(Fixture):
    def test_a_protocol_that_carries_a_bool_is_not_refused(self):
        self.assertEqual([], self.findings(BINDING))

    def test_a_protocol_with_no_carries_is_refused_and_says_how_to_hand_the_value(self):
        binding = copy.deepcopy(BINDING)
        del binding["inputs"]["supply"]["carries"]
        found = self.findings(binding)
        refusal = [f for f in found if "compute a value" in f]
        self.assertTrue(refusal, found)
        self.assertIn("`carries: <field>`", refusal[0])
        self.assertIn("beside `protocol`", refusal[0])

    def test_the_field_a_protocol_fills_must_be_a_truth_value(self):
        (self.tmp / "schema_supply.scxml").write_text(
            SCHEMA.replace('sce:type="bool"', 'sce:type="int32"'), encoding="utf-8")
        found = self.findings(BINDING)
        self.assertTrue(any("declare it `bool`" in f for f in found), found)

    def test_a_key_the_driver_does_not_read_is_still_refused_beside_a_protocol(self):
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["supply"]["equals"] = "1"
        found = self.findings(binding)
        self.assertTrue(any("`equals` computes a value" in f for f in found), found)


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's generator is not built; nothing can be run")
class AProtocolsValueReachesTheMachine(Fixture):
    def verified(self, binding):
        return verify(load_pack(self.tmp), self.binding_path(binding))

    def test_the_value_arrives_as_the_protocol_reads_it(self):
        result = self.verified(BINDING)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((3, 0, 0), (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_a_case_that_expects_the_other_value_fails(self):
        """The control: the second case passes only because `false` arrived."""
        examples = copy.deepcopy(EXAMPLES)
        examples["cases"][1]["expect"] = {"plant/out/lamp.value": 2}
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump(examples), encoding="utf-8")
        result = self.verified(BINDING)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(1, result.failed, [(r.name, r.refusal, r.failures)
                                            for r in result.results])


if __name__ == "__main__":
    unittest.main()
