"""A `map` that reads as the platform's own table and disagrees with it is refused.

`scaffold` writes an output's `map` keyed by the platform's own numbers, and a
writer who renames the rule is tempted to re-type it. Re-typing a long table by
hand loses an entry, and every symbol after the lost one then lands one number
early. `check` asked only whether each NAME the map writes is one the field
admits, so it passed -- and the document failed every case after the lost entry
and could not be judged on the last one (measured 2026-10-11: 240 failed, 452
unjudged, over one dropped symbol in a 54-symbol table).

Every case that must be refused is paired with the nearest one that must not be,
because a rule that refuses every map passes the first half of this file.
"""

from __future__ import annotations

import copy

from sce_author.landing import departure_from_the_platforms_table

from tests.test_refusals_actually_fire import BINDING, CONVENTIONS, MODEL, Fixture

# A field with twelve symbols, so that losing one still names more than nine
# tenths of them -- the share at which a map is read as the platform's table.
SYMBOLS = ["NONE", "OFF", "ON"] + [f"STEP_{n}" for n in range(3, 11)] + ["MAX"]
VALUES = {symbol: code for code, symbol in enumerate(SYMBOLS)}
THE_MARK = "reads as the platform's own table"


def model_with_a_long_field() -> dict:
    model = copy.deepcopy(MODEL)
    lamp = next(e for e in model["entries"] if e["address"] == "Plant.Out.Lamp")
    lamp["fields"]["Stat"]["values"] = dict(VALUES)
    return model


def table_without(*lost: str) -> dict:
    """The platform's table re-typed with `lost` left out, keys renumbered the
    way a copy does it: consecutively, from zero."""
    kept = [symbol for symbol in SYMBOLS if symbol not in lost]
    return {code: symbol for code, symbol in enumerate(kept)}


class TheFunction(Fixture):
    def test_the_platforms_own_table_is_not_a_departure(self):
        self.assertEqual("", departure_from_the_platforms_table({c: s for s, c in VALUES.items()}, VALUES))

    def test_a_table_that_lost_one_entry_is_a_departure_and_says_which(self):
        said = departure_from_the_platforms_table(table_without("ON"), VALUES)
        self.assertIn("2 lands STEP_3, which the model numbers 3", said)
        self.assertIn("(it numbers 2 ON)", said)
        self.assertIn("no entry for ON", said)
        self.assertIn("names 11 of the field's 12 symbols", said)

    def test_the_count_says_what_the_message_does_not_name(self):
        # Nine numbers land the wrong symbol (2 to 10); the message names three.
        self.assertIn("and 6 more", departure_from_the_platforms_table(table_without("ON"), VALUES))

    def test_a_field_with_no_value_space_has_nothing_to_depart_from(self):
        self.assertEqual("", departure_from_the_platforms_table({0: "ON"}, None))
        self.assertEqual("", departure_from_the_platforms_table({}, VALUES))


class TheCheckAsksIt(Fixture):
    def setUp(self):
        super().setUp()
        self.write_pack(model_with_a_long_field(), CONVENTIONS)

    def departures(self, table: dict) -> list[str]:
        rule = {"address": "Plant.Out.Lamp", "field": "Stat", "map": table}
        found = self.bind({**BINDING, "outputs": {"lamp": rule}})
        return [str(f) for f in found if THE_MARK in str(f)]

    def test_the_platforms_table_is_not_refused(self):
        self.assertEqual([], self.departures({c: s for s, c in VALUES.items()}))

    def test_the_table_that_lost_an_entry_is_refused_where_it_was_written(self):
        got = self.departures(table_without("ON"))
        self.assertEqual(1, len(got), got)
        self.assertIn("output lamp", got[0])
        self.assertIn("Plant.Out.Lamp.Stat", got[0])
        self.assertIn("no entry for ON", got[0])

    # The nearest maps that must NOT be refused: each is an own numbering, which
    # a document is free to have.
    def test_a_table_that_names_every_symbol_under_numbers_of_its_own_is_an_own_numbering(self):
        # A ramp whose steps are not the platform's signal numbers: every symbol lands once,
        # nothing is lost, and the keys are numbers the platform also uses.
        ramp = {c: s for s, c in VALUES.items()}
        ramp[3], ramp[4] = ramp[4], ramp[3]
        self.assertEqual([], self.departures(ramp))
        self.assertEqual("", departure_from_the_platforms_table(ramp, VALUES))

    def test_a_map_of_a_few_symbols_is_the_documents_own_numbering(self):
        self.assertEqual([], self.departures({0: "OFF", 1: "ON"}))

    def test_a_table_that_lost_more_than_a_tenth_is_not_read_as_the_platforms(self):
        self.assertEqual([], self.departures(table_without("ON", "STEP_5")))

    def test_a_full_table_keyed_by_numbers_the_platform_does_not_use_is_an_own_numbering(self):
        self.assertEqual([], self.departures({100 + c: s for s, c in VALUES.items()}))

    def test_a_full_table_keyed_by_tokens_is_an_own_numbering(self):
        self.assertEqual([], self.departures({f"level.{s}": s for s in SYMBOLS}))

    def test_a_table_keyed_by_signed_levels_is_an_own_numbering(self):
        self.assertEqual([], self.departures({-c: s for s, c in VALUES.items()}))
