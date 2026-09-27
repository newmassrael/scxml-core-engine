"""The author is told what the host does from the keys `verify` models.

What the host does -- when it runs a document, which positions a round
writes -- was a page written by hand for one corpus and handed to the author
beside the pack, while `verify` read the pack's `host`. The first time the two
parted (measured 2026-09-27) the page said the component writes its outputs at
the end of every round; an author wrote every output every round, and an event
the specification delays by two seconds announced its old value at 2 ms and
failed the platform's own test. `brief` now says it, from the pack.

Asserted here:

    section 8 states the activation and the write rule the pack declares,
    including how an output written only at certain moments is bound
                                                        (the discriminator)
    a pack that states neither says so, rather than being silent
    the platform's activation is what section 7 defers to
"""

from __future__ import annotations

import pathlib
import unittest

from sce_author import brief
from sce_author.pack import load_pack
from sce_author.prose import load_prose

HERE = pathlib.Path(__file__).resolve().parent
PACK = HERE / "fixtures" / "crossing"
HOST = "8. What the host does with the document"


class TheHostIsToldToTheAuthor(unittest.TestCase):
    def setUp(self):
        self.prose = load_prose([PACK / "specification.md"])
        self.pack = load_pack(PACK)

    def section(self, number: str) -> str:
        return "\n".join(dict(brief.sections(self.prose, self.pack))[number])

    def test_the_pack_host_is_what_the_author_reads(self):
        self.pack.conventions.host.update(
            {"activation": "on-change", "writes": "every-round"})
        said = self.section(HOST)
        self.assertIn("`activation: on-change`", said)
        self.assertIn("reaches the document not at all", said)
        self.assertIn("`writes: every-round`", said)
        # The move an author could not have known from "every round": an
        # output that takes its value later is held until it is sent.
        self.assertIn("`hold_last: true` with a `when_nothing_sent` its\n  `map` leaves out",
                      said)
        self.assertIn("rewriting its old value in the meantime announces that old\n"
                      "value first", said)

    def test_a_pack_that_says_nothing_of_its_host_says_so(self):
        self.pack.conventions.host.clear()
        said = self.section(HOST)
        self.assertIn("The pack does not say what the host does", said)
        self.assertNotIn("hold_last", said)

    def test_an_activation_alone_leaves_the_write_rule_unstated(self):
        self.pack.conventions.host.clear()
        self.pack.conventions.host["activation"] = "periodic"
        said = self.section(HOST)
        self.assertIn("once per period", said)
        self.assertIn("**What a round writes** is not stated", said)

    def test_remembering_defers_to_the_platform(self):
        said = self.section("7. When the answer depends on what happened before")
        self.assertIn("pack's `host` (section 8)", said)


if __name__ == "__main__":
    unittest.main()
