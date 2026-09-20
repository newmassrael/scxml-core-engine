"""The run keeps values the product will not, and now it says which.

A document that asks for a previous round is asking something OUTSIDE it to
keep one. The generated code takes this round's inputs and nothing else, so in
production the CALLER has to hold those values between calls. In verification
nobody negotiates that: the verifier holds them, for free, and the report said
nothing.

⚠ THE SHAPE IS THE ONE THIS PACKAGE ALREADY REFUSES ELSEWHERE. `check` turns
down a document that calls itself a pure computation while its binding reads a
remembered value, on the ground that everything reading the document is then
told it needs no memory and is wrong. The verifier was doing the same thing to
its own reader: a pass, with the memory that made it possible unmentioned.

⚠⚠ All three shapes count. `previous_of` and `state_of` name the previous
round outright; a protocol carrying a `latch` answers by which parameter moved
most recently, which is a fact about the round before. Counting two of the
three would under-report, and under-reporting is the defect being removed
rather than a smaller version of it.
"""

from __future__ import annotations

import unittest

from sce_author.verify import host_memory_of


class Conventions:
    """Only the part `host_memory_of` reads."""

    def __init__(self, protocols=None):
        self.protocols = protocols or {}


class ARunSaysWhatItRemembered(unittest.TestCase):
    def test_an_input_read_from_this_round_is_not_remembered(self):
        inputs = {"approaching": {"address": "plant/in/a", "equals": "YES"}}
        self.assertEqual([], host_memory_of(inputs, Conventions()))

    def test_previous_of_is_a_value_something_else_keeps(self):
        inputs = {"wasApproaching": {"previous_of": "approaching"}}
        self.assertEqual(["wasApproaching"],
                         host_memory_of(inputs, Conventions()))

    def test_state_of_is_one_too(self):
        """The document's own last output is still a round that has gone."""
        inputs = {"lastShown": {"state_of": "signal", "initial": 0}}
        self.assertEqual(["lastShown"], host_memory_of(inputs, Conventions()))

    def test_a_latched_protocol_is_one_too(self):
        """⚠ The shape a count of two would miss.

        A latch answers by which parameter changed most recently, so it can
        only be read by something that saw the round before.
        """
        conv = Conventions({"last-incremented": {"latch": {"both": "last"}}})
        inputs = {"supplyOn": {"protocol": "last-incremented",
                               "parameters": {"on_counter": "plant/count/on"}}}
        self.assertEqual(["supplyOn"], host_memory_of(inputs, conv))

    def test_a_protocol_without_a_latch_keeps_nothing(self):
        """A way of READING is not a way of remembering."""
        conv = Conventions({"scaled": {"parameters": ["raw"]}})
        inputs = {"level": {"protocol": "scaled",
                            "parameters": {"raw": "plant/in/raw"}}}
        self.assertEqual([], host_memory_of(inputs, conv))

    def test_a_protocol_the_pack_never_defined_keeps_nothing_it_can_name(self):
        """⚠ Silence about a protocol is not evidence that it latches.

        The core has nothing to read, so it says nothing here rather than
        guessing — the same answer `check` gives when asked whether such a
        document needs memory.
        """
        inputs = {"supplyOn": {"protocol": "never-declared"}}
        self.assertEqual([], host_memory_of(inputs, Conventions()))

    def test_reaching_back_without_saying_who_keeps_it_is_refused(self):
        """⚠ The other end of the same fact, enforced where it is written.

        The figure above reports the obligation once a run happens. This
        refuses the binding that creates one without naming a reason —
        because the two honest answers are usually available and rarely
        considered: a memory-bearing kind can hold the value itself, and a
        platform that publishes the earlier value has an address for it.
        Defaulting to "the caller will remember" is the third answer, and it
        is the one nobody has to agree to out loud.
        """
        import pathlib
        import tempfile

        import yaml

        from sce_author.check import read_binding
        from sce_author.errors import PackError

        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "b.yaml"
            path.write_text(yaml.safe_dump({
                "version": 1, "document": "d.scxml",
                "inputs": {"wasApproaching": {"previous_of": "approaching"}},
            }), encoding="utf-8")
            with self.assertRaises(PackError) as caught:
                read_binding(path)
            self.assertIn("caller_keeps", str(caught.exception))

    def test_the_names_come_back_sorted_and_whole(self):
        inputs = {
            "zLast": {"state_of": "signal"},
            "aPrev": {"previous_of": "approaching"},
            "plain": {"address": "plant/in/a", "equals": "YES"},
        }
        self.assertEqual(["aPrev", "zLast"], host_memory_of(inputs, Conventions()))


if __name__ == "__main__":
    unittest.main()
