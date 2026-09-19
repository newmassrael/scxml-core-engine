"""A reading idiom the pack names, defined well enough to be evaluated.

`conventions.protocols` used to declare a protocol by NAME and a prose note,
and the schema said so on purpose: what a protocol means is the pack's
runtime, never the core's. The cost of that seam was measured once `verify`
existed -- it could not run ANY document using one, which on the dogfood corpus
was 79 binding inputs out of the ones that matter.

So a protocol may now carry a definition as DATA. Not as code: a pack that can
run code is a program, and the whole boundary here rests on a pack being
something a caller supplies without granting execution.

⚠ The definition is a LATCH, and it is that shape because the idiom measured
is that shape: two cumulative counters where whichever ticked most recently is
the state. Being a latch, it needs the previous round -- which is why
`examples.ordered` had to become its own property, separate from
`independent_cases`. A shipped test log is both, and one flag could only say
one of them.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.pack import Case as RealCase, load_pack
from sce_author.verify import Latches, VerifyError, input_value, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

LADDER = {
    "parameters": ["on_counter", "off_counter"],
    "latch": {
        "set_when_changed": "on_counter",
        "clear_when_changed": "off_counter",
        "both": "clear",
        "initial": "clear",
    },
}


def Case(given, drove=()):
    """⚠ The REAL case type, not a stand-in.

    A local double was used here and it drifted the moment the type gained a
    field: every protocol case failed with an AttributeError on a name the
    real object had and the double did not. A double of a type this package
    owns buys nothing and hides exactly this.
    """
    return RealCase("", given, {}, None, tuple(drove))


class Conventions:
    def __init__(self, protocols):
        self.protocols = protocols


PARAMETERS = {"on_counter": "plant/count/on", "off_counter": "plant/count/off"}


class AProtocolCanBeDefinedAsData(unittest.TestCase):
    def drive(self, rounds, protocols=None):
        """Feed counter readings through the latch, in order."""
        latches = Latches(Conventions(protocols or {"ladder": LADDER}))
        rule = {"protocol": "ladder", "parameters": PARAMETERS}
        out = []
        for on, off in rounds:
            case = Case({"plant/count/on": on, "plant/count/off": off})
            out.append(input_value("gate", rule, case, latches))
        return out

    def test_it_is_clear_until_the_set_parameter_first_changes(self):
        self.assertEqual([False, True], self.drive([(0, 0), (1, 0)]))

    def test_it_stays_set_while_only_the_set_parameter_ticks(self):
        self.assertEqual([False, True, True, True],
                         self.drive([(0, 0), (1, 0), (2, 0), (3, 0)]))

    def test_the_clear_parameter_puts_it_back(self):
        self.assertEqual([False, True, False],
                         self.drive([(0, 0), (1, 0), (1, 1)]))

    def test_when_both_change_the_declared_winner_wins(self):
        """⚠ Declared, not assumed.

        The round where both tick is ordinary -- it is what the measured idiom
        does when the supply drops -- and both orders are plausible. Picking
        one silently would make a whole class of rounds answer wrongly with
        nothing to show for it.
        """
        self.assertEqual([False, True, False],
                         self.drive([(0, 0), (1, 0), (2, 1)]))
        setting = {"ladder": {**LADDER,
                              "latch": {**LADDER["latch"], "both": "set"}}}
        self.assertEqual([False, True, True],
                         self.drive([(0, 0), (1, 0), (2, 1)], setting))

    def test_reaching_a_later_rung_includes_the_earlier_ones(self):
        """⚠ The second clause of the idiom, left out of the first version.

        The pack's own note said "the rungs are cumulative" and only the
        latch was built. A component with ONE rung is unaffected -- it passed
        every case and the omission was invisible -- and a component with
        three had every event stay dark, because the input asking "has it been
        on at all" went false the moment a longer reading passed.
        """
        ladder = ["plant/count/on0", "plant/count/on500", "plant/count/on3500"]
        protocols = {"ladder": {**LADDER,
                                "latch": {**LADDER["latch"],
                                          "cumulative": ladder}}}
        latches = Latches(Conventions(protocols))

        def at(counts):
            given = dict(zip(ladder, counts))
            given["plant/count/off"] = 0
            return Case(given)

        def rung(which, case):
            rule = {"protocol": "ladder",
                    "parameters": {"on_counter": which,
                                   "off_counter": "plant/count/off"}}
            return input_value(f"gate:{which}", rule, case, latches)

        first = at((0, 0, 0))
        for which in ladder:
            self.assertFalse(rung(which, first))

        # The longest reading passes. Every rung at or below it is now on.
        later = at((0, 0, 1))
        self.assertTrue(rung(ladder[2], later))
        self.assertTrue(rung(ladder[0], later),
                        "the lowest rung is what 'on at all' means")
        self.assertTrue(rung(ladder[1], later))

    def test_a_rung_above_the_one_reached_stays_off(self):
        """The discriminator: cumulative is not "any change sets everything"."""
        ladder = ["plant/count/on0", "plant/count/on500", "plant/count/on3500"]
        protocols = {"ladder": {**LADDER,
                                "latch": {**LADDER["latch"],
                                          "cumulative": ladder}}}
        latches = Latches(Conventions(protocols))

        def at(counts):
            given = dict(zip(ladder, counts))
            given["plant/count/off"] = 0
            return Case(given)

        def rung(which, case):
            rule = {"protocol": "ladder",
                    "parameters": {"on_counter": which,
                                   "off_counter": "plant/count/off"}}
            return input_value(f"gate:{which}", rule, case, latches)

        for which in ladder:
            rung(which, at((0, 0, 0)))
        only_the_first = at((1, 0, 0))
        self.assertTrue(rung(ladder[0], only_the_first))
        self.assertFalse(rung(ladder[2], only_the_first),
                         "a longer reading has not passed yet")

    def test_a_reading_restated_unchanged_is_still_a_statement(self):
        """⚠⚠ The assumption that cost a whole component.

        The latch first read "what changed" by subtracting this case's values
        from the one before. That is wrong about records, not just incomplete:
        a counter RESTATED at the same reading is the record saying the thing
        happened again, and subtraction reads it as nothing happening. On one
        corpus an ignition ladder stood at identical readings for dozens of
        consecutive cases while the record went on asserting them, and every
        reading that asked "which rung most recently" answered no -- so every
        event stayed dark and the document was blamed.

        `drove` is the record saying what it set. When a case carries it, it
        is believed over any subtraction.
        """
        latches = Latches(Conventions({"ladder": LADDER}))
        rule = {"protocol": "ladder", "parameters": PARAMETERS}
        counts = {"plant/count/on": 7, "plant/count/off": 0}

        # Three rounds at IDENTICAL readings. Subtraction sees nothing; the
        # record says it set the on-counter every time.
        held = [input_value("gate", rule, Case(counts, drove=["plant/count/on"]),
                            latches) for _ in range(3)]
        self.assertEqual([True, True, True], held)

        # And the same three rounds WITHOUT the record saying so: nothing
        # changed, so nothing is asserted. Both readings are available and the
        # difference between them is the whole point.
        quiet = Latches(Conventions({"ladder": LADDER}))
        self.assertEqual(
            [False, False, False],
            [input_value("gate", rule, Case(counts), quiet) for _ in range(3)])

    def test_the_later_of_the_two_may_be_the_winner(self):
        """⚠⚠ `both: clear` was measured on one component and was wrong.

        That component had exactly one round where both parameters changed,
        and in it the clearing one happened to be listed later -- so `clear`
        and `last` gave the same answer and the two were indistinguishable.
        A different component then recorded the clearing one FIRST and the
        setting one last, and its telltale stayed dark.

        `last` needs no arbitrary winner: the record's own order says which
        was more recent, which is what this idiom is named for. A definition
        confirmed on one subject is not confirmed.
        """
        rule = {"protocol": "ladder", "parameters": PARAMETERS}
        protocols = {"ladder": {**LADDER,
                                "latch": {**LADDER["latch"], "both": "last"}}}
        counts = {"plant/count/on": 1, "plant/count/off": 1}

        later_on = Latches(Conventions(protocols))
        self.assertTrue(input_value("gate", rule, Case(
            counts, drove=["plant/count/off", "plant/count/on"]), later_on))

        later_off = Latches(Conventions(protocols))
        self.assertFalse(input_value("gate", rule, Case(
            counts, drove=["plant/count/on", "plant/count/off"]), later_off))

    def test_without_an_order_the_declared_default_decides(self):
        """A record that did not place both leaves no "later" to read."""
        rule = {"protocol": "ladder", "parameters": PARAMETERS}
        protocols = {"ladder": {**LADDER,
                                "latch": {**LADDER["latch"], "both": "last"}}}
        latches = Latches(Conventions(protocols))
        # Both change by subtraction, but the record names neither.
        counts = ({"plant/count/on": 0, "plant/count/off": 0},
                  {"plant/count/on": 1, "plant/count/off": 1})
        held = [input_value("gate", rule, Case(c), latches) for c in counts]
        self.assertEqual([False, False], held, "falls back to `clear`")

    def test_the_initial_state_is_declared_too(self):
        starting_set = {"ladder": {**LADDER,
                                   "latch": {**LADDER["latch"],
                                             "initial": "set"}}}
        self.assertEqual([True], self.drive([(0, 0)], starting_set))

    # --------------------------------------------------------- the refusals

    def test_a_protocol_the_pack_never_defines_is_refused_by_name(self):
        latches = Latches(Conventions({"ladder": {"parameters": ["on_counter",
                                                                 "off_counter"]}}))
        rule = {"protocol": "ladder", "parameters": PARAMETERS}
        with self.assertRaises(VerifyError) as caught:
            input_value("gate", rule, Case({}), latches)
        self.assertIn("never defines it", str(caught.exception))

    def test_a_missing_parameter_is_refused_by_name(self):
        latches = Latches(Conventions({"ladder": LADDER}))
        rule = {"protocol": "ladder",
                "parameters": {"on_counter": "plant/count/on"}}
        with self.assertRaises(VerifyError) as caught:
            input_value("gate", rule, Case({}), latches)
        self.assertIn("off_counter", str(caught.exception))

    def test_unordered_examples_cannot_supply_a_protocol_at_all(self):
        """⚠ Without an order there is no previous round to have changed from.

        Reading the file's line order as a timeline would be reading a promise
        nobody made, and the answer would be about that reading.
        """
        rule = {"protocol": "ladder", "parameters": PARAMETERS}
        with self.assertRaises(VerifyError) as caught:
            input_value("gate", rule, Case({}), latches=None)
        self.assertIn("ordered", str(caught.exception))

    def test_the_second_subject_matter_does_not_claim_an_order(self):
        """The crossing records are independent and NOT a timeline.

        Which is the point of two flags: it can be compared and it cannot be
        replayed, and one flag could not have said both.
        """
        pack = load_pack(CROSSING)
        self.assertTrue(pack.examples.independent)
        self.assertFalse(pack.examples.ordered)


class ABindingRefusesAYamlBoolean(unittest.TestCase):
    """⚠ `equals: ON` parses to `True`, and the schema then said only that a
    string was expected -- naming neither the cause nor the fix. This trap was
    walked into six times across two subject matters before the refusal below
    was written.
    """

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml", "examples.yaml",
                     "controller.resolved.scxml"):
            shutil.copy(CROSSING / name, self.tmp / name)

    def binding(self, mutate):
        doc = yaml.safe_load(
            (CROSSING / "controller.resolved.binding.yaml").read_text(encoding="utf-8"))
        mutate(doc)
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(doc), encoding="utf-8")
        return path

    def refusal(self, mutate):
        from sce_author.check import read_binding
        from sce_author.errors import PackError
        with self.assertRaises(PackError) as caught:
            read_binding(self.binding(mutate))
        return str(caught.exception)

    def test_a_boolean_where_a_symbol_belongs_says_which_position(self):
        def mutate(doc):
            doc["inputs"]["override"]["equals"] = True
        said = self.refusal(mutate)
        self.assertIn("inputs -> override -> equals", said)
        self.assertIn("quote", said)

    def test_a_boolean_inside_a_map_says_which_entry(self):
        def mutate(doc):
            doc["outputs"]["bell"]["map"][0] = False
        said = self.refusal(mutate)
        self.assertIn("outputs -> bell -> map -> 0", said)

    def test_the_keys_that_are_genuinely_booleans_are_left_alone(self):
        from sce_author.check import read_binding

        def mutate(doc):
            doc["outputs"]["bell"].pop("map")
            doc["outputs"]["bell"]["passthrough"] = True
        read_binding(self.binding(mutate))   # must not raise


if __name__ == "__main__":
    unittest.main()
