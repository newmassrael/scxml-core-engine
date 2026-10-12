"""A `<send>` whose `delay` is not a time reaches the author from the product's own lint.

A drafted statechart armed its ten-second wait as `delay="10000"`. The product reads a bare number as
no time and every backend raises the argument error at that `<send>` (W3C SCXML 6.2), so the wait was
never armed; the document's own catch-all took the error event, and the position it guarded was written
at 0 ms where ten seconds were expected. `validate_scxml` ran `check --lint` and said `accepted`, because
the document is valid SCXML with a defined meaning, and `verify` could say only where the write was
(measured 2026-10-12, a Qwen draft of `Washer_Low_Warning`, two cases of four).

The finding belongs to the product (`scxml/send-delay-invalid`, a `--lint` record), not to a second copy
of the time grammar here: this tool already runs `check --lint` for every draft, so a rule added there
reaches the author the moment it exists. Asserted with the real generator, each refusal beside the
nearest document that must be accepted:

  no unit       refused, naming the code, the delay as written and the line of the `<send>`
  a unit        accepted, with no such record
  computed      a `delayexpr` is run-time and is not judged
"""

from __future__ import annotations

import io
import json
import pathlib
import tempfile
import unittest

from sce_author import mcp
from sce_author.verify import _default_codegen

CODE = "scxml/send-delay-invalid"

WAIT = """<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle">
  <state id="idle">
    <transition event="start" target="waiting"/>
  </state>
  <state id="waiting">
    <onentry><send event="hold.done" {delay}/></onentry>
    <transition event="hold.done" target="idle"/>
  </state>
</scxml>
"""


def call(name: str, **arguments) -> dict:
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}
    output = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
    return json.loads(output.getvalue())["result"]


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class ADelayWithNoUnitReachesTheAuthor(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def validate(self, delay: str) -> dict:
        document = self.dir / "wait.scxml"
        document.write_text(WAIT.format(delay=delay), encoding="utf-8")
        result = call("validate_scxml", document=str(document))
        return json.loads(result["content"][0]["text"])

    def codes(self, report: dict) -> list[str]:
        return [record["code"] for record in report["diagnostics"]]

    def test_a_delay_with_no_unit_is_refused_and_named(self):
        report = self.validate('delay="10000"')
        self.assertEqual("refused", report["verdict"])
        self.assertEqual([CODE], self.codes(report))
        record = report["diagnostics"][0]
        self.assertEqual("10000", record["actual"])
        self.assertIn('delay="10000"', record["message"])
        self.assertIn("'waiting'", record["message"])

    def test_the_record_points_at_the_send(self):
        record = self.validate('delay="10000"')["diagnostics"][0]
        # The `<onentry><send …/></onentry>` line of `WAIT`: the sixth, counting the root.
        self.assertEqual(6, record["location"]["line"], record["location"])
        self.assertEqual(WAIT.splitlines()[5].index("<send") + 1, record["location"]["col"],
                         record["location"])

    # The nearest documents that must be accepted: the same wait, written so the product reads it.
    def test_the_same_wait_with_a_unit_is_accepted(self):
        for delay in ('delay="10s"', 'delay="10000ms"', 'delay="1.5s"'):
            report = self.validate(delay)
            self.assertEqual(("accepted", []), (report["verdict"], self.codes(report)), delay)

    def test_a_delay_computed_at_run_time_is_not_judged(self):
        report = self.validate('delayexpr="\'10000\'"')
        self.assertNotIn(CODE, self.codes(report))

    def test_a_send_with_no_delay_is_accepted(self):
        report = self.validate("")
        self.assertEqual(("accepted", []), (report["verdict"], self.codes(report)))


if __name__ == "__main__":
    unittest.main()
