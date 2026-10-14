"""`map: platform` lands a document's enumeration on the platform's symbols of the same names.

A table typed again by hand can lose an entry. Measured 2026-10-11: one writer re-typing a long
enumeration left one symbol out, every symbol after it landed on the wrong one, the last had no
entry at all, and `check` passed it. So the table is not typed: `map: platform` is read from the
model.

It is keyed by NAME. A position the platform gives a closed set of symbols is an enumeration of
the document's own, a value crosses the boundary as the variant the document chose, and the
platform's number for a symbol is in neither file. ⚠ It was keyed by the platform's number, for a
document that computed that number: such a document is written in one platform's numbering, and
`check` refuses it now whichever way its map is written (`closed_value_refusals`).

A shape cannot say what its author meant, so nothing here guesses from one: `map: platform` is
the platform's table, a map written out is the document's own naming, and both are judged by what
the document can produce (`landing.unmapped`) and by the cases `verify` runs.

Every case that must be refused is paired with the nearest one that must not be.
"""

from __future__ import annotations

import copy

import yaml

from sce_author.check import PackError, platform_table, read_binding
from sce_author.landing import PLATFORM_MAP
from sce_author.scaffold import draft
from sce_author.verify import verify

from tests.test_refusals_actually_fire import BINDING, CONVENTIONS, DOCUMENT, MODEL, Fixture

PLATFORM = {"NONE": "NONE", "OFF": "OFF", "ON": "ON", "MAX": "MAX"}


def lamp_rule(**more) -> dict:
    return {"address": "Plant.Out.Lamp", "field": "Stat", **more}


def lamp_values(*variants: str) -> str:
    """An enumeration of the document's own, numbered as the document likes."""
    lines = "\n".join(f'      <sce:variant name="{name}" value="{10 * (n + 1)}"/>'
                      for n, name in enumerate(variants))
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="lamp_values" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
{lines}
    </data>
  </datamodel>
</scxml>
"""


def enumerated(on: str = "ON", off: str = "OFF") -> str:
    """The fixture's document with the lamp as an enumeration it imports."""
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="fixture">
  <sce:import as="Lamp" src="lamp_values.scxml" kind="enum"/>
  <datamodel>
    <data id="mode" sce:type="bool" sce:direction="in"/>
    <data id="lamp" sce:type="enum:Lamp" sce:direction="out"
          expr="mode ? Lamp.{on} : Lamp.{off}"/>
  </datamodel>
</scxml>
"""


class TheTableIsRead(Fixture):
    def written(self, rule: dict):
        path = self.root / "fixture.binding.yaml"
        path.write_text(yaml.safe_dump({**BINDING, "outputs": {"lamp": rule}}), encoding="utf-8")
        return path

    def test_the_word_is_the_models_symbols_each_under_its_own_name(self):
        binding = read_binding(self.written(lamp_rule(map=PLATFORM_MAP)), self.pack())
        self.assertEqual(PLATFORM, binding["outputs"]["lamp"]["map"])

    def test_no_number_of_the_platforms_is_in_the_table(self):
        """⚠ The discriminator for the contract: the table used to be `{0: NONE, 1: OFF, ...}`,
        which is what let a document compute the platform's code and land it unchanged."""
        table, _why = platform_table(self.pack().model, lamp_rule(map=PLATFORM_MAP))
        self.assertTrue(all(isinstance(key, str) for key in table), table)
        self.assertEqual(list(table), list(table.values()))

    def test_a_written_out_map_is_left_as_it_is(self):
        own = {"off": "OFF", "on": "ON"}
        binding = read_binding(self.written(lamp_rule(map=own)), self.pack())
        self.assertEqual(own, binding["outputs"]["lamp"]["map"])

    def test_without_a_pack_the_word_stays_for_a_caller_that_reads_no_map(self):
        binding = read_binding(self.written(lamp_rule(map=PLATFORM_MAP)))
        self.assertEqual(PLATFORM_MAP, binding["outputs"]["lamp"]["map"])

    def test_the_table_follows_the_model_not_a_copy_of_it(self):
        model = copy.deepcopy(MODEL)
        lamp = next(e for e in model["entries"] if e["address"] == "Plant.Out.Lamp")
        lamp["fields"]["Stat"]["values"] = {"NONE": 0, "OFF": 1, "ON": 2, "MAX": 3, "EXTRA": 4}
        self.write_pack(model, CONVENTIONS)
        binding = read_binding(self.written(lamp_rule(map=PLATFORM_MAP)), self.pack())
        self.assertEqual({**PLATFORM, "EXTRA": "EXTRA"}, binding["outputs"]["lamp"]["map"])

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
    """The lamp as an enumeration of the document's own, landed by its variants' names."""

    def bind_with(self, rule: dict, document: str, variants=("OFF", "ON")):
        (self.root / "fixture.scxml").write_text(document, encoding="utf-8")
        (self.root / "lamp_values.scxml").write_text(lamp_values(*variants), encoding="utf-8")
        return [str(f) for f in self.bind({**BINDING, "outputs": {"lamp": rule}})]

    def test_the_platforms_table_lands_every_variant_on_the_symbol_of_its_name(self):
        self.assertEqual([], self.bind_with(lamp_rule(map=PLATFORM_MAP), enumerated()))

    def test_a_variant_the_platform_has_no_symbol_for_is_refused(self):
        said = self.bind_with(lamp_rule(map=PLATFORM_MAP), enumerated(on="DIM"),
                              variants=("OFF", "DIM"))
        self.assertEqual(1, len(said), said)
        self.assertIn("the map has no entry for 'DIM'", said[0])

    def test_a_written_out_table_that_lost_a_variant_is_refused(self):
        """Every variant is a value the document can produce, whichever its expression
        happens to name, so a table typed out owes each one an entry."""
        said = self.bind_with(lamp_rule(map={"OFF": "OFF", "ON": "ON"}), enumerated(),
                              variants=("OFF", "ON", "MAX"))
        self.assertEqual(1, len(said), said)
        self.assertIn("the map has no entry for 'MAX'", said[0])

    def test_the_same_document_through_the_platforms_table_is_not_refused(self):
        self.assertEqual([], self.bind_with(lamp_rule(map=PLATFORM_MAP), enumerated(),
                                            variants=("OFF", "ON", "MAX")))

    # The nearest maps that must NOT be refused: each names its values as the document does,
    # which a document is free to do, whatever the platform calls them.
    def test_variants_named_otherwise_are_the_documents_own(self):
        own = {"DARK": "OFF", "LIT": "ON"}
        self.assertEqual([], self.bind_with(lamp_rule(map=own), enumerated(on="LIT", off="DARK"),
                                            variants=("DARK", "LIT")))

    def test_a_text_the_document_names_its_values_by_is_the_documents_own(self):
        self.assertEqual([], self.bind_with(lamp_rule(map={"off": "OFF", "on": "ON"}), DOCUMENT))


class ANumberIsNotLandedOnAClosedSet(Fixture):
    """⚠ The discriminator for the rule. The lamp is one of a closed set of symbols; a number
    there is the platform's code for one, and the map it is keyed by does not change that."""

    NUMBERED = DOCUMENT.replace('sce:type="string"', 'sce:type="int32"').replace(
        "mode ? 'on' : 'off'", "mode ? 2 : 1")

    def found(self, rule: dict) -> list:
        (self.root / "fixture.scxml").write_text(self.NUMBERED, encoding="utf-8")
        return [str(f) for f in self.bind({**BINDING, "outputs": {"lamp": rule}})]

    def test_the_premise_the_document_computes_a_number(self):
        self.assertIn('sce:type="int32"', self.NUMBERED)
        self.assertIn("mode ? 2 : 1", self.NUMBERED)

    def test_a_number_through_a_table_keyed_by_the_platforms_codes_is_refused(self):
        said = [f for f in self.found(lamp_rule(map={1: "OFF", 2: "ON"})) if "closed set" in f]
        self.assertEqual(1, len(said), said)
        self.assertIn("output lamp", said[0])
        self.assertIn("Plant.Out.Lamp.Stat", said[0])
        self.assertIn("`int32`", said[0])
        self.assertIn("enum:<alias>", said[0])

    def test_a_number_through_the_platforms_table_is_refused_too(self):
        said = [f for f in self.found(lamp_rule(map=PLATFORM_MAP)) if "closed set" in f]
        self.assertEqual(1, len(said), said)

    def test_a_number_passed_through_to_a_closed_set_is_refused(self):
        said = [f for f in self.found(lamp_rule(passthrough=True)) if "closed set" in f]
        self.assertEqual(1, len(said), said)

    def test_a_number_on_a_field_that_is_a_number_is_not(self):
        rule = {"address": "Plant.Out.Lamp", "field": "Value", "passthrough": True}
        self.assertFalse([f for f in self.found(rule) if "closed set" in f])

    def test_verify_stops_on_it_in_the_words_check_used(self):
        """What `check` says first, `verify` does not run past: a verdict over a document
        written in the platform's numbers would be a verdict about that numbering."""
        rule = lamp_rule(map={1: "OFF", 2: "ON"})
        said = [f for f in self.found(rule) if "closed set" in f]
        self.assertEqual(1, len(said), said)
        result = verify(self.pack(), self.root / "fixture.binding.yaml")
        self.assertFalse(result.ran)
        self.assertIn("closed set", result.refusal)
        why = result.refusal.split(": ", 1)[1]
        self.assertIn(why, said[0])


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
