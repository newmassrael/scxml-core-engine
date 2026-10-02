"""A design that starts a child session is played, the child included.

A statechart may `<invoke src="child.scxml">`, and the generated parent starts the
child's own module by name (`import child_sm`). Two things stood in the way of
playing one, and the second was found while fixing the first:

  the child was never generated. The driver built the first document and left
  the others, and the parent died starting its child with `No module named
  'child_sm'`: every example of it was refused, with the engine's words.

  time did not see the child. The engine's next deadline
  (`time_until_next_scheduled_ms`) is how a host walks time one instant at a time,
  and it reported the parent's own scheduler only, while `advance_time` ticks the
  active children by the same delta. A child that re-arms a timer was dated from
  the end of the move, as the parent's was before time was walked: 600 ms in one
  step and 200 ms three times were two runs of one machine.

Asserted here, through the scenario driver and the tool a client calls:

    a parent and its child are generated together and the child's own timer fires
    a child that re-arms is read where real time put it, in one step or in three
    a child document the product refuses refuses the design, and names the child
"""

from __future__ import annotations

import json
import pathlib
import tempfile
import unittest

from sce_author import mcp
from sce_author.scenario_driver import run
from sce_author.verify import _default_codegen


def codegen_is_built() -> bool:
    return _default_codegen().exists()


needs_the_generator = unittest.skipUnless(codegen_is_built(),
                                          "the product's generator is not built")

_NS = ('xmlns="http://www.w3.org/2005/07/scxml" version="1.0" '
       'datamodel="ecmascript"')

PARENT = f"""<scxml {_NS} initial="run" name="parent">
  <state id="run">
    <invoke id="c1" type="http://www.w3.org/TR/scxml/" src="child.scxml"/>
    <transition event="x" target="end"/>
  </state>
  <final id="end"/>
</scxml>
"""

# Tells its parent 500 ms after it starts.
CHILD_ONE_TIMER = f"""<scxml {_NS} initial="a" name="child">
  <state id="a"><onentry><send event="x" target="#_parent" delay="500ms"/></onentry></state>
</scxml>
"""

# Tells its parent 400 ms after it starts, by two timers of 200 ms: the second is
# armed when the first fires, so it is dated from wherever the clock is then.
CHILD_REARMS = f"""<scxml {_NS} initial="a" name="child">
  <state id="a"><onentry><send event="tick" delay="200ms"/></onentry>
    <transition event="tick" target="b"/></state>
  <state id="b"><onentry><send event="tick" delay="200ms"/></onentry>
    <transition event="tick" target="c"/></state>
  <state id="c"><onentry><send event="x" target="#_parent"/></onentry></state>
</scxml>
"""

REFUSED_CHILD = f"""<scxml {_NS} initial="a" name="child">
  <state id="a"><transition event="go" target="nowhere"/></state>
</scxml>
"""


def scenario_set(*scenarios: dict) -> dict:
    return {
        "record": "sce-scenario-set", "v": 1,
        "specification": {"doc_id": "parent", "rev": "1"}, "origin": "ai-proposed",
        "interface": {"inputs": [], "outputs": []},
        "scenarios": list(scenarios),
    }


def scenario(identifier: str, *steps: dict) -> dict:
    return {"id": identifier, "quote": "the parent finishes when its child has told it",
            "steps": list(steps)}


def after(ms: int, finished: bool) -> dict:
    return {"advance_ms": ms, "expect": {"finished": finished}}


@needs_the_generator
class ADesignThatStartsAChild(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.work = pathlib.Path(self._tmp.name)

    def played(self, parent: str, child: str, *scenarios: dict) -> dict:
        (self.work / "parent.scxml").write_text(parent, encoding="utf-8")
        (self.work / "child.scxml").write_text(child, encoding="utf-8")
        (self.work / "set.json").write_text(json.dumps(scenario_set(*scenarios)),
                                            encoding="utf-8")
        return run(self.work / "set.json", self.work / "parent.scxml", _default_codegen(),
                   others=(self.work / "child.scxml",))

    @staticmethod
    def verdicts(answer: dict) -> dict:
        return {r["id"]: r["verdict"] for r in answer["judgement"] if r["kind"] == "verdict"}

    def test_the_child_is_started_and_its_own_timer_fires(self):
        """The parent leaves `run` when the child tells it, and not a millisecond
        sooner: 499 ms in, the child has not spoken."""
        answer = self.played(PARENT, CHILD_ONE_TIMER,
                             scenario("S1", after(499, False), after(1, True)))
        self.assertEqual({"S1": "pass"}, self.verdicts(answer), answer["judgement"])

    def test_a_child_that_re_arms_is_read_where_real_time_put_it(self):
        """⚠ The discriminator. The child speaks at 400 ms either way. Moving the
        parent's clock 600 ms in one jump left the child's second timer dated from
        the end of the jump, at 800, and the parent unfinished; walked to the
        child's own deadlines it is finished at 600, as it is when time passes in
        pieces. Measured 2026-10-02: the next deadline the engine reported was the
        parent's own."""
        answer = self.played(
            PARENT, CHILD_REARMS,
            scenario("ONE-STEP", after(600, True)),
            scenario("THREE-STEPS", after(200, False), after(200, True), after(200, True)))
        self.assertEqual({"ONE-STEP": "pass", "THREE-STEPS": "pass"}, self.verdicts(answer),
                         answer["judgement"])

    def test_a_child_the_product_refuses_refuses_the_design_and_is_named(self):
        answer = self.played(PARENT, REFUSED_CHILD, scenario("S1", after(600, True)))
        self.assertEqual({"S1": "not-judged"}, self.verdicts(answer), answer["judgement"])
        verdict = next(r for r in answer["judgement"] if r["kind"] == "verdict")
        self.assertEqual("design", verdict.get("cause"), verdict)
        self.assertIn("child.scxml", verdict["reason"], verdict)

    def test_a_client_hands_the_child_as_one_of_the_documents(self):
        """The tool takes the statechart first and the documents it uses after it;
        the child is one of them."""
        request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
            "name": "scxml_scenarios", "arguments": {
                "scenarios_text": json.dumps(scenario_set(
                    scenario("S1", after(499, False), after(1, True)))),
                "documents_text": [{"name": "parent.scxml", "text": PARENT},
                                   {"name": "child.scxml", "text": CHILD_ONE_TIMER}]}}}
        import io
        output = io.StringIO()
        mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
        result = json.loads(output.getvalue())["result"]
        self.assertFalse(result.get("isError"), result)
        reply = json.loads(result["content"][0]["text"])
        self.assertEqual("judged", reply["verdict"], reply)
        self.assertEqual(1, reply["counts"]["pass"], reply["counts"])


SETTLED = (f'<scxml {_NS} initial="run" name="other">'
           '<state id="run"><transition event="x" target="end"/></state>'
           '<final id="end"/></scxml>')


class TheChildrenOfADraft(unittest.TestCase):
    """Which documents a draft starts as child sessions, found without the product."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.work = pathlib.Path(self._tmp.name)

    def children(self, document: pathlib.Path) -> list:
        from sce_author.compare import _children_of

        return [path.name for path in _children_of(document)]

    def test_a_static_invoke_src_beside_the_draft_is_found_and_its_own_children_too(self):
        (self.work / "parent.scxml").write_text(PARENT, encoding="utf-8")
        (self.work / "child.scxml").write_text(
            CHILD_ONE_TIMER.replace(
                '<state id="a">',
                '<state id="a"><invoke type="http://www.w3.org/TR/scxml/" '
                'src="grandchild.scxml"/>'), encoding="utf-8")
        (self.work / "grandchild.scxml").write_text(CHILD_ONE_TIMER, encoding="utf-8")
        self.assertEqual(["child.scxml", "grandchild.scxml"],
                         self.children(self.work / "parent.scxml"))

    def test_a_name_with_no_file_behind_it_is_left_out_not_made_up(self):
        (self.work / "parent.scxml").write_text(PARENT, encoding="utf-8")
        self.assertEqual([], self.children(self.work / "parent.scxml"))

    def test_a_src_that_climbs_out_of_the_drafts_directory_is_not_read(self):
        inside = self.work / "drafts"
        inside.mkdir()
        (self.work / "child.scxml").write_text(CHILD_ONE_TIMER, encoding="utf-8")
        (inside / "parent.scxml").write_text(
            PARENT.replace('src="child.scxml"', 'src="../child.scxml"'), encoding="utf-8")
        self.assertEqual([], self.children(inside / "parent.scxml"))

    def test_a_document_that_does_not_parse_has_no_children(self):
        (self.work / "broken.scxml").write_text("<scxml", encoding="utf-8")
        self.assertEqual([], self.children(self.work / "broken.scxml"))


@needs_the_generator
class AComparisonStartsTheChildrenOfItsDrafts(unittest.TestCase):
    """A comparison used to be given one document per draft, so the child a draft
    starts was never built and the draft could not start. It was an honest answer
    and a poor one: the drafts of a design with a child could not be compared at
    all. The child is now built beside the draft that names it."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.work = pathlib.Path(self._tmp.name)

    def test_a_draft_with_its_child_beside_it_is_driven(self):
        from sce_author.compare import compare

        (self.work / "parent.scxml").write_text(PARENT, encoding="utf-8")
        (self.work / "child.scxml").write_text(CHILD_ONE_TIMER, encoding="utf-8")
        (self.work / "other.scxml").write_text(SETTLED, encoding="utf-8")
        behaviour = compare([self.work / "parent.scxml", self.work / "other.scxml"],
                            _default_codegen(), drives=10, steps=6)["behaviour"]
        self.assertEqual({}, behaviour["undriven"], behaviour)
        self.assertGreater(behaviour["distinct_observations"]["parent.scxml"], 1, behaviour)

    def test_the_tool_takes_the_children_as_text_and_builds_them_beside_the_drafts(self):
        import io

        request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
            "name": "compare", "arguments": {
                "documents_text": [{"name": "parent.scxml", "text": PARENT},
                                   {"name": "other.scxml", "text": SETTLED}],
                "companions_text": [{"name": "child.scxml", "text": CHILD_ONE_TIMER}]}}}
        output = io.StringIO()
        mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
        result = json.loads(output.getvalue())["result"]
        self.assertFalse(result.get("isError"), result)
        report = json.loads(result["content"][0]["text"])
        self.assertEqual(["parent.scxml", "other.scxml"], report["documents"])
        self.assertEqual({}, report["behaviour"]["undriven"], report["behaviour"])

    def test_the_companions_are_not_drafts_and_a_bad_list_is_an_argument_error(self):
        import io

        for companions in ([], "child.scxml", [3]):
            with self.subTest(companions=companions):
                request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                    "name": "compare", "arguments": {
                        "documents_text": [{"name": "parent.scxml", "text": PARENT},
                                           {"name": "other.scxml", "text": SETTLED}],
                        "companions_text": companions}}}
                output = io.StringIO()
                mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
                self.assertTrue(json.loads(output.getvalue())["result"].get("isError"))

    def compare(self, **arguments) -> dict:
        import io

        request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                   "params": {"name": "compare", "arguments": arguments}}
        output = io.StringIO()
        mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
        return json.loads(output.getvalue())["result"]

    def test_two_drafts_may_each_start_a_different_child_of_the_same_name(self):
        """The drafts being compared were written in separate conversations, so each
        calls its child `child.scxml` and means another document: one tells its parent
        at 500 ms, the other at 900 ms. A directory for all of them held one child, and
        a second of the same name was refused; each draft now has a directory of its own."""
        slower = CHILD_ONE_TIMER.replace("500ms", "900ms")
        result = self.compare(documents_text=[
            {"name": "fast.scxml", "text": PARENT.replace('name="parent"', 'name="fast"'),
             "companions": [{"name": "child.scxml", "text": CHILD_ONE_TIMER}]},
            {"name": "slow.scxml", "text": PARENT.replace('name="parent"', 'name="slow"'),
             "companions": [{"name": "child.scxml", "text": slower}]}])
        self.assertFalse(result.get("isError"), result)
        report = json.loads(result["content"][0]["text"])
        self.assertEqual(["fast.scxml", "slow.scxml"], report["documents"])
        self.assertEqual({}, report["behaviour"]["undriven"], report["behaviour"])
        self.assertEqual("judged", report["behaviour"]["verdict"], report["behaviour"])

    def test_each_draft_is_driven_against_its_own_child_and_not_a_neighbours(self):
        """The same two drafts, driven directly: after 600 ms the one whose child
        speaks at 500 ms has finished and the one whose child speaks at 900 ms has
        not. Were the children mixed up, both would have done the same."""
        from sce_author.compare import _Driven

        slower = CHILD_ONE_TIMER.replace("500ms", "900ms")
        for name, child, finished in (("fast", CHILD_ONE_TIMER, True), ("slow", slower, False)):
            directory = self.work / name
            directory.mkdir()
            (directory / "draft.scxml").write_text(PARENT, encoding="utf-8")
            (directory / "child.scxml").write_text(child, encoding="utf-8")
            driven = _Driven(directory / "draft.scxml", _default_codegen(),
                             self.work / f"built-{name}")
            seen = driven.trace([("time", 600)])
            self.assertEqual(finished, seen[-1][2], (name, seen))

    def test_a_shared_child_and_a_draft_s_own_may_both_be_given(self):
        result = self.compare(
            documents_text=[
                {"name": "a.scxml", "text": PARENT.replace('name="parent"', 'name="a"')},
                {"name": "b.scxml", "text": PARENT.replace('name="parent"', 'name="b"'),
                 "companions": [{"name": "extra.scxml", "text": CHILD_ONE_TIMER}]}],
            companions_text=[{"name": "child.scxml", "text": CHILD_ONE_TIMER}])
        self.assertFalse(result.get("isError"), result)

    def test_a_companion_named_like_another_file_of_its_draft_is_refused(self):
        result = self.compare(documents_text=[
            {"name": "a.scxml", "text": PARENT,
             "companions": [{"name": "child.scxml", "text": CHILD_ONE_TIMER},
                            {"name": "child.scxml", "text": CHILD_ONE_TIMER}]},
            {"name": "b.scxml", "text": SETTLED}])
        self.assertTrue(result.get("isError"), result)
        self.assertIn("two files are named 'child.scxml'", result["content"][0]["text"])

    def test_two_drafts_of_one_name_and_companions_beside_paths_are_refused(self):
        twin = self.compare(documents_text=[{"name": "a.scxml", "text": PARENT},
                                            {"name": "a.scxml", "text": SETTLED}])
        self.assertTrue(twin.get("isError"), twin)
        self.assertIn("two drafts are named 'a.scxml'", twin["content"][0]["text"])
        (self.work / "a.scxml").write_text(PARENT, encoding="utf-8")
        (self.work / "b.scxml").write_text(SETTLED, encoding="utf-8")
        beside = self.compare(documents=[str(self.work / "a.scxml"), str(self.work / "b.scxml")],
                              companions_text=[{"name": "child.scxml", "text": CHILD_ONE_TIMER}])
        self.assertTrue(beside.get("isError"), beside)
        self.assertIn("next to the draft", beside["content"][0]["text"])


@needs_the_generator
class ADesignThatCannotStartIsAnAnswer(unittest.TestCase):
    """A draft whose child is not there cannot start it. That is the design's
    answer, said in the engine's words, not a traceback out of the tool."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.work = pathlib.Path(self._tmp.name)

    def test_a_comparison_names_the_draft_it_could_not_start(self):
        from sce_author.compare import compare

        (self.work / "parent.scxml").write_text(PARENT, encoding="utf-8")
        # No child.scxml beside it: nothing for the comparison to build.
        (self.work / "other.scxml").write_text(SETTLED, encoding="utf-8")
        behaviour = compare([self.work / "parent.scxml", self.work / "other.scxml"],
                            _default_codegen(), drives=10, steps=6)["behaviour"]
        self.assertEqual("not judged", behaviour["verdict"], behaviour)
        self.assertEqual(["parent.scxml"], list(behaviour["undriven"]), behaviour)
        self.assertIn("could not start", behaviour["undriven"]["parent.scxml"])
        self.assertIn("child_sm", behaviour["undriven"]["parent.scxml"])

    def test_a_verification_refuses_a_design_it_could_not_start(self):
        import shutil

        import yaml

        from sce_author.pack import load_pack
        from sce_author.verify import verify
        from tests.test_a_statechart_is_driven_not_called import (
            BINDING, CROSSING, DELAYED, EXAMPLES)

        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.work / name)
        invoking = DELAYED.replace(
            '<state id="dark">',
            '<state id="dark"><invoke id="c1" type="http://www.w3.org/TR/scxml/" '
            'src="child.scxml"/>')
        (self.work / "signal.scxml").write_text(invoking, encoding="utf-8")
        (self.work / "child.scxml").write_text(CHILD_ONE_TIMER, encoding="utf-8")
        (self.work / "examples.yaml").write_text(yaml.safe_dump({
            **EXAMPLES, "cases": [{
                "name": "a train is detected",
                "given": {"plant/in/train-approach": "APPROACHING"},
                "drove": ["plant/in/train-approach"],
                "elapsed_ms": {"min": 600, "max": 600},
                "expect": {"plant/out/road-signal.value": "DARK"}}]}), encoding="utf-8")
        (self.work / "b.yaml").write_text(
            yaml.safe_dump({**BINDING, "activation": "on-change"}), encoding="utf-8")
        result = verify(load_pack(self.work), self.work / "b.yaml", _default_codegen())
        self.assertFalse(result.ran, "a design that could not start was judged")
        self.assertIn("could not start", result.refusal)
        self.assertIn("child_sm", result.refusal)


if __name__ == "__main__":
    unittest.main()
