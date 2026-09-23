"""Generated code that does not import is an answer, not a traceback.

`verify` drives the product's python lowering of the document it was given.
When the generator accepts a document and writes python that cannot be
imported, the verifier used to die on the import with a traceback that named
a temporary file and nothing else -- measured 2026-09-23 on a model-written
document with a multi-target transition, rendered `target=State.A B`.

What failed there is the product's lowering, not the document and not the
verifier, and the report has to say so: which file, which line, and whose
defect it is.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest
from unittest import mock

import yaml

from sce_author import verify as verify_module
from sce_author.pack import load_pack
from sce_author.verify import VerifyError, _default_codegen, load, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"


class LoadingWhatWasGenerated(unittest.TestCase):
    def test_a_syntax_error_names_the_file_the_line_and_the_generator(self):
        with tempfile.TemporaryDirectory() as tmp:
            into = pathlib.Path(tmp) / "built"
            into.mkdir()
            (into / "signal_sm.py").write_text(
                "x = 1\ntarget = State.A B\n", encoding="utf-8")
            with self.assertRaises(VerifyError) as raised:
                load(into, pathlib.Path("signal.scxml"))
        said = str(raised.exception)
        for words in ("signal_sm.py:2", "SyntaxError", "does not import",
                      "defect in the generator"):
            self.assertIn(words, said)

    def test_nothing_written_is_said_as_such(self):
        with tempfile.TemporaryDirectory() as tmp:
            into = pathlib.Path(tmp) / "built"
            into.mkdir()
            with self.assertRaises(VerifyError) as raised:
                load(into, pathlib.Path("signal.scxml"))
        self.assertIn("wrote no python", str(raised.exception))


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's generator is not built; nothing can be run")
class VerifyReportsItInsteadOfDying(unittest.TestCase):
    def test_the_run_is_refused_with_the_loaders_words(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            for name in ("interface-model.yaml", "conventions.yaml"):
                shutil.copy(CROSSING / name, root / name)
            (root / "signal.scxml").write_text(
                """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="null" initial="dark">
  <state id="dark">
    <transition event="train.approaching" target="flashing"/>
  </state>
  <state id="flashing">
    <onentry><send event="signal.flashing" type="x-sce-host"/></onentry>
  </state>
</scxml>
""", encoding="utf-8")
            (root / "examples.yaml").write_text(yaml.safe_dump({
                "version": 1, "origin": "written for this test",
                "independent_cases": True, "ordered": True,
                "cases": [{"name": "a train approaches",
                           "given": {"plant/in/train-approach": "APPROACHING"},
                           "drove": ["plant/in/train-approach"],
                           "expect": {"plant/out/road-signal.value": "FLASHING"}}],
            }), encoding="utf-8")
            binding = root / "b.yaml"
            binding.write_text(yaml.safe_dump({
                "version": 1, "document": "signal.scxml",
                "inputs": {"approaching": {"address": "plant/in/train-approach",
                                           "becomes": "APPROACHING",
                                           "event": "train.approaching"}},
                "outputs": {"roadSignal": {
                    "address": "plant/out/road-signal", "field": "value",
                    "sent": {"processor": "x-sce-host"},
                    "when_nothing_sent": "signal.dark",
                    "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"}}},
            }), encoding="utf-8")
            said = "the product generated python for signal.scxml that does not import"
            with mock.patch.object(verify_module, "load", side_effect=VerifyError(said)):
                result = verify(load_pack(root), binding)
        self.assertFalse(result.ran)
        self.assertEqual(said, result.refusal)


if __name__ == "__main__":
    unittest.main()
