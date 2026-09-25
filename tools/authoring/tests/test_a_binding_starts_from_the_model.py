"""A binding can start from what the interface model already decides.

Half of a binding is copying: which positions exist, their fields, their value
spaces. Writers that failed on this corpus failed on that half -- a missing
`version`, an output rule with no `field` -- and never reached the half that
reads the specification. `scaffold` writes the copied half and must write
nothing else, because a tool that decided which condition gives which value
would be the mechanical translator this core refuses to be
(`test_the_server_offers_no_way_to_write_a_document`).
"""

from __future__ import annotations

import io
import json
import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author import mcp
from sce_author.check import check, read_binding
from sce_author.pack import load_pack
from sce_author.scaffold import ScaffoldError, draft, identifier, write

HERE = pathlib.Path(__file__).resolve().parent
PACK = HERE / "fixtures" / "crossing"

# Every key that states a reading of the specification or of the deployment.
# The skeleton may hold none of them: each is a decision the writer makes.
DECISION_KEYS = {"equals", "equals_any", "not_equals", "becomes", "event",
                 "absent", "when_absent", "range", "protocol", "parameters",
                 "previous_of", "state_of", "clock", "variant_is", "when",
                 "also", "sent", "when_nothing_sent", "hold_last", "internal",
                 "unresolved", "assumed", "initial"}


class ABindingStartsFromTheModel(unittest.TestCase):
    def setUp(self):
        self.pack = load_pack(PACK)
        self.dir = pathlib.Path(tempfile.mkdtemp(prefix="sce_scaffold_test_"))
        shutil.copy(PACK / "controller.scxml", self.dir / "controller.scxml")
        self.out = self.dir / "controller.binding.yaml"

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def skeleton(self, activation=None) -> dict:
        write(self.pack, "controller.scxml", self.out, activation)
        return yaml.safe_load(self.out.read_text(encoding="utf-8"))

    def test_the_skeleton_is_a_binding_the_checker_can_read(self):
        """The whole point: every gap left comes back as a refusal about a
        rule, never as a file `check` cannot open."""
        self.skeleton("on-change")
        read_binding(self.out)                  # schema-valid, or this raises
        findings = check(self.pack, self.out)
        self.assertTrue(findings, "the skeleton decides nothing, so check must "
                                  "have something to say")
        rules = set(yaml.safe_load(self.out.read_text())["outputs"])
        named = {f.where.split(" ", 1)[1] for f in findings if f.where.startswith("output ")}
        self.assertTrue(named & rules, "refusals should name the skeleton's rules")

    def test_every_output_position_is_written_once_with_its_field(self):
        got = self.skeleton()["outputs"]
        positions = [(r["address"], r.get("field", "")) for r in got.values()]
        expected = [(e.address, f.name) for e in self.pack.model.outputs() for f in e.fields]
        self.assertEqual(sorted(expected), sorted(positions))
        self.assertEqual(len(positions), len(set(positions)))

    def test_a_value_space_lands_by_the_platforms_own_numbers(self):
        """The reference binding beside this fixture was written by hand, and
        its maps are keyed exactly so: the skeleton reproduces the half of a
        correct binding that is not a decision."""
        got = {r["address"]: r for r in self.skeleton()["outputs"].values()}
        reference = read_binding(PACK / "controller.binding.yaml")["outputs"]
        for rule in reference.values():
            self.assertEqual(rule["field"], got[rule["address"]]["field"])
            self.assertEqual(rule["map"], got[rule["address"]]["map"])

    def test_a_field_with_no_value_space_passes_through(self):
        pack = load_pack(PACK)
        entry = pack.model.outputs()[0]
        stripped = [type(f)(f.name, None, "int32") for f in entry.fields]
        pack.model.entries[pack.model.entries.index(entry)] = type(entry)(
            entry.address, entry.role, entry.names, tuple(stripped), entry.note)
        got = yaml.safe_load(draft(pack, "controller.scxml"))["outputs"]
        rule = next(r for r in got.values() if r["address"] == entry.address)
        self.assertIs(True, rule.get("passthrough"))
        self.assertNotIn("map", rule)

    def test_the_skeleton_decides_nothing(self):
        doc = self.skeleton()
        for side in ("inputs", "outputs"):
            for name, rule in doc[side].items():
                self.assertFalse(DECISION_KEYS & set(rule),
                                 f"{side} {name} carries a decision: {DECISION_KEYS & set(rule)}")

    def test_activation_is_written_only_when_it_is_given(self):
        self.assertNotIn("activation", self.skeleton())
        self.out.unlink()
        self.assertEqual("periodic", self.skeleton("periodic")["activation"])
        with self.assertRaises(ScaffoldError):
            draft(self.pack, "controller.scxml", "sometimes")

    def test_an_existing_binding_is_never_overwritten(self):
        self.out.write_text("version: 1\ndocument: mine.scxml\n", encoding="utf-8")
        with self.assertRaises(ScaffoldError):
            write(self.pack, "controller.scxml", self.out)
        self.assertEqual("version: 1\ndocument: mine.scxml\n", self.out.read_text())

    def test_rule_names_are_identifiers_and_the_output_is_stable(self):
        text = draft(self.pack, "controller.scxml")
        self.assertEqual(text, draft(self.pack, "controller.scxml"))
        doc = yaml.safe_load(text)
        names = list(doc["inputs"]) + list(doc["outputs"])
        self.assertEqual(len(names), len(set(names)))
        for name in names:
            self.assertRegex(name, r"^[a-z][A-Za-z0-9]*$")
        self.assertEqual("inTrainApproach", identifier("IN_TrainApproach"))
        self.assertEqual("x42", identifier("X42"))
        self.assertEqual("p2Stage", identifier("2 stage"))

    def test_the_server_writes_it_and_refuses_to_overwrite(self):
        args = {"pack": str(PACK), "document": "controller.scxml", "binding": str(self.out)}
        first = mcp.call_tool("scaffold", args)
        self.assertFalse(first.get("isError"), first)
        self.assertTrue(self.out.is_file())
        second = mcp.call_tool("scaffold", args)
        self.assertTrue(second.get("isError"))


if __name__ == "__main__":
    unittest.main()
