"""A guess written where no value rests on it is listed as unplaced, not left out.

A guess is credited or blamed through the values that read it, so a mark on a `<data>` can be
refuted or held. A mark on a state or a region has no value behind it: no case can blame it and
no alternative can be run in its place. Measured 2026-10-10, three of five writers given one
specification and its one picture put the reading of that picture on such an element, and `gaps`
named none of them -- a document with a guess in it read, in the report, like one with none.

Asserted here:

    a mark on an element that holds no value comes back, as unplaced
    the author is not asked for candidates it cannot be tried with
    a mark on a `<data>` stays what it was, and is not listed twice
    the report puts it after the untested and before what is held
    the core's reading of a document's marks is the product's: same ids, same reasons,
        same candidates, on the attribute form and the child-element form     (parity)
"""

from __future__ import annotations

import json
import subprocess
import unittest

import yaml

from sce_author.check import read_document
from sce_author.gaps import report
from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import Fixture

DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="placed"
       sce:assumed="ORDER" sce:assumed-reason="the drawing shows no limit between them"
       sce:assumed-candidates="none limit">
  <datamodel sce:assumed="READING"
             sce:assumed-reason="read from a drawing: the later input decides"
             sce:assumed-candidates="later earlier">
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          sce:assumed="lamp-threshold"
          sce:assumed-reason="no threshold is given; read as above zero"
          sce:assumed-candidates="1 0"
          expr="count &gt; 0 ? 1 : 0"/>
  </datamodel>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "placed.scxml",
    "inputs": {"count": {"address": "Plant.Input.Count", "when_absent": 0}},
    "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Stat",
                         "map": {0: "OFF", 1: "ON"}}},
}

CASE = {"name": "two counted",
        "given": {"Plant.Input.Count": 2}, "drove": ["Plant.Input.Count"],
        "expect": {"Plant.Out.Lamp.Stat": "ON"}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class AGuessOnAnElementWithNoValueIsStillListed(Fixture):
    def setUp(self):
        super().setUp()
        (self.root / "placed.scxml").write_text(DOCUMENT, encoding="utf-8")
        self.binding_path = self.root / "placed.binding.yaml"
        self.binding_path.write_text(yaml.safe_dump(BINDING), encoding="utf-8")
        (self.pack_dir / "examples.yaml").write_text(yaml.safe_dump({
            "version": 1, "origin": "written for this test",
            "independent_cases": True, "cases": [CASE]}), encoding="utf-8")
        self.result = verify(self.pack(), self.binding_path)
        self.assertTrue(self.result.ran, self.result.refusal)

    def unplaced(self):
        return {a.marker: a for a in self.result.assumptions.values() if a.status == "unplaced"}

    def test_a_mark_on_an_element_that_holds_no_value_is_listed_as_unplaced(self):
        found = self.unplaced()
        self.assertEqual({"READING", "ORDER"}, set(found))
        self.assertEqual("datamodel", found["READING"].subject)
        self.assertEqual(("later", "earlier"), found["READING"].candidates)
        self.assertEqual("the drawing shows no limit between them", found["ORDER"].reason)
        self.assertEqual([], found["READING"].positions)

    def test_a_mark_on_a_data_element_is_what_it_was_and_is_not_listed_twice(self):
        lamp = self.result.assumptions["document:lamp"]
        self.assertEqual(("held", True), (lamp.status, lamp.placed))
        about_lamp = [a for a in self.result.assumptions.values()
                      if a.marker == "lamp-threshold"]
        self.assertEqual(1, len(about_lamp))

    def test_the_gap_report_carries_it_with_its_fix_and_asks_nothing_of_the_author(self):
        gaps = report(self.result, self.pack())
        placed = [g for g in gaps if g.kind == "unplaced"]
        self.assertEqual({"READING", "ORDER"}, {g.marker for g in placed})
        for gap in placed:
            self.assertIn("`<data>`", gap.fix)
            self.assertEqual("", gap.ask)
        kinds = [g.kind for g in gaps]
        self.assertLess(kinds.index("unplaced"), kinds.index("held"))

    def test_the_cores_reading_of_the_marks_is_the_products(self):
        """Parity. The product is the authority on what a mark is; the core reads the same
        document itself so that `check` needs no built generator, and this is what keeps the
        two from drifting."""
        done = subprocess.run([str(_default_codegen()), "unresolved", str(self.root / "placed.scxml")],
                              capture_output=True, text=True, check=True)
        product = sorted((r["id"], r.get("reason") or "", tuple(r.get("candidates") or ()))
                         for r in map(json.loads, done.stdout.splitlines()) if r["kind"] == "assumed")
        core = sorted((m.marker, m.reason, m.candidates)
                      for m in read_document(self.root / "placed.scxml").marks)
        self.assertEqual(product, core)
        self.assertEqual(3, len(core))

    def test_the_same_holds_for_a_state_and_for_the_child_element_form(self):
        statechart = self.root / "chart.scxml"
        statechart.write_text(
            '<?xml version="1.0" encoding="UTF-8"?>\n'
            '<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"\n'
            '       version="1.0" initial="a" name="chart">\n'
            '  <state id="a" sce:assumed="ON_STATE" sce:assumed-reason="r1"\n'
            '         sce:assumed-candidates="p q">\n'
            '    <sce:assumed id="AS_CHILD" reason="r2" candidates="x y z"/>\n'
            '    <transition event="go" target="b"/>\n'
            '  </state>\n'
            '  <state id="b"/>\n'
            '</scxml>\n', encoding="utf-8")
        done = subprocess.run([str(_default_codegen()), "unresolved", str(statechart)],
                              capture_output=True, text=True, check=True)
        product = sorted((r["id"], r.get("reason") or "", tuple(r.get("candidates") or ()))
                         for r in map(json.loads, done.stdout.splitlines()) if r["kind"] == "assumed")
        core = sorted((m.marker, m.reason, m.candidates) for m in read_document(statechart).marks)
        self.assertEqual(product, core)
        self.assertEqual(["AS_CHILD", "ON_STATE"], [row[0] for row in core])


if __name__ == "__main__":
    unittest.main()
