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
    "host_memory": "host_memory",
    "assumed_preconditions": "assumed_preconditions",
    "refuted": "refuted_assumptions",
    # ⚠ Carried, not left off: the model that wrote the binding is the
    # likeliest reader, and it will only keep declaring unknowns if it can see
    # that doing so now costs just the positions that turn on them.
    "unresolved": "unresolved",
    # Carried under the same key: an open value is one question whether the
    # binding or the document is the one that left it open.
    "unresolved_outputs": "unresolved",
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
            host_memory=["wasApproaching"],
            assumed_preconditions={"powered": {
                "expression": "true",
                "reason": "the controller only runs while energised"}},
            refuted={"plant/out/c.value": "the author called this a guess"},
            unresolved={"override": "nobody has said which address this is"},
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

    def test_an_unknown_travels_with_its_reason_and_its_cost(self):
        """⚠ The reason is the message: it is what gets taken to whoever knows
        the address. And the cost travels beside it, per case, so a client can
        tell an unknown that blocked nothing from one that blocked everything.
        """
        case = CaseResult(name="a case", refusal="rests on the unknown",
                          undetermined=["plant/out/a.value"])
        payload = verification_payload(
            Verification(backend="python", results=[case],
                         unresolved={"override": "nobody has said"},
                         unresolved_outputs={"chime": "no play mode given"}))
        self.assertEqual({"override": "nobody has said"},
                         payload["unresolved"]["inputs"])
        self.assertEqual({"chime": "no play mode given"},
                         payload["unresolved"]["outputs"])
        self.assertEqual(1, payload["unresolved"]["withheld_positions"])
        self.assertEqual(["plant/out/a.value"],
                         payload["cases"][0]["undetermined"])

    def test_a_refusal_is_sent_instead_of_counts_rather_than_beside_them(self):
        self.assertFalse(Verification(refusal="the pack has no examples").ran)


if __name__ == "__main__":
    unittest.main()
