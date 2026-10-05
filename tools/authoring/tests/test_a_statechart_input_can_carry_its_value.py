"""A statechart is handed a VALUE as the data of the event its input rule sends.

A statechart is driven by events, and until `carries` a binding could send only
the news that an address changed, never what it changed to. A component that counts
time AND reads a number -- a stored total the clock adds seconds to -- had no
surface: a `transform` is handed values and has no clock, a statechart has a clock
and was handed no values. The only way left was one rule per value, `becomes: "0"`
through `becomes: "99"`, each with an event of its own: a hundred rules for a stored
hour, and the writers who were given the stored totals (twelve addresses) left them
unconnected and wrote that a statechart cannot receive them.

Asserted here:

  check     a rule that `carries` is held to the event-schema the document imports:
            the schema must exist for the rule's event, have the field, and the field
            must be a type that a number, a truth value or a text can fill; and the
            address must be readable as that type
  verify    the value reaches `_event.data.<field>` as the field's type -- a case
            that drives 7 and then 33 leaves the machine holding 7 and then 33, which
            the same rule sent bare could not
  control   the same machine driven by a bare rule never learns the value, so the
            cases above pass because the value arrived and for no other reason
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
       version="1.0" sce:kind="event-schema" name="schema_stored"
       sce:event-name="stored.set">
  <datamodel>
    <data id="value" sce:type="uint16" sce:direction="in"/>
  </datamodel>
</scxml>
"""

# `stored.set` fills `total` with the value it carries; `bump` adds one. Neither
# is a guess about time: what is measured is that a NUMBER reaches the machine.
METER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="sce-static" initial="running" sce:kind="statechart"
       name="meter">
  <sce:import kind="event-schema" src="schema_stored.scxml" as="Stored"/>
  <datamodel>
    <data id="total" sce:type="uint16" sce:direction="out" expr="0"/>
  </datamodel>
  <state id="running">
    <transition event="stored.set" type="internal">
      <assign location="total" expr="_event.data.value"/>
    </transition>
    <transition event="bump" type="internal">
      <assign location="total" expr="total + 1"/>
    </transition>
  </state>
</scxml>
"""

MODEL = {
    "version": 1,
    "entries": [
        {"address": "plant/in/stored", "role": "input", "names": ["IN_Stored"],
         "type": "integer", "range": {"minimum": 0, "maximum": 99}},
        {"address": "plant/in/label", "role": "input", "names": ["IN_Label"],
         "type": "text"},
        {"address": "plant/in/bump", "role": "input", "names": ["IN_Bump"],
         "values": {"IDLE": 0, "PRESS": 1}},
        {"address": "plant/out/total", "role": "output", "names": ["OUT_Total"],
         "fields": {"value": {"type": "integer", "range": {"minimum": 0}}}},
    ],
}

CONVENTIONS = {
    "version": 1,
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
        {"name": "a stored total of 7",
         "given": {"plant/in/stored": 7},
         "drove": ["plant/in/stored"],
         "expect": {"plant/out/total.value": 7}},
        {"name": "a stored total of 33",
         "given": {"plant/in/stored": 33},
         "drove": ["plant/in/stored"],
         "expect": {"plant/out/total.value": 33}},
        {"name": "one more",
         "given": {"plant/in/stored": 33, "plant/in/bump": "PRESS"},
         "drove": ["plant/in/bump"],
         "expect": {"plant/out/total.value": 34}},
    ],
}

BINDING = {
    "version": 1,
    "document": "meter.scxml",
    "inputs": {
        "stored": {"address": "plant/in/stored", "event": "stored.set",
                   "carries": "value"},
        "bump": {"address": "plant/in/bump", "becomes": "PRESS", "event": "bump"},
    },
    "outputs": {
        "total": {"address": "plant/out/total", "field": "value",
                  "passthrough": True},
    },
}


def codegen_is_built() -> bool:
    return _default_codegen().exists()


class Fixture(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        (self.tmp / "schema_stored.scxml").write_text(SCHEMA, encoding="utf-8")
        (self.tmp / "meter.scxml").write_text(METER, encoding="utf-8")
        for name, body in (("interface-model.yaml", MODEL),
                           ("conventions.yaml", CONVENTIONS),
                           ("examples.yaml", EXAMPLES)):
            (self.tmp / name).write_text(yaml.safe_dump(body), encoding="utf-8")

    def binding_path(self, binding) -> pathlib.Path:
        path = self.tmp / "meter.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return path

    def findings(self, binding) -> list[str]:
        pack = load_pack(self.tmp)
        return [f"{f.where}: {f.message}" if hasattr(f, "message") else str(f)
                for f in check(pack, self.binding_path(binding))]


class CheckHoldsACarryingRuleToTheSchema(Fixture):
    def test_a_rule_that_carries_a_declared_field_is_not_refused(self):
        self.assertEqual([], [f for f in self.findings(BINDING) if "carries" in f],
                         self.findings(BINDING))

    def test_a_field_the_schema_does_not_have(self):
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["stored"]["carries"] = "total"
        found = self.findings(binding)
        self.assertTrue(any("no such field" in f and "value" in f for f in found), found)

    def test_an_event_with_no_schema(self):
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["stored"]["event"] = "unrelated"
        found = self.findings(binding)
        self.assertTrue(any("imports no event-schema" in f for f in found), found)

    def test_a_text_address_into_a_number_field(self):
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["stored"]["address"] = "plant/in/label"
        found = self.findings(binding)
        self.assertTrue(any("a text" in f and "uint16" in f for f in found), found)

    def test_a_key_without_a_driver_is_still_refused(self):
        """The control for `carries` having been added to the keys the driver
        reads: a key it does NOT read is refused as before."""
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["stored"]["equals"] = "7"
        found = self.findings(binding)
        self.assertTrue(any("`equals` computes a value" in f for f in found), found)


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class AValueReachesTheMachine(Fixture):
    def verified(self, binding):
        return verify(load_pack(self.tmp), self.binding_path(binding))

    def test_the_value_arrives_as_the_fields_type(self):
        result = self.verified(BINDING)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((3, 0, 0), (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_sent_bare_the_machine_never_learns_it(self):
        """The control. Without `carries` the event arrives and the field does
        not, so the machine reads the value it was declared with and the
        first case cannot pass -- which is what makes the test above mean
        the value arrived."""
        binding = copy.deepcopy(BINDING)
        del binding["inputs"]["stored"]["carries"]
        result = self.verified(binding)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(0, result.passed, [(r.name, r.refusal, r.failures)
                                            for r in result.results])

    def test_a_value_the_field_cannot_hold_is_refused_by_name(self):
        examples = copy.deepcopy(EXAMPLES)
        examples["cases"][0]["given"]["plant/in/stored"] = 70000
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump(examples), encoding="utf-8")
        model = copy.deepcopy(MODEL)
        model["entries"][0]["range"] = {"minimum": 0, "maximum": 100000}
        (self.tmp / "interface-model.yaml").write_text(yaml.safe_dump(model), encoding="utf-8")
        result = self.verified(BINDING)
        first = result.results[0]
        self.assertFalse(first.failures == [] and not first.refusal and not first.undetermined,
                         "a 70000 into a uint16 passed")


if __name__ == "__main__":
    unittest.main()
