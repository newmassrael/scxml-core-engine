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
from sce_author.verify import received_by, unreceived_drives, verify

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
        case = Case("", {}, {}, None, ("plant/in/obstacle", ELSEWHERE, "plant/out/bell"))
        self.assertEqual(sorted([ELSEWHERE, "plant/out/bell"]), unreceived_drives(case, model))
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


if __name__ == "__main__":
    unittest.main()
