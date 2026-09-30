"""An `accepted` answer says what it still leaves to a person.

Drafts a specification owner was shown as `accepted` were not finished
designs. One left a count `sce:unresolved`; one sent an output to
`#_parent`, where nothing here invokes it; one sent it to the machine
itself, which discards it. The product accepts all three -- each is valid
SCXML -- and an owner reading only the verdict saw a finished design.

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
       version="1.0" name="gate_open" initial="idle">
  <state id="idle">
    <transition event="start" target="waiting"/>
  </state>
  <state id="waiting">
    <transition event="finish" target="idle"/>
    <transition event="timeout" target="idle" sce:unresolved="timeout-policy"
                sce:unresolved-reason="the specification does not say what a timeout does"/>
  </state>
</scxml>
"""

TO_PARENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="gate_parent" initial="idle">
  <state id="idle">
    <transition event="start" target="waiting"/>
  </state>
  <state id="waiting">
    <onentry><send event="announce" target="#_parent"/></onentry>
    <transition event="finish" target="idle"/>
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
        self.assertIn("timeout-policy", answer["open"][0])
        self.assertIn("--strict-unresolved", answer["open"][0])
        self.assertIn("settle each item in `open`", answer["next"])

    def test_a_send_to_a_parent_nothing_invokes_is_accepted_and_says_so(self):
        answer = validated(TO_PARENT)
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual(1, len(answer["open"]), answer["open"])
        self.assertIn("announce", answer["open"][0])
        self.assertIn("check --document", answer["open"][0])

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
            "document_text": OPEN_QUESTION.replace('name="gate_open"', 'name="gate_open" x=', 1),
            "document_name": "draft.scxml"})
        self.assertTrue(refused.get("isError"))
        self.assertNotIn("open", refused["content"][0]["text"])


class OpenMattersAreRelayedNotWritten(unittest.TestCase):
    """The sentences are the product's (`open` on its manifest, one module in
    Rust). This layer carries them in order and writes none: a manifest that
    carries no `open` yields nothing, and a fact the manifest states only in
    its other fields is not turned into a sentence here."""

    def test_a_manifest_without_open_yields_nothing(self):
        self.assertEqual([], open_matters({}))
        self.assertEqual([], open_matters({"needs_parent": False, "unresolved": []}))

    def test_the_facts_in_the_other_fields_are_not_asserted_here(self):
        # `open` is what the product concluded from these; deriving it again
        # would be a second author of the same answer.
        manifest = {
            "unresolved": [{"kind": "unresolved", "id": "q1"}],
            "needs_parent": True,
            "parent_sends": [{"event": "Out", "state": "s"}],
            "needs_host_processor": True,
        }
        self.assertEqual([], open_matters(manifest))

    def test_each_sentence_is_carried_in_the_products_order(self):
        manifest = {"open": [
            {"kind": "question", "message": "first"},
            {"kind": "assumed", "message": "second"},
            {"kind": "parent", "message": "third"},
        ]}
        self.assertEqual(["first", "second", "third"], open_matters(manifest))


if __name__ == "__main__":
    unittest.main()
