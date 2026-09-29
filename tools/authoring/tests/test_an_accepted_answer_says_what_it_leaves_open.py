"""An `accepted` answer says what it still leaves to a person.

Measured 2026-09-30: three drafts a specification owner was shown as
`accepted` were not finished designs. One left a retry count
`sce:unresolved`; one sent its request to `#_parent`, where nothing here
invokes it; one sent its request to the machine itself, which discards it.
The product accepts all three -- each is valid SCXML -- and an owner reading
only the verdict saw a finished design.

`validate_scxml` keeps the product's verdict and adds `open` (a line for
each thing the run leaves to a person) and `next`, read from the product's
own manifest and from nothing else. A run that leaves nothing has neither,
so an answer that was complete stays byte for byte what it was.
"""

from __future__ import annotations

import json
import pathlib
import tempfile
import unittest

from sce_author.mcp import call_tool
from sce_author.verify import _default_codegen, open_matters

OPEN_QUESTION = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="retry_open" initial="idle">
  <state id="idle">
    <transition event="request" target="waiting"/>
  </state>
  <state id="waiting">
    <transition event="response" target="idle"/>
    <transition event="timeout" target="idle" sce:unresolved="retry-count"
                sce:unresolved-reason="the specification does not say how many retries"/>
  </state>
</scxml>
"""

TO_PARENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="retry_parent" initial="idle">
  <state id="idle">
    <transition event="request" target="waiting"/>
  </state>
  <state id="waiting">
    <onentry><send event="SendRequest" target="#_parent"/></onentry>
    <transition event="response" target="idle"/>
  </state>
</scxml>
"""

TO_THE_HOST = TO_PARENT.replace('target="#_parent"', 'type="x-host"')

# The host serves its type only when the build is told so; without that
# the product publishes the need, which is what `open` says.
FINISHED = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="finished" initial="idle">
  <state id="idle"><transition event="go" target="done"/></state>
  <final id="done"/>
</scxml>
"""


def validated(text: str) -> dict:
    answer = call_tool("validate_scxml", {"document_text": text, "document_name": "draft.scxml"})
    assert not answer.get("isError"), answer
    return json.loads(answer["content"][0]["text"])


@unittest.skipUnless(_default_codegen().exists(),
                     "the manifest is the product's; build sce-codegen first")
class AnAcceptedAnswerSaysWhatItLeavesOpen(unittest.TestCase):
    def test_a_draft_with_an_open_question_is_accepted_and_says_so(self):
        answer = validated(OPEN_QUESTION)
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual(1, len(answer["open"]), answer["open"])
        self.assertIn("retry-count", answer["open"][0])
        self.assertIn("--strict-unresolved", answer["open"][0])
        self.assertIn("settle each item in `open`", answer["next"])

    def test_a_send_to_a_parent_nothing_invokes_is_accepted_and_says_so(self):
        answer = validated(TO_PARENT)
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual(1, len(answer["open"]), answer["open"])
        self.assertIn("SendRequest", answer["open"][0])
        self.assertIn("validate_scxml_set", answer["open"][0])

    def test_a_send_to_the_host_names_the_processor_the_host_must_serve(self):
        answer = validated(TO_THE_HOST)
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual(1, len(answer["open"]), answer["open"])
        self.assertIn("host has to serve it", answer["open"][0])

    def test_a_document_that_leaves_nothing_has_neither_field(self):
        answer = validated(FINISHED)
        self.assertEqual("accepted", answer["verdict"])
        self.assertNotIn("open", answer)
        self.assertNotIn("next", answer)

    def test_a_refusal_is_left_as_the_product_gave_it(self):
        refused = call_tool("validate_scxml", {
            "document_text": OPEN_QUESTION.replace('name="retry_open"', 'name="retry_open" x=', 1),
            "document_name": "draft.scxml"})
        self.assertTrue(refused.get("isError"))
        self.assertNotIn("open", refused["content"][0]["text"])


class OpenMattersIsReadFromTheManifestAlone(unittest.TestCase):
    """No sentence of its own: a manifest that carries nothing yields nothing,
    and a fact the manifest does not carry is not asserted."""

    def test_an_empty_manifest_yields_nothing(self):
        self.assertEqual([], open_matters({}))
        self.assertEqual([], open_matters({"needs_parent": False, "unresolved": []}))

    def test_each_fact_the_manifest_carries_yields_one_line(self):
        manifest = {
            "unresolved": [
                {"kind": "unresolved", "id": "q1"},
                {"kind": "unresolved", "id": "q2"},
                {"kind": "assumed", "id": "a1"},
            ],
            "needs_parent": True,
            "parent_sends": [{"event": "Out", "state": "s"}, {"state": "s"}],
            "needs_host_processor": True,
        }
        lines = open_matters(manifest)
        self.assertEqual(4, len(lines), lines)
        self.assertIn("2 question(s)", lines[0])
        self.assertIn("q1, q2", lines[0])
        self.assertIn("1 value(s)", lines[1])
        self.assertIn("(Out)", lines[2])
        self.assertIn("error.execution", lines[3])

    def test_an_assumption_alone_is_a_line_and_not_a_question(self):
        lines = open_matters({"unresolved": [{"kind": "assumed", "id": "a1"}]})
        self.assertEqual(1, len(lines))
        self.assertNotIn("question", lines[0])


if __name__ == "__main__":
    unittest.main()
