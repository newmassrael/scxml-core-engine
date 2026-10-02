"""An engine call that would never return hands control back, and says so.

`send_event`, `advance_time` and `initialize` run the main event loop until the
external queue is empty. A machine whose handler sends itself an external event
and answers it with a transition that sends it again never empties it: every
macrostep ends, so the microstep ceiling never applies, and the call did not
return. Measured 2026-10-02 through the scenario driver, whose only bound was its
processor-time limit: stopped after 25.5 s and judged `environment`, "another
machine may differ", when a design that does this does it on every host.

The engine now takes at most `max_external_events_per_call` external events in one
call, and a call it hands back with events still queued is counted
(`truncated_event_chains`, with `last_truncated_event`). It is a ceiling this engine
chooses and not a rule of W3C SCXML, as the microstep ceiling is: the specification
bounds neither. What is asserted here is the engine's own part, run in a process of
its own because a process loads ONE design:

    the default budget, the event the call was still taking, and the count
    the budget is exact: a call that takes exactly it and empties the queue counts
        zero, and one more event queued counts one
    a cut call leaves the queue as it was, so the next call gets a budget of its own
    the host can choose the budget, and a budget that takes no event is refused

The same machine through `scxml_scenarios` (a verdict about the design, at once) and
through `compare` is held in `test_a_hostile_design_ends_in_an_answer` and
`test_drafts_are_compared_at_every_level`.
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import unittest

from sce_author.verify import _default_codegen

AUTHORING = pathlib.Path(__file__).resolve().parents[1]

needs_the_generator = unittest.skipUnless(
    _default_codegen().exists(), "the design is built by the product's generator")

# The budget the engine applies, spelled here rather than read back from it. A test
# that asked the engine for its own limit would agree with any limit, including one an
# edit moved by three orders of magnitude.
DEFAULT_BUDGET = 10_000

# Counts the events it handles: each `again` that its guard lets through sends one
# `tick` to the host, so the ticks are how many external events the call took.
COUNTER = """<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="idle">
  <datamodel><data id="n" expr="0"/></datamodel>
  <state id="idle"><transition event="go" target="busy"/></state>
  <state id="busy">
    <onentry><send event="again"/></onentry>
    <transition event="again" cond="n &lt; {limit}" target="busy">
      <assign location="n" expr="n + 1"/>
      <send event="tick" type="x-sce-host"/>
    </transition>
  </state>
</scxml>
"""

CHILD = r"""
import json, pathlib, sys, tempfile

from sce_author import lowering, verify
from sce_author.lowering import SendRecorder
from sce_author.scenario_play import _resolve_event

request = json.load(sys.stdin)
with tempfile.TemporaryDirectory() as work:
    work = pathlib.Path(work)
    document = work / "design.scxml"
    document.write_text(request["design"], encoding="utf-8")
    into = work / "built"
    into.mkdir()
    build = verify.generate(document, pathlib.Path(request["codegen"]), into,
                            serves=("x-sce-host",))
    assert not build.refusal, build.refusal
    module = lowering.load(into, document)
    engine = module.create_engine()
    sink = SendRecorder()
    engine.register_event_processor("x-sce-host", sink)
    if request.get("budget") is not None:
        engine.set_max_external_events_per_call(request["budget"])
    engine.initialize()
    policy = engine.policy
    seen = []
    for name in request["send"]:
        event, _ = _resolve_event(policy, name)
        engine.send_event(event)
        last = engine.last_truncated_event()
        seen.append({
            "ticks": len(sink.take()),
            "cut": engine.truncated_event_chains(),
            "last": policy.get_event_name(last) if last is not None else None,
            "budget": engine.max_external_events_per_call(),
        })
    print(json.dumps(seen))
"""


def played(design: str, send: list, budget: int | None = None) -> list:
    """What each `send_event` of `send` came to, in a process of its own."""
    done = subprocess.run(
        [sys.executable, "-c", CHILD],
        input=json.dumps({"design": design, "send": send, "budget": budget,
                          "codegen": str(_default_codegen())}),
        capture_output=True, text=True, timeout=120,
        env={"PATH": "/usr/bin:/bin", "PYTHONPATH": str(AUTHORING),
             "SCE_CODEGEN": str(_default_codegen())})
    assert done.returncode == 0, done.stderr[-2000:]
    return json.loads(done.stdout.strip().splitlines()[-1])


@needs_the_generator
class AnEngineCallThatWouldNeverReturnHandsControlBack(unittest.TestCase):
    def test_a_machine_that_sends_itself_an_event_for_ever_is_cut_at_the_default_budget(self):
        first = played(COUNTER.format(limit=10 ** 9), ["go"])[0]
        self.assertEqual(1, first["cut"], first)
        self.assertEqual("again", first["last"], first)
        self.assertEqual(DEFAULT_BUDGET, first["budget"], first)
        # `go` and then every `again` but the one still queued: the budget's worth.
        self.assertEqual(DEFAULT_BUDGET - 1, first["ticks"], first)

    def test_the_budget_is_exact(self):
        # `go`, four `again` that tick, and a fifth that no transition takes: six
        # external events, after which the queue is empty.
        design = COUNTER.format(limit=4)
        exactly = played(design, ["go"], budget=6)[0]
        self.assertEqual((0, 4), (exactly["cut"], exactly["ticks"]), exactly)
        one_short = played(design, ["go"], budget=5)[0]
        self.assertEqual((1, 4), (one_short["cut"], one_short["ticks"]), one_short)
        self.assertEqual("again", one_short["last"], one_short)
        two_short = played(design, ["go"], budget=4)[0]
        self.assertEqual((1, 3), (two_short["cut"], two_short["ticks"]), two_short)

    def test_a_cut_call_leaves_the_queue_so_the_next_call_has_a_budget_of_its_own(self):
        design = COUNTER.format(limit=4)
        first, second = played(design, ["go", "go"], budget=4)
        self.assertEqual((1, 3), (first["cut"], first["ticks"]), first)
        # The second call takes what the first left (one more `again`, which ticks),
        # then its own `go` (nothing takes it in `busy`), and the rest of the chain.
        self.assertEqual(4 - 3, second["ticks"], second)
        self.assertEqual(1, second["cut"], second)

    def test_a_host_can_choose_the_budget_and_a_budget_that_takes_no_event_is_refused(self):
        self.assertEqual(7, played(COUNTER.format(limit=2), ["go"], budget=7)[0]["budget"])
        for refused in (0, -1, 2.5, True, "5"):
            with self.subTest(budget=refused):
                with self.assertRaises(AssertionError) as caught:
                    played(COUNTER.format(limit=2), ["go"], budget=refused)
                self.assertIn("a whole number of events", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
