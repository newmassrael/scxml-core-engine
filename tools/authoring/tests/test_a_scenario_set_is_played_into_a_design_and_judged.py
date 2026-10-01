"""A scenario set is played into a design on the Python lowering, and the
product judges what was seen.

The driver (`sce_author/scenario_driver.py`) writes down what the machine did;
the verdicts are `sce-codegen judge-scenarios`', passed through. These cases
hold the line between the two: a driver that fills a hole it was not given
would make a design look right, or wrong, on words no run contained.

The set is the retry client of `sce-build/tests/fixtures/scenario_sets/`, the
same examples the product's judge is tested on with a hand-written trace, so
the question here is only whether a real run produces that trace.
"""

from __future__ import annotations

import copy
import io
import json
import pathlib
import tempfile
import unittest

from sce_author import mcp
from sce_author.scenario_driver import ENGINE_NAME, drive, judge, run
from sce_author.verify import _default_codegen

ROOT = pathlib.Path(__file__).resolve().parents[3]
RETRY_SET = ROOT / "sce-build" / "tests" / "fixtures" / "scenario_sets" / "retry-client.scenarios.json"

MACHINE = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       datamodel="ecmascript" initial="idle">
  <datamodel>
    <data id="n" expr="0"/>
    {open}
  </datamodel>
  <state id="idle"><transition event="RequestNeeded" target="waiting"/></state>
  <state id="waiting">
    <onentry>
      <assign location="n" expr="n + 1"/>
      {request}
      <send id="t" event="deadline" delay="200ms"/>
    </onentry>
    <onexit><cancel sendid="t"/></onexit>
    <transition event="ResponseReceived" target="completed"/>
    <transition event="deadline" cond="n &lt; {attempts}" target="waiting"/>
    <transition event="deadline" cond="n &gt;= {attempts}" target="timedOut">
      {timeout}
    </transition>
  </state>
  <final id="completed"/>
  <final id="timedOut"/>
</scxml>
"""

REQUEST = '<send event="SendRequest" type="x-sce-host"{target}/>'
TIMEOUT = '<send event="TimeoutError" type="x-sce-host"{target}/>'


def machine(attempts: int = 3, target: str = "", opened: bool = False) -> str:
    """The retry client: three requests then a timeout, or `attempts` of them.
    `opened` leaves the route of both outputs as an open decision, which is
    what a draft does when the specification never says who the caller is."""
    if opened:
        request = ('<send event="SendRequest" typeexpr="callerProcessor" '
                   'targetexpr="callerTarget"/>')
        timeout = ('<send event="TimeoutError" typeexpr="callerProcessor" '
                   'targetexpr="callerTarget"/>')
        # Two of the three say the same words, as a decision that blocks
        # several sends does when each of them is marked with it.
        decisions = ('<data id="callerProcessor" sce:unresolved="caller-routing" '
                     'sce:unresolved-reason="who the caller is was not said."/>'
                     '<data id="callerKind" sce:unresolved="caller-routing" '
                     'sce:unresolved-reason="who the caller is was not said."/>'
                     '<data id="callerTarget" sce:unresolved="caller-routing" '
                     'sce:unresolved-reason="where the caller is was not said"/>')
    else:
        spelled = f' target="{target}"' if target else ""
        request = REQUEST.format(target=spelled)
        timeout = TIMEOUT.format(target=spelled)
        decisions = ""
    return MACHINE.format(attempts=attempts, request=request, timeout=timeout, open=decisions)


def codegen_is_built() -> bool:
    return _default_codegen().exists()


# ⚠ The domain-free job of the authoring-core lane builds nothing on purpose
# (`.github/workflows/authoring-core.yml`), so a case that spawns the product's
# generator is skipped there and judged in `verify-with-codegen`. Landed without
# this on 2026-10-01 and the lane was red for two pushes before anyone read it.
needs_the_generator = unittest.skipUnless(codegen_is_built(),
                                          "the product's generator is not built")


@needs_the_generator
class Played(unittest.TestCase):
    """Shared plumbing: a work directory, a design, a set. Every case built on
    it generates a design with the product, so the skip is inherited."""

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.work = pathlib.Path(self.directory.name)
        self.codegen = _default_codegen()

    def design(self, text: str) -> pathlib.Path:
        path = self.work / "retry.scxml"
        path.write_text(text, encoding="utf-8")
        return path

    def set_with(self, change=None) -> pathlib.Path:
        """The retry set, optionally changed by `change(dict)`, written beside the design."""
        spec = json.loads(RETRY_SET.read_text(encoding="utf-8"))
        if change:
            change(spec)
        path = self.work / "retry.scenarios.json"
        path.write_text(json.dumps(spec), encoding="utf-8")
        return path

    @staticmethod
    def verdicts(answer: dict) -> dict:
        return {r["id"]: r["verdict"] for r in answer["judgement"] if r["kind"] == "verdict"}

    @staticmethod
    def summary(answer: dict) -> dict:
        return answer["judgement"][0]


class TestTheMachineThatKeepsItsPromises(Played):
    def test_the_correct_machine_passes_every_example(self):
        """W3C SCXML 6.2: a delayed send fires at its due time and a cancel
        removes it. The examples are about exactly that, and a run of the
        lowering has to produce the observations the product's judge was
        tested on."""
        answer = run(self.set_with(), self.design(machine()), self.codegen)
        self.assertEqual(
            {"T1": "pass", "T2-boundary": "pass", "T3-at-most-three": "pass",
             "T4-response-completes": "pass", "T5-response-stops-the-timer": "pass",
             "T6-response-after-a-retry": "pass"},
            self.verdicts(answer), self.summary(answer))
        self.assertEqual(ENGINE_NAME, self.summary(answer)["engine"]["name"])

    def test_a_machine_that_retries_once_too_often_fails_the_one_example_about_it(self):
        """Four requests where the specification says at most three. Only the
        example about the count fails, and it fails on a check that was
        observed, not on one the driver could not see."""
        answer = run(self.set_with(), self.design(machine(attempts=4)), self.codegen)
        verdicts = self.verdicts(answer)
        self.assertEqual("fail", verdicts["T3-at-most-three"], verdicts)
        self.assertEqual("pass", verdicts["T1"], verdicts)
        self.assertEqual("pass", verdicts["T4-response-completes"], verdicts)
        self.assertEqual(1, self.summary(answer)["fail"], self.summary(answer))

    def test_the_trace_names_the_set_it_was_taken_against(self):
        """The judge refuses a trace taken against another set, so the digest
        is how a driver says which one it ran; a wrong digest judges nothing."""
        path = self.set_with()
        trace = drive(path, self.design(machine()), self.codegen)
        self.assertTrue(trace["scenario_set"]["sha256"])
        trace["scenario_set"]["sha256"] = "0" * 64
        records = judge(path, trace, self.codegen)
        self.assertFalse(records[0]["judged"], records[0])
        self.assertEqual(0, records[0]["pass"], records[0])


class TestWhatTheDriverRefusesToInvent(Played):
    def test_an_open_route_refuses_the_run_and_says_why_not_fail(self):
        """W3C SCXML 4.9: a send whose route is open raises
        `error.communication` and ends the entry block, so the timer after it
        is never armed. Judging that run would fail the machine on timing it
        was never allowed to keep. The driver refuses, naming the open
        decision, and the product reports `not-judged`."""
        answer = run(self.set_with(), self.design(machine(opened=True)), self.codegen)
        verdicts = self.verdicts(answer)
        self.assertEqual({"not-judged"}, set(verdicts.values()), verdicts)
        self.assertEqual(0, self.summary(answer)["fail"], self.summary(answer))
        reasons = [r["reason"] for r in answer["judgement"]
                   if r["kind"] == "verdict" and r["reason"]]
        self.assertTrue(all("error.communication" in reason for reason in reasons), reasons)
        self.assertTrue(all("caller-routing" in reason for reason in reasons), reasons)
        # Said once, and without the full stop the sentence supplies itself.
        for reason in reasons:
            self.assertEqual(1, reason.count("who the caller is was not said"), reason)
            self.assertNotIn("..", reason)
            self.assertIn("2 open decision(s)", reason)
        # An open route is the design's, whichever machine plays it.
        causes = {r.get("cause") for r in answer["judgement"] if r["kind"] in ("verdict", "gap")}
        self.assertEqual({"design"}, causes, answer["judgement"])

    def test_an_output_sent_through_another_route_is_refused_with_both_named(self):
        """The interface is what the owner accepted. A machine that leaves by
        a different door is not what the examples describe, and counting its
        event anyway would pass it."""
        def declare(spec):
            for output in spec["interface"]["outputs"]:
                output["via"] = {"type": "x-sce-host", "target": "billing"}

        answer = run(self.set_with(declare), self.design(machine(target="elsewhere")),
                     self.codegen)
        self.assertNotIn("pass", self.verdicts(answer).values())
        reasons = " ".join(r["reason"] or "" for r in answer["judgement"] if r["kind"] == "verdict")
        self.assertIn("elsewhere", reasons)
        self.assertIn("billing", reasons)

    def test_a_declared_route_is_served_so_a_send_with_a_target_is_built(self):
        """The Python lowering refuses a send that names a target as well as a
        type unless the type is declared to the build, and its first pass
        ends in that refusal before a manifest could name the type. The
        interface's `via` is how the driver knows the type before it builds."""
        def declare(spec):
            for output in spec["interface"]["outputs"]:
                output["via"] = {"type": "x-sce-host", "target": "billing"}

        answer = run(self.set_with(declare), self.design(machine(target="billing")), self.codegen)
        self.assertEqual({"pass"}, set(self.verdicts(answer).values()), self.summary(answer))

    def test_an_input_the_design_never_names_refuses_that_run_only(self):
        """An example whose input reaches nothing in the design is not a
        machine that ignored it: it cannot be played. The other examples are
        still judged."""
        def rename(spec):
            # Declared in the interface, so the set is usable; the design
            # has no transition on it.
            spec["interface"]["inputs"].append({"name": "ResponseArrived"})
            spec["scenarios"][3]["steps"][2]["send"] = "ResponseArrived"

        answer = run(self.set_with(rename), self.design(machine()), self.codegen)
        verdicts = self.verdicts(answer)
        self.assertEqual("not-judged", verdicts["T4-response-completes"], verdicts)
        self.assertEqual("pass", verdicts["T1"], verdicts)
        reason = next(r["reason"] for r in answer["judgement"]
                      if r["kind"] == "verdict" and r["id"] == "T4-response-completes")
        self.assertIn("ResponseArrived", reason)

    def test_a_scenario_that_cannot_run_is_not_played(self):
        """`blocked` and `awaiting-decision` are the set's own words about an
        example nobody can run yet; the driver leaves them to the judge."""
        def block(spec):
            scenario = copy.deepcopy(spec["scenarios"][0])
            scenario["id"] = "T7-waits"
            scenario["status"] = "blocked"
            scenario["blocked_by"] = "who the caller is"
            spec["scenarios"].append(scenario)

        path = self.set_with(block)
        trace = drive(path, self.design(machine()), self.codegen)
        self.assertNotIn("T7-waits", [r["scenario"] for r in trace["runs"]])
        verdicts = self.verdicts({"judgement": judge(path, trace, self.codegen)})
        self.assertEqual("blocked", verdicts["T7-waits"], verdicts)

    def test_a_design_that_is_not_a_statechart_is_refused_whole(self):
        document = self.design("""<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="transform" initial="a"><state id="a"/></scxml>
""")
        answer = run(self.set_with(), document, self.codegen)
        self.assertEqual({"not-judged"}, set(self.verdicts(answer).values()))
        self.assertEqual(0, self.summary(answer)["pass"], self.summary(answer))


COUNTER = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       datamodel="ecmascript" initial="idle">
  <datamodel>{declaration}</datamodel>
  <state id="idle">
    <transition event="coin" target="counting">
      <assign location="{item}" expr="{item} + _event.data.value"/>
    </transition>
  </state>
  <state id="counting">
    <transition event="coin">
      <assign location="{item}" expr="{item} + _event.data.value"/>
    </transition>
  </state>
</scxml>
"""


def counter(item: str, declaration: str | None = None) -> str:
    """A machine that adds each coin's value to `item`, declared as a typed
    output unless `declaration` says otherwise."""
    if declaration is None:
        declaration = f'<data id="{item}" sce:type="int32" sce:direction="out" expr="0"/>'
    return COUNTER.format(item=item, declaration=declaration)


def counting_set(item: str) -> dict:
    """Examples about the machine's states and the data item it keeps."""
    return {
        "record": "sce-scenario-set", "v": 1,
        "specification": {"doc_id": "counter", "rev": "1"},
        "origin": "ai-proposed",
        "interface": {"inputs": [{"name": "coin", "payload": {"value": "integer"}}],
                      "outputs": [], "conditions": ["idle", "counting"], "data": [item]},
        "scenarios": [{"id": "C1", "quote": "coins add up", "steps": [
            {"expect": {"condition": "idle", "data": {item: 0}}},
            {"send": "coin", "payload": {"value": 100},
             "expect": {"condition": "counting", "data": {item: 100}}},
            {"send": "coin", "payload": {"value": 500}, "expect": {"data": {item: 600}}},
        ]}],
    }


class TestWhatTheEngineDoesNotShow(Played):
    """Reported by a reviewer on 2026-10-01 against the first landing, three
    ways a run said `pass` or `fail` about something the engine had not shown
    the design doing. Each is held here as the reviewer reproduced it."""

    def play_set(self, spec: dict, text: str, name: str = "design.scxml") -> dict:
        path = self.work / "design.scenarios.json"
        path.write_text(json.dumps(spec), encoding="utf-8")
        document = self.work / name
        document.write_text(text, encoding="utf-8")
        return run(path, document, self.codegen)

    @staticmethod
    def one_example(inputs, steps, conditions=None) -> dict:
        interface = {"inputs": [{"name": n} for n in inputs], "outputs": []}
        if conditions:
            interface["conditions"] = conditions
        return {"record": "sce-scenario-set", "v": 1,
                "specification": {"doc_id": "design", "rev": "1"}, "origin": "ai-proposed",
                "interface": interface,
                "scenarios": [{"id": "E1", "quote": "an example", "steps": steps}]}

    @staticmethod
    def reason_of(answer: dict) -> str:
        return next(r["reason"] or "" for r in answer["judgement"] if r["kind"] == "verdict")

    def test_a_macrostep_the_engine_cut_short_is_not_a_machine_that_waits(self):
        """W3C SCXML 3.13 lets a macrostep fail to terminate, and the engine
        stops one after a ceiling and counts it. Every other reading of the
        machine says it is fine: it is running, it names a state, the call
        returned. A cyclic eventless transition used to pass an example that
        says 'it waits in `ready`'."""
        loop = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="ready">'
                '<state id="ready"><transition target="ready"/></state></scxml>')
        spec = self.one_example([], [{"expect": {"condition": "ready", "outbound": [],
                                                 "finished": False}}], ["ready"])
        answer = self.play_set(spec, loop)
        self.assertEqual({"E1": "not-judged"}, self.verdicts(answer), answer["judgement"])
        self.assertIn("stable", self.reason_of(answer))
        self.assertIn("ready", self.reason_of(answer))

    def test_a_chain_that_settles_is_not_mistaken_for_one_that_does_not(self):
        """A hundred microsteps and then rest is ordinary, and counts zero."""
        chain = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" '
                 'datamodel="ecmascript" initial="a"><datamodel><data id="n" expr="0"/></datamodel>'
                 '<state id="a"><transition cond="n &lt; 100" target="a">'
                 '<assign location="n" expr="n + 1"/></transition>'
                 '<transition cond="n &gt;= 100" target="b"/></state><state id="b"/></scxml>')
        spec = self.one_example([], [{"expect": {"condition": "b"}}], ["a", "b"])
        answer = self.play_set(spec, chain)
        self.assertEqual({"E1": "pass"}, self.verdicts(answer), answer["judgement"])

    def test_the_same_time_passes_the_same_whether_given_in_one_step_or_three(self):
        """W3C SCXML 6.2: a delay is measured from when the send executes.
        The engine's clock moves where the host puts it, and a host that moves
        it past three deadlines at once gets a timer armed at the END of the
        move, not at the deadline that armed it. Measured: the retry machine
        passed three steps of 200 ms and failed one step of 600 ms."""
        request = {"send": "RequestNeeded", "expect": {"outbound": [{"event": "SendRequest"}]}}
        resend = [{"event": "SendRequest"}, {"event": "SendRequest"}, {"event": "TimeoutError"}]

        def example(steps):
            spec = json.loads(RETRY_SET.read_text(encoding="utf-8"))
            spec["scenarios"] = [{"id": "E1", "quote": spec["scenarios"][0]["quote"],
                                  "steps": steps}]
            return spec

        three = [request,
                 {"advance_ms": 200, "expect": {"outbound": [{"event": "SendRequest"}]}},
                 {"advance_ms": 200, "expect": {"outbound": [{"event": "SendRequest"}]}},
                 {"advance_ms": 200, "expect": {"outbound": [{"event": "TimeoutError"}],
                                                "finished": True}}]
        at_once = [request, {"advance_ms": 600, "expect": {"outbound": resend,
                                                            "finished": True}}]
        for steps in (three, at_once):
            answer = self.play_set(example(steps), machine())
            self.assertEqual({"E1": "pass"}, self.verdicts(answer), answer["judgement"])

    def test_a_design_the_engine_cannot_start_is_an_answer_for_each_example(self):
        """Found while checking whether a design with a child session could be
        played at all: the generated parent imports the child's module by a
        bare name the loader does not put on the path, and `initialize()` died
        with ModuleNotFoundError, which took the whole call down with it. Each
        example is refused with what the engine said instead. A design that
        starts a child is the trigger today; any failure to start is the case."""
        parent = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">'
                  '<state id="a"><invoke type="scxml"><content>'
                  '<scxml version="1.0" initial="c"><state id="c"/></scxml></content></invoke>'
                  '<transition event="go" target="b"/></state><state id="b"/></scxml>')
        spec = self.one_example(["go"], [{"send": "go", "expect": {"condition": "b"}}],
                                ["a", "b"])
        spec["scenarios"].append({"id": "E2", "quote": "an example", "steps": [
            {"send": "go"}, {"advance_ms": 100, "expect": {"condition": "b"}}]})
        answer = self.play_set(spec, parent)
        self.assertEqual({"E1": "not-judged", "E2": "not-judged"}, self.verdicts(answer),
                         answer["judgement"])
        for record in answer["judgement"]:
            if record["kind"] == "verdict":
                self.assertIn("could not start the design", record["reason"])
        self.assertEqual(0, self.summary(answer)["fail"], self.summary(answer))

    @staticmethod
    def named(event: str, guard: str, script: str = "") -> str:
        """A machine that finishes when its guard holds, on a transition for
        `event`. `script` is the design's own helper code."""
        return ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" '
                f'datamodel="ecmascript" initial="ready">{script}<state id="ready">'
                f'<transition event="{event}" cond="{guard}" target="done"/></state>'
                '<final id="done"/></scxml>')

    # W3C SCXML 5.10 + 3.12.1: `request.new` matches a transition on `request`
    # and `_event.name` is `request.new`. Every spelling of reading it.
    READS_THE_NAME = {
        "a guard": ("_event.name == 'request.new'", ""),
        "a bracket": ("_event['name'] == 'request.new'", ""),
        "a computed key": ("_event['na' + 'me'] == 'request.new'", ""),
        "a helper that aliases the event": (
            "isNew()", "<script>function isNew() { var e = _event; "
                       "return e.name == 'request.new'; }</script>"),
        "a helper handed the event": (
            "isNew(_event)", "<script>function isNew(e) { return e.name == "
                             "'request.new'; }</script>"),
    }

    def test_a_design_sees_the_name_an_input_arrived_under_however_it_reads_it(self):
        """The engine carries an event as the descriptor the design declares,
        so a design reading the name was told `request` for `request.new`: a
        correct design failed its example, and a wrong one passed. A first
        repair scanned the design's text for `_event.name` and refused; an
        outside review showed a helper function and a computed key walk past a
        scan, so the name itself now travels with the event."""
        spec = self.one_example(["request.new"],
                                [{"send": "request.new", "expect": {"finished": True}}])
        for how, (guard, script) in self.READS_THE_NAME.items():
            with self.subTest(reads=how):
                answer = self.play_set(spec, self.named("request", guard, script))
                self.assertEqual({"E1": "pass"}, self.verdicts(answer), answer["judgement"])

    def test_a_design_that_would_finish_on_the_longer_name_is_failed_not_passed(self):
        """The other direction: an example that says the machine stays in
        `ready` when `request.new` arrives, against a design that finishes on
        exactly that name. With the shorter name reported this design stayed
        put and the example PASSED."""
        spec = self.one_example(["request.new"],
                                [{"send": "request.new",
                                  "expect": {"condition": "ready", "finished": False}}],
                                ["ready"])
        for how, (guard, script) in self.READS_THE_NAME.items():
            with self.subTest(reads=how):
                answer = self.play_set(spec, self.named("request", guard, script))
                self.assertEqual({"E1": "fail"}, self.verdicts(answer), answer["judgement"])

    def test_the_shorter_name_is_still_the_name_when_that_is_what_arrives(self):
        spec = self.one_example(["request"],
                                [{"send": "request", "expect": {"finished": True}}])
        answer = self.play_set(spec, self.named("request", "_event.name == 'request'"))
        self.assertEqual({"E1": "pass"}, self.verdicts(answer), answer["judgement"])
        longer = self.play_set(
            self.one_example(["request.new"],
                             [{"send": "request.new", "expect": {"finished": False}}]),
            self.named("request", "_event.name == 'request'"))
        self.assertEqual({"E1": "pass"}, self.verdicts(longer), longer["judgement"])

    def test_a_name_matched_by_prefix_that_the_design_never_reads_is_delivered(self):
        quiet = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" '
                 'initial="ready"><state id="ready"><transition event="request" target="done"/>'
                 '</state><final id="done"/></scxml>')
        spec = self.one_example(["request.new"],
                                [{"send": "request.new", "expect": {"finished": True}}])
        answer = self.play_set(spec, quiet)
        self.assertEqual({"E1": "pass"}, self.verdicts(answer), answer["judgement"])

    def test_a_name_the_design_declares_exactly_keeps_its_name(self):
        exact = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" '
                 'datamodel="ecmascript" initial="ready"><state id="ready">'
                 '<transition event="request.new" cond="_event.name == \'request.new\'" '
                 'target="done"/></state><final id="done"/></scxml>')
        spec = self.one_example(["request.new"],
                                [{"send": "request.new", "expect": {"finished": True}}])
        answer = self.play_set(spec, exact)
        self.assertEqual({"E1": "pass"}, self.verdicts(answer), answer["judgement"])


class TestTheDesignIsTheModuleLoaded(unittest.TestCase):
    """`load` picked the generated module to import by comparing the document's
    stem with each file's stem, and the comparison could never be true: the
    generator names a file `<stem>_sm.py`. It fell to the first file the
    directory listing returned, which is right while there is one file and a
    coin toss once a design starts a child session and the generator writes the
    child's module beside the parent's. Found 2026-10-01 when a design named
    `design.scxml` was played and the machine that answered was its child."""

    def pick(self, document: str, files: list) -> str:
        from sce_author.verify import generated_module_of

        paths = [pathlib.Path(name) for name in files]
        return generated_module_of(paths, pathlib.Path(document)).name

    def test_the_parent_is_chosen_in_either_listing_order(self):
        parent, child = "design_sm.py", "design__sce_synth_invoke__invoke_0_sm.py"
        self.assertEqual(parent, self.pick("design.scxml", [parent, child]))
        self.assertEqual(parent, self.pick("design.scxml", [child, parent]))

    def test_a_separator_in_the_document_name_is_not_a_different_name(self):
        self.assertEqual("door_with_auto_close_sm.py",
                         self.pick("door-with-auto-close.scxml",
                                   ["x__sce_synth_invoke__invoke_0_sm.py",
                                    "door_with_auto_close_sm.py"]))

    def test_a_document_that_is_not_a_statechart_is_named_without_the_suffix(self):
        """A forge document's module is `<stem>.py`, and what it imports is
        written beside it. Found by the whole authoring suite on the first
        version of the repair, which looked only for `_sm`."""
        for listing in (["importing.py", "supply_level.py"],
                        ["supply_level.py", "importing.py"]):
            self.assertEqual("importing.py", self.pick("importing.scxml", listing))

    def test_a_lone_file_is_the_document_whatever_it_is_called(self):
        self.assertEqual("odd_name_sm.py", self.pick("whatever.scxml", ["odd_name_sm.py"]))

    def test_several_files_and_none_that_is_the_documents_is_refused_not_guessed(self):
        from sce_author.verify import VerifyError

        with self.assertRaises(VerifyError) as caught:
            self.pick("design.scxml", ["a_sm.py", "b_sm.py"])
        self.assertIn("a_sm.py", str(caught.exception))


class TestStatesAndData(Played):
    """The channels the retry examples never ask about."""

    def play(self, item: str, text: str, name: str = "counter.scxml") -> dict:
        path = self.work / "counter.scenarios.json"
        path.write_text(json.dumps(counting_set(item)), encoding="utf-8")
        document = self.work / name
        document.write_text(text, encoding="utf-8")
        return run(path, document, self.codegen)

    def gap_reasons(self, answer: dict) -> list:
        return [r["why"] for r in answer["judgement"] if r["kind"] == "gap"]

    def test_the_states_and_a_typed_item_are_read_and_compared(self):
        answer = self.play("balance", counter("balance"))
        self.assertEqual({"C1": "pass"}, self.verdicts(answer), answer["judgement"])

    def test_a_wrong_value_in_a_readable_item_is_a_failure_not_a_gap(self):
        wrong = counter("balance").replace("balance + _event.data.value", "balance + 1")
        answer = self.play("balance", wrong)
        self.assertEqual({"C1": "fail"}, self.verdicts(answer), answer["judgement"])
        self.assertEqual([], self.gap_reasons(answer))

    def test_an_item_named_like_the_document_has_no_reader_and_the_gap_says_why(self):
        """The C++ lowering names its class after the document, so a variable
        of the same name collides with it, and a reader exists in every
        backend or in none. Measured 2026-10-01: `credit` read nothing in a
        document called `credit.scxml` and read fine in `counter.scxml`, which
        is why the cause is the pair and not the name. The owner is told the
        generator's reason, not only that the driver could not read it."""
        answer = self.play("credit", counter("credit"), "credit.scxml")
        self.assertEqual({"C1": "not-judged"}, self.verdicts(answer), answer["judgement"])
        reasons = self.gap_reasons(answer)
        self.assertTrue(reasons and all("member-collision" in why for why in reasons), reasons)

        elsewhere = self.play("credit", counter("credit"), "counter.scxml")
        self.assertEqual({"C1": "pass"}, self.verdicts(elsewhere), elsewhere["judgement"])

    def test_an_item_named_like_a_cpp_keyword_has_no_reader_and_the_gap_says_why(self):
        answer = self.play("auto", counter("auto"))
        self.assertEqual({"C1": "not-judged"}, self.verdicts(answer), answer["judgement"])
        reasons = self.gap_reasons(answer)
        self.assertTrue(reasons and all("keyword" in why for why in reasons), reasons)

    def test_an_item_declared_with_no_initial_value_has_no_reader_and_the_gap_says_so(self):
        """The generator gives it neither a reader nor a reason; the gap says
        exactly that, and does not guess a cause."""
        both = ('<data id="balance" sce:type="int32" sce:direction="out" expr="0"/>'
                '<data id="empty"/>')
        answer = self.play("empty", counter("balance", both))
        self.assertEqual({"C1": "not-judged"}, self.verdicts(answer), answer["judgement"])
        reasons = self.gap_reasons(answer)
        self.assertTrue(reasons and all("no reader" in why and "no reason" in why
                                        for why in reasons), reasons)

    def test_an_item_the_design_never_declares_says_so(self):
        answer = self.play("balance", counter("total"))
        self.assertEqual({"C1": "not-judged"}, self.verdicts(answer), answer["judgement"])
        self.assertTrue(all("declares no data item" in why for why in self.gap_reasons(answer)),
                        self.gap_reasons(answer))


SPECIFICATION = (ROOT / "sce-build" / "tests" / "fixtures" / "scenario_sets"
                 / "retry-client.spec.txt").read_text(encoding="utf-8")


def call(**arguments) -> dict:
    """`scxml_scenarios` as a client calls it, over the transport."""
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": "scxml_scenarios", "arguments": arguments}}
    output = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
    return json.loads(output.getvalue())["result"]


def said(result: dict) -> dict:
    return json.loads(result["content"][0]["text"])


@needs_the_generator
class TestTheToolAnOwnersClientCalls(unittest.TestCase):
    def sets(self, change=None) -> str:
        spec = json.loads(RETRY_SET.read_text(encoding="utf-8"))
        if change:
            change(spec)
        return json.dumps(spec)

    def ask(self, scenarios: str, design: str = None, **more) -> dict:
        design = machine() if design is None else design
        return call(scenarios_text=scenarios,
                    documents_text=[{"name": "retry.scxml", "text": design}], **more)

    def test_the_owner_is_told_the_engine_and_what_a_pass_means(self):
        """A verdict is about an engine, and a pass is not a claim that the
        design is right. Both ride on the answer a client reads, so a client
        that summarises it has them to summarise."""
        result = self.ask(self.sets())
        self.assertFalse(result.get("isError"), result)
        reply = said(result)
        self.assertEqual("judged", reply["verdict"])
        self.assertEqual("Python lowering", reply["engine"]["name"])
        self.assertEqual(6, reply["counts"]["pass"], reply["counts"])
        self.assertIn("does not say the design is right", reply["means"])
        self.assertIs(False, reply["set"]["quotes_checked"])

    def test_a_failed_example_comes_with_what_was_expected_and_what_was_seen(self):
        reply = said(self.ask(self.sets(), machine(attempts=4)))
        self.assertEqual(1, reply["counts"]["fail"], reply["counts"])
        first = reply["failures"][0]
        self.assertEqual("T3-at-most-three", first["scenario"])
        self.assertEqual([{"event": "TimeoutError"}], first["expected"])
        self.assertEqual([{"event": "SendRequest"}], first["observed"])

    def test_with_the_specification_every_quote_is_checked_word_for_word(self):
        """An example anchored to a sentence the specification does not hold is
        an example about the client's reading, and is refused before a run."""
        def invent(spec):
            spec["scenarios"][0]["quote"] = "the client retries forever"

        reply = said(self.ask(self.sets(invent), specification_text=SPECIFICATION))
        self.assertEqual("set not usable", reply["verdict"], reply)
        self.assertIs(False, reply["set"]["usable"])
        self.assertTrue(reply["set"]["problems"], reply)
        self.assertNotIn("counts", reply)

        honest = said(self.ask(self.sets(), specification_text=SPECIFICATION))
        self.assertEqual("judged", honest["verdict"], honest)
        self.assertIs(True, honest["set"]["quotes_checked"])

    def test_a_design_that_cannot_be_played_is_told_once_per_scenario_not_twice(self):
        """The product records a whole-scenario gap beside the verdict that
        carries the same sentence; the owner reads each sentence once. A gap
        about a step or a check is another matter and stays."""
        reply = said(self.ask(self.sets(), machine(opened=True)))
        self.assertEqual(6, reply["counts"]["not-judged"], reply["counts"])
        self.assertEqual(0, reply["counts"]["fail"], reply["counts"])
        self.assertNotIn("gaps", reply)

    def test_a_set_with_problems_is_not_run(self):
        def break_it(spec):
            spec["scenarios"][0]["steps"][0]["send"] = "NotInTheInterface"

        reply = said(self.ask(self.sets(break_it)))
        self.assertEqual("set not usable", reply["verdict"], reply)
        self.assertNotIn("scenarios", reply)

    def test_a_file_that_is_not_a_scenario_set_is_an_answer_not_a_crash(self):
        result = self.ask(json.dumps({"hello": "world"}))
        self.assertTrue(result.get("isError"), result)
        self.assertNotIn("Traceback", result["content"][0]["text"])


if __name__ == "__main__":
    unittest.main()
