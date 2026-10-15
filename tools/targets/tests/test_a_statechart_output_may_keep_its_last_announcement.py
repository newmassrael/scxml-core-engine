"""A statechart's output may keep what it last announced, as a transform's may.

Some outputs are a slot written only when the machine sends one of a few events; a send that is no
value of the output (a test pulse), or a round that sends nothing, announces nothing, and the slot
keeps what it had. That is what the output does whatever the platform, so the logical interface
says it (`holds_last`) and the library keeps the value: a send outside `values` is no error, and
`when_nothing_sent` may name a token outside them, which says "nothing is announced".

The other fields of the platform's record for the slot (`also`, `when`) are the profile's, as for a
transform. Before there is a last announcement the slot holds nothing of the machine's: a record
that expects a value there describes the platform's own slot, which is counted (`unwritten`) and
not failed, as `verify` leaves such a position unjudged.

Needs the product's generator, a C++ compiler and the runtime sources for the cases that build a
library; without them those cases are skipped, and the lane that builds them refuses to run without.
"""

from __future__ import annotations

import copy
import re
import unittest

import yaml
from jsonschema import Draft202012Validator
from sce_author.pack import load_pack
from sce_author.verify import verify

from tests.support import (CHIME_FIXTURE, Working, codegen_is_built, compiler,
                           runtime_is_present)


class Chime(Working):
    fixture = CHIME_FIXTURE
    component = "chime"

    def failed_in_verify(self) -> set:
        result = verify(load_pack(self.pack), self.binding)
        self.assertFalse(result.refusal, result.refusal)
        return {r.name for r in result.results if r.failures}


class TheKeepingIsPlacedInTheInterface(Chime):
    def test_the_interface_says_the_output_keeps_its_last_announcement(self):
        split_dir = self.split()
        interface = self.interface(split_dir)
        signal = interface["outputs"]["signal"]
        self.assertTrue(signal["holds_last"])
        self.assertEqual("held", signal["when_nothing_sent"])
        self.assertEqual(["chime.ring", "chime.quiet"], signal["values"])
        self.assertEqual([], list(Draft202012Validator(self.schema()).iter_errors(interface)))

    def test_the_rest_of_the_platforms_record_is_the_profiles(self):
        split_dir = self.split()
        text = (split_dir / "logical" / "chime.interface.yaml").read_text(encoding="utf-8")
        for platform_only in ("Tag", "Note", "t3", "n7", "also:", "\n    when:"):
            self.assertNotIn(platform_only, text)
        profile = yaml.safe_load((split_dir / "profile" / "chime.yaml").read_text(encoding="utf-8"))
        self.assertEqual({"Tag": "t3"}, profile["outputs"]["signal"]["also"])
        self.assertEqual({"chime.ring": {"Note": "n7"}}, profile["outputs"]["signal"]["when"])

    def test_a_split_joins_back_to_the_binding(self):
        split_dir = self.split()
        joined = self.pack / "joined.binding.yaml"
        done = self.run_module("sce_targets.split", "join", split_dir / "logical" / "chime.interface.yaml",
                               split_dir / "profile" / "chime.yaml", joined)
        self.assertEqual(0, done.returncode, done.stderr)
        self.assertEqual(yaml.safe_load(self.binding.read_text(encoding="utf-8")),
                         yaml.safe_load(joined.read_text(encoding="utf-8")))

    def test_the_schema_refuses_keeping_that_is_no_boolean_true(self):
        base = self.interface(self.split())
        validator = Draft202012Validator(self.schema())
        self.assertEqual([], list(validator.iter_errors(base)), "the discriminator")
        wrong = copy.deepcopy(base)
        wrong["outputs"]["signal"]["holds_last"] = "yes"
        self.assertTrue(list(validator.iter_errors(wrong)))


@unittest.skipUnless(codegen_is_built(), "the product's code generator is not built in this tree")
@unittest.skipUnless(compiler(), "no C++ compiler")
@unittest.skipUnless(runtime_is_present(), "the runtime sources and headers are not in this tree")
class TheLibraryKeepsTheAnnouncement(Chime):
    def test_it_passes_where_verify_passes_and_counts_what_was_never_announced(self):
        self.assertEqual(set(), self.failed_in_verify())
        ran = self.built_and_replayed(self.split())
        self.assertEqual(0, ran["rc"], ran["log"])
        self.assertEqual(5, len(ran["cases"]))
        self.assertTrue(all(ran["cases"].values()), ran["log"])
        self.assertIn("unmapped 0", ran["log"])
        self.assertIn("unwritten 1", ran["log"], "the case before anything was announced")
        script = (self.root / "replay.txt").read_text(encoding="utf-8")
        self.assertNotIn("t3", script, "the platform's record is not asked of a library")
        self.assertNotIn("n7", script)

    def test_without_the_keeping_the_library_is_refused(self):
        """The control: it is `holds_last` that lets `held` stand for a round that announces
        nothing, and a send outside the values be no error."""
        split_dir = self.split()
        path = split_dir / "logical" / "chime.interface.yaml"
        interface = yaml.safe_load(path.read_text(encoding="utf-8"))
        del interface["outputs"]["signal"]["holds_last"]
        path.write_text(yaml.safe_dump(interface, sort_keys=False), encoding="utf-8")
        lib = self.root / "refused-lib"
        staged = self.root / "runtime"
        staged.mkdir()
        import shutil
        from sce_author.verify import _default_codegen
        from tests.support import RUNTIME_SOURCES, STANDALONE_TARGET
        for source in RUNTIME_SOURCES:
            shutil.copy2(source, staged / source.name)
        done = self.run_module("sce_targets.standalone", path, STANDALONE_TARGET, _default_codegen(),
                               lib, env_extra={"SCE_RUNTIME_SOURCE_DIR": str(staged)})
        self.assertEqual(3, done.returncode, done.stdout + done.stderr)
        self.assertIn("when_nothing_sent", done.stderr)

    def test_a_document_made_wrong_fails_the_same_cases_in_both(self):
        document = self.pack / "chime.scxml"
        text = document.read_text(encoding="utf-8")
        wrong = text.replace('event="chime.ring"', 'event="chime.quiet"')
        self.assertNotEqual(text, wrong, "the mutation must change the document")
        document.write_text(wrong, encoding="utf-8")

        said = self.failed_in_verify()
        ran = self.built_and_replayed(self.split())
        self.assertEqual(1, ran["rc"])
        self.assertTrue(said, "the mutation must be one a case can see")
        self.assertEqual(said, {n for n, ok in ran["cases"].items() if not ok})
        self.assertFalse(re.search(r"unmapped [1-9]", ran["log"]), ran["log"])


if __name__ == "__main__":
    unittest.main()
