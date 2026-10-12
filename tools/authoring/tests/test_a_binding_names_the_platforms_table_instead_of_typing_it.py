"""`map: platform` lands a document's numbers through the platform's own table, read from the model.

A table typed again by hand can lose an entry, and every symbol after the lost one then lands one number
early: measured 2026-10-11 on the 7,090 distinct output maps of the authoring survey, one writer re-typing
a 54-symbol enumeration left one symbol out -- 240 cases failed on the value and 452 could not be judged,
because the last symbol moved from 53 to 52 and the document's 53 had no entry, while `check` passed it.

The first answer was a rule that guessed from the SHAPE of a map whether it was a copy. A shape cannot say
what its author meant: a copy that lost its first entry and an own numbering that counts from another
start are the same map, and each correction of the rule refused another sound binding. What a writer
means is said, not guessed: `map: platform` is the platform's table, nothing is typed, and a map written
out is the document's own numbering, judged by what the document produces (`landing.unmapped`) and by
the cases `verify` runs.

Every case that must be refused is paired with the nearest one that must not be.
"""

from __future__ import annotations

import copy

import yaml

from sce_author.check import PackError, platform_table, read_binding
from sce_author.landing import PLATFORM_MAP
from sce_author.scaffold import draft

from tests.test_refusals_actually_fire import BINDING, CONVENTIONS, DOCUMENT, MODEL, Fixture

PLATFORM = {0: "NONE", 1: "OFF", 2: "ON", 3: "MAX"}


def lamp_rule(**more) -> dict:
    return {"address": "Plant.Out.Lamp", "field": "Stat", **more}


class TheTableIsRead(Fixture):
    def written(self, rule: dict):
        path = self.root / "fixture.binding.yaml"
        path.write_text(yaml.safe_dump({**BINDING, "outputs": {"lamp": rule}}), encoding="utf-8")
        return path

    def test_the_word_is_the_models_table_for_the_field(self):
        binding = read_binding(self.written(lamp_rule(map=PLATFORM_MAP)), self.pack())
        self.assertEqual(PLATFORM, binding["outputs"]["lamp"]["map"])

    def test_a_written_out_map_is_left_as_it_is(self):
        binding = read_binding(self.written(lamp_rule(map={0: "OFF", 1: "ON"})), self.pack())
        self.assertEqual({0: "OFF", 1: "ON"}, binding["outputs"]["lamp"]["map"])

    def test_without_a_pack_the_word_stays_for_a_caller_that_reads_no_map(self):
        binding = read_binding(self.written(lamp_rule(map=PLATFORM_MAP)))
        self.assertEqual(PLATFORM_MAP, binding["outputs"]["lamp"]["map"])

    def test_the_table_follows_the_model_not_a_copy_of_it(self):
        model = copy.deepcopy(MODEL)
        lamp = next(e for e in model["entries"] if e["address"] == "Plant.Out.Lamp")
        lamp["fields"]["Stat"]["values"] = {"NONE": 0, "OFF": 1, "ON": 2, "MAX": 3, "EXTRA": 4}
        self.write_pack(model, CONVENTIONS)
        binding = read_binding(self.written(lamp_rule(map=PLATFORM_MAP)), self.pack())
        self.assertEqual({**PLATFORM, 4: "EXTRA"}, binding["outputs"]["lamp"]["map"])

    def test_the_table_is_listed_in_the_models_own_order(self):
        table, why = platform_table(self.pack().model, lamp_rule(map=PLATFORM_MAP))
        self.assertEqual("", why)
        self.assertEqual(list(PLATFORM), list(table))


class TheWordIsRefusedWhereThereIsNoTable(Fixture):
    def refusal(self, rule: dict) -> str:
        path = self.root / "fixture.binding.yaml"
        path.write_text(yaml.safe_dump({**BINDING, "outputs": {"lamp": rule}}), encoding="utf-8")
        with self.assertRaises(PackError) as caught:
            read_binding(path, self.pack())
        return str(caught.exception)

    def test_a_field_that_is_not_an_enumeration(self):
        said = self.refusal({"address": "Plant.Out.Lamp", "field": "Value", "map": PLATFORM_MAP})
        self.assertIn("output lamp", said)
        self.assertIn("is not an enumeration", said)

    def test_an_address_the_model_does_not_declare(self):
        said = self.refusal({"address": "Plant.Out.Ghost", "field": "Stat", "map": PLATFORM_MAP})
        self.assertIn("'Plant.Out.Ghost' is not in the interface model", said)

    def test_a_field_the_address_does_not_have(self):
        said = self.refusal({"address": "Plant.Out.Lamp", "field": "Ghost", "map": PLATFORM_MAP})
        self.assertIn("has no field 'Ghost'", said)

    def test_any_other_word_is_not_a_map(self):
        said = self.refusal(lamp_rule(map="platfrom"))
        self.assertIn("platfrom", said)


class TheCheckJudgesWhatTheDocumentProduces(Fixture):
    """The document of the fixture produces 1 or 0; others are written below."""

    def bind_with(self, rule: dict, document: str = DOCUMENT):
        (self.root / "fixture.scxml").write_text(document, encoding="utf-8")
        return self.bind({**BINDING, "outputs": {"lamp": rule}})

    @staticmethod
    def producing(expr: str) -> str:
        return DOCUMENT.replace('expr="mode ? 1 : 0"', f'expr="{expr}"')

    def test_the_platforms_table_lands_what_the_document_computes(self):
        self.assertEqual([], self.bind_with(lamp_rule(map=PLATFORM_MAP)))

    def test_a_table_that_lost_an_entry_is_refused_where_the_document_needs_it(self):
        # The document computes the platform's own number 3, and the table typed again lost
        # its entry: nothing says so but what the document produces.
        lost = {0: "NONE", 1: "OFF", 2: "ON"}
        said = [str(f) for f in self.bind_with(lamp_rule(map=lost), self.producing("mode ? 3 : 0"))]
        self.assertEqual(1, len(said), said)
        self.assertIn("the map has no entry for 3", said[0])

    def test_the_same_document_through_the_platforms_table_is_not_refused(self):
        self.assertEqual([], self.bind_with(lamp_rule(map=PLATFORM_MAP), self.producing("mode ? 3 : 0")))

    def test_a_value_the_platforms_table_has_no_number_for_is_refused(self):
        said = [str(f) for f in self.bind_with(lamp_rule(map=PLATFORM_MAP), self.producing("mode ? 9 : 0"))]
        self.assertEqual(1, len(said), said)
        self.assertIn("the map has no entry for 9", said[0])

    # The nearest maps that must NOT be refused: each numbers its values as the document does,
    # which a document is free to do, whatever its shape looks like beside the platform's.
    def test_a_numbering_shifted_by_one_is_the_documents_own(self):
        shifted = {0: "OFF", 1: "ON", 2: "MAX"}
        self.assertEqual([], self.bind_with(lamp_rule(map=shifted), self.producing("mode ? 2 : 0")))

    def test_a_table_naming_every_symbol_under_other_numbers_is_the_documents_own(self):
        permuted = {0: "ON", 1: "OFF", 2: "NONE", 3: "MAX"}
        self.assertEqual([], self.bind_with(lamp_rule(map=permuted), self.producing("mode ? 1 : 0")))


class TheScaffoldWritesTheWordNotTheTable(Fixture):
    def test_an_enumerated_output_is_bound_to_the_platforms_table(self):
        written = yaml.safe_load(draft(self.pack(), "controller.scxml"))["outputs"]
        lamp = next(rule for rule in written.values()
                    if rule["address"] == "Plant.Out.Lamp" and rule.get("field") == "Stat")
        self.assertEqual(PLATFORM_MAP, lamp["map"])

    def test_an_output_that_is_not_enumerated_is_passed_through(self):
        written = yaml.safe_load(draft(self.pack(), "controller.scxml"))["outputs"]
        value = next(rule for rule in written.values()
                     if rule["address"] == "Plant.Out.Lamp" and rule.get("field") == "Value")
        self.assertTrue(value["passthrough"])
        self.assertNotIn("map", value)
