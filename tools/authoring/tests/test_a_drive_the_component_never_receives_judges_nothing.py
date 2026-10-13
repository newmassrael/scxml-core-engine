"""A case that drives what the component never receives is not judged.

A record is a test of the platform. It may set a value on an upstream
component, whose effect reaches this one only through that component, or on an
address this component simply does not read. That drive never arrives here, so
what the case expects may come from somewhere this document cannot see.
Measured 2026-09-26: such cases were counted as failures -- eleven for one
component that reads what an upstream component computes from what the case
drove, eight for one whose test drives a namespace its own subscriptions do not
contain -- and the document was blamed for a component it is not.

The interface model says what the component receives, so it decides.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import Case, load_pack
from sce_author.verify import (received_by, unaccounted_drives, unreceived_drives,
                               upstream_effects, verify)

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
ELSEWHERE = "plant/elsewhere/upstream-sensor"
BINDING = "controller_resolved.binding.yaml"


def codegen_is_built() -> bool:
    from sce_author.verify import _default_codegen
    return _default_codegen().exists()


class ADriveTheComponentNeverReceivesJudgesNothing(unittest.TestCase):
    def test_what_is_received_is_what_the_model_declares_as_read(self):
        model = load_pack(CROSSING).model
        self.assertTrue(received_by(model, "plant/in/train-approach"))
        self.assertFalse(received_by(model, "plant/out/bell"),
                         "an output is written, not received")
        self.assertFalse(received_by(model, ELSEWHERE))
        # ⚠ A drive of the component's OWN output is not among them: the
        # record is writing the slot, not sending the document anything, and
        # what it leaves there is `Planted`'s to judge
        # (test_a_value_the_record_planted_stands).
        case = Case("", {}, {}, None, ("plant/in/obstacle", ELSEWHERE, "plant/out/bell"))
        self.assertEqual([ELSEWHERE], unreceived_drives(case, model))
        self.assertEqual([], unreceived_drives(case, None), "no model, nothing to ask")

    @unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
    def test_the_case_that_drove_elsewhere_is_withheld_and_the_rest_judged(self):
        pack_dir = pathlib.Path(tempfile.mkdtemp(prefix="sce_unreceived_test_"))
        try:
            for f in CROSSING.iterdir():
                if f.is_file():
                    shutil.copy(f, pack_dir / f.name)
            # Two cases the untouched pack PASSES, so the only thing that can
            # change either verdict is what the record says it drove.
            # The resolved binding: the other one leaves an output open on
            # purpose, and a case resting on it is unjudged for that reason.
            baseline = verify(load_pack(CROSSING), CROSSING / BINDING)
            passing = [r.name for r in baseline.results if r.passed]
            self.assertGreaterEqual(len(passing), 2, "the fixture needs two passing cases")
            examples = yaml.safe_load((CROSSING / "examples.yaml").read_text(encoding="utf-8"))
            by_case = {c["name"]: c for c in examples["cases"]}
            first, second = by_case[passing[0]], by_case[passing[1]]
            first["drove"] = ["plant/in/train-approach", ELSEWHERE]
            second["drove"] = ["plant/in/train-approach"]
            (pack_dir / "examples.yaml").write_text(
                yaml.safe_dump(examples, sort_keys=False), encoding="utf-8")

            result = verify(load_pack(pack_dir), pack_dir / BINDING)
            self.assertTrue(result.ran, result.refusal)
            by_name = {r.name: r for r in result.results}
            withheld = by_name[first["name"]]
            self.assertFalse(withheld.judged)
            self.assertIn(ELSEWHERE, withheld.refusal)
            judged = by_name[second["name"]]
            self.assertTrue(judged.judged, judged.refusal)
            self.assertTrue(judged.passed, judged.failures)
        finally:
            shutil.rmtree(pack_dir, ignore_errors=True)


READS = ["plant/in/train-approach", "plant/in/barrier-position", "plant/in/obstacle",
         "plant/in/mains-power", "plant/in/local-override"]


class AWholeInputThatStatesWhatTheDocumentReadsHidesNothing(unittest.TestCase):
    """A record whose `given` is the whole input says what the platform's component held.

    A drive upstream is then only how the platform got there, and the case can be judged
    -- when it states EVERY address the document reads. One it leaves out may be where the
    expected value comes from, and the drive stays withheld.
    """

    def setUp(self):
        self.model = load_pack(CROSSING).model
        self.stated = {address: "x" for address in READS}

    def case(self, given):
        return Case("", given, {}, None, ("plant/in/train-approach", ELSEWHERE))

    def test_every_read_address_stated_hides_nothing(self):
        self.assertEqual([], unaccounted_drives(self.case(self.stated), self.model, READS, True))

    def test_one_read_address_left_out_keeps_the_drive_hidden(self):
        given = {a: v for a, v in self.stated.items() if a != "plant/in/obstacle"}
        self.assertEqual([ELSEWHERE], unaccounted_drives(self.case(given), self.model, READS, True))

    def test_a_record_that_is_not_the_whole_input_keeps_the_drive_hidden(self):
        self.assertEqual([ELSEWHERE],
                         unaccounted_drives(self.case(self.stated), self.model, READS, False))

    def test_a_case_that_drove_only_what_is_received_has_nothing_to_hide(self):
        case = Case("", self.stated, {}, None, ("plant/in/train-approach",))
        self.assertEqual([], unaccounted_drives(case, self.model, READS, False))

    def test_the_input_an_upstream_drive_moved_is_what_a_machine_is_told(self):
        before = {a: "x" for a in READS}
        after = {**before, "plant/in/obstacle": "y"}
        case = self.case(after)
        self.assertEqual(["plant/in/obstacle"],
                         upstream_effects(case, before, self.model, READS, True))
        self.assertEqual([], upstream_effects(case, {}, self.model, READS, True),
                         "nothing before it: a first value is not shown to have moved")
        self.assertEqual([], upstream_effects(case, before, self.model, READS, False))
        self.assertEqual([], upstream_effects(self.case(before), before, self.model, READS, True),
                         "an input that holds its value did not move")

    @unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
    def test_judged_end_to_end_only_when_the_record_states_everything(self):
        results = {}
        for label, states_override, independent in (("whole", True, True),
                                                    ("one left out", False, True),
                                                    ("not the whole input", True, False)):
            pack_dir = pathlib.Path(tempfile.mkdtemp(prefix="sce_whole_input_test_"))
            try:
                for f in CROSSING.iterdir():
                    if f.is_file():
                        shutil.copy(f, pack_dir / f.name)
                examples = yaml.safe_load((CROSSING / "examples.yaml").read_text(encoding="utf-8"))
                examples["independent_cases"] = independent
                baseline = verify(load_pack(CROSSING), CROSSING / BINDING)
                name = [r.name for r in baseline.results if r.passed][0]
                case = {c["name"]: c for c in examples["cases"]}[name]
                case["drove"] = ["plant/in/train-approach", ELSEWHERE]
                if states_override:
                    case["given"]["plant/in/local-override"] = "OFF"
                (pack_dir / "examples.yaml").write_text(
                    yaml.safe_dump(examples, sort_keys=False), encoding="utf-8")
                got = {r.name: r for r in verify(load_pack(pack_dir), pack_dir / BINDING).results}
                results[label] = got[name]
            finally:
                shutil.rmtree(pack_dir, ignore_errors=True)
        self.assertTrue(results["whole"].judged, results["whole"].refusal)
        self.assertTrue(results["whole"].passed, results["whole"].failures)
        self.assertFalse(results["one left out"].judged)
        self.assertFalse(results["not the whole input"].judged)


if __name__ == "__main__":
    unittest.main()
