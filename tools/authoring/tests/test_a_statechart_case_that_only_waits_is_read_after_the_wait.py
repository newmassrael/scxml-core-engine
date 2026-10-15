"""A case that only waits is read after the wait; a step's early deliveries are dated.

Two ways a statechart's case was withheld although the record dates it, both measured on
2026-10-15 against components whose documents the platform passes:

    a case drives an address at the value the platform already held, so under `on-change`
    nothing reaches the document; the situation it reads is one an EARLIER round started (a
    delayed act the machine is waiting on), and the record reads the component `elapsed_ms`
    later. It was refused as "no round of its to judge"           (the discriminator)
    a step the host hands over in several deliveries arms a wait in the first, and the
    second reached a machine nobody would date                    (the discriminator)

Asserted here, through `verify`:

    a restatement read after the wait an earlier round started is judged on what the wait does
    a wait that does not do what the case expects fails the case (the verdict is not vacuous)
    a restatement with nothing pending is still no round to judge
    a restatement that records no time is not dated
    a step whose first delivery arms a wait is judged once the wait is read
    a step that records no time is still refused where a wait is pending
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.verify import _default_codegen, verify
from tests.test_a_round_is_what_the_host_runs import (
    A, B, HOST, LATCH_BINDING, PER_DELIVERY, latch_model)
from tests.test_refusals_actually_fire import CONVENTIONS, MODEL, Fixture

MODE = "Plant.Input.SupplyMode"
LAMP = "Plant.Out.Lamp.Stat"

# The lamp comes on `delay` after the supply goes high, and goes off when it falls to low.
WAITER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="idle" sce:kind="statechart" name="waiter">
  <state id="idle">
    <transition event="supply.high" target="waiting"/>
  </state>
  <state id="waiting">
    <onentry>{timer}</onentry>
    <transition event="timeout" target="shown"><send event="on" type="x-sce-lamp"/></transition>
    <transition event="supply.low" target="idle"><send event="off" type="x-sce-lamp"/></transition>
  </state>
  <state id="shown">
    <transition event="supply.low" target="idle"><send event="off" type="x-sce-lamp"/></transition>
  </state>
</scxml>
"""

WAITER_BINDING = {
    "version": 1, "document": "waiter.scxml", "activation": "on-change",
    "inputs": {"high": {"address": MODE, "becomes": "HIGH", "event": "supply.high"},
               "low": {"address": MODE, "becomes": "LOW", "event": "supply.low"}},
    "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Stat",
                         "sent": {"processor": "x-sce-lamp"}, "when_nothing_sent": "off",
                         "map": {"on": "ON", "off": "OFF"}}},
}


def high(name, lamp, **extra):
    return {"name": name, "given": {MODE: "HIGH"}, "drove": [MODE], "expect": {LAMP: lamp}, **extra}


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class ACaseThatOnlyWaitsIsReadAfterTheWait(Fixture):
    def judge(self, cases, timer='<send event="timeout" delay="1000ms"/>'):
        self.write_pack(MODEL, {**CONVENTIONS, "host": HOST}, {
            "version": 1, "origin": "written for this test", "ordered": True,
            "independent_cases": True, "cases": cases})
        (self.root / "waiter.scxml").write_text(WAITER.format(timer=timer), encoding="utf-8")
        path = self.root / "waiter.binding.yaml"
        path.write_text(yaml.safe_dump(WAITER_BINDING), encoding="utf-8")
        result = verify(self.pack(), path)
        self.assertTrue(result.ran, result.refusal)
        return result

    @staticmethod
    def counts(result):
        return result.passed, result.failed, result.unjudged

    def test_a_restatement_is_read_after_the_wait_an_earlier_round_started(self):
        """⚠ The discriminator. The second case restates the supply the first drove, so nothing
        reaches the document; the lamp it reads on 1000 ms later is the first round's wait."""
        result = self.judge([high("high", "OFF", elapsed_ms=100),
                             high("high, a second later", "ON", elapsed_ms=1000)])

        self.assertEqual((2, 0, 0), self.counts(result),
                         [(r.name, r.failures, r.refusal) for r in result.results])

    def test_a_wait_that_does_not_do_what_the_case_expects_fails_it(self):
        """The reading is a verdict, not a pass: this wait ends after the window."""
        result = self.judge([high("high", "OFF", elapsed_ms=100),
                             high("high, a second later", "ON", elapsed_ms=1000)],
                            timer='<send event="timeout" delay="5000ms"/>')

        self.assertEqual((1, 1, 0), self.counts(result))
        self.assertEqual("high, a second later", [r.name for r in result.results if r.failures][0])

    def test_a_restatement_with_nothing_pending_has_still_no_round_to_judge(self):
        result = self.judge([high("high", "OFF", elapsed_ms=100),
                             high("high, a second later", "ON", elapsed_ms=1000)], timer="")

        self.assertEqual((1, 0, 1), self.counts(result))

    def test_a_restatement_that_records_no_time_is_not_dated(self):
        result = self.judge([high("high", "OFF", elapsed_ms=100),
                             high("high, a second later", "ON")])

        self.assertEqual((1, 0, 1), self.counts(result))


# `a` arms a wait; the lamp comes on when it ends. `b` reaches the machine and changes nothing.
ARMS = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="idle" sce:kind="statechart" name="arms">
  <state id="idle">
    <transition event="a.on" target="armed"/>
    <transition event="a.off b.on b.off"/>
  </state>
  <state id="armed">
    <onentry><send event="done" delay="1000ms"/></onentry>
    <transition event="done" target="shown"><send event="on" type="x-sce-lamp"/></transition>
    <transition event="b.on b.off a.off"/>
  </state>
  <state id="shown">
    <transition event="a.off" target="idle"><send event="off" type="x-sce-lamp"/></transition>
    <transition event="b.on b.off"/>
  </state>
</scxml>
"""


def arming_case(setup_time):
    """`a` and `b` go on in ONE setup step (two deliveries, `a` arms the wait); `b` then goes off
    and the lamp is read as the wait ends."""
    setup = {"given": {A: "ON", B: "ON"}, "drove": [A, B]}
    if setup_time:
        setup["elapsed_ms"] = {"min": 0, "max": 20}
    return {"name": "armed", "given": {A: "ON", B: "OFF"}, "drove": [B],
            "expect": {"Plant.Out.Lamp.Stat": "ON"},
            "elapsed_ms": {"min": 990, "max": 1020}, "before": [setup]}


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class AStepWhoseFirstDeliveryArmsAWaitIsDated(Fixture):
    def judge(self, cases):
        self.write_pack(latch_model(), {**CONVENTIONS, "host": PER_DELIVERY}, {
            "version": 1, "origin": "written for this test", "ordered": True,
            "independent_cases": False, "cases": cases})
        (self.root / "arms.scxml").write_text(ARMS, encoding="utf-8")
        path = self.root / "arms.binding.yaml"
        path.write_text(yaml.safe_dump({**LATCH_BINDING, "document": "arms.scxml"}), encoding="utf-8")
        result = verify(self.pack(), path)
        self.assertTrue(result.ran, result.refusal)
        return result

    def test_a_step_whose_first_delivery_arms_a_wait_is_judged_once_the_wait_is_read(self):
        """⚠ The discriminator. The setup step's first delivery arms a 1000 ms wait and its second
        reaches a machine that is waiting; the step records its time, so both are dated."""
        result = self.judge([arming_case(setup_time=True)])

        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         [(r.name, r.failures, r.refusal) for r in result.results])

    def test_the_steps_own_reading_time_is_not_the_time_its_early_deliveries_were_handed_over(self):
        """⚠ The discriminator for the bound. The step is read as the wait ends (1000 to 1050 ms
        after the drive); its first delivery armed that wait and was handed over at once, not
        up to 1050 ms later. Taken as the step's bound, the slack reached the deadline and the
        run was given up (fifteen cases of an emergency-call warning, 2026-10-15)."""
        both = {"name": "armed", "given": {A: "ON", B: "ON"}, "drove": [A, B],
                "expect": {"Plant.Out.Lamp.Stat": "ON"},
                "elapsed_ms": {"min": 1000, "max": 1050}}

        result = self.judge([both])

        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         [(r.name, r.failures, r.refusal) for r in result.results])

    def test_a_step_that_records_no_time_is_still_refused_where_a_wait_is_pending(self):
        result = self.judge([arming_case(setup_time=False)])

        self.assertEqual((0, 0, 1), (result.passed, result.failed, result.unjudged))


if __name__ == "__main__":
    unittest.main()
