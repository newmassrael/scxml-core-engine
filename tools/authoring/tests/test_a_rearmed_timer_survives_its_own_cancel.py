"""A timer a state arms on entry and cancels on exit fires again when the state
is re-entered by its own timeout.

Measured 2026-10-01 on a retry machine drafted from a prose specification: the
state armed `<send id="t" delay="200ms"/>` on entry, cancelled `t` on exit, and
re-entered itself on the timeout while attempts remained. The first timer fired
and the second never did, so the machine sent its request twice instead of
three times and never reported the timeout. The cause was the Python runtime's
scheduler, which remembered a cancelled id and dropped the next entry that
carried it (`backends/python/tests/scheduler/` holds the scheduler's side).

The same defect made two drafts of one door specification look like two
behaviours, because one cancelled its timer on exit and the other did not, and
every behaviour verdict `compare` and `verify` gave about a document of this
shape was a verdict about the runtime.

These cases drive a generated machine, not the scheduler, because the symptom
is what an owner's draft does.
"""

from __future__ import annotations

import pathlib
import tempfile
import unittest

from sce_author.compare import _Driven
from sce_author.verify import _default_codegen

MACHINE = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="idle">
  <datamodel><data id="n" expr="0"/></datamodel>
  <state id="idle"><transition event="go" target="waiting"/></state>
  <state id="waiting">
    <onentry>
      <assign location="n" expr="n + 1"/>
      <send id="t" event="deadline" delay="200ms"/>
    </onentry>
    {cancel}
    <transition event="deadline" cond="n &lt; 3" target="waiting"/>
    <transition event="deadline" cond="n == 3" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"""

RUN = [("event", "go"), ("time", 200), ("time", 200), ("time", 200)]


def final_after(cancel: str) -> list[bool]:
    """Whether the machine had finished after each step."""
    with tempfile.TemporaryDirectory() as directory:
        work = pathlib.Path(directory)
        document = work / "retry.scxml"
        document.write_text(MACHINE.format(cancel=cancel), encoding="utf-8")
        driven = _Driven(document, _default_codegen(), work / "built")
        return [observation[2] for observation in driven.trace(RUN)]


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class ARearmedTimerSurvivesItsOwnCancel(unittest.TestCase):
    def test_a_state_that_cancels_its_timer_on_exit_still_times_out_on_the_third_wait(self):
        # Entered at 0ms, re-entered at 200ms and 400ms, done at 600ms.
        self.assertEqual([False, False, False, False, True],
                         final_after('<onexit><cancel sendid="t"/></onexit>'))

    def test_the_same_machine_without_the_cancel_does_the_same(self):
        # The control: what the cancel must not change.
        self.assertEqual([False, False, False, False, True], final_after(""))


if __name__ == "__main__":
    unittest.main()
