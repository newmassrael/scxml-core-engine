"""`check` names an input the pack's cases leave unset, before `verify` refuses them.

Measured 2026-09-26: a 43-input document passed `check` with no refusal and
then had every one of its 196 cases refused by `verify` -- three inputs read
as-is were unset in most cases, and a case with nothing there has no safe
reading. The writer, judged afterwards, never saw `verify`. Which addresses
each case sets is not an answer, so an authoring pack may carry the cases
with their values withheld; this is asked of those as well.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import check
from sce_author.delivery import owes_absence
from sce_author.pack import load_pack

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "crossing"
# Set by 4 of the fixture's 5 cases.
PARTLY_SET = "plant/in/barrier-position"


class OwesAbsence(unittest.TestCase):
    def test_an_as_is_read_with_nothing_said_owes_it(self):
        self.assertTrue(owes_absence({"address": "a"}))
        self.assertTrue(owes_absence({"address": "a", "range": [0, 9]}))

    def test_a_reading_that_answers_absence_itself_does_not(self):
        for rule in ({"address": "a", "when_absent": 0},
                     {"address": "a", "equals": "ON"},
                     {"address": "a", "equals_any": ["ON"]},
                     {"address": "a", "not_equals": "ON"},
                     {"address": "a", "absent": True},
                     {"address": "a", "event": "e"},
                     {"protocol": "p", "parameters": {}},
                     {"clock": True}):
            self.assertFalse(owes_absence(rule), rule)


class ACaseThatSetsNothingThere(unittest.TestCase):
    def setUp(self):
        self.dir = pathlib.Path(tempfile.mkdtemp(prefix="sce_unset_input_"))
        self.pack_dir = self.dir / "pack"
        shutil.copytree(FIXTURE, self.pack_dir)
        self.binding = self.pack_dir / "controller.binding.yaml"

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def bind(self, rule):
        doc = yaml.safe_load(self.binding.read_text(encoding="utf-8"))
        doc["inputs"]["barrierRaw"] = rule
        self.binding.write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")

    def unset(self):
        return [f.detail for f in check(load_pack(self.pack_dir), self.binding)
                if "set nothing at" in f.detail]

    def withhold_values(self):
        path = self.pack_dir / "examples.yaml"
        doc = yaml.safe_load(path.read_text(encoding="utf-8"))
        for case in doc["cases"]:
            case["given"] = {k: None for k in case["given"]}
            case["expect"] = {k: None for k in case.get("expect") or {}}
        path.write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")

    def test_an_as_is_read_of_an_address_one_case_leaves_unset_is_named(self):
        self.bind({"address": PARTLY_SET})
        found = self.unset()
        self.assertEqual(1, len(found), found)
        self.assertIn("1 of 5 case(s)", found[0])
        self.assertIn("when_absent", found[0])

    def test_saying_what_nothing_reads_as_answers_it(self):
        self.bind({"address": PARTLY_SET, "when_absent": 0})
        self.assertEqual([], self.unset())

    def test_a_comparison_answers_it_itself(self):
        self.bind({"address": PARTLY_SET, "equals": "DOWN"})
        self.assertEqual([], self.unset())

    def test_cases_with_their_values_withheld_say_the_same(self):
        self.withhold_values()
        self.bind({"address": PARTLY_SET})
        found = self.unset()
        self.assertEqual(1, len(found), found)
        self.assertIn("1 of 5 case(s)", found[0])


if __name__ == "__main__":
    unittest.main()
