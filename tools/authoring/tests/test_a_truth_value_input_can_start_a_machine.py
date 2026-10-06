"""An input that is a truth value can be what starts a statechart's work.

A host can tell a component that it has been powered on through a signal that is a truth
value, not an enumeration. The specification writes a value for some outputs at that
moment, and the host's own test drives the signal and expects the output. A rule that names
`becomes` for such an address has no value space to be read against, so what it names is
the spelling the record gives (`True`), compared as the record's value is compared
everywhere else.

  check     the rule is not refused for an address with no value space
  verify    the case that drives the address at `True` reaches the machine as the rule's event,
            and the case that drives it at `False` does not
  control   a rule that waits for `False` does not start the work, so the pass above means
            the event arrived
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

POWER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="sce-static" initial="off" sce:kind="statechart"
       name="power">
  <datamodel>
    <data id="applied" sce:type="uint8" sce:direction="out" expr="0"/>
  </datamodel>
  <state id="off">
    <transition event="power.on" target="on">
      <assign location="applied" expr="1"/>
    </transition>
  </state>
  <state id="on"/>
</scxml>
"""

MODEL = {
    "version": 1,
    "entries": [
        {"address": "plant/in/power", "role": "input", "names": ["IN_Power"], "type": "boolean"},
        {"address": "plant/out/applied", "role": "output", "names": ["OUT_Applied"],
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
        {"name": "the power is reported off",
         "given": {"plant/in/power": "False"},
         "drove": ["plant/in/power"],
         "delivered": ["plant/in/power"],
         "expect": {"plant/out/applied.value": 0}},
        {"name": "the power comes on",
         "given": {"plant/in/power": "True"},
         "drove": ["plant/in/power"],
         "delivered": ["plant/in/power"],
         "expect": {"plant/out/applied.value": 1}},
    ],
}

BINDING = {
    "version": 1,
    "document": "power.scxml",
    "inputs": {
        "powerOn": {"address": "plant/in/power", "event": "power.on", "becomes": "True"},
    },
    "outputs": {
        "applied": {"address": "plant/out/applied", "field": "value", "passthrough": True},
    },
}


def codegen_is_built() -> bool:
    return _default_codegen().exists()


class Fixture(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        (self.tmp / "power.scxml").write_text(POWER, encoding="utf-8")
        for name, body in (("interface-model.yaml", MODEL),
                           ("conventions.yaml", CONVENTIONS),
                           ("examples.yaml", EXAMPLES)):
            (self.tmp / name).write_text(yaml.safe_dump(body), encoding="utf-8")

    def binding_path(self, binding) -> pathlib.Path:
        path = self.tmp / "power.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return path

    def findings(self, binding) -> list[str]:
        pack = load_pack(self.tmp)
        return [f"{f.where}: {f.message}" if hasattr(f, "message") else str(f)
                for f in check(pack, self.binding_path(binding))]


class CheckAdmitsBecomesOnATruthValue(Fixture):
    def test_the_rule_is_not_refused(self):
        found = [f for f in self.findings(BINDING) if "powerOn" in f or "plant/in/power" in f]
        self.assertEqual([], found, self.findings(BINDING))


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class TheTruthValueReachesTheMachine(Fixture):
    def verified(self, binding):
        return verify(load_pack(self.tmp), self.binding_path(binding))

    def outcome(self, result):
        return [(r.name, bool(r.failures), r.refusal) for r in result.results]

    def test_the_case_at_true_starts_the_work_and_the_case_at_false_sends_nothing(self):
        """`True` fires the rule and the machine does the work. `False` fires nothing, and a
        case that drove an address no rule turned into an event is withheld, not passed."""
        result = self.verified(BINDING)
        self.assertTrue(result.ran, result.refusal)
        by_name = {r.name: r for r in result.results}
        self.assertTrue(by_name["the power comes on"].passed, self.outcome(result))
        off = by_name["the power is reported off"]
        self.assertFalse(off.passed, self.outcome(result))
        self.assertIn("no input rule turns any of that into an event", off.refusal)
        self.assertEqual((1, 0, 1), (result.passed, result.failed, result.unjudged),
                         self.outcome(result))

    def test_a_rule_that_waits_for_false_does_not_start_the_work(self):
        """The control. Alone, the case at `True` meets a rule that names `False`: no event
        reaches the machine, the output stays 0, and the case is not a pass."""
        examples = copy.deepcopy(EXAMPLES)
        examples["cases"] = examples["cases"][1:]
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump(examples), encoding="utf-8")
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["powerOn"]["becomes"] = "False"
        result = self.verified(binding)
        names = {r.name: r for r in result.results}
        started = names.get("the power comes on")
        self.assertIsNotNone(started, self.outcome(result))
        self.assertFalse(started.passed, self.outcome(result))


if __name__ == "__main__":
    unittest.main()
