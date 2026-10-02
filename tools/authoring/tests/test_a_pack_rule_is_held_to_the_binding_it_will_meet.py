"""A pack's precondition rule is held to what the binding it will meet may say.

A review of the landed pack check found three rules it called clean that something
later refused or crashed on (reproduced, each with its own fixture):

  * a rule comparing an address with a symbol the address does not admit
    (`equals: TYPO` against a value space of ACTIVE and INACTIVE): `check-pack`
    said clean and the binding check refused the same rule afterwards;
  * a rule whose `parameters` was a list: an `AttributeError` out of the loader,
    where a list of problems was owed;
  * a replacement in `preconditions.normalise` naming a group its pattern lacks:
    clean at load, and an `IndexError` out of `re` in the middle of reading a
    specification.

One cause under the first two: a rule is "a binding input as the platform writes
it", and the pack held it to which protocol and which address it names and to
nothing else. The binding schema is the one place that says what an input rule may
contain, and the interface model the one that says which symbols an address takes;
both are asked here, and the binding check asks the same helpers.
"""

from __future__ import annotations

import json
import pathlib
import re
import tempfile
import unittest

from sce_author.errors import PackError
from sce_author.pack import check_pack, load_pack, rule_symbols

MODEL = {"version": 1, "entries": [
    {"address": "System.Input.Active", "role": "input", "names": ["In_Active"],
     "values": {"INACTIVE": 0, "ACTIVE": 1}},
    {"address": "System.Output.Alarm", "role": "output", "names": ["Alarm"],
     "values": {"INACTIVE": 0, "ACTIVE": 1}}]}

CONVENTIONS = {
    "version": 1,
    "name_classes": [{"pattern": "In_[A-Za-z0-9_]+", "role": "supplied"}],
    "preconditions": {"inputs": {"x": "whether the input is active"},
                      "phrases": {"enabled": "x"}}}


class Pack(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name)

    def pack(self, *, rule: dict | None = None, protocols: dict | None = None,
             normalise: list | None = None) -> pathlib.Path:
        conventions = json.loads(json.dumps(CONVENTIONS))
        if rule is not None:
            conventions["preconditions"]["inputs"]["x"] = {"note": "whether it is active",
                                                           "rule": rule}
        if protocols is not None:
            conventions["protocols"] = protocols
        if normalise is not None:
            conventions["preconditions"]["normalise"] = normalise
        (self.root / "interface-model.json").write_text(json.dumps(MODEL), encoding="utf-8")
        (self.root / "conventions.json").write_text(json.dumps(conventions), encoding="utf-8")
        return self.root

    def problems(self, root: pathlib.Path) -> list[str]:
        return [str(p) for p in check_pack(root).problems]

    def refused(self, root: pathlib.Path, said: str) -> str:
        """`check_pack` lists the problem and `load_pack` refuses with the same
        sentence: never an exception of another type."""
        listed = self.problems(root)
        self.assertTrue(any(said in line for line in listed), listed)
        with self.assertRaises(PackError) as caught:
            load_pack(root)
        self.assertEqual(listed[0], str(caught.exception))
        return listed[0]


class ASymbolTheAddressDoesNotTake(Pack):
    def test_a_rule_comparing_an_address_with_a_symbol_it_does_not_admit_is_refused(self):
        sentence = self.refused(
            self.pack(rule={"address": "System.Input.Active", "equals": "TYPO"}),
            "does not admit")
        self.assertIn("'TYPO'", sentence)
        self.assertIn("equals", sentence)
        self.assertIn("ACTIVE, INACTIVE", sentence)

    def test_every_key_that_names_a_symbol_is_held_to_the_value_space(self):
        for key, value in (("not_equals", "TYPO"), ("becomes", "TYPO"),
                           ("equals_any", ["ACTIVE", "TYPO"])):
            with self.subTest(key=key):
                self.refused(
                    self.pack(rule={"address": "System.Input.Active", key: value}),
                    "does not admit")

    def test_a_symbol_the_address_does_take_is_clean(self):
        for rule in ({"address": "System.Input.Active", "equals": "ACTIVE"},
                     {"address": "System.Input.Active", "equals_any": ["ACTIVE", "INACTIVE"]},
                     {"address": "System.Input.Active", "not_equals": "INACTIVE"}):
            with self.subTest(rule=rule):
                self.assertEqual([], self.problems(self.pack(rule=rule)))

    def test_the_symbols_a_rule_names_are_listed_in_the_order_written(self):
        self.assertEqual(
            [("equals", "A"), ("not_equals", "B"), ("becomes", "C"),
             ("equals_any", "D"), ("equals_any", "E")],
            rule_symbols({"equals": "A", "not_equals": "B", "becomes": "C",
                          "equals_any": ["D", "E"], "address": "x"}))
        self.assertEqual([], rule_symbols({"address": "x"}))


class ARuleOfTheWrongShape(Pack):
    def test_parameters_that_are_not_an_object_are_a_problem_not_an_exception(self):
        sentence = self.refused(
            self.pack(rule={"protocol": "edge", "parameters": ["not-an-object"]},
                      protocols={"edge": {"parameters": ["signal"]}}),
            "parameters")
        self.assertIn("is not of type 'object'", sentence)

    def test_a_key_the_binding_vocabulary_does_not_have_is_named(self):
        sentence = self.refused(
            self.pack(rule={"address": "System.Input.Active", "equalss": "ACTIVE"}),
            "equalss")
        self.assertIn("Additional properties are not allowed", sentence)

    def test_a_rule_of_the_wrong_shape_is_not_held_to_anything_further(self):
        """It names an address the model lacks and a key nobody knows: the shape is
        the problem, said once, and the rest is not guessed at from a rule that is
        not one."""
        listed = self.problems(self.pack(rule={"address": "Nowhere", "equalss": "X"}))
        self.assertEqual(1, len(listed), listed)
        self.assertIn("equalss", listed[0])

    def test_previous_of_without_caller_keeps_is_a_rule_a_pack_may_state(self):
        """`caller_keeps` is the BINDING's commentary on why its caller holds the
        value; the rule a pack states is compared without it."""
        self.assertEqual([], self.problems(self.pack(rule={"previous_of": "x"})))


class AReplacementItsPatternCannotFill(Pack):
    def test_a_group_the_pattern_lacks_is_a_problem_at_load(self):
        sentence = self.refused(
            self.pack(normalise=[{"from": "x", "to": "\\g<missing>"}]),
            "is not a replacement")
        self.assertIn("preconditions.normalise", sentence)
        self.assertIn("missing", sentence)

    def test_a_numbered_group_beyond_the_pattern_and_a_bad_escape_are_problems(self):
        for replacement in ("\\2", "\\q"):
            with self.subTest(replacement=replacement):
                self.refused(self.pack(normalise=[{"from": "(x)", "to": replacement}]),
                             "is not a replacement")

    def test_a_replacement_that_fits_its_pattern_loads_and_applies_without_error(self):
        root = self.pack(normalise=[{"from": "(?P<word>colour)", "to": "\\g<word>s"},
                                    {"from": "(a)(b)", "to": "\\2\\1"}])
        self.assertEqual([], self.problems(root))
        for source, replacement in load_pack(root).conventions.normalise:
            re.sub(source, replacement, "colour ab")  # never raises

    def test_the_other_problems_of_the_pack_are_still_listed_beside_it(self):
        listed = self.problems(self.pack(
            rule={"address": "System.Input.Active", "equals": "TYPO"},
            normalise=[{"from": "x", "to": "\\g<missing>"}]))
        self.assertEqual(2, len(listed), listed)


if __name__ == "__main__":
    unittest.main()
