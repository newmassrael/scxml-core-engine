"""What a round writes, and whether there is one, as the platform has it.

Two places where the run and the platform parted on 2026-09-27, both found by
setting `verify`'s verdicts beside the platform's own, case by case:

    a case whose drives reach no input the binding reads runs nothing on an
    `on-change` host, so a host that states its writes announces nothing and
    the case fails -- where it passed, computed anyway     (the discriminator)
    such a failure is at the positions the case waited on, so the guess
    behind them is blamed                               (the discriminator)
    without the host stated, the run is as it was
    a drive that does reach a bound input is a round as before
    a restatement the record says the platform DELIVERED is a round
                                                        (the discriminator)
    a delivery of something the step did not drive is refused

    `check`: two `when` values that land the same value write different
    fields of one record, so one shown after the other carries its fields
                                                        (the discriminator)
    every such value writing every field is not refused
    a value going off that writes fewer fields than the ways of showing is
    not compared with them

A third, found on 2026-10-14 against a platform's acceptance validator:

    under `host.rounds: per-delivery` a step that drives two deliveries is two
    rounds in the order driven, read once settled  (the discriminator)
    without `rounds` a step is one round, as it was
    addresses the model puts in one delivery are one round
    a round another delivery follows lands what it wrote (`hold_last`)
    a transform is activated once per delivery: what it keeps moves between
    them                                          (the discriminator)
    `delivered_with` on an output is refused
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest

import yaml

from sce_author.errors import PackError
from sce_author.pack import load_model
from sce_author.verify import _default_codegen, deliveries, verify
from tests.test_refusals_actually_fire import CONVENTIONS, MODEL, Fixture

CONSTANT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="constant">
  <datamodel>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="lamp" sce:type="string" sce:direction="out" expr="'on'"/>
  </datamodel>
</scxml>
"""

# The lamp is one of a closed set of symbols, so the document names it and the map lands the name.
CONSTANT_BINDING = {
    "version": 1, "document": "constant.scxml",
    "inputs": {"count": {"address": "Plant.Input.Count", "when_absent": 0}},
    "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Stat",
                         "map": {"on": "ON"}}},
}

HOST = {"activation": "on-change", "writes": "every-round"}


def case(name, drove, value):
    return {"name": name, "given": {drove: value}, "drove": [drove],
            "expect": {"Plant.Out.Lamp.Stat": "ON"}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class ARoundIsWhatTheHostRuns(Fixture):
    def run_cases(self, cases, host, document=CONSTANT):
        conventions = {**CONVENTIONS, "host": host} if host else CONVENTIONS
        self.write_pack(MODEL, conventions, {
            "version": 1, "origin": "written for this test", "ordered": True,
            "independent_cases": False, "cases": cases})
        (self.root / "constant.scxml").write_text(document, encoding="utf-8")
        path = self.root / "constant.binding.yaml"
        path.write_text(yaml.safe_dump(CONSTANT_BINDING), encoding="utf-8")
        result = verify(self.pack(), path)
        self.assertTrue(result.ran, result.refusal)
        return result

    def test_a_drive_that_reaches_no_bound_input_is_no_round(self):
        """⚠ The discriminator. The supply mode is an input of this
        component, and the binding reads only the count: changing the mode
        runs nothing on this host. The document's constant was computed and
        passed anyway; the platform announced nothing and failed it."""
        result = self.run_cases([
            case("counted", "Plant.Input.Count", 3),
            case("supply changed", "Plant.Input.SupplyMode", "HIGH")], HOST)
        verdicts = {r.name: r for r in result.results}
        self.assertTrue(verdicts["counted"].passed)
        failed = verdicts["supply changed"]
        self.assertFalse(failed.passed)
        self.assertEqual("Plant.Out.Lamp.Stat", failed.failures[0][0])
        self.assertIn("no input the binding reads changed", failed.failures[0][2])

    def test_a_round_that_never_ran_is_blamed_on_the_guess_behind_the_position(self):
        """⚠ The discriminator for attribution. The author wrote the lamp as
        a constant because the text never decides it; a case changing only
        the input the platform decides it by is failed by that choice. It
        used to fail at a pseudo-position no guess stands behind, and the
        guess came back as one nothing compared."""
        guessed = CONSTANT.replace(
            '''expr="'on'"''', '''expr="'on'" sce:assumed="lamp-constant" '''
                        'sce:assumed-reason="the text never decides the lamp"')
        result = self.run_cases([
            case("counted", "Plant.Input.Count", 3),
            case("supply changed", "Plant.Input.SupplyMode", "HIGH")], HOST, guessed)
        guess = result.assumptions["document:lamp"]
        self.assertEqual("refuted", guess.status)
        self.assertEqual("supply changed", guess.refuted_by[0][0])

    def test_a_restated_bound_input_is_no_round_either(self):
        result = self.run_cases([
            case("counted", "Plant.Input.Count", 3),
            case("counted again", "Plant.Input.Count", 3)], HOST)
        self.assertFalse({r.name: r for r in result.results}["counted again"].passed)

    def test_a_restatement_the_platform_delivered_is_a_round(self):
        """⚠ The discriminator for `delivered`. The same symbol twice is no
        change to the core -- but the platform decides on the reading it
        carries, which the record knows and the value space does not."""
        again = case("counted again", "Plant.Input.Count", 3)
        again["delivered"] = ["Plant.Input.Count"]
        result = self.run_cases([case("counted", "Plant.Input.Count", 3), again], HOST)
        self.assertTrue({r.name: r for r in result.results}["counted again"].passed)

    def test_a_delivery_of_something_not_driven_is_refused(self):
        stray = case("counted", "Plant.Input.Count", 3)
        stray["delivered"] = ["Plant.Input.SupplyMode"]
        self.write_pack(MODEL, {**CONVENTIONS, "host": HOST}, {
            "version": 1, "origin": "written for this test", "ordered": True,
            "cases": [stray]})
        with self.assertRaises(PackError) as caught:
            self.pack()
        self.assertIn("a delivery is of a drive", str(caught.exception))

    def test_without_the_host_stated_the_run_is_as_it_was(self):
        result = self.run_cases([
            case("counted", "Plant.Input.Count", 3),
            case("supply changed", "Plant.Input.SupplyMode", "HIGH")], None)
        self.assertTrue(all(r.passed for r in result.results),
                        [(r.name, r.failures) for r in result.results])


# A record address: an event's status, its identifier and its sound.
EVENT_MODEL = {**MODEL, "entries": MODEL["entries"] + [{
    "address": "Plant.Out.Event", "role": "output", "names": ["Fig_Event"],
    "fields": {"Stat": {"values": {"OFF": 1, "ON": 2}}, "ID": {"type": "text"},
               "Count": {"type": "number"}, "Duration": {"type": "number"}}}]}

EVENT_DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="fixture">
  <datamodel>
    <data id="count" sce:type="int32" sce:direction="in"/>
    <data id="event" sce:type="string" sce:direction="out"
          expr="count &gt; 10 ? 'many' : (count &gt; 0 ? 'few' : 'none')"/>
  </datamodel>
</scxml>
"""


# Three values of the document's own, two of which land the same symbol.
def event_binding(when):
    return {"version": 1, "document": "fixture.scxml",
            "inputs": {"count": {"address": "Plant.Input.Count", "when_absent": 0}},
            "outputs": {"event": {"address": "Plant.Out.Event", "field": "Stat",
                                  "map": {"none": "OFF", "few": "ON", "many": "ON"},
                                  "when": when}}}


class ARecordIsWrittenWhole(Fixture):
    def found(self, when):
        self.write_pack(EVENT_MODEL, CONVENTIONS)
        (self.root / "fixture.scxml").write_text(EVENT_DOCUMENT, encoding="utf-8")
        return [str(f) for f in self.bind(event_binding(when))]

    def test_two_ways_of_showing_that_write_different_fields_are_refused(self):
        """⚠ The discriminator. Shown as `few` after `many`, the event would carry
        `many`'s duration: `few` does not write one."""
        found = self.found({"few": {"ID": "A", "Count": 1},
                            "many": {"ID": "B", "Duration": 10}})
        said = [f for f in found if "write different fields" in f]
        self.assertEqual(1, len(said), found)
        self.assertIn("'few' leaves Duration", said[0])
        self.assertIn("'many' leaves Count", said[0])

    def test_every_way_of_showing_writing_every_field_is_not_refused(self):
        found = self.found({"few": {"ID": "A", "Count": 1, "Duration": 0},
                            "many": {"ID": "B", "Count": 0, "Duration": 10}})
        self.assertFalse([f for f in found if "write different fields" in f], found)

    def test_going_off_is_not_compared_with_showing(self):
        """The identifier says what is turning off; the sound is not
        rewritten. That is a different value of the event's status, and not
        a second story about the same one."""
        found = self.found({"none": {"ID": "A"},
                            "few": {"ID": "A", "Count": 1, "Duration": 0},
                            "many": {"ID": "B", "Count": 0, "Duration": 10}})
        self.assertFalse([f for f in found if "write different fields" in f], found)


# Two inputs the platform hands over in two deliveries, unless a test names one for both.
A, B = "Plant.Input.A", "Plant.Input.B"


def latch_model(a_delivery="left", b_delivery="right"):
    def entry(address, name, delivery):
        out = {"address": address, "role": "input", "names": [name],
               "values": {"OFF": 0, "ON": 1}}
        if delivery:
            out["delivered_with"] = delivery
        return out
    return {**MODEL, "entries": MODEL["entries"] + [
        entry(A, "In_A", a_delivery), entry(B, "In_B", b_delivery)]}


# `a` turns the lamp on; `b` reaches the machine and sends nothing.
LATCHES = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="idle" sce:kind="statechart" name="latches">
  <state id="idle">
    <transition event="a.on"><send event="on" type="x-sce-lamp"/></transition>
    <transition event="b.on a.off b.off"/>
  </state>
</scxml>
"""

LATCH_BINDING = {
    "version": 1, "document": "latches.scxml", "activation": "on-change",
    "inputs": {"aOn": {"address": A, "becomes": "ON", "event": "a.on"},
               "aOff": {"address": A, "becomes": "OFF", "event": "a.off"},
               "bOn": {"address": B, "becomes": "ON", "event": "b.on"},
               "bOff": {"address": B, "becomes": "OFF", "event": "b.off"}},
    "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Stat",
                         "sent": {"processor": "x-sce-lamp"}, "when_nothing_sent": "off",
                         "map": {"on": "ON", "off": "OFF"}}},
}

# The lamp is on while `a` is and `b` is not; the slot keeps where it last showed
# (`hold_last`) unless `kept` is false.
KEEPER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="keeper">
  <datamodel>
    <data id="a" sce:type="bool" sce:direction="in"/>
    <data id="b" sce:type="bool" sce:direction="in"/>
    <data id="lamp" sce:type="string" sce:direction="out"
          expr="a &amp;&amp; !b ? 'shown' : 'quiet'"/>
  </datamodel>
</scxml>
"""


# `quiet` is the value the map leaves out under `hold_last`: the slot keeps what it showed.
def keeper_binding(kept):
    lamp = ({"map": {"shown": "ON"}, "hold_last": True} if kept
            else {"map": {"shown": "ON", "quiet": "OFF"}})
    return {"version": 1, "document": "keeper.scxml", "activation": "on-change",
            "inputs": {"a": {"address": A, "equals": "ON"}, "b": {"address": B, "equals": "ON"}},
            "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Stat", **lamp}}}


def both_on(name, order, lamp):
    """Both inputs turn on in one step, driven in `order`, after a setup that turns both off."""
    return {"name": name, "given": {A: "ON", B: "ON"}, "drove": order,
            "expect": {"Plant.Out.Lamp.Stat": lamp},
            "before": [{"given": {A: "OFF", B: "OFF"}, "drove": [A, B]}]}


PER_DELIVERY = {**HOST, "rounds": "per-delivery"}


class AStepIsCutIntoTheRoundsTheHostRuns(Fixture):
    """What a step that drives two deliveries reads as, per `host.rounds`.

    ⚠ The shape measured on a platform's acceptance validator (2026-10-14): the first round
    turns an output on, the second sends it nothing, and the case reads the step settled.
    One round per step reads the last send of the whole step and says ON; the platform
    said OFF, and failed the case that one-round reading passed.
    """

    def judge(self, document, name, binding, cases, host, model=None):
        conventions = {**CONVENTIONS, "host": host} if host else CONVENTIONS
        self.write_pack(model or latch_model(), conventions, {
            "version": 1, "origin": "written for this test", "ordered": True,
            "independent_cases": False, "cases": cases})
        (self.root / name).write_text(document, encoding="utf-8")
        path = self.root / name.replace(".scxml", ".binding.yaml")
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        result = verify(self.pack(), path)
        self.assertTrue(result.ran, result.refusal)
        return {r.name: r for r in result.results}

    def statechart(self, cases, host, model=None):
        return self.judge(LATCHES, "latches.scxml", LATCH_BINDING, cases, host, model)

    @unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
    def test_the_step_is_read_after_its_last_delivery(self):
        """⚠ The discriminator: `a then b` reads OFF -- `b`'s round sent the lamp nothing."""
        verdicts = self.statechart([
            both_on("a then b, off", [A, B], "OFF"),
            both_on("b then a, on", [B, A], "ON"),
            both_on("a then b, on", [A, B], "ON")], PER_DELIVERY)
        self.assertTrue(verdicts["a then b, off"].passed, verdicts["a then b, off"].failures)
        self.assertTrue(verdicts["b then a, on"].passed, verdicts["b then a, on"].failures)
        self.assertFalse(verdicts["a then b, on"].passed,
                         "one round per step: the platform failed this case")

    @unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
    def test_without_rounds_stated_a_step_is_one_round_as_it_was(self):
        verdicts = self.statechart([both_on("a then b, on", [A, B], "ON")], HOST)
        self.assertTrue(verdicts["a then b, on"].passed, verdicts["a then b, on"].failures)

    @unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
    def test_addresses_of_one_delivery_are_one_round(self):
        verdicts = self.statechart([both_on("a then b, on", [A, B], "ON")], PER_DELIVERY,
                                   latch_model("both", "both"))
        self.assertTrue(verdicts["a then b, on"].passed, verdicts["a then b, on"].failures)

    @unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
    def test_a_transform_is_activated_once_per_delivery(self):
        """⚠ The discriminator for a computation: `a then b` is two activations. The first sees
        `a` alone and shows the lamp, the second sees both and writes nothing, so the slot
        keeps ON. Read as one activation, the step sees both, writes nothing, and the slot
        was never written; driven `b then a`, neither activation shows it."""
        def run(cases, model=None):
            return self.judge(KEEPER, "keeper.scxml", keeper_binding(True), cases,
                              PER_DELIVERY, model)
        verdicts = run([both_on("a then b", [A, B], "ON")])
        self.assertTrue(verdicts["a then b"].passed,
                        (verdicts["a then b"].failures, verdicts["a then b"].refusal))
        verdicts = run([both_on("b then a", [B, A], "ON")])
        self.assertFalse(verdicts["b then a"].passed)
        verdicts = run([both_on("a then b, one delivery", [A, B], "ON")],
                       latch_model("both", "both"))
        self.assertFalse(verdicts["a then b, one delivery"].passed,
                         "one delivery is one activation, which sees both")

    @unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
    def test_a_transform_that_keeps_nothing_reads_the_settled_inputs(self):
        verdicts = self.judge(KEEPER, "keeper.scxml", keeper_binding(False),
                              [both_on("both on", [A, B], "OFF")], PER_DELIVERY)
        self.assertTrue(verdicts["both on"].passed, verdicts["both on"].failures)

    @unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
    def test_a_round_another_follows_lands_what_it_wrote(self):
        """`a` writes the lamp on, `b` writes it nothing, and the lamp keeps its last value
        (`hold_last`): the settled step shows what the FIRST round wrote. Without that round
        landed, the slot would still hold what the setup left."""
        held = LATCHES.replace(
            '<transition event="b.on a.off b.off"/>',
            '<transition event="a.off"><send event="off" type="x-sce-lamp"/></transition>\n'
            '    <transition event="b.on b.off"/>')
        binding = {**LATCH_BINDING, "outputs": {"lamp": {
            "address": "Plant.Out.Lamp", "field": "Stat", "sent": {"processor": "x-sce-lamp"},
            "when_nothing_sent": "quiet", "map": {"on": "ON", "off": "OFF"}, "hold_last": True}}}
        verdicts = self.judge(held, "latches.scxml", binding,
                              [both_on("a then b", [A, B], "ON")], PER_DELIVERY)
        self.assertTrue(verdicts["a then b"].passed,
                        (verdicts["a then b"].failures, verdicts["a then b"].refusal))

    def test_deliveries_keep_the_order_the_record_drove(self):
        model = load_model_of(latch_model("left", None))
        self.assertEqual([[B], [A]], deliveries([B, A], model, PER_DELIVERY))
        self.assertEqual([[B, A]], deliveries([B, A], model, HOST))
        both = load_model_of(latch_model("both", "both"))
        self.assertEqual([[B, A]], deliveries([B, A], both, PER_DELIVERY))

    def test_an_output_is_not_said_to_be_delivered(self):
        model = latch_model()
        model["entries"] = [dict(e, delivered_with="out") if e["address"] == "Plant.Out.Lamp"
                            else e for e in model["entries"]]
        self.write_pack(model, CONVENTIONS, None)
        with self.assertRaises(PackError) as caught:
            self.pack()
        self.assertIn("delivered_with is said of an `output`", str(caught.exception))


def load_model_of(model):
    """The model a pack built from `model` holds, read the way `load_pack` reads it."""
    with tempfile.TemporaryDirectory() as tmp:
        path = pathlib.Path(tmp) / "interface-model.yaml"
        path.write_text(yaml.safe_dump(model), encoding="utf-8")
        return load_model([path])


if __name__ == "__main__":
    unittest.main()
