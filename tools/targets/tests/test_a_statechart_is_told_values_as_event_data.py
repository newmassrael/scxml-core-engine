"""A statechart that is told a value, not only that something changed, is carried as a transform is.

A statechart is driven by events. An input rule with `carries` sends its event with the signal's
value as the event's data, and one with a `protocol` sends the state the pack reads over several
addresses (a supply that is on when its on-counter moved last). The first belongs in the logical
interface, because the specification names the value; the second names a state there and keeps the
counters in the profile, because the counters are a platform's encoding.

The made-up gauge is told a stored number and a supply, and says HIGH or LOW for the first and LIT or
DARK for the second.

Needs the product's generator, a C++ compiler and the runtime sources for the cases that build a
library; without them those cases are skipped, and the lane that builds them refuses to run without.
"""

from __future__ import annotations

import copy
import unittest

import yaml
from jsonschema import Draft202012Validator
from sce_author.pack import load_pack
from sce_author.verify import verify

from tests.support import (GAUGE_FIXTURE, Working, codegen_is_built, compiler,
                           runtime_is_present)


class Gauge(Working):
    fixture = GAUGE_FIXTURE
    component = "gauge"

    def failed_in_verify(self) -> set:
        result = verify(load_pack(self.pack), self.binding)
        self.assertFalse(result.refusal, result.refusal)
        return {r.name for r in result.results if r.failures or r.refusal}


class TheValuesAreSplit(Gauge):
    def test_the_interface_names_the_value_and_the_state_and_no_counter(self):
        split_dir = self.split()
        interface = self.interface(split_dir)
        self.assertEqual([], list(Draft202012Validator(self.schema()).iter_errors(interface)))
        self.assertEqual({"signal": "IN_Stored", "event": "stored.set", "carries": "value"},
                         interface["inputs"]["stored"])
        self.assertEqual({"logical": True, "event": "supply.set", "carries": "value"},
                         interface["inputs"]["supply"])
        self.assertEqual({"IN_Stored": {"type": "number"}}, interface["signals"])
        text = (split_dir / "logical" / "gauge.interface.yaml").read_text(encoding="utf-8")
        for platform_only in ("plant/", "on_counter", "off_counter", "last-incremented"):
            self.assertNotIn(platform_only, text)

    def test_the_counters_are_the_profiles(self):
        profile = yaml.safe_load((self.split() / "profile" / "gauge.yaml").read_text(encoding="utf-8"))
        self.assertEqual({"protocol": "last-incremented",
                          "parameters": {"on_counter": "plant/in/on", "off_counter": "plant/in/off"}},
                         profile["protocols"]["supply"])
        self.assertEqual("plant/in/stored", profile["signals"]["IN_Stored"]["address"])

    def test_a_split_joins_back_to_the_binding(self):
        split_dir = self.split()
        joined = self.pack / "joined.binding.yaml"
        done = self.run_module("sce_targets.split", "join", split_dir / "logical" / "gauge.interface.yaml",
                               split_dir / "profile" / "gauge.yaml", joined)
        self.assertEqual(0, done.returncode, done.stderr)
        self.assertEqual(yaml.safe_load(self.binding.read_text(encoding="utf-8")),
                         yaml.safe_load(joined.read_text(encoding="utf-8")))

    def test_the_schema_refuses_what_the_platform_owns_here_too(self):
        base = self.interface(self.split())
        validator = Draft202012Validator(self.schema())
        self.assertEqual([], list(validator.iter_errors(base)), "the discriminator")
        counted = copy.deepcopy(base)
        counted["inputs"]["supply"]["protocol"] = "last-incremented"
        self.assertTrue(list(validator.iter_errors(counted)), "a counter protocol in the interface")
        half = copy.deepcopy(base)
        del half["inputs"]["supply"]["carries"]
        self.assertTrue(list(validator.iter_errors(half)), "an event with nothing it carries")


@unittest.skipUnless(codegen_is_built(), "the product's code generator is not built in this tree")
@unittest.skipUnless(compiler(), "no C++ compiler")
@unittest.skipUnless(runtime_is_present(), "the runtime sources and headers are not in this tree")
class TheLibraryTellsTheMachineTheValues(Gauge):
    def test_it_passes_where_verify_passes(self):
        self.assertEqual(set(), self.failed_in_verify())
        ran = self.built_and_replayed(self.split())
        self.assertEqual(0, ran["rc"], ran["log"])
        self.assertEqual(5, len(ran["cases"]))
        self.assertTrue(all(ran["cases"].values()), ran["log"])

    def test_a_stored_number_made_wrong_fails_the_same_cases_in_both(self):
        self.mutate("_event.data.value &gt; 50", "_event.data.value &gt; 80")
        said = self.failed_in_verify()
        self.assertEqual({"a stored number above the threshold"}, said)
        ran = self.built_and_replayed(self.split())
        self.assertEqual(said, {n for n, ok in ran["cases"].items() if not ok})

    def test_a_supply_made_wrong_fails_the_same_cases_in_both(self):
        self.mutate('event="indicator.lit"', 'event="indicator.dark"')
        said = self.failed_in_verify()
        self.assertEqual({"the supply comes on", "the supply comes on again"}, said)
        ran = self.built_and_replayed(self.split())
        self.assertEqual(said, {n for n, ok in ran["cases"].items() if not ok})

    def mutate(self, before: str, after: str) -> None:
        document = self.pack / "gauge.scxml"
        text = document.read_text(encoding="utf-8")
        wrong = text.replace(before, after)
        self.assertNotEqual(text, wrong, "the mutation must change the document")
        document.write_text(wrong, encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
