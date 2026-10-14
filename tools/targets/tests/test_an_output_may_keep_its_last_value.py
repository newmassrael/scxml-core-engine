"""An output may keep what it last announced, and a library keeps it the same way `verify` does.

Some outputs are a slot that is written only when the document produces one of a few values; in
between, the slot keeps the last one. That is what the output does whatever the platform, so the
logical interface says it (`holds_last`) and the library keeps the value: a value outside `values`
is no announcement, not an error.

Before there is a last value the slot holds nothing of the component's: a record that expects a
value there describes what the platform's own slot held, which is not the library's to show. It is
counted (`unwritten`), not failed, as `verify` leaves such a position unjudged.

Needs the product's generator and a C++ compiler; without either the library cases are skipped,
and the lane that builds them refuses to run without them.
"""

from __future__ import annotations

import copy
import re
import unittest

import yaml
from jsonschema import Draft202012Validator
from sce_author.pack import load_pack
from sce_author.verify import verify

from tests.support import LATCH_FIXTURE, Working, codegen_is_built, compiler


class Latch(Working):
    fixture = LATCH_FIXTURE
    component = "latch"

    def failed_in_verify(self) -> set:
        result = verify(load_pack(self.pack), self.binding)
        self.assertFalse(result.refusal, result.refusal)
        return {r.name for r in result.results if r.failures}


class TheKeepingIsPlacedInTheInterface(Latch):
    def test_the_interface_says_the_output_keeps_its_last_value(self):
        interface = self.interface(self.split())
        self.assertEqual({"values": ["HIGH", "LOW"], "holds_last": True,
                          "assumed": "the slot keeps what it last said between the two bands"},
                         interface["outputs"]["alarm"])
        self.assertEqual([], list(Draft202012Validator(self.schema()).iter_errors(interface)))

    def test_a_split_joins_back_to_the_binding(self):
        split_dir = self.split()
        joined = self.pack / "joined.binding.yaml"
        done = self.run_module("sce_targets.split", "join", split_dir / "logical" / "latch.interface.yaml",
                               split_dir / "profile" / "latch.yaml", joined)
        self.assertEqual(0, done.returncode, done.stderr)
        self.assertEqual(yaml.safe_load(self.binding.read_text(encoding="utf-8")),
                         yaml.safe_load(joined.read_text(encoding="utf-8")))

    def test_the_rest_of_the_platforms_record_is_the_profiles(self):
        split_dir = self.split()
        text = (split_dir / "logical" / "latch.interface.yaml").read_text(encoding="utf-8")
        for platform_only in ("Tag", "Note", "ALARM", "over", "also:", "\n    when:"):
            self.assertNotIn(platform_only, text)
        profile = yaml.safe_load((split_dir / "profile" / "latch.yaml").read_text(encoding="utf-8"))
        self.assertEqual({"Tag": "ALARM"}, profile["outputs"]["alarm"]["also"])
        self.assertEqual({"HIGH": {"Note": "over"}}, profile["outputs"]["alarm"]["when"])

    def test_the_schema_refuses_keeping_what_has_no_values(self):
        base = self.interface(self.split())
        validator = Draft202012Validator(self.schema())
        self.assertEqual([], list(validator.iter_errors(base)), "the discriminator")
        passed_through = copy.deepcopy(base)
        passed_through["outputs"]["alarm"] = {"passthrough": True, "holds_last": True}
        self.assertTrue(list(validator.iter_errors(passed_through)))


@unittest.skipUnless(codegen_is_built(), "the product's code generator is not built in this tree")
@unittest.skipUnless(compiler(), "no C++ compiler")
class TheLibraryKeepsTheValue(Latch):
    def test_it_passes_where_verify_passes_and_counts_what_was_never_said(self):
        self.assertEqual(set(), self.failed_in_verify())
        ran = self.built_and_replayed(self.split())
        self.assertEqual(0, ran["rc"], ran["log"])
        self.assertEqual(5, len(ran["cases"]))
        self.assertTrue(all(ran["cases"].values()), ran["log"])
        self.assertIn("unmapped 0", ran["log"])
        self.assertIn("unwritten 1", ran["log"], "the case before anything was said")
        script = (self.root / "replay.txt").read_text(encoding="utf-8")
        self.assertIn("expect alarm HIGH", script)
        self.assertNotIn("ALARM", script, "the platform's record is not asked of a library")
        self.assertNotIn("over", script)

    def test_without_the_keeping_the_same_values_are_unmapped(self):
        """The control: it is `holds_last` that makes a value outside `values` no error."""
        split_dir = self.split()
        path = split_dir / "logical" / "latch.interface.yaml"
        interface = yaml.safe_load(path.read_text(encoding="utf-8"))
        del interface["outputs"]["alarm"]["holds_last"]
        path.write_text(yaml.safe_dump(interface, sort_keys=False), encoding="utf-8")
        ran = self.built_and_replayed(split_dir)
        self.assertEqual(1, ran["rc"])
        self.assertTrue(re.search(r"unmapped [1-9]", ran["log"]), ran["log"])

    def test_a_document_made_wrong_fails_the_same_cases_in_both(self):
        document = self.pack / "latch.scxml"
        text = document.read_text(encoding="utf-8")
        wrong = text.replace("'HIGH' : (level", "'LOW' : (level")
        self.assertNotEqual(text, wrong, "the mutation must change the document")
        document.write_text(wrong, encoding="utf-8")

        said = self.failed_in_verify()
        ran = self.built_and_replayed(self.split())
        self.assertEqual(1, ran["rc"])
        self.assertTrue(said, "the mutation must be one a case can see")
        self.assertEqual(said, {n for n, ok in ran["cases"].items() if not ok})


if __name__ == "__main__":
    unittest.main()
