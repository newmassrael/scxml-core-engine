"""A pass is a pass for ONE lowering, and the report has to say which.

The product emits six backends. This verifier drives one of them: Python's
generated form imports into this process and its runtime is a package this
process can reach, so the document becomes an object and driving it is calling
methods. Nothing else here is.

⚠ THAT IS A LIMITATION, AND THE COST OF LEAVING IT UNSAID IS THE POINT. Most
of this product ships as C++. A reader handed `12 passed, 0 failed` finishes
the sentence themselves, in the generous direction, about the thing they are
about to ship. The backend belongs beside the counts for the same reason
`unasserted` does: a figure that leaves its scope to the reader gets the scope
the reader was hoping for.

⚠⚠ And the parity suite does not close the gap, though it looks as if it
should. `native_action_backend_parity` compares what the six emitters WRITE,
byte for byte. How they BEHAVE under these cases is a different claim, and
nothing in this tree makes it.

So a backend this cannot drive is refused rather than silently swapped for the
one it can, and the refusal names the three things that would have to exist.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import load_pack
from sce_author.verify import DRIVEN_BACKENDS, _default_codegen, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
CLOSED = CROSSING / "controller_resolved.binding.yaml"


def codegen_is_built() -> bool:
    return _default_codegen().exists()


class AVerdictNamesTheLoweringItIsAbout(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml",
                     "examples.yaml", "controller_resolved.scxml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        self.pack = load_pack(self.tmp)
        self.binding = self.tmp / "b.yaml"
        self.binding.write_text(CLOSED.read_text(encoding="utf-8"),
                                encoding="utf-8")

    def test_a_backend_it_cannot_drive_is_refused_not_swapped(self):
        """Silently running python instead would answer a question nobody asked."""
        result = verify(self.pack, self.binding, backend="cpp")
        self.assertFalse(result.ran)
        self.assertIn("cpp", result.refusal)

    def test_the_refusal_names_what_would_have_to_exist(self):
        """A refusal that only says no leaves its reader with no next step."""
        refusal = verify(self.pack, self.binding, backend="rust").refusal
        for owed in ("build step", "host program", "wire"):
            self.assertIn(owed, refusal,
                          f"the refusal does not say a {owed} is missing: {refusal}")

    def test_python_is_the_one_it_drives(self):
        """Stated here so a backend added to the set has to come past a test."""
        self.assertEqual({"python"}, set(DRIVEN_BACKENDS))

    @unittest.skipUnless(codegen_is_built(),
                         "the product's generator is not built; nothing runs")
    def test_a_verdict_carries_the_backend_it_was_taken_from(self):
        result = verify(self.pack, self.binding)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual("python", result.backend,
                         "a verdict that does not name its lowering is read "
                         "as one about whichever lowering the reader ships")


if __name__ == "__main__":
    unittest.main()
