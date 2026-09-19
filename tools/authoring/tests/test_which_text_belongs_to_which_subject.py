"""The partition a whole question class is built on.

`gate-off-unstated` asks whether the specification says what an output shows
when its precondition is false, and it can only ask that about the text
belonging to THAT output. So the document is cut into blocks, one per address,
and the cut is made on the names each address goes by.

⚠ THE CUT IS THE CLASS. Give a line to the wrong address and the question is
asked of the wrong paragraph: the off value is found in text about something
else and the gap goes unreported, or it is missed in text that states it and a
question is raised against a specification that answered.

⚠⚠ Nothing pinned this rule until the rule was measured and found to be
deciding ownership by two accidents -- a substring match and dictionary order.
The whole product suite stayed green while 95 findings moved on a 129-pack
corpus, which is what a rule with no test looks like from the inside.
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest

from sce_author.prose import load_prose


class WhichTextBelongsToWhichSubject(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def blocks(self, text: str, universe):
        path = self.tmp / "spec.md"
        path.write_text(text, encoding="utf-8")
        return load_prose([path]).blocks(universe)

    # ------------------------------------------------------ what it carries

    def test_a_subject_owns_the_lines_after_the_one_that_names_it(self):
        """A table puts the other case on a continuation row that never
        repeats the name, and that row is exactly where the off value is."""
        got = self.blocks(
            "Lamp shows ON while the door is open\n"
            "  otherwise OFF\n"
            "Bell rings while a train approaches\n",
            {"a/lamp": ("Lamp",), "a/bell": ("Bell",)},
        )
        self.assertIn("otherwise OFF", got["a/lamp"])
        self.assertNotIn("otherwise OFF", got["a/bell"])

    def test_text_before_any_subject_belongs_to_none_of_them(self):
        got = self.blocks(
            "This document describes the cabin.\n"
            "Lamp shows ON while the door is open\n",
            {"a/lamp": ("Lamp",)},
        )
        self.assertNotIn("describes the cabin", got["a/lamp"])

    # ------------------------------------------- the two accidents, removed

    def test_a_name_is_matched_as_a_token_not_as_letters(self):
        """⚠ The defect. `Lamp` is inside `OUT_LampState`, so a substring
        match hands a line about one address to another -- and the two are
        neighbours in exactly the documents where this matters.
        """
        got = self.blocks(
            "OUT_LampState shows ON while the door is open\n"
            "  otherwise OFF\n",
            {"a/lamp": ("Lamp",), "a/state": ("OUT_LampState",)},
        )
        self.assertIn("otherwise OFF", got["a/state"])
        self.assertEqual("", got["a/lamp"],
                         "a line that never names this address was given to it")

    def test_the_longest_name_takes_a_line_that_carries_two(self):
        """⚠ The second accident: the winner used to be whichever key the
        universe happened to list first, and the universe arrives in address
        order -- so the alphabet decided. Specificity is a reason; `sorted()`
        is not.
        """
        text = "Lamp is dark unless LampBrightness is above the threshold\n"
        universe = {"a/lamp": ("Lamp",), "a/bright": ("LampBrightness",)}
        got = self.blocks(text, universe)
        self.assertIn("LampBrightness", got["a/bright"])
        self.assertEqual("", got["a/lamp"])
        # The discriminator: the same line, with the keys given in the other
        # order, must land the same way. Under the old rule it did not.
        flipped = self.blocks(text, dict(reversed(list(universe.items()))))
        self.assertEqual(got["a/bright"], flipped["a/bright"])
        self.assertEqual(got["a/lamp"], flipped["a/lamp"])

    # ------------------------------------- why an interface model can travel

    def test_a_spelling_the_document_never_uses_does_not_move_the_cut(self):
        """⚠ THE PROPERTY THAT LETS AN INTERFACE MODEL BE A PLATFORM'S.

        A platform publishes every spelling an address goes by; a
        specification writes one or two. If a spelling nobody wrote can move
        the partition, then the model has to be rebuilt per document and
        cannot be published once -- which is what a measurement over 129 packs
        found: 132 findings moved on the addition of one name per address.
        Under the rule below, 4 did.

        ⚠⚠ The extra spelling is `State`, which is what a platform's own leaf
        segment looks like and which sits inside `OUT_LampState`. It is given
        to the address listed FIRST, because the rule this replaces awarded a
        line to the first key that matched -- so an extra spelling on a LATER
        key could never take anything, and a draft that put them the other way
        round passed under the old rule too. A test that cannot fail is not
        evidence, whatever it asserts.
        """
        text = ("OUT_LampState shows ON while the door is open\n"
                "  otherwise OFF\n"
                "Bell rings while a train approaches\n")
        lean = self.blocks(text, {"a/bell": ("Bell",),
                                  "a/lamp": ("OUT_LampState",)})
        wide = self.blocks(text, {"a/bell": ("Bell", "State"),
                                  "a/lamp": ("OUT_LampState",)})
        self.assertEqual(lean, wide)
        self.assertIn("otherwise OFF", wide["a/lamp"])


if __name__ == "__main__":
    unittest.main()
