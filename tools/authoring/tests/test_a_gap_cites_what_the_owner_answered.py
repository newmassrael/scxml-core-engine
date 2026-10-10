"""A mark that applies an answer the owner gave is not a gap.

`gaps` lists a mark written on an element no value rests on as `unplaced`: nothing can try it. A
mark that cites the owner's decision (`sce:assumed="D1"`) or a house rule of the profile
(`sce:assumed="H1"`) is not a guess to be tried but a standing answer applied, and listing it with
the guesses sends the owner to settle what they already settled. The product says which markers
cite a house rule (`sce-codegen unresolved --profile`) and the record says which ids it holds; the
core reads both and does not decide what a house rule is.

Asserted here:

    the ids the owner answered are the record's and the product's house-rule marks
    a cited mark on a state is `cited`, and a guess beside it stays `unplaced`  (the discriminator)
    given nothing, no mark is cited
    the command and the tool take the record and the profile
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.__main__ import main
from sce_author.decisions import cited_by_binding, cited_markers
from sce_author.gaps import report
from sce_author.mcp import call_tool
from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark" sce:kind="statechart">
  <state id="dark">
    <transition event="train.approaching" target="flashing" sce:assumed="D1"/>
    <transition event="train.cleared" target="dark" sce:assumed="H1"/>
    <transition event="train.occupied" target="flashing" sce:assumed="GUESS"
                sce:assumed-reason="nobody said"/>
  </state>
  <state id="flashing">
    <onentry><send event="signal.flashing" type="x-sce-host"/></onentry>
  </state>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "signal.scxml",
    "inputs": {"approaching": {"address": "plant/in/train-approach", "becomes": "APPROACHING",
                               "event": "train.approaching"}},
    "outputs": {"roadSignal": {"address": "plant/out/road-signal", "field": "value",
                               "sent": {"processor": "x-sce-host"},
                               "when_nothing_sent": "signal.dark",
                               "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"}}},
}

EXAMPLES = {"version": 1, "origin": "written for this test", "independent_cases": True,
            "ordered": True,
            "cases": [{"name": "a train approaches",
                       "given": {"plant/in/train-approach": "APPROACHING"},
                       "drove": ["plant/in/train-approach"],
                       "expect": {"plant/out/road-signal.value": "FLASHING"}}]}

RECORD = {"record": "sce-decision-record", "v": 1, "specification": {"doc_id": "door-spec"},
          "decisions": [{"id": "D1", "question": "Does an approaching train flash?",
                         "answer": "Yes."}]}
PROFILE = {"record": "sce-authoring-profile", "v": 1,
           "house_rules": [{"id": "H1", "rule": "An event a state does not mention is ignored."}]}


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class AGapCitesWhatTheOwnerAnswered(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump(EXAMPLES), encoding="utf-8")
        (self.tmp / "signal.scxml").write_text(DOCUMENT, encoding="utf-8")
        self.binding = self.tmp / "b.yaml"
        self.binding.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        self.record = self.tmp / "decisions.json"
        self.record.write_text(json.dumps(RECORD), encoding="utf-8")
        self.profile = self.tmp / "profile.json"
        self.profile.write_text(json.dumps(PROFILE), encoding="utf-8")
        self.pack = load_pack(self.tmp)
        self.result = verify(self.pack, self.binding)
        self.assertTrue(self.result.ran, self.result.refusal)

    def kinds(self, cited=frozenset()):
        return {g.marker: g.kind for g in report(self.result, self.pack, None, (), None, cited)
                if g.marker}

    def test_the_ids_are_the_records_and_the_products_mark_for_a_house_rule(self):
        cited, refused = cited_by_binding(self.binding, self.record, self.profile)
        self.assertEqual(("", {"D1", "H1"}), (refused, set(cited)))

    def test_with_only_the_record_a_house_rule_is_not_yet_a_citation(self):
        cited, _ = cited_markers(self.tmp / "signal.scxml", self.record, None)
        self.assertEqual({"D1"}, set(cited))

    def test_a_cited_mark_is_cited_and_a_guess_beside_it_is_unplaced(self):
        """The discriminator: without it every unplaced mark would be called cited."""
        cited, _ = cited_by_binding(self.binding, self.record, self.profile)
        self.assertEqual({"D1": "cited", "H1": "cited", "GUESS": "unplaced"}, self.kinds(cited))

    def test_given_nothing_no_mark_is_cited(self):
        self.assertEqual({"D1": "unplaced", "H1": "unplaced", "GUESS": "unplaced"}, self.kinds())

    def test_the_cited_ones_come_last_and_say_there_is_nothing_to_settle(self):
        cited, _ = cited_by_binding(self.binding, self.record, self.profile)
        gaps = report(self.result, self.pack, None, (), None, cited)
        self.assertEqual("cited", gaps[-1].kind)
        self.assertIn("nothing here to settle", gaps[-1].fix)

    def test_the_command_takes_the_record_and_the_profile(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            ran = main(["gaps", "--pack", str(self.tmp), "--binding", str(self.binding),
                        "--decisions", str(self.record), "--profile", str(self.profile)])
        self.assertEqual(0, ran)
        text = out.getvalue()
        self.assertIn("CITED", text)
        self.assertEqual(2, text.count("CITED"))
        self.assertIn("UNPLACED", text)

    def test_the_tool_takes_them_too(self):
        answer = call_tool("gaps", {"pack": str(self.tmp), "binding": str(self.binding),
                                    "decisions": str(self.record), "profile": str(self.profile)})
        self.assertFalse(answer.get("isError"), answer)
        payload = json.loads(answer["content"][0]["text"])
        self.assertEqual(2, payload["counts"]["cited"])
        self.assertEqual(1, payload["counts"]["unplaced"])


if __name__ == "__main__":
    unittest.main()
