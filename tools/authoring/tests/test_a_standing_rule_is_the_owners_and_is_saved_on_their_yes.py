"""A standing rule is the owner's: it keeps their words and is saved on their yes.

A house rule lets a draft apply an answer instead of asking. That authority is the
owner's, and a client that can write a rule into the profile gives itself
permission it was never given. The other failure is an owner who has to write the
profile's JSON and so never gets to say a rule at all.

`scxml_house_rule` takes the owner's words, shows the rule beside them, and saves
nothing until the client says the owner said yes to THAT wording. What it records
is modest on purpose: `relayed`, a client's report, because the product was not in
the conversation. These cases hold the line between what the tool can decide
(whether the words are the owner's, whether the rule repeats one, whether the id
is one token) and what it can only report.
"""

from __future__ import annotations

import io
import json
import unittest

from sce_author import house_rule, mcp
from sce_author.verify import _default_codegen

WORDS = ("When the screen shows nothing for an event, just ignore it. "
         "A door that is open stays open until somebody says otherwise.")

IGNORE = {"quote": "just ignore it",
          "rule": "An event a state does not mention is ignored."}
OPEN = {"quote": "A door that is open stays open until somebody says otherwise",
        "rule": "A door that is open stays open until told."}


def call(name: str, **arguments) -> dict:
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}
    output = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
    return json.loads(output.getvalue())["result"]


def profile(*rules: dict) -> str:
    return json.dumps({"record": "sce-authoring-profile", "v": 1, "name": "owner",
                       "house_rules": list(rules)})


class WhatIsSavedAndWhenTest(unittest.TestCase):
    def test_the_first_call_saves_nothing_and_shows_the_rule_beside_the_owners_words(self):
        built = house_rule.build(None, [IGNORE])
        self.assertIsNone(built.profile_text)
        reply = house_rule.answer(built)
        self.assertEqual("proposal", reply["status"])
        self.assertNotIn("profile_text", reply)
        self.assertEqual([{"id": "H1", "rule": IGNORE["rule"], "your_words": "just ignore it"}],
                         reply["rules"])
        self.assertIn("just ignore it", reply["tell_the_owner"])
        self.assertIn("nothing was saved", reply["next"])

    def test_the_owners_yes_makes_a_profile_that_says_relayed_and_keeps_their_words(self):
        built = house_rule.build(None, [IGNORE, OPEN], confirmed=True)
        made = json.loads(built.profile_text)
        self.assertEqual({"record": "sce-authoring-profile", "v": 1}, {
            k: made[k] for k in ("record", "v")})
        self.assertEqual(
            [{"id": "H1", "rule": IGNORE["rule"], "quote": "just ignore it",
              "confirmation": "relayed"},
             {"id": "H2", "rule": OPEN["rule"], "quote": OPEN["quote"],
              "confirmation": "relayed"}],
            made["house_rules"])
        self.assertEqual("ready-to-save", house_rule.answer(built)["status"])

    def test_a_rule_joins_the_profile_the_owner_already_keeps_and_leaves_it_as_it_was(self):
        held = {"id": "H1", "rule": "Unknown events are ignored."}
        text = profile(held)
        made = json.loads(house_rule.build(text, [IGNORE], confirmed=True).profile_text)
        self.assertEqual("owner", made["name"])
        self.assertEqual(held, made["house_rules"][0])
        self.assertEqual("H2", made["house_rules"][1]["id"])

    def test_the_id_is_the_lowest_one_not_in_use(self):
        text = profile({"id": "H2", "rule": "something else entirely"})
        self.assertEqual("H1", house_rule.build(text, [IGNORE]).rules[0].id)
        both = house_rule.build(profile({"id": "H1", "rule": "a"}), [IGNORE, OPEN])
        self.assertEqual(["H2", "H3"], [r.id for r in both.rules])


class WhatIsRefusedTest(unittest.TestCase):
    def refused(self, *args, said: str, **kwargs):
        with self.assertRaises(house_rule.HouseRuleError) as caught:
            house_rule.build(*args, **kwargs)
        self.assertIn(said, str(caught.exception))

    def test_a_quote_that_is_not_the_owners_words_attributes_nothing_to_them(self):
        self.refused(None, [{"quote": "always ignore it", "rule": "x"}],
                     owner_words=WORDS, said="is not in the owner's words")
        # The same quote against the words it is in, word for word, is taken.
        self.assertEqual("H1", house_rule.build(None, [IGNORE], owner_words=WORDS).rules[0].id)

    def test_without_the_owners_words_the_quote_is_taken_and_the_answer_says_so(self):
        built = house_rule.build(None, [IGNORE])
        self.assertTrue(any("not held to the owner's words" in n for n in built.notes), built.notes)
        self.assertEqual([], house_rule.build(None, [IGNORE], owner_words=WORDS).notes)

    def test_a_rule_with_no_words_of_the_owners_is_not_saved(self):
        for request in ({"rule": "x"}, {"quote": "", "rule": "x"}, {"quote": "  ", "rule": "x"}):
            with self.subTest(request=request):
                self.refused(None, [request], said="no `quote`")

    def test_a_rule_that_says_nothing_or_takes_a_field_it_does_not_have_is_refused(self):
        self.refused(None, [{"quote": "q"}], said="no `rule`")
        self.refused(None, [{"quote": "q", "rule": "x", "approved": True}],
                     said="'approved'")
        self.refused(None, [], said="non-empty list")
        self.refused(None, "ignore it", said="non-empty list")

    def test_a_rule_the_profile_already_holds_is_not_held_twice(self):
        held = profile({"id": "H7", "rule": "an event a state does not  mention is ignored."})
        self.refused(held, [IGNORE], said="what H7 already says")
        self.refused(None, [IGNORE, dict(IGNORE, quote="ignore it")],
                     said="what H1 already says")

    def test_an_id_is_one_token_and_names_one_rule(self):
        self.refused(None, [dict(IGNORE, id="H 1")], said="one token")
        self.refused(profile({"id": "H1", "rule": "a"}), [dict(IGNORE, id="H1")],
                     said="already holds")
        self.assertEqual("ignore-unmentioned",
                         house_rule.build(None, [dict(IGNORE, id="ignore-unmentioned")]).rules[0].id)

    def test_what_is_not_a_profile_the_tool_can_extend_is_refused(self):
        self.refused('{"record": "something-else", "v": 1}', [IGNORE], said="is not a")
        self.refused('{"record": "sce-authoring-profile", "v": 2}', [IGNORE], said="version")
        self.refused("[1, 2]", [IGNORE], said="is not a")
        self.refused("not json", [IGNORE], said="not JSON")
        self.refused('{"record": "sce-authoring-profile", "v": 1, "v": 1}', [IGNORE],
                     said="is written twice")
        self.refused('{"record": "sce-authoring-profile", "v": 1, "house_rules": [3]}',
                     [IGNORE], said="house_rules")


class WhatEveryAcceptanceSaysOfARuleTest(unittest.TestCase):
    def test_every_rule_is_said_on_the_authority_it_stands_on(self):
        said = house_rule.standing([
            {"id": "H1", "quote": "just ignore it", "confirmation": "relayed"},
            {"id": "H2", "quote": "keep it open"},
            {"id": "H3"}])
        self.assertEqual(3, len(said))
        self.assertIn("a client reported that the owner said yes", said[0])
        self.assertIn("which the product did not see", said[0])
        self.assertIn("no record that the owner confirmed", said[1])
        self.assertIn("nothing records where it came from", said[2])
        self.assertEqual([], house_rule.standing(None))


needs_the_generator = unittest.skipUnless(_default_codegen().exists(),
                                          "the product's generator is not built")


class TheToolTest(unittest.TestCase):
    def answer(self, **arguments) -> dict:
        result = call("scxml_house_rule", **arguments)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        return json.loads(result["content"][0]["text"])

    def test_the_tool_is_offered_and_says_it_saves_nothing_on_the_first_call(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "scxml_house_rule")
        self.assertEqual(["rules"], tool["inputSchema"]["required"])
        properties = tool["inputSchema"]["properties"]
        for name in ("profile", "profile_text", "owner_words", "owner_words_text",
                     "owner_confirmed"):
            self.assertIn(name, properties)
        self.assertIn("SAVES NOTHING", tool["description"])
        self.assertIn("Never set `owner_confirmed` on your own say-so", tool["description"])

    def test_a_proposal_needs_no_product_and_returns_no_profile(self):
        reply = self.answer(rules=[IGNORE], owner_words_text=WORDS)
        self.assertEqual("proposal", reply["status"])
        self.assertNotIn("profile_text", reply)

    def test_a_quote_the_owner_did_not_say_is_an_argument_error(self):
        result = call("scxml_house_rule", rules=[dict(IGNORE, quote="always")],
                      owner_words_text=WORDS)
        self.assertTrue(result.get("isError"))
        self.assertIn("not in the owner's words", result["content"][0]["text"])

    def test_the_confirmation_must_be_a_boolean(self):
        result = call("scxml_house_rule", rules=[IGNORE], owner_confirmed="yes")
        self.assertTrue(result.get("isError"))

    @needs_the_generator
    def test_a_confirmed_rule_comes_back_as_a_profile_the_product_reads(self):
        reply = self.answer(rules=[IGNORE, OPEN], owner_words_text=WORDS,
                            owner_confirmed=True)
        self.assertEqual("ready-to-save", reply["status"])
        made = json.loads(reply["profile_text"])
        self.assertEqual(["relayed", "relayed"],
                         [r["confirmation"] for r in made["house_rules"]])
        # The profile that came back is one the tool can extend again.
        more = self.answer(profile_text=reply["profile_text"],
                           rules=[{"quote": "stay put", "rule": "Nothing moves unasked."}],
                           owner_confirmed=True)
        self.assertEqual(["H1", "H2", "H3"],
                         [r["id"] for r in json.loads(more["profile_text"])["house_rules"]])


@needs_the_generator
class TheProfileIsWrittenOnALocalServerTest(unittest.TestCase):
    """`out` turns a confirmed profile into a file: whole or not at all, and never
    over somebody's later edit."""

    def setUp(self):
        import pathlib
        import tempfile

        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)
        self.out = self.dir / "owner-profile.json"

    def ask(self, rules=None, **arguments) -> dict:
        return call("scxml_house_rule", rules=rules or [IGNORE], owner_words_text=WORDS,
                    owner_confirmed=True, out=str(self.out), **arguments)

    def saved(self, **arguments) -> dict:
        result = self.ask(**arguments)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        return json.loads(result["content"][0]["text"])

    def refused(self, said: str, rules=None, **arguments) -> str:
        result = self.ask(rules, **arguments)
        self.assertTrue(result.get("isError"), result["content"][0]["text"])
        text = result["content"][0]["text"]
        self.assertIn(said, text)
        return text

    def test_a_new_profile_is_written_and_the_answer_says_where_and_what(self):
        import hashlib

        reply = self.saved()
        self.assertEqual("saved", reply["status"])
        self.assertNotIn("profile_text", reply)
        written = self.out.read_bytes()
        self.assertEqual({"path": str(self.out), "sha256": hashlib.sha256(written).hexdigest(),
                          "previous_sha256": None, "rules": ["H1"]}, reply["saved"])
        self.assertEqual("relayed", json.loads(written)["house_rules"][0]["confirmation"])
        self.assertEqual([], [p.name for p in self.dir.iterdir() if p.name.startswith(".")],
                         "no temporary file is left behind")

    def test_a_rule_joins_the_file_that_is_there_when_it_is_given_as_the_profile(self):
        import hashlib

        self.saved()
        before = self.out.read_bytes()
        result = call("scxml_house_rule", rules=[OPEN], owner_words_text=WORDS,
                      owner_confirmed=True, out=str(self.out), profile=str(self.out))
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        reply = json.loads(result["content"][0]["text"])
        self.assertEqual(hashlib.sha256(before).hexdigest(), reply["saved"]["previous_sha256"])
        self.assertEqual(["H1", "H2"],
                         [r["id"] for r in json.loads(self.out.read_bytes())["house_rules"]])

    def test_a_file_that_is_there_is_not_replaced_unless_it_was_given(self):
        self.saved()
        before = self.out.read_bytes()
        self.refused("already exists")
        self.assertEqual(before, self.out.read_bytes())

    def test_an_edit_made_since_the_profile_was_read_is_not_overwritten(self):
        self.saved()
        read = self.dir / "read.json"
        read.write_bytes(self.out.read_bytes())
        edited = self.out.read_bytes().replace(b"ignored", b"dropped")
        self.out.write_bytes(edited)
        # A NEW rule, so the refusal is about the file and not about a repeat.
        self.refused("changed since you read it", rules=[OPEN], profile=str(read))
        self.assertEqual(edited, self.out.read_bytes())

    def test_a_request_repeated_after_success_adds_no_second_revision(self):
        self.saved()
        before = self.out.read_bytes()
        self.refused("already says", profile=str(self.out))
        self.assertEqual(before, self.out.read_bytes())

    def test_a_file_with_windows_line_ends_is_compared_by_its_bytes(self):
        self.saved()
        crlf = self.out.read_bytes().replace(b"\n", b"\r\n")
        self.out.write_bytes(crlf)
        result = call("scxml_house_rule", rules=[OPEN], owner_words_text=WORDS,
                      owner_confirmed=True, out=str(self.out), profile=str(self.out))
        self.assertFalse(result.get("isError"), result["content"][0]["text"])

    def test_out_goes_with_the_owners_yes_and_a_missing_directory_is_a_sentence(self):
        result = call("scxml_house_rule", rules=[IGNORE], out=str(self.out))
        self.assertTrue(result.get("isError"))
        self.assertIn("owner_confirmed", result["content"][0]["text"])
        self.out = self.dir / "nowhere" / "profile.json"
        self.refused("is not a directory")

    def test_a_remote_caller_cannot_name_a_path(self):
        result = mcp.call_tool("scxml_house_rule",
                               {"rules": [IGNORE], "owner_confirmed": True,
                                "out": str(self.out)}, remote=True)
        self.assertTrue(result.get("isError"))
        self.assertFalse(self.out.exists())


if __name__ == "__main__":
    unittest.main()
