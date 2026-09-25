"""A run leaves nothing behind -- not on disk, and not in the interpreter.

`verify` writes the document's generated lowering, and for a document with open
values a copy of the document, and both used to stay in the system temp
directory after the run. A document is often somebody's specification in
executable form; measured 2026-09-25, 3,313 such directories had piled up on
one machine in a day. And a long-lived caller -- the MCP server answers every
request in one process -- also kept one `sys.path` root and one package per run
it had ever made.
"""

from __future__ import annotations

import pathlib
import shutil
import sys
import tempfile
import unittest

from sce_author.pack import load_pack
from sce_author.verify import VerifyError, verify

HERE = pathlib.Path(__file__).resolve().parent
PACK = HERE / "fixtures" / "crossing"


def codegen_is_built() -> bool:
    from sce_author.verify import _default_codegen
    return _default_codegen().exists()


class ARunLeavesNothingBehind(unittest.TestCase):
    def setUp(self):
        # The system temp directory is shared; the run is pointed at its own,
        # so "nothing left" can be counted rather than guessed.
        self.temp = pathlib.Path(tempfile.mkdtemp(prefix="sce_leave_test_"))
        self.saved = tempfile.tempdir
        tempfile.tempdir = str(self.temp)
        self.path_before = list(sys.path)
        self.modules_before = set(sys.modules)

    def tearDown(self):
        tempfile.tempdir = self.saved
        shutil.rmtree(self.temp, ignore_errors=True)

    def assertNothingLeft(self):
        self.assertEqual([], sorted(p.name for p in self.temp.iterdir()))
        self.assertEqual([], [p for p in sys.path if p not in self.path_before
                              and str(self.temp) in p])
        stray = [k for k in set(sys.modules) - self.modules_before
                 if str(self.temp) in str(getattr(sys.modules[k], "__file__", "") or "")]
        self.assertEqual([], stray)

    @unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
    def test_a_judged_run_leaves_nothing(self):
        result = verify(load_pack(PACK), PACK / "controller.binding.yaml")
        self.assertTrue(result.ran, result.refusal)
        self.assertGreater(result.passed, 0)
        self.assertNothingLeft()

    @unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
    def test_a_run_with_open_values_leaves_no_copy_of_the_document(self):
        """The resolved-placeholder copy of the document is the part most
        worth not leaving: it is the specification itself."""
        result = verify(load_pack(PACK), PACK / "controller_resolved.binding.yaml")
        self.assertTrue(result.ran, result.refusal)
        self.assertNothingLeft()

    def test_a_run_that_stops_early_leaves_nothing(self):
        with self.assertRaises(VerifyError):
            verify(load_pack(PACK), PACK / "controller.binding.yaml",
                   codegen=self.temp / "no-such-generator")
        self.assertNothingLeft()


if __name__ == "__main__":
    unittest.main()
