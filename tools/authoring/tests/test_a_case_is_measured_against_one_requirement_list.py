"""The reproducibility harness measures every draft of a case against ONE list.

Measured 2026-10-01: a client left to number its own `sce:req` made the ids up
in twelve drafts of fifteen, so drafts counted against their own authors' ids
share no denominator. The harness therefore gives each case the owner's list,
made once, copies it beside the specification under fixed names, and scores each
draft by the product's own `requirements` records. These tests hold the harness
to that: a figure about dangling ids that came from a harness which read the
document itself, or counted an unwritten draft as a draft that claimed nothing,
would be a number about nothing.
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import sys
import tempfile
import unittest

from sce_author import mcp
from sce_author.verify import _default_codegen

EVAL = pathlib.Path(__file__).resolve().parent.parent / "eval"


def _load(name: str):
    spec = importlib.util.spec_from_file_location(name, EVAL / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    sys.path.insert(0, str(EVAL))
    try:
        spec.loader.exec_module(module)
    finally:
        sys.path.remove(str(EVAL))
    return module


kind_choice = _load("kind_choice")
reproducibility = _load("reproducibility")

SPEC = ("The lamp starts off. Pressing the switch turns it on; pressing it "
        "again turns it off. After 30 seconds on, it turns itself off. "
        "Nothing else changes it.")

QUOTES = [
    {"quote": "The lamp starts off.", "statement": "initial state is off"},
    {"quote": "Pressing the switch turns it on", "statement": "switch turns on"},
    {"quote": "pressing it again turns it off", "statement": "switch turns off"},
    {"quote": "After 30 seconds on, it turns itself off.", "statement": "timeout"},
    {"quote": "Nothing else changes it.", "statement": "no other change",
     "modality": "shall_not"},
]

LAMP = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" datamodel="ecmascript" name="lamp" initial="off">
  <state id="off" sce:req="R1">
    <transition event="press" target="on" sce:req="R2"/>
  </state>
  <state id="on" sce:req="R2">
    <transition event="press" target="off" sce:req="R99"/>
  </state>
</scxml>
"""

CASE = {"id": "lamp", "kind": "statechart", "prose": SPEC}


def the_owners_list(directory: pathlib.Path) -> pathlib.Path:
    """The list as `scxml_requirement_set` makes it, filed as the harness reads it."""
    made = json.loads(mcp.call_tool("scxml_requirement_set", {
        "specification_text": SPEC, "requirements": QUOTES, "doc_id": "lamp",
    })["content"][0]["text"])
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "lamp.manifest_text.json").write_text(made["manifest_text"], encoding="utf-8")
    (directory / "lamp.sidecar_text.json").write_text(made["sidecar_text"], encoding="utf-8")
    return directory


def stand_in_client(document: str | None) -> list[str]:
    """A client that writes `document` as the draft, as a real one would, and
    says the request it was given, which is what the transcript then holds."""
    write = ("" if document is None
             else f"pathlib.Path('draft.scxml').write_text({document!r}); ")
    return [sys.executable, "-c", f"import pathlib, sys; {write}print(sys.argv[1:])",
            "{prompt}"]


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class EachDraftIsScoredAgainstTheOwnersList(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.dir = pathlib.Path(self._tmp.name)
        self.list = the_owners_list(self.dir / "lists")

    def run_lamp(self, document: str | None) -> dict:
        out = self.dir / ("run-" + str(len(list(self.dir.glob("run-*")))))
        out.mkdir()
        return kind_choice.run_case(CASE, stand_in_client(document), out, timeout=60,
                                    model=None, requirements=self.list)

    def test_the_list_is_filed_beside_the_specification_and_the_request_says_where(self):
        result = self.run_lamp(LAMP)
        work = self.dir / "run-0" / "lamp"
        for given, filed in (("lamp.manifest_text.json", "requirements.manifest.json"),
                             ("lamp.sidecar_text.json", "requirements.sidecar.json")):
            self.assertEqual((self.list / given).read_bytes(), (work / filed).read_bytes())
        self.assertIn(kind_choice.MANIFEST_REQUEST, (work / "transcript.txt").read_text())
        self.assertIn("requirements", result)

    def test_the_outcomes_are_the_products_and_a_dangling_id_is_named(self):
        found = self.run_lamp(LAMP)["requirements"]
        # R1 and R2 are claimed, R99 is cited and not in the list, and R3 (the
        # switch turning it off) is carried by nothing now.
        self.assertEqual(["R1", "R2"], found["ids"]["implemented"])
        self.assertEqual(["R99"], found["ids"]["dangling"])
        self.assertIn("R3", found["ids"]["missing"])
        # Only a requirement the design implements has a part that carries it:
        # the dangling id's node is in the product's records, not counted as
        # one of the owner's requirements being met.
        self.assertEqual(["R1", "R2"], sorted(found["node_paths"]))
        # R2 is claimed twice, on a transition and on a state, and both are named.
        self.assertEqual(["states.off.transitions[0]", "states.on"], found["node_paths"]["R2"])

    def test_a_draft_that_was_not_written_is_none_and_not_a_draft_that_claimed_nothing(self):
        self.assertIsNone(self.run_lamp(None)["requirements"])

    def test_without_a_list_the_result_is_what_it_was(self):
        out = self.dir / "plain"
        out.mkdir()
        result = kind_choice.run_case(CASE, stand_in_client(LAMP), out, timeout=60, model=None)
        self.assertNotIn("requirements", result)
        self.assertEqual([], sorted(p.name for p in (out / "lamp").glob("requirements.*")))


class TheAgreementIsCountedOverMeasuredDrafts(unittest.TestCase):
    def draft(self, implemented: dict, *, missing=(), dangling=()) -> dict:
        ids = {"implemented": sorted(implemented)}
        if missing:
            ids["missing"] = sorted(missing)
        if dangling:
            ids["dangling"] = sorted(dangling)
        return {"ids": ids, "node_paths": implemented}

    def test_dangling_and_missing_are_counted_by_draft(self):
        # Unequal on purpose: one draft with a dangling id, two with a missing
        # one, so a count of the wrong outcome is a different number.
        agreement = reproducibility.requirement_agreement({
            "rep1": self.draft({"R1": ["states.a"]}),
            "rep2": self.draft({"R1": ["states.a"]}, missing=["R2"]),
            "rep3": self.draft({"R1": ["states.a"]}, dangling=["R9"]),
            "rep4": self.draft({"R1": ["states.a"]}, missing=["R3"]),
        })
        self.assertEqual(4, agreement["measured"])
        self.assertEqual(3, agreement["no_dangling"])
        self.assertEqual(2, agreement["none_missing"])
        self.assertEqual({"rep3": ["R9"]}, agreement["dangling"])
        self.assertEqual({"rep2": ["R2"], "rep4": ["R3"]}, agreement["missing"])

    def test_a_draft_the_product_could_not_measure_is_named_and_counted_nowhere(self):
        agreement = reproducibility.requirement_agreement({
            "rep1": self.draft({"R1": ["states.a"]}),
            "rep2": {"refused": "the document is not well formed"},
            "rep3": None,
        })
        self.assertEqual(1, agreement["measured"])
        self.assertEqual(["rep2", "rep3"], agreement["unmeasured"])
        # An unreadable draft is not one that left nothing dangling.
        self.assertEqual(1, agreement["no_dangling"])

    def test_the_parts_of_the_design_that_carry_a_requirement_are_counted_distinct(self):
        agreement = reproducibility.requirement_agreement({
            "rep1": self.draft({"R1": ["states.a"], "R10": ["states.b"]}),
            "rep2": self.draft({"R1": ["states.a"], "R10": ["states.c"]}),
            "rep3": self.draft({"R1": ["states.a"]}),
        })
        self.assertEqual({"R1": 1, "R10": 2}, agreement["distinct_node_paths"])
        self.assertEqual({"R1": 3, "R10": 2}, agreement["implemented_by"])

    def test_the_ids_are_in_the_order_the_owner_numbered_them(self):
        agreement = reproducibility.requirement_agreement({
            "rep1": self.draft({"R10": ["x"], "R2": ["y"], "R1": ["z"]}),
        })
        self.assertEqual(["R1", "R2", "R10"], list(agreement["implemented_by"]))
        self.assertEqual(["R1", "R2", "R10"], list(agreement["distinct_node_paths"]))


if __name__ == "__main__":
    unittest.main()
