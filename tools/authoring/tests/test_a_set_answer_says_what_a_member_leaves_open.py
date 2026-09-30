"""A set that comes back `accepted` says what one of its members leaves open.

A statechart with a closed interface and the event-schemas it imports are
checked together, as a set. One schema can say that the specification does
not settle what its event carries: `sce:unresolved` on its `<datamodel>`. The
product accepts that, and a set answer that carried no `open` was the
silence `open` exists to end -- the owner reading it saw a finished design.

`validate_scxml_set` keeps the product's verdict and adds `open` and `next`,
read from the product's manifest and from nothing else, as `validate_scxml`
does for one document. A set that leaves nothing open has neither.
"""

from __future__ import annotations

import json
import unittest

from sce_author.mcp import call_tool
from sce_author.verify import _default_codegen

STATECHART = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="job" initial="idle" datamodel="ecmascript"
       sce:interface="closed">
  <sce:import src="job_requested.scxml" kind="event-schema" as="Requested"/>
  <sce:import src="job_cancelled.scxml" kind="event-schema" as="Cancelled"/>
  <state id="idle">
    <transition event="job.requested" target="running"/>
  </state>
  <state id="running">
    <transition event="job.cancelled" target="idle"/>
  </state>
</scxml>
"""

NO_DATA = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="job_requested"
       sce:event-name="job.requested">
  <datamodel sce:payload="none"/>
</scxml>
"""

NOT_SETTLED = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="job_cancelled"
       sce:event-name="job.cancelled">
  <datamodel sce:unresolved="cancel-payload"
             sce:unresolved-reason="the specification does not say whether a cancellation names a reason"/>
</scxml>
"""


def validated_set(cancelled: str) -> dict:
    answer = call_tool("validate_scxml_set", {"documents_text": [
        {"name": "job.scxml", "text": STATECHART},
        {"name": "job_requested.scxml", "text": NO_DATA},
        {"name": "job_cancelled.scxml", "text": cancelled},
    ]})
    assert not answer.get("isError"), answer
    return json.loads(answer["content"][0]["text"])


@unittest.skipUnless(_default_codegen().exists(),
                     "the manifest is the product's; build sce-codegen first")
class ASetAnswerSaysWhatAMemberLeavesOpen(unittest.TestCase):
    def test_a_member_that_leaves_its_payload_open_makes_the_set_say_so(self):
        answer = validated_set(NOT_SETTLED)
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual(1, len(answer["open"]), answer["open"])
        self.assertIn("cancel-payload", answer["open"][0])
        self.assertIn("settle each item in `open`", answer["next"])

    def test_the_record_names_the_member_it_was_written_in(self):
        answer = validated_set(NOT_SETTLED)
        record = answer["manifest"]["unresolved"][0]
        self.assertEqual("job_cancelled.scxml", record["location"]["file"])

    def test_a_statechart_that_drops_its_closed_declaration_says_so(self):
        # The draft a check was got past by removing the declaration, every
        # import left where it was: the answer has to say the boundary is open.
        opened = STATECHART.replace('\n       sce:interface="closed">', '\n       >')
        self.assertNotEqual(STATECHART, opened)
        answer = call_tool("validate_scxml_set", {"documents_text": [
            {"name": "job.scxml", "text": opened},
            {"name": "job_requested.scxml", "text": NO_DATA},
            {"name": "job_cancelled.scxml", "text": NO_DATA.replace(
                "job_requested", "job_cancelled").replace(
                "job.requested", "job.cancelled")},
        ]})
        self.assertFalse(answer.get("isError"), answer)
        answer = json.loads(answer["content"][0]["text"])
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual(1, len(answer["open"]), answer["open"])
        self.assertIn("sce:interface=\"closed\"", answer["open"][0])
        self.assertIn("(Requested, Cancelled)", answer["open"][0])

    def test_a_set_whose_members_settle_everything_has_neither_field(self):
        settled = NOT_SETTLED.replace(
            'sce:unresolved="cancel-payload"\n             '
            'sce:unresolved-reason="the specification does not say whether a '
            'cancellation names a reason"',
            'sce:payload="none"')
        self.assertNotEqual(NOT_SETTLED, settled)
        answer = validated_set(settled)
        self.assertEqual("accepted", answer["verdict"])
        self.assertNotIn("open", answer)
        self.assertNotIn("next", answer)


if __name__ == "__main__":
    unittest.main()
