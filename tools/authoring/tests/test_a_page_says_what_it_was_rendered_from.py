"""The pseudocode page is one block and what it was rendered from is another.

The page and the check used to be two calls over a path. A document edited
between them put a page on the owner's screen that no check had seen, and
nothing in the answer said which bytes it came from. Now the check runs on the
document the page was just rendered from, inside the one call, and the answer
names those bytes by digest, says what the product made of that same document,
and says that no owner accepted anything.

The digest is NEVER inside the page: `compare` asks whether two drafts render
to the same page, and a digest in it would make every pair differ.
"""

from __future__ import annotations

import hashlib
import io
import json
import pathlib
import tempfile
import unittest
from unittest import mock

from sce_author import mcp, verify
from sce_author.verify import _default_codegen

NS = ('xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" '
      'version="1.0"')

ACCEPTED = f"""<scxml {NS} initial="Idle">
  <state id="Idle"><transition event="go" target="Busy"/></state>
  <state id="Busy"><transition event="done" target="Idle"/></state>
</scxml>
"""

# An orphan state: the design-time lint refuses it, and the page still renders.
REFUSED = f"""<scxml {NS} initial="Idle">
  <state id="Idle"/>
  <state id="Orphan"/>
</scxml>
"""

LEAVES_ONE_OPEN = f"""<scxml {NS} initial="Idle" datamodel="ecmascript">
  <datamodel>
    <data id="wait" sce:unresolved="wait-time"
          sce:unresolved-reason="the specification does not say how long to wait"/>
  </datamodel>
  <state id="Idle"><transition event="go" target="Busy"/></state>
  <state id="Busy"><transition event="done" target="Idle"/></state>
</scxml>
"""


def call(name: str, **arguments) -> dict:
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}
    output = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
    return json.loads(output.getvalue())["result"]


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class APageSaysWhatItWasRenderedFrom(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def write(self, text: str, name: str = "client.scxml") -> pathlib.Path:
        path = self.dir / name
        path.write_text(text, encoding="utf-8")
        return path

    def blocks(self, **arguments) -> tuple[str, str]:
        result = call("render_scxml_pseudocode", **arguments)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        self.assertEqual(2, len(result["content"]), result["content"])
        return result["content"][0]["text"], result["content"][1]["text"]

    def test_the_answer_is_the_page_and_then_a_note_naming_the_bytes_it_came_from(self):
        document = self.write(ACCEPTED)
        page, note = self.blocks(document=str(document))
        # The first block is exactly what the generator renders, untouched.
        self.assertEqual(verify.pseudo_page(document, None, None)[0], page)
        self.assertIn("state Idle:", page)
        self.assertIn(f"sha256 {sha256(ACCEPTED)}", note)
        self.assertIn("Product check of that document alone: accepted.", note)
        self.assertIn("Owner acceptance: none recorded", note)

    def test_the_digest_is_a_note_about_the_page_and_never_part_of_it(self):
        _, note = self.blocks(document=str(self.write(ACCEPTED)))
        page, _ = self.blocks(document=str(self.write(ACCEPTED, "other.scxml")))
        # Two files with the same bytes give the same page whatever they are
        # called, so the `compare` tool's page level still sees them as one.
        self.assertNotIn(sha256(ACCEPTED), page)
        self.assertNotIn("sha256", page)
        self.assertIn(sha256(ACCEPTED), note)

    def test_a_document_handed_as_text_is_named_by_the_bytes_that_were_checked(self):
        _, note = self.blocks(document_text=ACCEPTED, document_name="door.scxml")
        self.assertIn(f"sha256 {sha256(ACCEPTED)}", note)

    def test_a_document_the_product_refuses_is_said_refused_beside_its_page(self):
        page, note = self.blocks(document=str(self.write(REFUSED)))
        # The page still renders, and the note will not let it read as approved.
        self.assertIn("state Idle", page)
        self.assertIn("REFUSED", note)
        self.assertIn("does not accept", note)
        # ⚠ Not "not run": a refusal is the product's ANSWER, returned as the
        # second value of the check, and reading only the first would drop it.
        self.assertNotIn("not run", note)
        self.assertNotIn("accepted.", note)

    def test_what_the_document_leaves_open_is_said_and_silence_is_not_taken_for_none(self):
        _, open_note = self.blocks(document=str(self.write(LEAVES_ONE_OPEN)))
        self.assertIn("Left open: 1 question(s)", open_note)
        self.assertIn("wait-time", open_note)
        _, closed_note = self.blocks(document=str(self.write(ACCEPTED)))
        self.assertIn("Left open: nothing is marked open, which is not the same as "
                      "nothing being open.", closed_note)

    def test_the_owners_profile_holds_the_check_of_the_page(self):
        profile = self.write(json.dumps({"record": "sce-authoring-profile", "v": 1,
                                         "interface": "closed"}), "profile.json")
        document = self.write(ACCEPTED)
        _, without = self.blocks(document=str(document))
        _, held = self.blocks(document=str(document), profile=str(profile))
        # An open interface is what the profile forbids: the same document is
        # accepted without the profile and refused under it.
        self.assertIn("accepted.", without)
        self.assertIn("REFUSED", held)
        self.assertIn("profile/", held)

    def test_a_document_that_changes_while_it_is_checked_has_no_page_to_stand_on(self):
        document = self.write(ACCEPTED)
        real = verify.validate_scxml

        def edited_during_the_check(*args, **kwargs):
            answer = real(*args, **kwargs)
            document.write_text(ACCEPTED + "<!-- edited -->\n", encoding="utf-8")
            return answer

        with mock.patch.object(verify, "validate_scxml", edited_during_the_check):
            lines, refusal = verify.page_provenance(document)
        self.assertEqual("", lines)
        self.assertIn("changed while it was being checked", refusal)

    def test_the_tool_and_the_instructions_say_there_are_two_blocks(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "render_scxml_pseudocode")
        self.assertIn("TWO blocks", tool["description"])
        self.assertIn("profile", tool["inputSchema"]["properties"])
        self.assertIn("sha256", mcp.SERVER_INSTRUCTIONS)
        # The instructions send the client to the check for the page, since a
        # second call was skipped fourteen times of fifteen (measured).
        self.assertIn("answer of validate_scxml", mcp.SERVER_INSTRUCTIONS)


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class ACheckCarriesThePageOfWhatItChecked(unittest.TestCase):
    """The page rides on the check, so a client that never makes a second call
    still has it. Measured 2026-09-30, fifteen drafts by a real client: one
    showed a page, fourteen skipped the tool that renders it."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def write(self, text: str, name: str = "client.scxml") -> pathlib.Path:
        path = self.dir / name
        path.write_text(text, encoding="utf-8")
        return path

    def test_an_accepted_document_comes_with_the_page_of_the_bytes_checked(self):
        document = self.write(ACCEPTED)
        result = call("validate_scxml", document=str(document))
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        self.assertEqual(2, len(result["content"]))
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("accepted", answer["verdict"])
        self.assertEqual([{"document": "client.scxml", "sha256": sha256(ACCEPTED),
                           "page": "block 2"}], answer["pages"])
        # The block is the page itself, untouched and raw, not a JSON string.
        self.assertEqual(verify.pseudo_page(document, None, None)[0],
                         result["content"][1]["text"])
        self.assertNotIn("sha256", result["content"][1]["text"])
        self.assertIn("no owner acceptance is recorded", answer["show"])

    def test_a_refused_document_has_no_page_to_show(self):
        result = call("validate_scxml", document=str(self.write(REFUSED)))
        self.assertEqual(1, len(result["content"]))
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("refused", answer["verdict"])
        self.assertNotIn("pages", answer)

    def test_a_document_handed_as_text_is_named_by_the_bytes_checked(self):
        result = call("validate_scxml", document_text=ACCEPTED, document_name="door.scxml")
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual(sha256(ACCEPTED), answer["pages"][0]["sha256"])
        self.assertEqual("door.scxml", answer["pages"][0]["document"])

    def test_a_set_comes_with_one_page_per_document_in_the_order_given(self):
        other = ACCEPTED.replace("Idle", "Waiting").replace("Busy", "Working")
        result = call("validate_scxml_set", documents_text=[
            {"name": "first.scxml", "text": ACCEPTED},
            {"name": "second.scxml", "text": other}])
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        self.assertEqual(3, len(result["content"]))
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual([("first.scxml", sha256(ACCEPTED), "block 2"),
                          ("second.scxml", sha256(other), "block 3")],
                         [(p["document"], p["sha256"], p["page"]) for p in answer["pages"]])
        self.assertIn("state Idle:", result["content"][1]["text"])
        self.assertIn("state Waiting:", result["content"][2]["text"])

    def test_a_document_that_changes_while_it_is_checked_gets_no_page(self):
        document = self.write(ACCEPTED)
        real = mcp.run_scxml_validation

        def edited_during_the_check(*args, **kwargs):
            answer = real(*args, **kwargs)
            document.write_text(ACCEPTED + "<!-- edited -->\n", encoding="utf-8")
            return answer

        with mock.patch.object(mcp, "run_scxml_validation", edited_during_the_check):
            result = call("validate_scxml", document=str(document))
        self.assertEqual(1, len(result["content"]))
        entry = json.loads(result["content"][0]["text"])["pages"][0]
        self.assertIsNone(entry["page"])
        self.assertIn("changed while it was being checked", entry["page_refusal"])

    def test_a_page_the_product_will_not_render_is_said_and_the_verdict_stands(self):
        document = self.write(ACCEPTED)
        with mock.patch.object(mcp, "pseudo_page", lambda *a, **k: ("", "cannot render\nwhy")):
            result = call("validate_scxml", document=str(document))
        self.assertEqual(1, len(result["content"]))
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("accepted", answer["verdict"])
        self.assertIsNone(answer["pages"][0]["page"])
        self.assertEqual("cannot render", answer["pages"][0]["page_refusal"])


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class TheOwnerReadsThePageInTheirOwnLanguage(unittest.TestCase):
    """Measured 2026-10-01 in a review of a real client's answer: the page came
    out in English for an owner who reads Korean, though the product has a Korean
    lexicon. The words the grammar spends change; what the document wrote does
    not."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def page(self, tool: str, **arguments) -> str:
        result = call(tool, **arguments)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        self.assertEqual(2, len(result["content"]), result["content"])
        return result["content"][1]["text"]

    def test_a_lexicon_reaches_the_page_and_changes_the_grammar_not_the_names(self):
        english = self.page("validate_scxml", document_text=ACCEPTED, document_name="a.scxml")
        korean = self.page("validate_scxml", document_text=ACCEPTED, document_name="a.scxml",
                           lexicon="ko")
        self.assertNotEqual(english, korean)
        self.assertIn("lexicon=ko", korean)
        # What the document wrote is never translated: its state names are on
        # both pages as the author spelled them.
        for name in ("Idle", "Busy"):
            self.assertIn(name, english)
            self.assertIn(name, korean)

    def test_a_set_takes_the_lexicon_too(self):
        page = self.page("validate_scxml_set",
                         documents_text=[{"name": "a.scxml", "text": ACCEPTED}],
                         lexicon="ko")
        self.assertIn("lexicon=ko", page)

    def test_a_lexicon_the_product_does_not_know_costs_the_page_and_not_the_verdict(self):
        result = call("validate_scxml", document_text=ACCEPTED, document_name="a.scxml",
                      lexicon="zz")
        self.assertEqual(1, len(result["content"]))
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("accepted", answer["verdict"])
        self.assertIsNone(answer["pages"][0]["page"])
        # The product's own refusal, which names the real set: nothing here
        # keeps a list of lexicons that would go stale the day one is added.
        self.assertIn("invalid value 'zz'", answer["pages"][0]["page_refusal"])

    def test_the_lexicon_is_offered_on_both_tools(self):
        for name in ("validate_scxml", "validate_scxml_set"):
            with self.subTest(tool=name):
                tool = next(t for t in mcp.TOOLS if t["name"] == name)
                self.assertIn("lexicon", tool["inputSchema"]["properties"])

    def test_a_lexicon_that_is_no_name_is_an_argument_error(self):
        self.assertTrue(call("validate_scxml", document_text=ACCEPTED,
                             document_name="a.scxml", lexicon=7).get("isError"))


if __name__ == "__main__":
    unittest.main()
