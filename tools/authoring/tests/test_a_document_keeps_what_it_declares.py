"""A document that needs the round before declares it, and is run that way.

A transform that reads `previous(x)` keeps a value from one activation to the
next, and the product generates a HOLDER for it: one object, one `update` per
activation. Until this, the only way to say "the round before" was for the
BINDING to feed the value back in, which left the document claiming a purity it
did not have and left the memory to whoever hosts the code.

So `verify` drives the holder the product names in its manifest: one per run,
one `update` per round -- setup steps included -- and every output read from
the record that one activation returns. What that needs, and refuses without:

  ordered examples   an activation before this one exists only on a timeline
  activation         what "previous" means is the host's schedule, not the
                     document's, and records replay only one activation each
                     (`on-change`)

and what it stops at, rather than judging on a guess: a round that could not be
driven, and a kept value that came to depend on an input nobody has placed.

The fixture subject matter is the one `test_refusals_actually_fire` uses -- a
supply and a lamp -- and means nothing.
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.verify import _default_codegen, verify
from tests.test_refusals_actually_fire import BINDING as FIXTURE_BINDING
from tests.test_refusals_actually_fire import CONVENTIONS, MODEL, Fixture

# ON only in the round the supply reaches HIGH. The document keeps the supply's
# last value itself -- the move `check` names for a binding that kept it. The
# count is there so a round can be left undrivable: a number the case does not
# supply has no reading.
KEPT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="edge">
  <datamodel>
    <data id="mode" sce:type="bool" sce:direction="in" sce:initial="false"/>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="(count &gt;= 0 &amp;&amp; mode &amp;&amp; !previous(mode)) ? 1 : 0"/>
  </datamodel>
</scxml>
"""

# The same lamp with no memory: ON whenever the supply is HIGH.
FORGETS = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="edge">
  <datamodel>
    <data id="mode" sce:type="bool" sce:direction="in"/>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out"
          expr="(count &gt;= 0 &amp;&amp; mode) ? 1 : 0"/>
  </datamodel>
</scxml>
"""

INPUTS = {**FIXTURE_BINDING["inputs"],
          "count": {"address": "Plant.Input.Count"}}

LOW, HIGH = 1, 2
OFF, ON = 1, 2


def case(name, supply, lamp, **extra):
    return {"name": name,
            "given": {"Plant.Input.SupplyMode": supply, "Plant.Input.Count": 1},
            "drove": ["Plant.Input.SupplyMode"],
            "expect": {"Plant.Out.Lamp.Stat": lamp}, **extra}


# ⚠ "rises" and "stays high" drive the SAME input and require different lamps,
# which no document answering from this round alone can satisfy. That pair is
# what makes a pass here mean the memory worked.
TIMELINE = [
    case("low", LOW, OFF),
    case("rises", HIGH, ON),
    case("stays high", HIGH, OFF),
    case("falls", LOW, OFF),
    case("rises again", HIGH, ON),
]


def examples(cases, ordered=True):
    return {"version": 1, "origin": "written for this test",
            "independent_cases": True, "ordered": ordered, "cases": list(cases)}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class ADocumentKeepsWhatItDeclares(Fixture):
    def run_document(self, document, cases, ordered=True, **binding_keys):
        (self.root / "edge.scxml").write_text(document, encoding="utf-8")
        binding = {**FIXTURE_BINDING, "document": "edge.scxml",
                   "inputs": INPUTS, **binding_keys}
        path = self.root / "edge.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        self.write_pack(MODEL, CONVENTIONS, examples(cases, ordered))
        return verify(self.pack(), path)

    def counts(self, result):
        return (result.passed, result.failed, result.unjudged)

    def detail(self, result):
        return [(r.name, r.refusal, r.failures) for r in result.results]

    # ------------------------------------------------------------- running

    def test_a_document_that_keeps_its_own_value_passes(self):
        result = self.run_document(KEPT, TIMELINE, activation="on-change")
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((5, 0, 0), self.counts(result), self.detail(result))

    def test_the_timeline_needs_the_memory(self):
        """The discriminator. Without it the pass above proves nothing: a
        document that lit the lamp whenever the supply was high would pass a
        timeline that never held the supply high twice."""
        result = self.run_document(FORGETS, TIMELINE)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(["stays high"],
                         [r.name for r in result.results if not r.passed])

    def test_a_setup_step_is_an_activation_too(self):
        """⚠ The holder moves on a `before` step as on any round. A setup that
        holds the supply high makes the case's own high round NOT a rise --
        which a holder advanced only on judged rounds would get wrong."""
        result = self.run_document(KEPT, [
            case("high, already", HIGH, OFF, before=[
                {"given": {"Plant.Input.SupplyMode": HIGH,
                           "Plant.Input.Count": 1},
                 "drove": ["Plant.Input.SupplyMode"]}])],
            activation="on-change")
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), self.counts(result), self.detail(result))

    # ------------------------------------------------------------ refusing

    def test_unordered_examples_are_refused(self):
        result = self.run_document(KEPT, TIMELINE, ordered=False,
                                   activation="on-change")
        self.assertFalse(result.ran)
        self.assertIn("ordered", result.refusal)

    def test_a_binding_that_does_not_say_how_the_host_runs_it_is_refused(self):
        result = self.run_document(KEPT, TIMELINE)
        self.assertFalse(result.ran)
        self.assertIn("activation", result.refusal)

    def test_a_periodic_host_cannot_be_replayed_from_records(self):
        result = self.run_document(KEPT, TIMELINE, activation="periodic")
        self.assertFalse(result.ran)
        self.assertIn("'periodic'", result.refusal)

    def test_a_document_that_keeps_nothing_needs_no_answer(self):
        """The discriminator for the two above: the key is asked of a
        document that keeps values, not of every document."""
        result = self.run_document(FORGETS, TIMELINE, ordered=False)
        self.assertTrue(result.ran, result.refusal)

    # --------------------------------------------------- stopping, not guessing

    def test_a_round_that_cannot_be_driven_stops_what_follows(self):
        """⚠ An activation that did not happen is one the kept values never
        saw. Judging the next round from the state before it would judge a
        state the host could not have been in."""
        broken = {"name": "no count reported",
                  "given": {"Plant.Input.SupplyMode": HIGH},
                  "drove": ["Plant.Input.SupplyMode"],
                  "expect": {"Plant.Out.Lamp.Stat": ON}}
        result = self.run_document(
            KEPT, [TIMELINE[0], broken, TIMELINE[1]], activation="on-change")
        self.assertTrue(result.ran, result.refusal)
        by_name = {r.name: r for r in result.results}
        self.assertTrue(by_name["low"].passed, self.detail(result))
        self.assertTrue(by_name["no count reported"].refusal, self.detail(result))
        self.assertIn("could not be driven", by_name["rises"].refusal)

    def test_a_kept_value_that_depends_on_an_unplaced_input_stops_the_run(self):
        """⚠ An input nobody has placed is tried at both values -- one holder
        each. That is sound while the holders keep the same values; the
        round after which they do not is where the kept values stop being
        known, and nothing later is judged on either guess."""
        document = KEPT.replace(
            '<data id="lamp"',
            '<data id="gate" sce:type="bool" sce:direction="in" '
            'sce:initial="false"/>\n    <data id="lamp"'
        ).replace("!previous(mode)", "!previous(mode) &amp;&amp; previous(gate)")
        inputs = {**INPUTS,
                  "gate": {"unresolved": "the platform list is not available"}}
        result = self.run_document(document, TIMELINE, activation="on-change",
                                   inputs=inputs)
        self.assertTrue(result.ran, result.refusal)
        later = [r for r in result.results if r.name != "low"]
        self.assertTrue(later)
        for r in later:
            self.assertIn("no later round starts from a known state", r.refusal,
                          self.detail(result))

    def test_an_unplaced_input_the_kept_values_never_read_is_tried_as_before(self):
        """The discriminator: the same unplaced input, kept by nothing. Every
        holder keeps the same values, so every round is still judged -- and
        the lamp, which reads the input, is withheld rather than guessed."""
        document = KEPT.replace(
            '<data id="lamp"',
            '<data id="gate" sce:type="bool" sce:direction="in"/>\n'
            '    <data id="lamp"'
        ).replace("!previous(mode)", "!previous(mode) &amp;&amp; gate")
        inputs = {**INPUTS,
                  "gate": {"unresolved": "the platform list is not available"}}
        result = self.run_document(document, TIMELINE, activation="on-change",
                                   inputs=inputs)
        self.assertTrue(result.ran, result.refusal)
        for r in result.results:
            self.assertNotIn("no later round starts from a known state",
                             r.refusal, self.detail(result))
        # The lamp reads the unplaced input in the rounds that rise, and is
        # withheld there -- neither passed nor failed on a guess.
        self.assertEqual(["rises", "rises again"],
                         [r.name for r in result.results if r.undetermined],
                         self.detail(result))


class CheckNamesTheMove(Fixture):
    def test_a_binding_that_remembers_is_told_where_the_memory_goes(self):
        """`check` refused a transform whose binding reaches back before this,
        and said only that the kind did not say so. It now names the move:
        `previous()` in the document, with the field's `sce:initial`."""
        binding = {**FIXTURE_BINDING, "inputs": {
            **FIXTURE_BINDING["inputs"],
            "was": {"previous_of": "mode", "initial": False,
                    "caller_keeps": "measuring the named move"}}}
        found = str(self.bind(binding))
        self.assertIn("reads an earlier round", found)
        self.assertIn("previous(mode)", found)
        self.assertIn("sce:initial", found)
        self.assertIn("drop the input 'was'", found)


if __name__ == "__main__":
    unittest.main()
