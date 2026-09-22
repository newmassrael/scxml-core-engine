"""A case that says "from this, move to that" has to state the "from".

A record from a platform's tests often has two parts: a precondition that puts
the inputs somewhere, then the input the case is about. The examples format
kept only where a case ENDED, so a pack built from such records folded the
precondition into the final values and the "from" was gone.

⚠ What that cost, measured on the first document written against such a pack:
the specification said "UNLOCK becomes LOCK", a document reading that
literally -- a transition out of UNLOCK and nowhere else -- was never shown the
UNLOCK and failed, while a document that read it as "not LOCK, then LOCK"
passed. The loose reading was the wrong one, and the only thing the examples
could reward was it.

So a case carries `before`: steps driven in order ahead of it, moving whatever
a round moves -- a machine's state, a remembered previous value -- and never
judged. Asserted here:

  reading    the steps are loaded, and what they drive counts as driven
  a machine  a literal transition passes WITH its setup and fails without it,
             and what the setup sent is not what the case is judged on
  a history  a previous value comes from the setup step, and a setup step that
             cannot be driven leaves the case unjudged, naming the step
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import load_examples, load_pack
from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import BINDING as FIXTURE_BINDING
from tests.test_refusals_actually_fire import Fixture

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

# The road signal flashes when a train approaches a crossing that was CLEAR --
# a transition out of one named state and no other. Before anything has been
# reported the machine is in neither.
LITERAL = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="unknown" sce:kind="statechart">
  <state id="unknown">
    <transition event="train.cleared" target="clear"/>
  </state>
  <state id="clear">
    <transition event="train.approaching" target="approaching">
      <send event="signal.flashing" type="x-sce-host"/>
    </transition>
  </state>
  <state id="approaching">
    <transition event="train.cleared" target="clear">
      <send event="signal.dark" type="x-sce-host"/>
    </transition>
  </state>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "signal.scxml",
    "inputs": {
        "approaching": {"address": "plant/in/train-approach",
                        "becomes": "APPROACHING", "event": "train.approaching"},
        "cleared": {"address": "plant/in/train-approach",
                    "becomes": "CLEAR", "event": "train.cleared"},
    },
    "outputs": {
        "roadSignal": {
            "address": "plant/out/road-signal", "field": "value",
            "sent": {"processor": "x-sce-host"},
            "when_nothing_sent": "signal.dark",
            "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"},
        },
    },
}


def examples(*cases):
    return {"version": 1, "origin": "written for this test",
            "independent_cases": True, "ordered": True, "cases": list(cases)}


APPROACH = {"name": "a train approaches",
            "given": {"plant/in/train-approach": "APPROACHING"},
            "drove": ["plant/in/train-approach"],
            "expect": {"plant/out/road-signal.value": "FLASHING"}}
FROM_CLEAR = [{"given": {"plant/in/train-approach": "CLEAR"},
               "drove": ["plant/in/train-approach"]}]


class TheStepsAreRead(unittest.TestCase):
    def test_a_step_is_a_case_with_nothing_expected_of_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "examples.yaml"
            path.write_text(yaml.safe_dump(examples(
                {**APPROACH, "before": FROM_CLEAR})), encoding="utf-8")
            got = load_examples([path])
        case = got.cases[0]
        self.assertEqual(1, len(case.before))
        step = case.before[0]
        self.assertEqual({"plant/in/train-approach": "CLEAR"}, step.given)
        self.assertEqual(("plant/in/train-approach",), step.drove)
        self.assertEqual({}, step.expect)
        self.assertIn("(before 1)", step.name)
        self.assertEqual(1, got.case_count, "a step is not a case of its own")

    def test_what_a_step_drives_counts_as_driven(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "examples.yaml"
            path.write_text(yaml.safe_dump(examples(
                {**APPROACH, "before": [{"given": {"plant/in/obstacle": "NONE"},
                                         "drove": ["plant/in/obstacle"]}]})),
                encoding="utf-8")
            got = load_examples([path])
        self.assertIn("plant/in/obstacle", got.driven)


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AMachineIsSetUpBeforeItIsJudged(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(LITERAL, encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        self.binding = path

    def run_with(self, *cases):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump(examples(*cases)), encoding="utf-8")
        return verify(load_pack(self.tmp), self.binding)

    def counts(self, result):
        return (result.passed, result.failed, result.unjudged)

    def test_a_literal_transition_fails_without_its_setup(self):
        """The discriminator. Without it the case below proves nothing: a
        document that flashed on any approach would pass it too."""
        result = self.run_with(APPROACH)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 1, 0), self.counts(result))

    def test_and_passes_with_it(self):
        result = self.run_with({**APPROACH, "before": FROM_CLEAR})
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), self.counts(result),
                         [(r.name, r.refusal, r.failures) for r in result.results])
        self.assertEqual(["a train approaches"], [r.name for r in result.results],
                         "a setup step is never reported as a case")

    def test_what_the_setup_sent_is_not_what_the_case_is_judged_on(self):
        """The setup flashes the signal; the case then drives something that
        sends nothing, so its own round reads the resting value."""
        result = self.run_with({
            "name": "a second approach report changes nothing",
            "before": FROM_CLEAR + [{"given": {"plant/in/train-approach": "APPROACHING"},
                                     "drove": ["plant/in/train-approach"]}],
            "given": {"plant/in/train-approach": "APPROACHING"},
            "drove": ["plant/in/train-approach"],
            "expect": {"plant/out/road-signal.value": "DARK"}})
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), self.counts(result),
                         [(r.name, r.refusal, r.failures) for r in result.results])


# A computation over this round and the last: ON only in the round the supply
# reaches HIGH. What "the last round" is decides everything.
EDGE = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="edge">
  <datamodel>
    <data id="mode" sce:type="bool" sce:direction="in"/>
    <data id="wasMode" sce:type="bool" sce:direction="in"/>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="(mode &amp;&amp; !wasMode &amp;&amp; count &gt;= 0) ? 1 : 0"/>
  </datamodel>
</scxml>
"""


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class APreviousValueComesFromTheSetup(Fixture):
    def run_edge(self, case):
        (self.root / "edge.scxml").write_text(EDGE, encoding="utf-8")
        inputs = {**FIXTURE_BINDING["inputs"],
                  "wasMode": {"previous_of": "mode",
                              "caller_keeps": "the edge is a fact about two rounds"},
                  "count": {"address": "Plant.Input.Count", "number": True}}
        binding = {**FIXTURE_BINDING, "document": "edge.scxml", "inputs": inputs}
        path = self.root / "edge.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        self.write_pack_examples([case])
        return verify(self.pack(), path)

    def write_pack_examples(self, cases):
        (self.pack_dir / "examples.yaml").write_text(
            yaml.safe_dump(examples(*cases)), encoding="utf-8")

    def test_the_edge_is_seen_because_the_setup_held_it_low(self):
        result = self.run_edge({
            "name": "the supply rises",
            "before": [{"given": {"Plant.Input.SupplyMode": 1, "Plant.Input.Count": 1},
                        "drove": ["Plant.Input.SupplyMode"]}],
            "given": {"Plant.Input.SupplyMode": 2, "Plant.Input.Count": 1},
            "drove": ["Plant.Input.SupplyMode"],
            "expect": {"Plant.Out.Lamp.Stat": 2}})
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_a_setup_step_that_cannot_be_driven_names_itself(self):
        """⚠ A number the step never supplies. The case cannot be driven as
        its record states it, and saying which step failed is the only thing
        that sends the reader to the right line."""
        result = self.run_edge({
            "name": "the supply rises",
            "before": [{"given": {"Plant.Input.SupplyMode": 1},
                        "drove": ["Plant.Input.SupplyMode"]}],
            "given": {"Plant.Input.SupplyMode": 2, "Plant.Input.Count": 1},
            "drove": ["Plant.Input.SupplyMode"],
            "expect": {"Plant.Out.Lamp.Stat": 2}})
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 0, 1), (result.passed, result.failed, result.unjudged))
        self.assertIn("(before 1)", result.results[0].refusal)


if __name__ == "__main__":
    unittest.main()
