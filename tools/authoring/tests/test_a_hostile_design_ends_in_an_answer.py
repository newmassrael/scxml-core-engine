"""A design nobody has read ends in an answer, however it is written.

`scxml_scenarios` plays an AI client's design. A design is code: it can loop for
ever inside the Lua machine, re-send itself without end, grow until the host is
out of memory. Measured 2026-10-01 on the first landing, against the tool as a
client calls it, when the design was played in the server's own process:

    re-sends itself at zero delay     never returned (`initialize()`)
    endless `<script>` loop           never returned; a 3 s `SIGALRM` handler
                                      did not run, because the loop is inside
                                      Lua's C code
    a cyclic `<raise>`                ended: the engine stops the macrostep
    deep recursion                    ended: a Lua error, then `error.execution`
    a timer every millisecond         ended: the driver's bound on timer instants

This file is what the tool owes whoever calls it. It was written BEFORE the
thing that keeps the promise, with the three cases that hung marked
`expectedFailure`; the day the runner (`process.run_isolated`, one child per
example under limits the kernel enforces and a clock outside it) made them pass
the marker came off, as the lane said it would.

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

# Run in the child. Reads {"design", "scenarios", "max_time_stops", "limits"} on
# stdin and prints the tool's reply and which engine modules this process holds.
# The bound on timer instants and the child limits are lowered here so a case
# that ends by reaching one ends quickly; the case is about the refusal, not
# about the number.
CHILD = r"""
import json, sys
sys.path.insert(0, %(tools)r)
from sce_author import mcp, process, scenario_driver
request = json.load(sys.stdin)
if request.get("max_time_stops"):
    scenario_driver.MAX_TIME_STOPS = request["max_time_stops"]
if request.get("limits"):
    scenario_driver.LIMITS = process.Limits(**request["limits"])
reply = mcp.call_tool("scxml_scenarios", {
    "scenarios_text": request["scenarios"],
    "documents_text": [{"name": "hostile.scxml", "text": request["design"]}]})
held = sorted(name for name in sys.modules if name.split(".")[0] in ("sce_runtime", "lupa"))
print(json.dumps({"reply": reply, "engine_modules_held": held}))
"""

# Run in a child for `compare` and `verify`, which drive a design without going
# through the scenario tool. Reads {"kind", ...} on stdin; prints what the call
# came to and which engine modules this process holds when it is done.
CHILD_OTHERS = r"""
import json, pathlib, sys
sys.path.insert(0, %(tools)r)
from sce_author import mcp, process
request = json.load(sys.stdin)
process.SESSION_LIMITS = process.Limits(**request["limits"])
if request["kind"] == "compare":
    reply = mcp.call_tool("compare", {"documents_text": request["documents"]})
    out = json.loads(reply["content"][0]["text"])
else:
    from sce_author.pack import load_pack
    from sce_author.verify import verify
    result = verify(load_pack(pathlib.Path(request["pack"])), pathlib.Path(request["binding"]))
    out = {"ran": result.ran, "refusal": result.refusal,
           "results": [{"name": r.name, "refusal": r.refusal, "failures": len(r.failures)}
                       for r in result.results],
           "passed": result.passed, "failed": result.failed}
held = sorted(name for name in sys.modules if name.split(".")[0] in ("sce_runtime", "lupa"))
print(json.dumps({"out": out, "engine_modules_held": held}))
"""

HEAD = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" '
        'initial="a"><datamodel><data id="n" expr="0"/></datamodel>')

# A loop is ended by its three seconds of processor time, which is how long it
# takes on any host. The clock stays far away: a FINE example in the same call
# (nine of them in one case) must not be refused for the machine being busy,
# which at a load average of 52 on 32 cores it was measured to be (2026-10-02).
QUICK = {"wall_seconds": 30.0, "cpu_seconds": 3, "memory_mb": 1024}


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

    def __init__(self, finished: bool, reply: dict | None, why: str,
                 held: list | None = None) -> None:
        self.finished, self.reply, self.why, self.held = finished, reply, why, held or []

    def verdicts(self) -> dict:
        return {s["id"]: s for s in (self.reply or {}).get("scenarios", [])}


def run_tool(design: str, scenarios: str, *, limit: float, max_time_stops: int = 0,
             limits: dict | None = None) -> Outcome:
    def cap_memory():
        # A safety net for the machine this runs on, not part of what is tested:
        # the tool brings its own limits for the designs it plays.
        resource.setrlimit(resource.RLIMIT_AS, (4 * 2**30, 4 * 2**30))

    try:
        done = subprocess.run(
            [sys.executable, "-c", CHILD % {"tools": str(TOOLS)}],
            input=json.dumps({"design": design, "scenarios": scenarios,
                              "max_time_stops": max_time_stops, "limits": limits}),
            capture_output=True, text=True, timeout=limit, preexec_fn=cap_memory,
            env={**os.environ})
    except subprocess.TimeoutExpired:
        return Outcome(False, None, f"the tool had not returned after {limit:.0f} s")
    if done.returncode != 0:
        tail = (done.stderr.strip().splitlines() or ["no output"])[-1]
        return Outcome(False, None, f"the process died (status {done.returncode}): {tail}")
    printed = json.loads(done.stdout)
    return Outcome(True, json.loads(printed["reply"]["content"][0]["text"]), "",
                   printed["engine_modules_held"])


@needs_the_generator
class TestWhatEndsInAnAnswerBecauseOfTheDesign(unittest.TestCase):
    """The design's own doing, the same on every machine: `cause: design`."""

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


@needs_the_generator
class TestWhatEndsInAnAnswerBecauseOfTheMachine(unittest.TestCase):
    """Each of these used to keep the tool from ever returning. The machine
    stops them, so the refusal is about the machine, `cause: environment`: a
    machine with more clock or memory might play the example further, and the
    answer says a second run may differ."""

    def ends_in_a_refusal(self, design: str, steps: list, causes=("environment",)) -> dict:
        out = run_tool(design, set_of([settles(steps)]), limit=60, limits=QUICK)
        self.assertTrue(out.finished, out.why)
        verdict = out.verdicts()["H1"]
        self.assertEqual("not-judged", verdict["verdict"], out.reply)
        self.assertIn(verdict.get("cause"), causes, verdict)
        return verdict

    def test_a_design_that_re_sends_itself_at_zero_delay_ends_in_a_refusal(self):
        """W3C SCXML Appendix D: one external event is one macrostep and the loop
        runs while the queue is not empty, so this is legal and never ends. The
        engine is right; the caller needs a bound."""
        verdict = self.ends_in_a_refusal(
            HEAD + '<state id="a"><onentry><send event="t" delay="0ms"/></onentry>'
                   '<transition event="t" target="a"/></state></scxml>',
            [{"expect": {"condition": "a"}}])
        self.assertIn("did not finish", verdict["reason"])

    def test_an_endless_script_loop_ends_in_a_refusal(self):
        """The loop is inside Lua's C code: no timer in the process can stop it,
        so the stop comes from outside it (the processor-time limit here)."""
        self.ends_in_a_refusal(
            HEAD + '<state id="a"><onentry><script>while (true) { n = n + 1; }</script>'
                   '</onentry></state></scxml>',
            [{"expect": {"condition": "a"}}])

    def test_a_script_that_doubles_a_string_for_ever_ends_in_a_refusal(self):
        """Either the Lua machine runs out of memory and the engine reports an
        error nobody answered (the design's), or the kernel's limit ends the
        process (the machine's). Both are an answer, and neither is a pass."""
        self.ends_in_a_refusal(
            HEAD + '<state id="a"><onentry><script>var s = "x"; while (true) { s = s + s; }'
                   '</script></onentry></state></scxml>',
            [{"expect": {"condition": "a"}}], causes=("design", "environment"))

    def test_one_hostile_example_costs_only_its_own_verdict(self):
        """Nine examples of a design that is fine, and a tenth that sends it the
        one input that makes it loop. The nine keep their verdicts."""
        design = (HEAD + '<state id="a"><transition event="ping" target="a"/>'
                         '<transition event="bomb"><script>while (true) { n = n + 1; }</script>'
                         '</transition></state></scxml>')
        fine = [settles([{"send": "ping", "expect": {"condition": "a"}}], f"F{i}")
                for i in range(9)]
        bad = settles([{"send": "bomb", "expect": {"condition": "a"}}], "BOMB")
        out = run_tool(design, set_of([*fine, bad], inputs=["ping", "bomb"]), limit=90,
                       limits=QUICK)
        self.assertTrue(out.finished, out.why)
        verdicts = out.verdicts()
        self.assertEqual({"pass"}, {verdicts[f"F{i}"]["verdict"] for i in range(9)}, verdicts)
        self.assertEqual("not-judged", verdicts["BOMB"]["verdict"], verdicts)
        self.assertEqual("environment", verdicts["BOMB"].get("cause"), verdicts)


def run_others(request: dict, *, limit: float = 120) -> tuple:
    """`(finished, out, held, why)` for `compare` or `verify` run in a child."""
    def cap_memory():
        resource.setrlimit(resource.RLIMIT_AS, (4 * 2**30, 4 * 2**30))

    try:
        done = subprocess.run(
            [sys.executable, "-c", CHILD_OTHERS % {"tools": str(TOOLS)}],
            input=json.dumps(request), capture_output=True, text=True, timeout=limit,
            preexec_fn=cap_memory, env={**os.environ})
    except subprocess.TimeoutExpired:
        return False, None, [], f"the call had not returned after {limit:.0f} s"
    if done.returncode != 0:
        tail = (done.stderr.strip().splitlines() or ["no output"])[-1]
        return False, None, [], f"the process died (status {done.returncode}): {tail}"
    printed = json.loads(done.stdout)
    return True, printed["out"], printed["engine_modules_held"], ""


TOGGLE = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle">'
          '<state id="idle">{inside}<transition event="go" target="busy"/></state>'
          '<state id="busy"><transition event="stop" target="idle"/></state></scxml>')
LOOPS = ('<onentry><script>while (true) { }</script></onentry>')


@needs_the_generator
class TestACompareThatMeetsAHostileDraft(unittest.TestCase):
    """`compare` plays every draft. One that never returns used to keep the whole
    comparison, and the server answering it, from ever finishing."""

    def compare(self, drafts: list) -> tuple:
        return run_others({"kind": "compare", "documents": drafts,
                           "limits": {"wall_seconds": 6.0, "cpu_seconds": 120,
                                      "memory_mb": 1024}})

    def test_the_other_drafts_are_still_compared_and_the_hostile_one_is_named(self):
        finished, out, held, why = self.compare([
            {"name": "first.scxml", "text": TOGGLE.format(inside="")},
            {"name": "second.scxml", "text": TOGGLE.format(inside="")},
            {"name": "hostile.scxml", "text": TOGGLE.format(inside=LOOPS)},
        ])
        self.assertTrue(finished, why)
        behaviour = out["behaviour"]
        self.assertEqual("judged", behaviour["verdict"], behaviour)
        self.assertEqual([["first.scxml", "second.scxml"]], behaviour["classes"], behaviour)
        self.assertIn("hostile.scxml", behaviour["undriven"], behaviour)
        self.assertIn("stopped", behaviour["undriven"]["hostile.scxml"])
        self.assertIn("may play it further", behaviour["undriven"]["hostile.scxml"])

    def test_the_server_that_compared_never_held_the_engine(self):
        finished, out, held, why = self.compare([
            {"name": "first.scxml", "text": TOGGLE.format(inside="")},
            {"name": "second.scxml", "text": TOGGLE.format(inside="")},
        ])
        self.assertTrue(finished, why)
        self.assertEqual("judged", out["behaviour"]["verdict"], out["behaviour"])
        self.assertEqual([], held)


# A statechart the verifier's own tests drive, with its flashing state made to
# loop. The cases and the binding are theirs.
def hostile_pack(tmp: pathlib.Path, document: str) -> pathlib.Path:
    import shutil

    import yaml

    from tests.test_a_statechart_is_driven_not_called import BINDING, CROSSING, EXAMPLES

    for name in ("interface-model.yaml", "conventions.yaml"):
        shutil.copy(CROSSING / name, tmp / name)
    (tmp / "signal.scxml").write_text(document, encoding="utf-8")
    (tmp / "examples.yaml").write_text(yaml.safe_dump(EXAMPLES), encoding="utf-8")
    (tmp / "b.yaml").write_text(yaml.safe_dump(BINDING), encoding="utf-8")
    return tmp / "b.yaml"


@needs_the_generator
class TestAVerifyThatMeetsAHostileDocument(unittest.TestCase):
    """`verify` replays the cases through one machine. A document that never
    returns used to keep the verifier, and whatever called it, from ever
    finishing."""

    def verify(self, document: str) -> tuple:
        import tempfile

        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            binding = hostile_pack(root, document)
            return run_others({"kind": "verify", "pack": str(root), "binding": str(binding),
                               "limits": {"wall_seconds": 6.0, "cpu_seconds": 120,
                                          "memory_mb": 1024}})

    def test_a_document_that_loops_when_driven_is_refused_case_by_case(self):
        """The first case drives the machine into the state that loops. The run
        is refused with the machine's words; nothing passes, nothing fails, and
        the call returns."""
        from tests.test_a_statechart_is_driven_not_called import DOCUMENT

        looping = DOCUMENT.replace(
            '<onentry>\n      <send event="signal.flashing" type="x-sce-host"/>',
            '<onentry>\n      <script>while (true) { }</script>\n'
            '      <send event="signal.flashing" type="x-sce-host"/>')
        self.assertNotEqual(DOCUMENT, looping)
        finished, out, held, why = self.verify(looping)
        self.assertTrue(finished, why)
        self.assertEqual((0, 0), (out["passed"], out["failed"]), out)
        refusals = [r["refusal"] for r in out["results"]] + [out["refusal"]]
        self.assertTrue(any("the process that plays the design stopped" in text
                            for text in refusals), out)
        self.assertEqual([], held)

    def test_a_document_that_loops_on_start_is_refused_whole(self):
        from tests.test_a_statechart_is_driven_not_called import DOCUMENT

        looping = DOCUMENT.replace(
            '<state id="dark">',
            '<state id="dark">\n    <onentry><script>while (true) { }</script></onentry>', 1)
        self.assertNotEqual(DOCUMENT, looping)
        finished, out, held, why = self.verify(looping)
        self.assertTrue(finished, why)
        self.assertFalse(out["ran"], out)
        self.assertIn("stopped", out["refusal"])
        self.assertEqual([], held)


@needs_the_generator
class TestWhatTheReplyTellsTheOwner(unittest.TestCase):
    def test_the_reply_says_how_the_run_was_kept_apart_and_bounded(self):
        """A verdict states what it was made under. A reader of `pass` should know
        whether the design had been kept apart from the server that played it."""
        out = run_tool(HEAD + '<state id="a"/></scxml>',
                       set_of([settles([{"expect": {"condition": "a"}}])]), limit=60)
        self.assertTrue(out.finished, out.why)
        self.assertEqual(sys.platform.startswith("linux"),
                         out.reply["isolation"] == "process+rlimit", out.reply["isolation"])
        for bound in ("wall_seconds", "cpu_seconds", "memory_mb", "output_mb",
                      "time_stops_per_step"):
            self.assertIn(bound, out.reply["limits"])

    def test_the_process_that_serves_the_client_never_holds_the_engine(self):
        """The whole point of the child: a design's machine is not in the server.
        Neither the runtime nor the Lua binding is imported by the process that
        answered, only by the children that played."""
        out = run_tool(HEAD + '<state id="a"/></scxml>',
                       set_of([settles([{"expect": {"condition": "a"}}])]), limit=60)
        self.assertTrue(out.finished, out.why)
        self.assertEqual([], out.held)
        self.assertEqual("pass", out.verdicts()["H1"]["verdict"], out.reply)


if __name__ == "__main__":
    unittest.main()
