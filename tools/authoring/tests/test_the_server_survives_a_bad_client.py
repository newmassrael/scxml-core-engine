"""One bad frame must not end the session.

⚠ This file exists because it was measured missing, and what it found was the
worst defect in the package. Thirteen malformed frames were fed to the server
and it produced ZERO replies: the first one -- a bare JSON array where an
object belongs -- reached `message.get` and raised, `serve` did not guard the
call, and the loop ended. Every later request, the well-formed ones included,
went unanswered. A client sees that as a hang, not as its own mistake, and the
session is gone.

Two properties are asserted here, and the second is the one that catches a
regression the first would miss:

    every frame gets a reply
    a GOOD call placed after all the bad ones still works

Without the second, a server that replied "error" to everything forever would
pass. And no reply may carry a traceback: this program's internals are not an
answer to somebody else's mistake.
"""

from __future__ import annotations

import io
import json
import pathlib
import unittest

from sce_author import mcp

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
SPEC = CROSSING / "specification.md"

MALFORMED = [
    "[1, 2, 3]",                 # an array where an object belongs
    '"hello"',                   # a bare string
    "42",                        # a bare number
    "null",
    "{ this is not json",
    json.dumps({"jsonrpc": "2.0", "id": 1}),                       # no method
    json.dumps({"jsonrpc": "2.0", "id": 2, "method": 7}),          # method not a string
    json.dumps({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": [1]}),                                    # params not an object
    json.dumps({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
                "params": {"name": "questions", "arguments": "nope"}}),
    json.dumps({"jsonrpc": "2.0", "id": 5, "method": "tools/call",
                "params": {"name": "questions"}}),                  # no arguments
    json.dumps({"jsonrpc": "2.0", "id": 6, "method": "tools/call",
                "params": {"name": "questions",
                           "arguments": {"pack": str(CROSSING),
                                         "prose": ["/no/such/file.md"]}}}),
    json.dumps({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
                "params": {"name": "questions",
                           "arguments": {"pack": str(CROSSING), "prose": 5}}}),
    json.dumps({"jsonrpc": "2.0", "id": 8, "method": "tools/call",
                "params": {"name": "no-such-tool", "arguments": {}}}),
]

GOOD = json.dumps({
    "jsonrpc": "2.0", "id": 99, "method": "tools/call",
    "params": {"name": "questions",
               "arguments": {"pack": str(CROSSING), "prose": [str(SPEC)],
                             "kind": "example-shows-memory"}}})


class TheServerSurvivesABadClient(unittest.TestCase):
    def converse(self, lines):
        out = io.StringIO()
        mcp.serve(io.StringIO("\n".join(lines) + "\n"), out)
        return [json.loads(l) for l in out.getvalue().splitlines() if l.strip()]

    def test_every_malformed_frame_gets_a_reply(self):
        replies = self.converse(MALFORMED)
        self.assertEqual(
            len(MALFORMED), len(replies),
            "one reply per frame; a frame that ends the loop takes the session")

    def test_a_good_call_after_every_bad_one_still_works(self):
        """The property a server that only ever errors would still fail."""
        replies = self.converse([*MALFORMED, GOOD])
        self.assertEqual(len(MALFORMED) + 1, len(replies))
        last = replies[-1]
        self.assertEqual(99, last.get("id"))
        self.assertNotIn("error", last, f"the good call was refused: {last}")
        payload = json.loads(last["result"]["content"][0]["text"])
        self.assertEqual(1, payload["version"])
        # The planted memory gap in the second subject matter, still found
        # after the server has been handed thirteen broken frames.
        self.assertEqual(
            ["plant/out/bell.value"],
            [q["subject"] for q in payload["questions"]])

    def test_no_reply_carries_a_traceback(self):
        for reply in self.converse([*MALFORMED, GOOD]):
            body = json.dumps(reply, ensure_ascii=False)
            self.assertNotIn(
                "Traceback (most recent call last)", body,
                "this program's internals are not an answer to a client's mistake")

    def test_an_argument_mistake_says_what_to_send(self):
        """A message naming the exception class tells the caller nothing.

        `KeyError: 'pack'` and `TypeError: 'int' object is not iterable` were
        both true and neither said what to send instead.
        """
        for args, expected in (
            ({}, "'pack' is required"),
            ({"pack": str(CROSSING)}, "'prose' is required"),
            ({"pack": str(CROSSING), "prose": 5}, "has to be a LIST"),
            ({"pack": str(CROSSING), "prose": str(SPEC)}, "has to be a LIST"),
            ({"pack": str(CROSSING), "prose": [str(SPEC)], "kind": 3},
             "'kind' has to be"),
        ):
            result = mcp.call_tool("questions", args)
            self.assertTrue(result.get("isError"), f"{args} should be refused")
            said = result["content"][0]["text"]
            self.assertIn(expected, said, f"{args} said {said!r}")


if __name__ == "__main__":
    unittest.main()
