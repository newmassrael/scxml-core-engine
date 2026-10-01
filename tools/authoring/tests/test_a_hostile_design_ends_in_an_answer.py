"""A design nobody has read ends in an answer, however it is written.

`scxml_scenarios` plays an AI client's design in the server's own process. A
design is code: it can loop for ever inside the Lua machine, re-send itself
without end, grow until the host is out of memory. Measured 2026-10-01 on the
first landing, against the tool as a client calls it:

    re-sends itself at zero delay     never returned (`initialize()`)
    endless `<script>` loop           never returned; a 3 s `SIGALRM` handler
                                      did not run, because the loop is inside
                                      Lua's C code
    a cyclic `<raise>`                ends: the engine stops the macrostep
    deep recursion                    ends: a Lua error, then `error.execution`
    a timer every millisecond         ends: the driver's bound on timer instants

This file is the specification of what the tool owes whoever calls it, and it
is written BEFORE the thing that keeps the promise (a runner that plays each
example in a process of its own, with limits and a watchdog outside it; see
`claudedocs/rfc-driving-untrusted-designs.md`). The cases that cannot pass yet
are `expectedFailure`: each one shows what is missing, and each turns the lane
red the day it starts to pass, so the marker cannot be forgotten.

Every case runs the tool in a CHILD process with a wall-clock limit of its own,
so a design that never returns costs this suite its limit and not its life.
"""

from __future__ import annotations

import json
import os
import pathlib
import resource
import subprocess
import sys
import unittest

from sce_author.verify import _default_codegen

TOOLS = pathlib.Path(__file__).resolve().parents[1]

# Run in the child. Reads {"design": text, "scenarios": text, "max_time_stops": n}
# on stdin and prints the tool's reply. The bound on timer instants is lowered
# here so a case that ends by reaching it ends quickly; the case is about the
# refusal, not about the number.
CHILD = r"""
import json, sys
sys.path.insert(0, %(tools)r)
from sce_author import mcp, scenario_driver
request = json.load(sys.stdin)
if request.get("max_time_stops"):
    scenario_driver.MAX_TIME_STOPS = request["max_time_stops"]
reply = mcp.call_tool("scxml_scenarios", {
    "scenarios_text": request["scenarios"],
    "documents_text": [{"name": "hostile.scxml", "text": request["design"]}]})
print(json.dumps(reply))
"""

HEAD = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" '
        'initial="a"><datamodel><data id="n" expr="0"/></datamodel>')


def needs_the_generator(case):
    return unittest.skipUnless(_default_codegen().exists(),
                               "the product's generator is not built")(case)


def set_of(examples: list, inputs: list | None = None) -> str:
    return json.dumps({
        "record": "sce-scenario-set", "v": 1,
        "specification": {"doc_id": "hostile", "rev": "1"}, "origin": "ai-proposed",
        "interface": {"inputs": [{"name": name} for name in (inputs or [])], "outputs": [],
                      "conditions": ["a", "b"]},
        "scenarios": examples})


def settles(steps: list, ident: str = "H1") -> dict:
    return {"id": ident, "quote": "the machine stays in a", "steps": steps}


class Outcome:
    """What running the tool in a child came to."""

    def __init__(self, finished: bool, reply: dict | None, why: str) -> None:
        self.finished, self.reply, self.why = finished, reply, why

    def verdicts(self) -> dict:
        return {s["id"]: s for s in (self.reply or {}).get("scenarios", [])}


def run_tool(design: str, scenarios: str, *, limit: float, max_time_stops: int = 0) -> Outcome:
    def cap_memory():
        # A safety net for the machine this runs on, not part of what is tested:
        # the cases that would eat memory are skipped until the tool brings its
        # own limit.
        resource.setrlimit(resource.RLIMIT_AS, (4 * 2**30, 4 * 2**30))

    try:
        done = subprocess.run(
            [sys.executable, "-c", CHILD % {"tools": str(TOOLS)}],
            input=json.dumps({"design": design, "scenarios": scenarios,
                              "max_time_stops": max_time_stops}),
            capture_output=True, text=True, timeout=limit, preexec_fn=cap_memory,
            env={**os.environ})
    except subprocess.TimeoutExpired:
        return Outcome(False, None, f"the tool had not returned after {limit:.0f} s")
    if done.returncode != 0:
        tail = (done.stderr.strip().splitlines() or ["no output"])[-1]
        return Outcome(False, None, f"the process died (status {done.returncode}): {tail}")
    reply = json.loads(done.stdout)
    return Outcome(True, json.loads(reply["content"][0]["text"]), "")


@needs_the_generator
class TestWhatAlreadyEndsInAnAnswer(unittest.TestCase):
    """Closed by the first landing. Held here so they stay closed."""

    def refused(self, design: str, steps: list, **more) -> dict:
        out = run_tool(design, set_of([settles(steps)]), limit=90, **more)
        self.assertTrue(out.finished, out.why)
        verdict = out.verdicts()["H1"]
        self.assertEqual("not-judged", verdict["verdict"], out.reply)
        self.assertEqual("design", verdict.get("cause"), verdict)
        return verdict

    def test_a_cyclic_raise_is_stopped_and_not_mistaken_for_a_machine_that_waits(self):
        verdict = self.refused(
            HEAD + '<state id="a"><onentry><raise event="e"/></onentry>'
                   '<transition event="e"><raise event="e"/></transition></state></scxml>',
            [{"expect": {"condition": "a"}}])
        self.assertIn("stable configuration", verdict["reason"])

    def test_deep_recursion_in_a_script_is_an_execution_error_not_a_crash(self):
        self.refused(
            HEAD + '<state id="a"><onentry><script>function f(k) { return f(k + 1) + 1; } '
                   'f(0);</script></onentry></state></scxml>',
            [{"expect": {"condition": "a"}}])

    def test_a_timer_that_fires_every_millisecond_ends_at_the_bound_on_instants(self):
        verdict = self.refused(
            HEAD + '<state id="a"><onentry><send event="t" delay="1ms"/></onentry>'
                   '<transition event="t" target="a"/></state></scxml>',
            [{"advance_ms": 100000, "expect": {"condition": "a"}}], max_time_stops=300)
        self.assertIn("300", verdict["reason"])

    def test_the_reply_says_it_was_not_kept_apart_from_the_process_it_ran_in(self):
        """A driver that runs in the caller's process is kept apart from nothing,
        and a verdict that did not say so would read as better protected than it
        was. The runner replaces this sentence with what it measured."""
        out = run_tool(HEAD + '<state id="a"/></scxml>',
                       set_of([settles([{"expect": {"condition": "a"}}])]), limit=60)
        self.assertTrue(out.finished, out.why)
        self.assertIn("none", out.reply["isolation"])
        self.assertIn("time_stops_per_step", out.reply["limits"])


@needs_the_generator
class TestWhatTheToolOwesAndDoesNotYetPay(unittest.TestCase):
    """Each of these hangs the tool today. They pass when a design is played in
    a process of its own under limits."""

    def ends_in_a_refusal(self, design: str, steps: list) -> None:
        out = run_tool(design, set_of([settles(steps)]), limit=6)
        self.assertTrue(out.finished, out.why)
        verdict = out.verdicts()["H1"]
        self.assertEqual("not-judged", verdict["verdict"], out.reply)
        self.assertIn(verdict.get("cause"), ("design", "environment"), verdict)

    @unittest.expectedFailure
    def test_a_design_that_re_sends_itself_at_zero_delay_ends_in_a_refusal(self):
        """W3C SCXML Appendix D: one external event is one macrostep and the loop
        runs while the queue is not empty, so this is legal and never ends. The
        engine is right; the caller needs a bound."""
        self.ends_in_a_refusal(
            HEAD + '<state id="a"><onentry><send event="t" delay="0ms"/></onentry>'
                   '<transition event="t" target="a"/></state></scxml>',
            [{"expect": {"condition": "a"}}])

    @unittest.expectedFailure
    def test_an_endless_script_loop_ends_in_a_refusal(self):
        """The loop is inside Lua's C code: no timer in this process can stop
        it, so the watchdog has to be outside the process or inside the VM."""
        self.ends_in_a_refusal(
            HEAD + '<state id="a"><onentry><script>while (true) { n = n + 1; }</script>'
                   '</onentry></state></scxml>',
            [{"expect": {"condition": "a"}}])

    @unittest.expectedFailure
    def test_one_hostile_example_costs_only_its_own_verdict(self):
        """Nine examples of a design that is fine, and a tenth that sends it the
        one input that makes it loop. The nine keep their verdicts."""
        design = (HEAD + '<state id="a"><transition event="ping" target="a"/>'
                         '<transition event="bomb"><script>while (true) { n = n + 1; }</script>'
                         '</transition></state></scxml>')
        fine = [settles([{"send": "ping", "expect": {"condition": "a"}}], f"F{i}")
                for i in range(9)]
        bad = settles([{"send": "bomb", "expect": {"condition": "a"}}], "BOMB")
        out = run_tool(design, set_of([*fine, bad], inputs=["ping", "bomb"]), limit=12)
        self.assertTrue(out.finished, out.why)
        verdicts = out.verdicts()
        self.assertEqual({"pass"}, {verdicts[f"F{i}"]["verdict"] for i in range(9)}, verdicts)
        self.assertEqual("not-judged", verdicts["BOMB"]["verdict"], verdicts)
        self.assertEqual("environment", verdicts["BOMB"].get("cause"), verdicts)

    @unittest.skip("running it without a memory limit would take this machine with it; "
                   "it is written for the runner, which brings the limit")
    def test_a_script_that_doubles_a_string_for_ever_ends_in_a_refusal(self):
        self.ends_in_a_refusal(
            HEAD + '<state id="a"><onentry><script>var s = "x"; while (true) { s = s + s; }'
                   '</script></onentry></state></scxml>',
            [{"expect": {"condition": "a"}}])


if __name__ == "__main__":
    unittest.main()
