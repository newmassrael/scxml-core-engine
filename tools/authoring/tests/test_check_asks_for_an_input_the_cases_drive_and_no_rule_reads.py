"""`check` asks for an input the product's cases drive, the specification names, and no rule reads.

Measured 2026-10-04 over 386 stored documents: on one component every document read none of
eighteen inputs its specification names and the product's cases drive, and said so only at
`verify` -- "no input the binding reads changed, so the host ran nothing" -- which the writer does
not have. Those documents failed 140 cases on average where the rest failed 11.

Three things must all hold, so that a refusal is not a guess, and each is a case below:

  driven      the pack's cases set the address (the product does something with it)
  named       the specification names it (the writer had it in front of them)
  unread      no rule of the binding reads it

and the check stays quiet where one of them is missing, which is most of the time.
"""

from __future__ import annotations

import copy
import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import check
from sce_author.pack import load_pack
from sce_author.prose import load_prose

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "crossing"
ASKED = "reads none of"


class AnUnreadDrivenInput(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.dir = pathlib.Path(self._tmp.name)
        for name in FIXTURE.iterdir():
            if name.is_file():
                shutil.copy(name, self.dir / name.name)
        self.binding = self.dir / "controller.binding.yaml"

    def pack(self):
        return load_pack(self.dir)

    def prose(self, extra: str = ""):
        spec = self.dir / "specification.md"
        if extra:
            spec.write_text(spec.read_text(encoding="utf-8") + "\n" + extra + "\n", encoding="utf-8")
        return load_prose([spec])

    def edit_binding(self, change):
        doc = yaml.safe_load(self.binding.read_text(encoding="utf-8"))
        change(doc)
        self.binding.write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")

    def edit_examples(self, change):
        path = self.dir / "examples.yaml"
        doc = yaml.safe_load(path.read_text(encoding="utf-8"))
        change(doc)
        path.write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")

    def asked(self, prose=None):
        return [f for f in check(self.pack(), self.binding, self.prose() if prose is None else prose)
                if ASKED in f.detail]

    # ------------------------------------------------------------ the quiet case

    def test_the_shipped_pair_asks_nothing(self):
        """The reference binding reads every input the cases drive and the prose names."""
        self.assertEqual([], self.asked())

    # ----------------------------------------------------------- all three hold

    def drop_rule_reading(self, address):
        def change(doc):
            doc["inputs"] = {k: v for k, v in doc["inputs"].items() if v.get("address") != address}
        self.edit_binding(change)

    def test_a_driven_named_unread_input_is_asked_for_by_address_and_name(self):
        self.drop_rule_reading("plant/in/obstacle")
        found = self.asked()
        self.assertEqual(1, len(found), [str(f) for f in found])
        self.assertIn("plant/in/obstacle", found[0].detail)
        self.assertIn("IN_ObstacleDetector", found[0].detail)
        self.assertIn("reads none of 1 input(s)", found[0].detail)

    def test_the_finding_says_what_it_costs_and_how_to_answer_it(self):
        self.drop_rule_reading("plant/in/obstacle")
        said = self.asked()[0].detail
        self.assertIn("a case that drives one of these runs nothing", said)
        self.assertIn("infrastructure", said)

    def test_it_is_one_finding_however_many_inputs_are_unread(self):
        for address in ("plant/in/obstacle", "plant/in/barrier-position"):
            self.drop_rule_reading(address)
        found = self.asked()
        self.assertEqual(1, len(found))
        self.assertIn("reads none of 2 input(s)", found[0].detail)
        for address in ("plant/in/obstacle", "plant/in/barrier-position"):
            self.assertIn(address, found[0].detail)

    def test_a_long_list_is_cut_and_says_how_many_more(self):
        many = {"version": 1}
        # nine more inputs the cases drive and the prose names, none read
        def grow_model():
            path = self.dir / "interface-model.yaml"
            doc = yaml.safe_load(path.read_text(encoding="utf-8"))
            for n in range(9):
                doc["entries"].append({"address": f"plant/in/extra{n}", "role": "input",
                                       "names": [f"IN_Extra{n}"], "values": {"NONE": 0, "A": 1}})
            path.write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")
        grow_model()
        self.edit_examples(lambda d: [c["given"].update({f"plant/in/extra{n}": "A" for n in range(9)})
                                      for c in d["cases"]])
        found = self.asked(self.prose("IN_Extra0 IN_Extra1 IN_Extra2 IN_Extra3 IN_Extra4 IN_Extra5 "
                                      "IN_Extra6 IN_Extra7 IN_Extra8 are mentioned."))
        self.assertEqual(1, len(found))
        self.assertIn("reads none of 9 input(s)", found[0].detail)
        self.assertIn("and 1 more", found[0].detail)
        self.assertNotIn("plant/in/extra8", found[0].detail)

    # ------------------------------------------------------- one of the three is missing

    def test_an_input_the_cases_never_drive_is_not_asked_for(self):
        """No evidence the product reads it: the cases never set it."""
        self.drop_rule_reading("plant/in/obstacle")
        self.edit_examples(lambda d: [c["given"].pop("plant/in/obstacle", None) for c in d["cases"]])
        self.assertEqual([], self.asked())

    def test_an_input_the_specification_never_names_is_not_asked_for(self):
        """The writer did not have it in front of them: that is `questions`' business, not a refusal."""
        self.drop_rule_reading("plant/in/obstacle")
        spec = self.dir / "specification.md"
        spec.write_text(spec.read_text(encoding="utf-8").replace("IN_ObstacleDetector", "the detector"),
                        encoding="utf-8")
        self.assertEqual([], self.asked(load_prose([spec])))

    def test_an_input_a_rule_reads_under_any_name_is_not_asked_for(self):
        def rename(doc):
            doc["inputs"]["somethingElse"] = doc["inputs"].pop("obstacleNone")
        self.edit_binding(rename)
        self.assertEqual([], self.asked())

    def test_plumbing_the_pack_declares_is_not_asked_for(self):
        self.drop_rule_reading("plant/in/obstacle")
        path = self.dir / "conventions.yaml"
        doc = yaml.safe_load(path.read_text(encoding="utf-8"))
        doc["infrastructure"] = ["plant/in/obstacle"]
        path.write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")
        self.assertEqual([], self.asked())

    def test_a_pack_with_no_cases_asks_nothing(self):
        self.drop_rule_reading("plant/in/obstacle")
        (self.dir / "examples.yaml").unlink()
        self.assertEqual([], self.asked())

    def test_it_is_asked_only_when_the_specification_is_handed_over(self):
        """Without prose there is no 'named', so nothing is asked, exactly as for preconditions."""
        self.drop_rule_reading("plant/in/obstacle")
        self.assertEqual([], [f for f in check(self.pack(), self.binding) if ASKED in f.detail])

    def test_an_address_read_through_a_protocol_parameter_counts_as_read(self):
        """`addresses_of` is the one answer to what a rule reads, parameters included."""
        def via_protocol(doc):
            doc["inputs"]["viaProtocol"] = {"protocol": "p", "parameters": {"x": "plant/in/obstacle"}}
        self.drop_rule_reading("plant/in/obstacle")
        self.edit_binding(via_protocol)
        # the pack declares no protocol `p`, so `check` has other things to say; only OUR finding is asserted
        self.assertEqual([], self.asked())


if __name__ == "__main__":
    unittest.main()
