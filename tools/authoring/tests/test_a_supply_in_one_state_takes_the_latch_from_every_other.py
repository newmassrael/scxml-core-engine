"""A reading that is in exactly one state at a time, held by a latch over two parameters.

The pack's supply is a set of counters where the one that moved last is the state:
`ElapsedOn0ms` and its longer rungs, `ElapsedOff0ms` at the bottom of the ladder and
`ElapsedOff700ms` beside it. A latch over two parameters hears one other state only --
"IGN1 on" cleared on `ElapsedOff0ms` and "IGN1 off after 700 ms" on `ElapsedOn0ms` -- so a
round that moved only `ElapsedOff700ms` left the first true, one that moved only
`ElapsedOff0ms` left the second true, and the rung the ladder remembered outlived a move to a
state that is not on the ladder. The platform holds one state, and all three were false there.
Measured 2026-10-07: five cases of one component, each judged right by the host.

A pack says which addresses are the states of one reading with `latch.exclusive`.
"""

from __future__ import annotations

import unittest

from sce_author.pack import Case as RealCase, Entry, Field, Model
from sce_author.verify import Latches, input_value

OFF0, ON0, ON500, OFF700 = "plant/off0", "plant/on0", "plant/on500", "plant/off700"
LADDER = [OFF0, ON0, ON500]

PROTOCOLS = {
    "last-incremented": {
        "parameters": ["on_counter", "off_counter"],
        "latch": {
            "set_when_changed": "on_counter",
            "clear_when_changed": "off_counter",
            "both": "last",
            "initial": "clear",
            "cumulative": LADDER,
            "exclusive": [*LADDER, OFF700],
        },
    }
}

ON_SIDE = {"protocol": "last-incremented",
           "parameters": {"on_counter": ON0, "off_counter": OFF0}}
LONGER_ON_SIDE = {"protocol": "last-incremented",
                  "parameters": {"on_counter": ON500, "off_counter": OFF0}}
AFTER_700 = {"protocol": "last-incremented",
             "parameters": {"on_counter": OFF700, "off_counter": ON0}}


class Conventions:
    def __init__(self, protocols):
        self.protocols = protocols


def case(*drove):
    """⚠ The REAL case type, as in the neighbouring protocol tests."""
    return RealCase("", {a: 1 for a in drove}, {}, None, tuple(drove))


def receiving(addresses):
    entries = [Entry(a, "input", (), (Field("", None, "uint64"),)) for a in addresses]
    return Model(entries=entries, by_address={e.address: e for e in entries})


class ASupplyInOneStateTakesTheLatchFromEveryOther(unittest.TestCase):
    def setUp(self):
        self.latches = Latches(Conventions(PROTOCOLS))

    def read(self, *drove):
        """What the three inputs say after a round that moved `drove`."""
        c = case(*drove)
        return {name: input_value(name, rule, c, self.latches)
                for name, rule in (("on", ON_SIDE), ("longer", LONGER_ON_SIDE),
                                   ("after700", AFTER_700))}

    def test_a_move_to_a_state_off_the_ladder_clears_the_on_side(self):
        self.read(ON0)
        said = self.read(OFF700)
        self.assertFalse(said["on"], "the supply is off after 700 ms, not on")
        self.assertTrue(said["after700"])

    def test_the_rung_the_ladder_remembered_does_not_outlive_it(self):
        """⚠ The second half: a round that moves nothing on the ladder asked the ladder where
        it stood, and it answered with the rung it had held before the supply left it."""
        self.read(ON0)
        self.read(OFF700)
        quiet = self.read("plant/unrelated")
        self.assertFalse(quiet["on"])
        self.assertFalse(quiet["longer"])
        self.assertTrue(quiet["after700"])

    def test_a_move_to_the_bottom_of_the_ladder_clears_the_state_off_the_ladder(self):
        self.read(OFF700)
        said = self.read(OFF0)
        self.assertFalse(said["after700"], "the supply is at off 0 ms now, not at off 700 ms")
        self.assertFalse(said["on"])

    def test_a_rung_on_the_ladder_takes_the_supply_back(self):
        self.read(ON0)
        self.read(OFF700)
        said = self.read(ON0)
        self.assertTrue(said["on"])
        self.assertFalse(said["after700"])

    def test_a_longer_rung_alone_is_on_and_not_off_after_700(self):
        """The ladder's own order is untouched: reaching a longer rung includes the shorter."""
        self.read(OFF700)
        said = self.read(ON500)
        self.assertTrue(said["longer"])
        self.assertTrue(said["on"])
        self.assertFalse(said["after700"])

    def test_of_a_rung_and_a_state_off_the_ladder_in_one_round_the_one_listed_last_holds(self):
        self.read(OFF0)
        self.assertEqual({"on": True, "longer": False, "after700": False},
                         self.read(OFF700, ON0))
        later = Latches(Conventions(PROTOCOLS))
        self.latches = later
        self.read(OFF0)
        self.assertEqual({"on": False, "longer": False, "after700": True},
                         self.read(ON0, OFF700))

    def test_a_pack_that_declares_nothing_exclusive_reads_as_it_always_did(self):
        """The discriminator: the key is what changes the answer, nothing else."""
        plain = {"last-incremented": {
            "parameters": PROTOCOLS["last-incremented"]["parameters"],
            "latch": {k: v for k, v in PROTOCOLS["last-incremented"]["latch"].items()
                      if k != "exclusive"}}}
        self.latches = Latches(Conventions(plain))
        self.read(ON0)
        self.assertTrue(self.read(OFF700)["on"], "no state is taken away without the declaration")

    def test_a_state_the_component_does_not_receive_is_not_heard(self):
        """As for `cumulative`: the platform's whole supply is the pack's, and a component is
        handed only the counters it reads."""
        model = receiving([OFF0, ON0, ON500])
        self.latches = Latches(Conventions(PROTOCOLS), model)
        self.read(ON0)
        self.assertTrue(self.read(OFF700)["on"],
                        "a counter the component does not read moved nothing it can see")


if __name__ == "__main__":
    unittest.main()
