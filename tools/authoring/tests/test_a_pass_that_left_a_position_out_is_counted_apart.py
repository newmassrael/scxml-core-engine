"""A case that passed without looking at every position the record expects is counted apart.

A case is refused only when NOTHING was compared. One whose other position agreed passes, and the
position no rule of the binding writes is a `----` line that nobody reads in a long report. Over a
document that leaves a whole output out (an actuator it was given no address for) that made "every
case passed" true and the component wrong: ten cases of one component, sixteen of another, which
the host's own tests then failed.

  the count     `passed_short` is the passed cases that left an expected position unchecked
  the report    the summary says so beside the counts
  control       a binding that writes both positions has none
"""

from __future__ import annotations

import copy
import pathlib
import tempfile
import unittest

import yaml

from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, verify
from tests.test_a_statechart_can_be_told_a_signal_fell_silent import (
    BINDING, CONVENTIONS, LEVEL_SCHEMA, MODEL, WATCH)

EXAMPLES = {
    "version": 1,
    "origin": "written for this test",
    "independent_cases": True,
    "ordered": True,
    "cases": [
        {"name": "a level of 7, and nothing lost",
         "given": {"plant/in/level": 7},
         "drove": ["plant/in/level"],
         "expect": {"plant/out/total.value": 7, "plant/out/lost.value": 0}},
    ],
}


def codegen_is_built() -> bool:
    return _default_codegen().exists()


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class APassThatLeftAPositionOut(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        (self.tmp / "schema_level.scxml").write_text(LEVEL_SCHEMA, encoding="utf-8")
        (self.tmp / "watch.scxml").write_text(WATCH, encoding="utf-8")
        for name, body in (("interface-model.yaml", MODEL), ("conventions.yaml", CONVENTIONS),
                           ("examples.yaml", EXAMPLES)):
            (self.tmp / name).write_text(yaml.safe_dump(body), encoding="utf-8")

    def verified(self, binding):
        path = self.tmp / "watch.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    def test_the_case_that_left_one_out_passes_and_is_counted_apart(self):
        binding = copy.deepcopy(BINDING)
        del binding["outputs"]["lost"]
        result = self.verified(binding)
        self.assertEqual((1, 0), (result.passed, result.failed), result.refusal)
        self.assertEqual(1, result.passed_short)

    def test_a_binding_that_writes_both_has_none(self):
        """The control: with both positions written nothing is short."""
        result = self.verified(copy.deepcopy(BINDING))
        self.assertEqual((1, 0), (result.passed, result.failed), result.refusal)
        self.assertEqual(0, result.passed_short)


if __name__ == "__main__":
    unittest.main()
