"""A position the binding writes is counted as written, however it is written.

A binding writes an output's value at `address.field`, and it may write more
beside it: `also` fills fields that accompany every value -- an event's ID next
to its status -- and `when` fills fields that accompany a particular one.
`output_values` has always written all three.

The COUNT did not. Both run paths counted `address.field` alone, so a run that
produced and judged an event's ID also reported, in the line beside its
counts, that "the examples read N address(es) the binding never writes" -- the
ID among them. A report that contradicts the run it summarises is read as the
run being wrong. And the count lived in two copies, one per path, which is how
a fix to one would have left the other still saying it.
"""

from __future__ import annotations

import unittest

from sce_author.verify import output_values, written_positions


class WhatABindingWritesIsCountedAsWritten(unittest.TestCase):
    def test_the_value_and_the_fields_beside_it_are_all_written(self):
        outputs = {"lamp": {
            "address": "plant/out/lamp", "field": "stat",
            "map": {0: "OFF", 1: "ON"},
            "also": {"id": "E1"},
            "when": {1: {"sound": "BEEP"}},
        }}
        bound, writes = written_positions(outputs)
        self.assertEqual({"plant/out/lamp.stat", "plant/out/lamp.id",
                          "plant/out/lamp.sound"}, bound)
        self.assertEqual({"lamp"}, set(writes.values()))

    def test_the_count_agrees_with_what_is_written(self):
        """The discriminator: whatever `output_values` can write, the count
        must include -- otherwise the two lines of one report disagree."""
        rule = {"address": "plant/out/lamp", "field": "stat",
                "map": {0: "OFF", 1: "ON"}, "also": {"id": "E1"},
                "when": {1: {"sound": "BEEP"}}}
        bound, _ = written_positions({"lamp": rule})
        for computed in (0, 1):
            self.assertLessEqual(set(output_values("lamp", rule, computed)), bound)

    def test_an_internal_or_unresolved_output_writes_nothing(self):
        outputs = {
            "scratch": {"internal": True},
            "later": {"unresolved": "the platform list is not available yet"},
        }
        self.assertEqual((set(), {}), written_positions(outputs))


if __name__ == "__main__":
    unittest.main()
