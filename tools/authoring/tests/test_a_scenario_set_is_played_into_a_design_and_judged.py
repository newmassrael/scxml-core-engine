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


class Played(unittest.TestCase):
    """Shared plumbing: a work directory, a design, a set."""

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
