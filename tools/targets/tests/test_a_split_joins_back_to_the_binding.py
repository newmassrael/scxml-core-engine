"""Splitting a binding loses nothing: the two files rebuild what the core reads as the original.

The binding is read through the core's own reader, so what has to survive is what the core
sees -- `map: platform` already expanded -- and not the bytes of the file. A key the split does
not place is refused, never dropped, because a dropped key is one the rebuilt binding would
silently lack.

Every refusal is paired with the nearest binding that must not be refused.
"""

from __future__ import annotations

import shutil
import unittest

import yaml

from tests.support import Working


class TheTwoFilesRebuildTheBinding(Working):
    def test_the_round_trip_reads_as_the_original(self):
        done = self.run_module("sce_targets.split", "roundtrip", self.binding, self.pack, self.spec,
                               self.root / "scratch")
        self.assertEqual(0, done.returncode, done.stdout + done.stderr)
        self.assertIn("roundtrip same", done.stdout)

    def test_the_pair_moves_together(self):
        """The interface names its document relative to itself, so a split moved with its pack
        still joins: no path of the machine that split it is in the file."""
        self.split(self.pack / "out")
        moved = self.root / "moved"
        shutil.copytree(self.pack, moved)
        joined = self.root / "joined" / "pump.binding.yaml"
        done = self.run_module("sce_targets.split", "join",
                               moved / "out" / "logical" / "pump.interface.yaml",
                               moved / "out" / "profile" / "pump.yaml", joined)
        self.assertEqual(0, done.returncode, done.stderr)
        rebuilt = yaml.safe_load(joined.read_text(encoding="utf-8"))
        self.assertTrue((joined.parent / rebuilt["document"]).resolve().samefile(moved / "pump.scxml"),
                        rebuilt["document"])


class AKeyNoFileCouldHoldIsRefused(Working):
    def refusal_after(self, edit) -> str:
        binding = yaml.safe_load(self.binding.read_text(encoding="utf-8"))
        edit(binding)
        self.binding.write_text(yaml.safe_dump(binding), encoding="utf-8")
        done = self.run_module("sce_targets.split", "split", self.binding, self.pack, self.spec,
                               self.root / "split")
        self.assertEqual(3, done.returncode, done.stdout + done.stderr)
        return done.stderr

    def test_the_binding_as_it_is_splits(self):
        """The discriminator: a split that refuses every binding passes every case below."""
        self.split()

    def test_an_output_key_no_file_places(self):
        """`hold_last` is a key binding.v1 admits and neither file has a place for: keeping what
        a position held is a rule about a platform's record, not about the document."""
        said = self.refusal_after(lambda b: b["outputs"]["pump"].update(hold_last=True))
        self.assertIn("keys not placed", said)
        self.assertIn("hold_last", said)

    def test_a_binding_the_core_itself_refuses_is_a_refusal_not_a_crash(self):
        said = self.refusal_after(lambda b: b["inputs"]["floatHigh"].update(hysteresis=3))
        self.assertIn("hysteresis", said)
        self.assertNotIn("Traceback", said)

    def test_an_output_that_is_neither_mapped_nor_passed_nor_internal(self):
        said = self.refusal_after(lambda b: b["outputs"]["fill"].pop("passthrough"))
        self.assertNotIn("Traceback", said)


if __name__ == "__main__":
    unittest.main()
