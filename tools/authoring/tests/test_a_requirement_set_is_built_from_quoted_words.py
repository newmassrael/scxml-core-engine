"""A requirement set is built from the words a client QUOTES out of the source.

The product consumes a closed requirement set and never derives one, so the
authoring pass builds it. What can be decided without trusting the client is
decided here: a quote must be in the source word for word and point at ONE
place, the ids come from where the words sit, and every sentence of the source
is a section so a sentence no requirement quotes shows as an empty one.

Measured 2026-10-01: five independent extractions of one specification agreed
on eleven of twelve quotes, and the drafts of it fell into five classes. What a
specification asks for converges where what a draft does with it does not.
"""

from __future__ import annotations

import io
import json
import pathlib
import tempfile
import unittest

from sce_author import mcp, requirement_set as rs
from sce_author.verify import _default_codegen

SPEC = ("The lamp starts off. Pressing the switch turns it on; pressing it "
        "again turns it off. After 30 seconds on, it turns itself off. "
        "Nothing else changes it.")

ALL = [
    {"quote": "The lamp starts off.", "statement": "initial state is off"},
    {"quote": "Pressing the switch turns it on", "statement": "switch turns on"},
    {"quote": "pressing it again turns it off", "statement": "switch turns off"},
    {"quote": "After 30 seconds on, it turns itself off.", "statement": "timeout"},
    {"quote": "Nothing else changes it.", "statement": "no other change",
     "modality": "shall_not"},
]


def call(name: str, **arguments) -> dict:
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}
    output = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
    return json.loads(output.getvalue())["result"]


class AQuoteIsAnchoredOrRefused(unittest.TestCase):
    def test_a_list_of_true_quotes_is_built_with_ids_from_reading_order(self):
        built = rs.build(SPEC, ALL)
        self.assertEqual([], built.refused)
        self.assertEqual(["R1", "R2", "R3", "R4", "R5"], [r.id for r in built.requirements])
        self.assertEqual(["S1", "S2", "S2", "S3", "S4"],
                         [r.sentence for r in built.requirements])

    def test_the_ids_do_not_depend_on_the_order_the_client_listed_them_in(self):
        forward = rs.build(SPEC, ALL)
        backward = rs.build(SPEC, list(reversed(ALL)))
        self.assertEqual([(r.id, r.quote) for r in forward.requirements],
                         [(r.id, r.quote) for r in backward.requirements])
        self.assertEqual(forward.manifest, backward.manifest)

    def test_a_quote_not_in_the_source_word_for_word_is_refused(self):
        items = ALL[:1] + [{"quote": "The lamp is off at the start",
                            "statement": "a rewording"}]
        built = rs.build(SPEC, items)
        self.assertEqual([1], [r.index for r in built.refused])
        self.assertIn("word for word", built.refused[0].why)
        # Nothing is offered while any quote cannot be anchored.
        self.assertIsNone(built.manifest)
        self.assertIsNone(built.sidecar)

    def test_a_quote_that_points_at_two_places_is_refused(self):
        # "turns it" is in "turns it on" and in "turns it off"; "turns it off" is
        # in the source once and is not ambiguous, which the control holds.
        built = rs.build(SPEC, [{"quote": "turns it", "statement": "a turn"}])
        self.assertEqual(1, len(built.refused))
        self.assertIn("more than once", built.refused[0].why)
        control = rs.build(SPEC, [{"quote": "turns it off", "statement": "off"}])
        self.assertEqual([], control.refused)

    def test_the_same_quote_twice_is_one_requirement_once(self):
        built = rs.build(SPEC, [ALL[0], ALL[0]])
        self.assertEqual([1], [r.index for r in built.refused])
        self.assertIn("same quote", built.refused[0].why)

    def test_every_refusal_is_listed_at_once(self):
        built = rs.build(SPEC, [
            {"quote": "not there", "statement": "x"},
            {"quote": "turns it", "statement": "y"},
            {"quote": "The lamp starts off.", "statement": "fine"},
            {"quote": "also not there", "statement": "z"}])
        self.assertEqual([0, 1, 3], [r.index for r in built.refused])

    def test_a_quote_copied_across_a_line_break_is_the_same_words(self):
        wrapped = "The lamp starts\noff. Pressing the switch turns it on."
        built = rs.build(wrapped, [{"quote": "The lamp   starts off.", "statement": "s"}])
        self.assertEqual([], built.refused)

    def test_a_modality_the_product_does_not_know_is_refused(self):
        built = rs.build(SPEC, [{"quote": "The lamp starts off.", "statement": "s",
                                 "modality": "should"}])
        self.assertIn("modality", built.refused[0].why)

    def test_a_request_that_is_no_list_of_quotes_is_an_error_not_a_refusal(self):
        for bad in (None, [], "quotes", {"quote": "x"}):
            with self.subTest(bad=bad):
                with self.assertRaises(rs.RequirementSetError):
                    rs.build(SPEC, bad)
        with self.assertRaises(rs.RequirementSetError):
            rs.build("   ", ALL)


class TheSourceChecksTheList(unittest.TestCase):
    """A sentence no requirement quotes is the one completeness check that
    something other than the list's author can make."""

    def test_a_sentence_no_quote_touches_is_listed(self):
        built = rs.build(SPEC, [i for i in ALL if not i["quote"].startswith("After")])
        self.assertEqual([("S3", "After 30 seconds on, it turns itself off.")],
                         built.unclaimed_sentences)

    def test_words_left_over_inside_a_claimed_sentence_are_listed_apart(self):
        built = rs.build(SPEC, [{"quote": "starts off", "statement": "s"}] + ALL[1:])
        self.assertEqual([], built.unclaimed_sentences)
        self.assertEqual([("S1", "The lamp")], built.unclaimed_words)

    def test_a_list_that_covers_the_source_leaves_nothing(self):
        built = rs.build(SPEC, ALL)
        self.assertEqual(([], []), (built.unclaimed_sentences, built.unclaimed_words))
        self.assertEqual(4, sum(1 for _ in built.manifest["sections"]))


class TheFilesAreWhatTheProductReads(unittest.TestCase):
    def test_the_manifest_is_coordinates_only_and_says_it_is_synthesized(self):
        manifest = rs.build(SPEC, ALL).manifest
        self.assertEqual({"ids": "synthesized", "trace": "none",
                          "modality_convention": "english-modal-verbs",
                          "method": "ai-pass-1"}, manifest["extraction"])
        # No sentence of the specification is in the manifest: the product
        # refuses a manifest with prose in any string, and the sidecar is where
        # the words go.
        self.assertNotIn("lamp", json.dumps(manifest))
        self.assertNotIn("switch", json.dumps(manifest))

    def test_the_sidecar_holds_the_quote_and_never_the_paraphrase(self):
        sidecar = rs.build(SPEC, ALL).sidecar
        self.assertEqual("The lamp starts off.", sidecar["text"]["R1"])
        self.assertNotIn("initial state is off", json.dumps(sidecar))

    def test_a_shall_not_requirement_says_so_in_the_manifest(self):
        entries = {e["id"]: e for e in rs.build(SPEC, ALL).manifest["requirements"]}
        self.assertEqual("shall_not", entries["R5"]["modality"])
        self.assertNotIn("modality", entries["R1"])

    def test_the_answer_is_refused_without_files_and_built_with_them(self):
        refused = rs.answer(rs.build(SPEC, [{"quote": "nope", "statement": "x"}]))
        self.assertEqual("refused", refused["verdict"])
        self.assertNotIn("manifest_text", refused)
        built = rs.answer(rs.build(SPEC, ALL))
        self.assertEqual("built", built["verdict"])
        self.assertIn("synthesized", built["basis"])
        self.assertEqual(len(ALL), len(built["requirements"]))
        json.loads(built["manifest_text"])
        json.loads(built["sidecar_text"])

    def test_what_comes_next_names_the_files_to_save_and_the_checks_that_take_the_list(self):
        # Measured 2026-10-01: no client saved the list or its sidecar (0 of 20
        # Sonnet runs, and a GPT run saved the list and not the sidecar), so the
        # ids in a design meant nothing once the conversation ended; and the
        # old text named two tools and not the checks, so the list was never
        # given to the one a design with companion files goes through.
        built = rs.answer(rs.build(SPEC, ALL))
        said = built["next"]
        for name in ("requirements.manifest.json", "requirements.sidecar.json",
                     "validate_scxml", "validate_scxml_set", "scxml_acceptance_report"):
            self.assertIn(name, said)
        self.assertIn("exactly as returned", said)
        # A client that cannot write files is told what to do instead.
        self.assertIn("cannot write files", said)

    def test_a_refusal_does_not_tell_the_client_to_save_a_list_there_is_not(self):
        refused = rs.answer(rs.build(SPEC, [{"quote": "nope", "statement": "x"}]))
        self.assertNotIn("requirements.manifest.json", refused["next"])


class TheToolTakesTheSpecificationAsTextOrPath(unittest.TestCase):
    def test_the_tool_is_offered_with_its_two_forms_of_the_specification(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "scxml_requirement_set")
        properties = tool["inputSchema"]["properties"]
        self.assertIn("specification", properties)
        self.assertIn("specification_text", properties)
        self.assertEqual(["requirements"], tool["inputSchema"]["required"])
        self.assertIn("word for word", tool["description"])

    def test_a_specification_handed_as_text_is_built(self):
        result = call("scxml_requirement_set", specification_text=SPEC, requirements=ALL)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("built", answer["verdict"])
        self.assertEqual(["R1", "R2", "R3", "R4", "R5"],
                         [r["id"] for r in answer["requirements"]])

    def test_a_specification_handed_as_a_path_is_built(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "lamp.md"
            path.write_text(SPEC, encoding="utf-8")
            result = call("scxml_requirement_set", specification=str(path),
                          requirements=ALL, doc_id="lamp")
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("built", answer["verdict"])
        self.assertEqual("lamp", json.loads(answer["manifest_text"])["doc_id"])

    def test_a_refusal_is_an_answer_the_client_can_act_on_not_an_error(self):
        result = call("scxml_requirement_set", specification_text=SPEC,
                      requirements=[{"quote": "not there", "statement": "x"}])
        self.assertFalse(result.get("isError"))
        answer = json.loads(result["content"][0]["text"])
        self.assertEqual("refused", answer["verdict"])
        self.assertEqual(0, answer["refused"][0]["item"])

    def test_a_missing_list_or_a_bad_name_is_an_argument_error(self):
        for arguments in ({"specification_text": SPEC},
                          {"specification_text": SPEC, "requirements": ALL,
                           "doc_id": "has space"}):
            with self.subTest(arguments=sorted(arguments)):
                self.assertTrue(call("scxml_requirement_set", **arguments).get("isError"))


LAMP = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="lamp" initial="off">
  <state id="off" sce:req="R1">
    <transition event="press" target="on" sce:req="R2"/>
  </state>
  <state id="on">
    <transition event="press" target="off" sce:req="R3"/>
    <transition event="hum" target="on"/>
  </state>
</scxml>
"""


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class TheProductReadsThemAndFindsWhatTheDesignLeaves(unittest.TestCase):
    """The end of the road, through the real product: a draft that carries
    three of the five requirements, and one element nobody asked for."""

    def report(self):
        built = call("scxml_requirement_set", specification_text=SPEC, requirements=ALL,
                     doc_id="lamp")
        answer = json.loads(built["content"][0]["text"])
        page = call("scxml_acceptance_report", document_text=LAMP,
                    manifest_text=answer["manifest_text"],
                    sidecar_text=answer["sidecar_text"], variant="base")
        self.assertFalse(page.get("isError"), page["content"][0]["text"])
        return page["content"][0]["text"]

    def test_each_requirement_shows_its_sentence_beside_what_the_design_does(self):
        text = self.report()
        self.assertIn("says: Pressing the switch turns it on", text)
        self.assertIn("Cited              off | press", text)

    def test_a_requirement_no_element_carries_is_missing_and_says_so(self):
        text = self.report()
        self.assertIn("missing        R4", text)
        self.assertNotIn("missing        R1", text)

    def test_a_requirement_met_by_absence_is_not_called_missing(self):
        """R5 is "Nothing else changes it", passed as `shall_not`. No element
        carries it either, and the product does not call it missing: an
        annotation on the one handler that IS allowed says nothing about the
        one that must not exist, so it asks for a scenario instead. Passing the
        modality through is what earns the distinction; left out, R5 would read
        `missing` beside R4 as though they were the same failure."""
        text = self.report()
        self.assertIn("needs-scenario R5", text)
        self.assertNotIn("missing        R5", text)

    def test_an_element_nobody_asked_for_is_counted(self):
        self.assertIn("unclaimed", self.report())

    def test_a_sentence_no_requirement_quotes_is_the_source_coverage_row_at_zero(self):
        built = call("scxml_requirement_set", specification_text=SPEC,
                     requirements=[i for i in ALL if not i["quote"].startswith("After")],
                     doc_id="lamp")
        answer = json.loads(built["content"][0]["text"])
        self.assertEqual("S3", answer["unclaimed_sentences"][0]["sentence"])
        page = call("scxml_acceptance_report", document_text=LAMP,
                    manifest_text=answer["manifest_text"],
                    sidecar_text=answer["sidecar_text"], variant="base")
        # The product's own source-coverage section names the empty division.
        self.assertRegex(page["content"][0]["text"], r"S3\s+0")


if __name__ == "__main__":
    unittest.main()
