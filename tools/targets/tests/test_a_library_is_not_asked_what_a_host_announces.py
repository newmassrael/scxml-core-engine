"""A library is not asked what only a host publishes.

A component that `announces_old_off` has an event slot the host moves in order: when the slot goes
from one event to another, the old event is published off first. A case read as the first
announcement (`observed: first`) then expects (old identifier, OFF), and that is not the state the
round ends in. A library built from the logical interface has no host; it can show only the state
a round ends in, so it failed such a case where `verify` passed it (four cases of one real
component, in a corpus of 236).

The replay therefore does not ask the library for the identifier and the status of such a slot
when the record expects the status OFF: that is the one reading where the first announcement and
the end state can differ. A status ON is still asked, because a slot that ends ON and announces
something else first would have been announced off, and the record would say OFF.

What is left is sound, not sharp: the library can fail only cases `verify` fails, and a document
made wrong where only the host's reading shows it is caught by `verify` alone.

Needs the product's generator and a C++ compiler; without either the cases are skipped, and the
lane that builds them refuses to run without them.
"""

from __future__ import annotations

import re
import unittest

from sce_author.pack import load_pack
from sce_author.verify import verify

from tests.support import BEACON_FIXTURE, Working, codegen_is_built, compiler


class Beacon(Working):
    fixture = BEACON_FIXTURE
    component = "beacon"

    def said(self) -> dict:
        result = verify(load_pack(self.pack), self.binding)
        self.assertFalse(result.refusal, result.refusal)
        return {r.name: not r.failures and not r.refusal for r in result.results}

    def asked_of_the_library(self) -> dict:
        """Case name -> the `expect` lines the replay script holds for it."""
        blocks, name = {}, None
        for line in (self.root / "replay.txt").read_text(encoding="utf-8").splitlines():
            if line.startswith("case 1 "):
                name = line[7:]
                blocks[name] = []
            elif line.startswith("case 0 "):
                name = None
            elif line.startswith("expect") and name is not None:
                blocks[name].append(line)
        return blocks


@unittest.skipUnless(codegen_is_built(), "the product's code generator is not built in this tree")
@unittest.skipUnless(compiler(), "no C++ compiler")
class TheLibraryIsAskedOnlyWhatItCanShow(Beacon):
    def test_the_record_passes_in_verify_and_in_the_library(self):
        said = self.said()
        self.assertEqual(4, len(said))
        self.assertTrue(all(said.values()), said)
        ran = self.built_and_replayed(self.split())
        self.assertEqual(0, ran["rc"], ran["log"])
        self.assertTrue(all(ran["cases"].values()), ran["log"])

    def test_the_host_reading_of_an_off_slot_is_not_asked(self):
        self.built_and_replayed(self.split())
        asked = self.asked_of_the_library()
        self.assertEqual(
            {"the side changes while it is lit": [], "the west beacon is lit": []},
            {n: l for n, l in asked.items() if not l})
        # The same slot, read where it ends ON, is asked: both of its positions.
        self.assertEqual(2, len(asked["the beacon lights on the east side"]))
        self.assertEqual(2, len(asked["and lit again"]))

    def test_a_document_made_wrong_is_still_failed_where_the_library_can_see_it(self):
        document = self.pack / "beacon.scxml"
        text = document.read_text(encoding="utf-8")
        wrong = text.replace("'K1'", "'K9'")
        self.assertNotEqual(text, wrong, "the mutation must change the document")
        document.write_text(wrong, encoding="utf-8")

        said = self.said()
        ran = self.built_and_replayed(self.split())
        self.assertEqual(1, ran["rc"])
        failed_in_library = {n for n, ok in ran["cases"].items() if not ok}
        failed_in_verify = {n for n, ok in said.items() if not ok}
        self.assertEqual({"the beacon lights on the east side"}, failed_in_library)
        # Sound: never a failure `verify` does not also report. Not sharp: `verify` also fails the
        # case that reads the first announcement, which only a host publishes.
        self.assertLessEqual(failed_in_library, failed_in_verify)
        self.assertIn("the side changes while it is lit", failed_in_verify - failed_in_library)


if __name__ == "__main__":
    unittest.main()
