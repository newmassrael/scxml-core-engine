"""The standalone library is judged by the same examples, and means what `verify` means.

`verify` runs the document against the pack's examples and says which cases fail. A library
built from the logical interface alone, replaying the same examples, has to fail the same cases:
a target that passes where `verify` fails (or the other way round) reads the interface
differently from the core, and nothing downstream could tell.

Both directions are measured. A document that is right passes everywhere, and a document made
wrong in one place fails the same case in both -- a parity of passes alone is the parity of two
things that judge nothing.

Needs the product's generator and a C++ compiler; without either the cases are skipped, and the
lane that builds them refuses to run without them.
"""

from __future__ import annotations

import unittest

from sce_author.pack import load_pack
from sce_author.verify import verify

from tests.support import Working, codegen_is_built, compiler


def verify_says(case_pack, binding) -> dict:
    """What `verify` says of each case: its name -> whether it passed."""
    result = verify(load_pack(case_pack), binding)
    assert not result.refusal, result.refusal
    return {r.name: not r.failures and not r.refusal for r in result.results}


@unittest.skipUnless(codegen_is_built(), "the product's code generator is not built in this tree")
@unittest.skipUnless(compiler(), "no C++ compiler")
class TheLibraryAgreesWithVerify(Working):
    def test_a_right_document_passes_every_case_in_both(self):
        ran = self.built_and_replayed(self.split())
        self.assertEqual(0, ran["rc"], ran["log"])
        self.assertEqual(verify_says(self.pack, self.binding), ran["cases"])
        self.assertEqual(8, len(ran["cases"]))
        self.assertTrue(all(ran["cases"].values()))

    def test_a_document_made_wrong_fails_the_same_case_in_both(self):
        document = self.pack / "pump.scxml"
        text = document.read_text(encoding="utf-8")
        wrong = text.replace("level &lt; 20", "level &lt; 10")
        self.assertNotEqual(text, wrong, "the mutation must change the document")
        document.write_text(wrong, encoding="utf-8")

        said = verify_says(self.pack, self.binding)
        ran = self.built_and_replayed(self.split())
        self.assertEqual(said, ran["cases"])
        self.assertEqual(1, ran["rc"])
        failed = sorted(name for name, ok in ran["cases"].items() if not ok)
        self.assertEqual(["automatic, low level, float low"], failed)

    def test_a_silent_signal_reads_as_the_binding_says(self):
        """`when_absent` and `range` are the two ways a number or a closed set that is not
        reporting reaches the document; both are examples of this pack."""
        ran = self.built_and_replayed(self.split())
        for name in ("a level outside its range reads as 0", "a silent mode reads as off",
                     "a silent float is not high"):
            self.assertTrue(ran["cases"][name], name)


if __name__ == "__main__":
    unittest.main()
