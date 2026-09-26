"""A precondition is judged by HOW it is read, and an input by whether anything reads it.

Measured 2026-09-26, the next document after the name-only precondition check
landed: the writer bound the supply input under the right name as a plain
comparison of one of the latch's counters, and passed. The pack now may give
the rule that reads each precondition input (`preconditions.inputs.<name>.rule`),
and `check` holds a binding to it -- a different reading under the name is
refused, the pack's reading under another name satisfies it. The same
document declared an input no expression read, which `verify` then left a
case unjudged over; `check` names that too.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.brief import assemble
from sce_author.check import check
from sce_author.pack import load_pack
from sce_author.prose import load_prose

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "crossing"
RULE = {"address": "plant/in/mains-power", "equals": "OK"}


class APack(unittest.TestCase):
    def setUp(self):
        self.dir = pathlib.Path(tempfile.mkdtemp(prefix="sce_precondition_rule_"))
        self.pack_dir = self.dir / "pack"
        shutil.copytree(FIXTURE, self.pack_dir)
        self.binding = self.pack_dir / "controller.binding.yaml"
        self.document = self.pack_dir / "controller.scxml"
        self.spec = load_prose([self.pack_dir / "specification.md"])

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def give_the_rule(self, rule=RULE):
        path = self.pack_dir / "conventions.yaml"
        conv = yaml.safe_load(path.read_text(encoding="utf-8"))
        conv["preconditions"]["inputs"]["poweredUp"] = {
            "note": "the installation is energised", "rule": rule}
        path.write_text(yaml.safe_dump(conv, sort_keys=False), encoding="utf-8")

    def bind(self, name, rule):
        doc = yaml.safe_load(self.binding.read_text(encoding="utf-8"))
        doc["inputs"][name] = rule
        self.binding.write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")

    def precondition_findings(self):
        return [str(f) for f in check(load_pack(self.pack_dir), self.binding, self.spec)
                if "precondition" in f.detail]


class ThePacksReading(APack):
    def test_the_pack_loads_the_rule_beside_the_note(self):
        self.give_the_rule()
        conv = load_pack(self.pack_dir).conventions
        self.assertEqual("the installation is energised", conv.precondition_inputs["poweredUp"])
        self.assertEqual(RULE, conv.precondition_rules["poweredUp"])

    def test_a_different_reading_under_the_name_is_refused_with_the_packs(self):
        self.give_the_rule()
        self.bind("poweredUp", {"address": "plant/in/mains-power", "equals": "FAIL"})
        found = self.precondition_findings()
        self.assertEqual(1, len(found), found)
        self.assertIn("reads it differently from the pack", found[0])
        self.assertIn("equals: OK", found[0])

    def test_the_packs_reading_under_the_name_is_accepted_whatever_is_noted(self):
        self.give_the_rule()
        self.bind("poweredUp", {**RULE, "note": "as the brief gives it"})
        self.assertEqual([], self.precondition_findings())

    def test_the_packs_reading_under_another_name_satisfies_it(self):
        self.give_the_rule()
        self.bind("energised", dict(RULE))
        self.assertEqual([], self.precondition_findings())

    def test_nothing_reading_it_is_refused_with_the_rule_to_write(self):
        self.give_the_rule()
        found = self.precondition_findings()
        self.assertEqual(1, len(found), found)
        self.assertIn("nothing here reads 'poweredUp'", found[0])
        self.assertIn("The pack reads it as", found[0])

    def test_events_of_an_address_the_reading_watches_are_accepted(self):
        """A statechart takes the condition as events, one per change of an
        address the pack's reading watches -- the same condition, observed
        the way a statechart observes anything."""
        self.give_the_rule()
        self.bind("poweredUp", {"address": "plant/in/mains-power", "event": "mains.change"})
        self.assertEqual([], self.precondition_findings())

    def test_events_of_another_address_are_refused(self):
        self.give_the_rule()
        self.bind("poweredUp", {"address": "plant/in/local-override", "event": "override.change"})
        found = self.precondition_findings()
        self.assertEqual(1, len(found), found)
        self.assertIn("does not watch", found[0])

    def test_without_a_rule_the_name_is_still_the_test(self):
        self.bind("poweredUp", {"address": "plant/in/mains-power", "equals": "FAIL"})
        self.assertEqual([], self.precondition_findings())

    def test_the_brief_tells_the_author_the_rule(self):
        self.give_the_rule()
        text = assemble(self.spec, load_pack(self.pack_dir))
        self.assertIn("bind it as `{address: plant/in/mains-power, equals: OK}`", text)


class AnInputNothingReads(APack):
    def test_the_reference_pair_reads_every_input_it_declares(self):
        found = [str(f) for f in check(load_pack(self.pack_dir), self.binding)
                 if "no expression reads it" in f.detail]
        self.assertEqual([], found)

    def test_a_declared_input_no_expression_reads_is_refused(self):
        text = self.document.read_text(encoding="utf-8")
        self.document.write_text(text.replace(
            "<datamodel>",
            '<datamodel>\n    <data id="spare" sce:type="bool" sce:direction="in"/>', 1),
            encoding="utf-8")
        found = [str(f) for f in check(load_pack(self.pack_dir), self.binding)
                 if "no expression reads it" in f.detail]
        self.assertEqual(1, len(found), found)
        self.assertIn("'spare'", found[0])


if __name__ == "__main__":
    unittest.main()
