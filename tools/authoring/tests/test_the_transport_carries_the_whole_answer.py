"""A command on two surfaces, answering with two different amounts.

The command line and the MCP transport offer the same commands, and a case in
`test_the_server_speaks_the_protocol` holds them to that. It holds them to the
same NAMES and says nothing about what comes back -- which is exactly where
the two drifted. `verify` grew a figure saying which written positions no case
ever expects, the command line printed it, and the transport did not: a caller
over MCP read "every case passed" with no way to learn how little had been
judged. That is the sentence the figure was added to prevent, reproduced one
surface over.

⚠ SO THE CHECK IS ON THE RESULT OBJECT, not on a list of keys somebody keeps
up to date. Every field of `Verification` has to be accounted for here, either
as a key the payload carries or as a deliberate omission with its reason. A
field added tomorrow and placed nowhere fails this, which is the only way the
pair stays honest without anybody remembering to look. ⚠⚠ It has already
earned that: `backend` was added to the result while this file was being
written, and it arrives here as a decision to make rather than as a surprise.

⚠⚠⚠ The payload is built by a function rather than read off a live run, and
that is load-bearing. A run needs the product's code generator; a checkout
without a build has not got one, and the cases that need it SKIP. A parity
case that skips is a parity case nobody is keeping.
"""

from __future__ import annotations

import dataclasses
import unittest

from sce_author.mcp import verification_payload
from sce_author.verify import CaseResult, Verification

# Every field of `Verification`, and where it goes on the wire. `None` means
# deliberately not carried, and the comment beside it is the reason.
CARRIES = {
    "refusal": None,   # sent INSTEAD of the payload. A run that did not happen
                       # has no counts, and counts of zero beside a reason read
                       # to a client as "nothing failed".
    "backend": "backend",
    "results": "cases",
    "unbound": "unbound",
    "unasserted": "unasserted",
    "refuted": "refuted_assumptions",
}


class TheTransportCarriesTheWholeAnswer(unittest.TestCase):
    def test_every_field_of_a_verification_is_placed(self):
        declared = {f.name for f in dataclasses.fields(Verification)}
        self.assertEqual(
            declared, set(CARRIES),
            "a field of Verification is neither carried over MCP nor "
            "deliberately left off -- decide which, and say so here")

    def test_the_wire_carries_every_field_that_has_a_key(self):
        result = Verification(
            backend="python",
            results=[CaseResult(name="a case")],
            unbound=["plant/out/a.value"],
            unasserted=["plant/out/b.value"],
            refuted={"plant/out/c.value": "the author called this a guess"},
        )
        payload = verification_payload(result)
        for field, key in CARRIES.items():
            if key is None:
                continue
            self.assertIn(key, payload,
                          f"{field} reaches no caller over MCP")
            self.assertTrue(payload[key],
                            f"{key} came back empty from a result that has it")

    def test_what_the_cases_never_looked_at_is_one_of_them(self):
        """⚠ The field this file was written for, named rather than left to
        the sweep above. A pass is a statement about the cases; without this
        a client cannot tell a run that judged two positions of nine from one
        that judged nine of nine.
        """
        payload = verification_payload(
            Verification(backend="python",
                         unasserted=["plant/out/b.value"]))
        self.assertEqual(["plant/out/b.value"], payload["unasserted"])

    def test_a_refusal_is_sent_instead_of_counts_rather_than_beside_them(self):
        self.assertFalse(Verification(refusal="the pack has no examples").ran)


if __name__ == "__main__":
    unittest.main()
