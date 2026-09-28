"""An output that takes its value after a delay is not written before it.

Every write is announced, and a host writes each output whose rule answers
when nothing was sent in every round. So an output whose value the document
decides only when a delayed event arrives is announced with its value from
BEFORE the wait in the round that starts it -- and a test reading the first
announcement after the drive reads that. Measured 2026-09-28: two of three
documents written from one specification had the shape and failed the
platform's test at 2 ms; the third bound the output `hold_last` and sent it
only when decided, and passed. It was no recorded guess, so nothing said so.

Asserted here, by `check`:

    an output sent WITH a delay, written every round, is refused
                                                        (the discriminator)
    so is one whose sends ask about a state a delayed event enters -- through
    the events that round raises, to a fixpoint           (the discriminator)
    the same outputs bound `hold_last` with an unmapped `when_nothing_sent`
    are not
    a document with no delay is not
"""

from __future__ import annotations

import copy
import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import check
from sce_author.pack import load_pack
from tests.test_a_statechart_is_driven_not_called import (
    BINDING, CROSSING, DELAYED, DOCUMENT)

HELD = {"when_nothing_sent": "signal.unchanged", "hold_last": True}

# The two-step shape: a delayed internal event ends a hold, the transition
# raises the event that enters the state the signal's send asks about.
CHAINED = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="main"
       sce:kind="statechart">
  <parallel id="main">
    <state id="hold" initial="idle">
      <state id="idle">
        <transition event="train.approaching" target="waiting"/>
      </state>
      <state id="waiting">
        <onentry><send event="hold.expired" delay="500ms"/></onentry>
        <transition event="hold.expired" target="idle">
          <raise event="lamp.flash"/>
        </transition>
      </state>
    </state>
    <state id="lamp" initial="dark">
      <state id="dark"><transition event="lamp.flash" target="flashing"/></state>
      <state id="flashing"/>
    </state>
    <state id="emit">
      <transition type="internal" event="*">
        <if cond="In('flashing')"><send type="x-sce-host" event="signal.flashing"/>
        <else/><send type="x-sce-host" event="signal.dark"/></if>
      </transition>
    </state>
  </parallel>
</scxml>
"""


class AnOutputIsNotAnnouncedBeforeItsDelay(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml", "examples.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)

    def found(self, document, **rule):
        (self.tmp / "signal.scxml").write_text(document, encoding="utf-8")
        binding = copy.deepcopy(BINDING)
        binding["outputs"]["roadSignal"].update(rule)
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return [str(f) for f in check(load_pack(self.tmp), path)
                if "delayed" in str(f)]

    def test_an_output_sent_with_a_delay_and_written_every_round_is_refused(self):
        found = self.found(DELAYED)
        self.assertEqual(1, len(found), found)
        self.assertIn("output roadSignal", found[0])
        self.assertIn("hold_last: true", found[0])

    def test_an_output_whose_send_asks_about_a_state_a_delay_enters_is_refused(self):
        found = self.found(CHAINED)
        self.assertEqual(1, len(found), found)
        self.assertIn("'hold.expired'", found[0])

    def test_held_until_sent_is_not_refused(self):
        self.assertEqual([], self.found(DELAYED, **HELD))
        self.assertEqual([], self.found(CHAINED, **HELD))

    def test_a_document_with_no_delay_is_not_refused(self):
        self.assertEqual([], self.found(DOCUMENT))


if __name__ == "__main__":
    unittest.main()
