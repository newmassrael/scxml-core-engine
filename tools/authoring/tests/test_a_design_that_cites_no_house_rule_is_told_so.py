"""A check says how many of the profile's house rules the design cites, and says so
when it cites none.

Measured 2026-10-02 (Sonnet, headless, a profile of three house rules handed over,
four specifications, eight runs): all eight applied the rules -- every timer event
was addressed to `#_internal` -- and named H1 to H3 in their closing words, and not
one cited a rule in the document. The product cannot see a rule applied without its
citation, so an acceptance of those designs would have said they applied none. The
instruction to cite was in the server's text and was not what the client acted on.

So the fact the product CAN see is said where the client reads it: the manifest
counts the rules the profile holds and the distinct ones the run cites, and the
check's answer carries both numbers and, for an explicit zero, a sentence.
"""

from __future__ import annotations

import io
import json
import unittest

from sce_author import mcp
from sce_author.verify import _default_codegen

DESIGN ="""<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle">
  <state id="idle"{cite}>
    <transition event="go" target="busy"/>
  </state>
  <state id="busy"/>
</scxml>
"""

CITE = (' sce:assumed="{rule}" sce:assumed-reason="the specification names no event '
        'for this state"')

TWO_RULES = json.dumps({
    "record": "sce-authoring-profile", "v": 1,
    "house_rules": [
        {"id": "H1", "rule": "An event a state does not mention is ignored."},
        {"id": "H2", "rule": "A timer's event is addressed to #_internal."}]})

NO_RULES = json.dumps({"record": "sce-authoring-profile", "v": 1, "name": "plain",
                       "guidance": ["Ask before writing."]})


def design(rule: str | None = None) -> str:
    return DESIGN.format(cite=CITE.format(rule=rule) if rule else "")


def check(document: str, profile: str | None = None) -> dict:
    arguments = {"document_text": document, "document_name": "job.scxml"}
    if profile is not None:
        arguments["profile_text"] = profile
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": "validate_scxml", "arguments": arguments}}
    out = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), out)
    result = json.loads(out.getvalue())["result"]
    assert not result.get("isError"), result["content"][0]["text"]
    return json.loads(result["content"][0]["text"])


@unittest.skipUnless(_default_codegen().exists(), "the product's generator is not built")
class TheCheckSaysHowManyRulesTheDesignCites(unittest.TestCase):
    def test_a_design_that_cites_none_is_told_so_and_the_zero_is_a_number(self):
        answer = check(design(), TWO_RULES)
        self.assertEqual("accepted", answer["verdict"], answer)
        self.assertEqual({"held": 2, "cited": 0}, answer["house_rules"])
        self.assertIn("holds 2 house rule(s) and this document cites none", answer["show"])
        self.assertIn('sce:assumed="<rule id>"', answer["show"])
        self.assertIn("never cite one you did not apply", answer["show"])
        # The product's own count is what the answer repeats.
        self.assertEqual({"held": 2, "cited": 0}, answer["manifest"]["profile"]["house_rules"])

    def test_a_design_that_cites_a_rule_is_not_told_to_cite_one(self):
        answer = check(design("H1"), TWO_RULES)
        self.assertEqual({"held": 2, "cited": 1}, answer["house_rules"])
        self.assertNotIn("cites none", answer["show"])

    def test_a_citation_of_an_id_the_profile_does_not_hold_is_no_rule(self):
        answer = check(design("H9"), TWO_RULES)
        self.assertEqual({"held": 2, "cited": 0}, answer["house_rules"])
        self.assertIn("cites none", answer["show"])

    def test_nothing_is_said_where_there_is_no_rule_to_cite(self):
        for profile in (None, NO_RULES):
            with self.subTest(profile=profile is not None):
                answer = check(design(), profile)
                self.assertEqual("accepted", answer["verdict"], answer)
                self.assertNotIn("house_rules", answer)
                self.assertNotIn("house rule", answer["show"])


if __name__ == "__main__":
    unittest.main()
