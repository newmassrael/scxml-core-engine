"""The one published key that reached the verdict with no test behind it.

Measured 2026-09-20 across the whole vocabulary and both subject matters:
`not_equals` was read by `check` and again by `verify`, and exercised by no
test, no pack and no binding anywhere. Live code on the path a verdict comes
down, held by nothing.

⚠ THAT IS WORSE THAN AN UNUSED KEY, and the distinction is why this file
exists rather than a deletion. An unused key is dead weight in a schema; an
untested one that RUNS is a branch the tool takes and nobody has ever watched
it take. The reachable-but-unwatched branch is the one that turns into a wrong
answer quietly.

⚠⚠ Its schema entry states a reason -- "a negation stays correct when the
enumeration grows, and a positive list silently narrows" -- and a stated
reason with no case behind it is exactly what this tree keeps finding to be
false. So the last case here IS that sentence, driven.
"""

from __future__ import annotations

import unittest

from sce_author.pack import Case as RealCase
from sce_author.pack import Entry, Field, Model
from sce_author.verify import input_value


def case(given):
    return RealCase("", given, {}, None, ())


def model_with(values):
    """One symbol-valued address, so the comparison goes through a space."""
    entry = Entry(address="plant/in/mode", role="input", names=("mode",),
                  fields=(Field(name="", values=values, type=None),))
    built = Model()
    built.entries.append(entry)
    built.by_address[entry.address] = entry
    for name in entry.names:
        built.by_name.setdefault(name, []).append(entry)
    return built


SPACE = {"OFF": 0, "LOW": 1, "HIGH": 2}
DRIVEN = {"plant/in/mode": 2}          # the record wrote the NUMBER


class ANegationIsEvaluatedAndNotAssumed(unittest.TestCase):
    def read(self, rule, given=None):
        return input_value("mode", {"address": "plant/in/mode", **rule},
                           case(given or DRIVEN), None, model_with(SPACE))

    def test_it_is_false_when_the_address_holds_the_named_symbol(self):
        self.assertFalse(self.read({"not_equals": "HIGH"}))

    def test_it_is_true_when_the_address_holds_any_other(self):
        self.assertTrue(self.read({"not_equals": "LOW"}))

    def test_it_is_the_negation_of_the_positive_rule_it_mirrors(self):
        """⚠ The discriminator. Each case above is satisfied by a branch that
        returns a constant; this one fails unless the two are opposites, over
        every symbol the space admits.
        """
        for symbol in SPACE:
            positive = self.read({"equals": symbol})
            negative = self.read({"not_equals": symbol})
            self.assertEqual(bool(positive), not bool(negative),
                             f"{symbol}: equals and not_equals agreed")

    def test_the_comparison_goes_through_the_value_space(self):
        """The record writes `2` where the binding writes `HIGH`, because a
        record and a specification are kept by different people. Comparing the
        spellings makes every symbol comparison false -- and a negation that
        is wrong this way reads as TRUE, which is the direction that passes.
        """
        self.assertFalse(self.read({"not_equals": "HIGH"}))
        self.assertFalse(self.read({"not_equals": "HIGH"},
                                   given={"plant/in/mode": "HIGH"}))

    def test_a_negation_survives_the_enumeration_growing(self):
        """⚠ THE SCHEMA'S OWN STATED REASON, driven rather than asserted.

        `not_equals` is kept distinct from `equals_any` on the ground that a
        negation stays correct when a value space grows while a positive list
        silently narrows. Nothing had ever shown that. Here the space gains a
        symbol the binding was written before: the positive list stops
        covering the new value, and the negation still does.
        """
        wider = dict(SPACE, CRITICAL=3)
        arrived = {"plant/in/mode": 3}

        def read(rule):
            return input_value("mode", {"address": "plant/in/mode", **rule},
                               case(arrived), None, model_with(wider))

        # "anything that is not OFF" -- written when OFF/LOW/HIGH was all there was.
        self.assertTrue(read({"not_equals": "OFF"}),
                        "the negation stopped covering a value added later")
        # The same intent as a positive list, written the same day.
        self.assertFalse(read({"equals_any": ["LOW", "HIGH"]}),
                         "fixture precondition: the positive list must be the "
                         "one that narrows, or the pair proves nothing")


if __name__ == "__main__":
    unittest.main()
