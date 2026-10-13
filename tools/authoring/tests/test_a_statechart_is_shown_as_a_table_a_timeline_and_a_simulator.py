"""A statechart is shown to a reader who is not a programmer, from the document and the real engine alone.

`views` draws three things (a table, a timeline, a simulator) and none of them knows what a document is
about. These cases hold the line between what is read and what is made up: the table is the document's own
structure, the timeline is what the engine did in the owner's examples, and the simulator is the engine's own
behaviour written down and then played against the engine to see that it was written down right.

The design is a gate that must be told to open, waits 300 ms for its sensor, and gives up for good after two
tries. It has a timer, a guard, a counter, an output through a host processor and a final state: everything a
view has to cope with that a plain two-state toggle does not.
"""

from __future__ import annotations

import json
import pathlib
import tempfile
import unittest

from sce_author.views import (FINISHED, Labels, ViewError, build, describe_cond, explore, name_html, output_names,
                              play, read_machine, table_model, timeline_model, timelines_html)
from sce_author.verify import _default_codegen

GATE = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       datamodel="ecmascript" initial="closed">
  <datamodel><data id="tries" expr="0"/></datamodel>
  <state id="closed"><transition event="open" target="opening"/></state>
  <state id="opening">
    <onentry>
      <assign location="tries" expr="tries + 1"/>
      <send event="motor" type="x-host"/>
      <send id="t" event="give.up" delay="300ms"/>
    </onentry>
    <onexit><cancel sendid="t"/></onexit>
    <transition event="sensor" target="open"/>
    <transition event="give.up" cond="tries &lt; 2" target="closed"/>
    <transition event="give.up" cond="tries &gt;= 2" target="stuck"/>
  </state>
  <state id="open"><transition event="close" target="closed"><assign location="tries" expr="0"/></transition></state>
  <final id="stuck"/>
</scxml>
"""

# The same gate with nothing that ever resets the counter: every open-close cycle reaches a situation not seen before.
GROWS = GATE.replace('<transition event="close" target="closed"><assign location="tries" expr="0"/></transition>',
                     '<transition event="close" target="closed"/>')

# Three lanes of one parallel state, and a rule that watches two of them: the shape a flat design never has.
PAIR = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="both">
  <parallel id="both">
    <state id="left" initial="l0"><state id="l0"><transition event="l.go" target="l1"><raise event="eval"/></transition></state>
      <state id="l1"/></state>
    <state id="right" initial="r0"><state id="r0"><transition event="r.go" target="r1"><raise event="eval"/></transition></state>
      <state id="r1"/></state>
    <state id="watch">
      <transition event="eval" cond="In('l1') &amp;&amp; In('r1')"><send event="both" type="x-host"/></transition>
    </state>
  </parallel>
</scxml>
"""

# After `pause` the machine is in `pause` whichever of a or b it left, and `resume` goes back to where it was.
# That memory is in no configuration and no data item, so a table of what the engine does cannot hold it.
REMEMBERS = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="work">
  <state id="work" initial="a">
    <history id="h" type="shallow"><transition target="a"/></history>
    <state id="a"><transition event="next" target="b"/></state>
    <state id="b"/>
    <transition event="pause" target="rest"/>
  </state>
  <state id="rest"><transition event="resume" target="h"/></state>
</scxml>
"""

SCENARIOS = {
    "record": "sce-scenario-set", "v": 1, "specification": {"doc_id": "gate", "rev": "1"}, "origin": "ai-proposed",
    "interface": {"why": "test", "inputs": [{"name": "open"}, {"name": "sensor"}, {"name": "close"}],
                  "outputs": [{"name": "motor"}]},
    "scenarios": [
        {"id": "G1-gives-up-for-good", "quote": "gives up for good after two tries",
         "steps": [{"send": "open"}, {"advance_ms": 300}, {"send": "open"}, {"advance_ms": 300}]},
        {"id": "G2-sensor-answers", "quote": "waits 300 ms for its sensor",
         "steps": [{"send": "open"}, {"advance_ms": 100}, {"send": "sensor"}]},
        {"id": "G3-undecided", "quote": "gives up for good after two tries", "status": "awaiting-decision",
         "decision": {"question": "after how many?", "alternatives": ["G1-gives-up-for-good"]},
         "steps": [{"send": "open"}]},
    ],
}


def codegen_is_built() -> bool:
    return _default_codegen().exists()


needs_the_generator = unittest.skipUnless(codegen_is_built(), "the product's generator is not built")


class Work(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.work = pathlib.Path(self.directory.name)

    def write(self, name: str, text: str) -> pathlib.Path:
        path = self.work / name
        path.write_text(text, encoding="utf-8")
        return path


class TheTableIsTheDocumentsOwnStructure(Work):
    def test_a_row_says_where_each_signal_goes_and_what_entering_does(self) -> None:
        machine = read_machine(self.write("gate.scxml", GATE))
        (table,) = table_model(machine, Labels())
        self.assertEqual(["open", "sensor", "give.up", "close"], table["events"])
        self.assertTrue(table["entering"])
        rows = {row["state"]: row for row in table["rows"]}
        self.assertEqual(["opening"], rows["closed"]["cells"]["open"][0]["targets"])
        # Two transitions answer the same signal; each keeps its own condition.
        waits = rows["opening"]["cells"]["give.up"]
        self.assertEqual([["closed"], ["stuck"]], [w["targets"] for w in waits])
        self.assertTrue(all(w["cond"] for w in waits))
        self.assertIn("tries &lt; 2", waits[0]["cond"])
        # What entering does is said, the timer with its delay and the host output with its processor.
        entering = " ".join(rows["opening"]["entry"])
        self.assertIn("tries ← tries + 1", entering)
        self.assertIn("motor (x-host)", entering)
        self.assertIn("300ms", entering)
        self.assertEqual("final", rows["stuck"]["kind"])

    def test_a_parallel_state_lists_its_lanes_and_gives_a_watching_region_as_a_rule(self) -> None:
        machine = read_machine(self.write("pair.scxml", PAIR))
        tables = {t["container"]: t for t in table_model(machine, Labels())}
        # `both` itself: the two lanes are named, and the region that only watches and acts is a rule, not a row:
        # no transition of it leaves a state, so a state-by-signal table has nothing to say about it.
        self.assertEqual(["left", "right"], tables["both"]["regions"])
        self.assertEqual([], tables["both"]["rows"])
        (rule,) = tables["both"]["rules"]
        self.assertEqual("watch", rule["region"])
        self.assertEqual([], rule["events"])        # `eval` is the machine's own signal, so it is not a signal to wait for
        self.assertTrue(rule["own"])
        self.assertIn("both (x-host)", rule["actions"][0])
        self.assertIn("&lsquo;l1&rsquo;", rule["cond"])
        # Each lane has a table of its own, and the root, which only holds `both`, has none.
        self.assertIn("left", tables)
        self.assertIn("right", tables)
        self.assertNotIn("", tables)

    def test_a_condition_over_states_is_a_list_and_one_over_values_is_left_as_written(self) -> None:
        machine = read_machine(self.write("pair.scxml", PAIR))
        said = describe_cond("(In('l1') && In('r1')) || (In('l0') && In('r0'))", Labels(), "ko", machine)
        self.assertIn("다음 중 하나일 때", said)
        self.assertEqual(2, said.count("&bull;"))
        self.assertIn("left", said)
        self.assertIn("r1", said)
        # An expression that is not about states is never paraphrased: a reading of it would be a guess.
        self.assertIn("tries &lt; 2", describe_cond("tries < 2", Labels(), "ko", machine))

    def test_what_the_machine_says_to_itself_is_left_out_of_a_cell_unless_asked_for(self) -> None:
        machine = read_machine(self.write("pair.scxml", PAIR))
        quiet = {t["container"]: t for t in table_model(machine, Labels())}["left"]
        loud = {t["container"]: t for t in table_model(machine, Labels(), internal=True)}["left"]
        row = lambda table: next(r for r in table["rows"] if r["state"] == "l0")  # noqa: E731
        self.assertEqual([], row(quiet)["cells"]["l.go"][0]["actions"])
        self.assertEqual(["내부 신호: eval"], row(loud)["cells"]["l.go"][0]["actions"])

    def test_a_cell_that_changes_nothing_is_quiet_and_one_that_moves_is_marked(self) -> None:
        from sce_author.views import _cell_html

        stay = [{"cond": "", "targets": ["a"], "internal": False, "actions": []}]
        move = [{"cond": "", "targets": ["b"], "internal": False, "actions": []}]
        self.assertIn('class="quiet"', _cell_html(stay, "a", Labels(), "ko"))
        self.assertIn('class="moves"', _cell_html(move, "a", Labels(), "ko"))
        # Staying put with something to say (an output, a condition) is not quiet: that is the cell's news.
        said = [{"cond": "", "targets": ["a"], "internal": False, "actions": ["보냄: x"]}]
        self.assertNotIn("quiet", _cell_html(said, "a", Labels(), "ko"))

    def test_a_region_with_two_rules_is_named_once(self) -> None:
        from sce_author.views import _rules_html

        rule = {"region": "w", "events": ["x"], "own": False, "cond": "", "actions": ["a"]}
        table = {"rules": [rule, {**rule, "events": ["y"]}, {**rule, "region": "z"}]}
        said = _rules_html(table, Labels(), "ko")
        self.assertEqual(2, said.count('<th scope="row"'))
        self.assertIn('rowspan="2"', said)

    def test_a_label_is_shown_beside_the_documents_name_and_never_instead_of_it(self) -> None:
        labels = Labels(states={"opening": "문이 열리는 중"})
        shown = name_html(labels, "states", "opening")
        self.assertIn("문이 열리는 중", shown)
        # The document's name stays on the page, as the tooltip of the words, and not as a second line in a cell.
        self.assertIn('title="opening"', shown)
        self.assertNotIn("문이", name_html(labels, "states", "closed"))

    def test_a_state_can_say_what_being_in_it_means(self) -> None:
        path = self.write("labels.json", json.dumps(
            {"states": {"opening": {"label": "열리는 중", "note": "모터가 돌고 센서를 기다린다"}}}))
        labels = Labels.read(path)
        self.assertEqual("열리는 중", labels.of("states", "opening"))
        self.assertNotIn("모터", name_html(labels, "states", "opening"))
        self.assertIn("모터가 돌고 센서를 기다린다", name_html(labels, "states", "opening", detail=True))
        bad = self.write("bad.json", json.dumps({"states": {"opening": {"label": "x", "colour": "red"}}}))
        with self.assertRaises(ViewError):
            Labels.read(bad)

    def test_an_unknown_key_in_a_labels_file_is_refused(self) -> None:
        path = self.write("labels.json", json.dumps({"states": {}, "colours": {}}))
        with self.assertRaises(ViewError) as caught:
            Labels.read(path)
        self.assertIn("colours", str(caught.exception))

    def test_two_outputs_that_share_an_event_name_keep_their_processors(self) -> None:
        names = output_names(["x-a:on", "x-b:on", "x-a:off"], Labels(outputs={"x-b:on": "second"}))
        self.assertEqual("second", names["x-b:on"])
        self.assertEqual("x-a:on", names["x-a:on"])
        self.assertEqual("off", names["x-a:off"])


@needs_the_generator
class TheTimelineIsWhatTheEngineDidInTheOwnersExamples(Work):
    def test_each_column_is_one_step_with_the_state_the_data_and_what_was_sent(self) -> None:
        document = self.write("gate.scxml", GATE)
        machine = read_machine(document)
        scenario = SCENARIOS["scenarios"][0]
        run = play(document, [scenario])["runs"][0]
        model = timeline_model(machine, scenario, run)
        (lane,) = [lane for lane in model["lanes"] if lane["container"] == ""]
        # The machine as it starts, then one column per step, and the run ends in a final state.
        self.assertEqual(["closed", "opening", "closed", "opening", FINISHED], lane["cells"])
        sent = [column["outputs"] for column in model["columns"]]
        self.assertEqual([[], ["x-host:motor"], [], ["x-host:motor"], []], sent)
        (counter,) = [v for v in model["values"] if v["name"] == "tries"]
        self.assertEqual([0, 1, 1, 2, 2], counter["cells"])
        self.assertEqual([0, 0, 300, 300, 600], [column["clock"] for column in model["columns"]])

    def test_a_folded_set_names_each_scenario_once_and_opens_only_the_first(self) -> None:
        document = self.write("gate.scxml", GATE)
        sets = self.write("gate.scenarios.json", json.dumps(SCENARIOS))
        folded, notes = timelines_html(document, sets, Labels(), fold=True)
        plain, _ = timelines_html(document, sets, Labels(), fold=False)
        # In a fold the summary carries the name and the sentence, so the figure inside must not repeat them.
        self.assertEqual(2, folded.count("<details"))
        self.assertEqual(1, folded.count("<details class=\"sv-fold\" open"))
        self.assertNotIn("<figcaption>", folded)
        self.assertEqual(2, plain.count("<figcaption>"))
        self.assertEqual(1, len(notes))

    def test_a_scenario_that_waits_on_a_decision_is_not_drawn(self) -> None:
        document = self.write("gate.scxml", GATE)
        sets = self.write("gate.scenarios.json", json.dumps(SCENARIOS))
        written = build(document, self.work / "out", scenario_set=sets, simulator=False)
        timeline = (self.work / "out" / "timeline.html").read_text(encoding="utf-8")
        self.assertIn("G1-gives-up-for-good", timeline)
        self.assertIn("G2-sensor-answers", timeline)
        self.assertNotIn("G3-undecided", timeline)
        self.assertTrue(any("G3-undecided" in line and "not drawn" in line for line in written))


@needs_the_generator
class TheSimulatorIsTheEnginesBehaviourCheckedAgainstTheEngine(Work):
    def test_a_press_leads_where_the_engine_went_and_a_timer_is_not_a_button(self) -> None:
        graph = explore(self.write("gate.scxml", GATE), walks=60)
        names = [a.get("name", "time") for a in graph["alphabet"]]
        # `give.up` is the machine's own timer: it happens when time passes and is never pressed.
        self.assertNotIn("give.up", names)
        self.assertEqual(["time"], [n for n in names if n == "time"])
        self.assertEqual(0, graph["checked"]["differences"])
        self.assertGreater(graph["checked"]["steps"], 100)
        press = names.index("open")
        to, sent, moved = graph["edges"][0][press]
        self.assertIn("opening", graph["nodes"][to]["cfg"])
        self.assertEqual(["x-host:motor"], sent)
        self.assertEqual(1, graph["nodes"][to]["data"]["tries"])
        # Time passing from there reaches the timer's deadline and no further.
        passing = names.index("time")
        to, sent, moved = graph["edges"][to][passing]
        self.assertEqual(300, moved)
        self.assertIn("closed", graph["nodes"][to]["cfg"])

    def test_a_design_with_more_situations_than_the_limit_is_refused_not_cut_short(self) -> None:
        with self.assertRaises(ViewError) as caught:
            explore(self.write("gate.scxml", GATE), max_nodes=3)
        self.assertIn("situations", str(caught.exception))

    def test_a_counter_nothing_resets_means_the_table_never_closes_and_is_said_so(self) -> None:
        with self.assertRaises(ViewError) as caught:
            explore(self.write("grows.scxml", GROWS), max_depth=8, seconds=120)
        self.assertIn("never closes", str(caught.exception))

    def test_a_memory_the_table_cannot_hold_is_found_by_the_walks_and_refused(self) -> None:
        # Two different pasts reach `rest` looking identical, and `resume` answers each differently. A table made
        # from what can be seen would give a button the wrong answer, so the walks must catch it.
        with self.assertRaises(ViewError) as caught:
            explore(self.write("remembers.scxml", REMEMBERS), walks=200)
        self.assertIn("disagreed", str(caught.exception))

    def test_the_pages_are_written_and_a_page_that_cannot_be_made_is_said_not_silently_dropped(self) -> None:
        document = self.write("gate.scxml", GATE)
        written = build(document, self.work / "ok", scenario_set=self.write("s.json", json.dumps(SCENARIOS)))
        for page in ("table.html", "timeline.html", "simulator.html"):
            self.assertTrue((self.work / "ok" / page).exists(), page)
        remembered = build(self.write("remembers.scxml", REMEMBERS), self.work / "no")
        self.assertTrue((self.work / "no" / "table.html").exists())
        self.assertFalse((self.work / "no" / "simulator.html").exists())
        self.assertTrue(any("disagreed" in line for line in remembered), remembered)
        self.assertTrue(all(isinstance(line, str) for line in written))


class TheCommandSaysWhatItWroteAndRefusesWhatItCannotRead(Work):
    def test_the_command_prints_each_page_it_wrote(self) -> None:
        import contextlib
        import io

        from sce_author.__main__ import main

        document = self.write("gate.scxml", GATE)
        out, shown = self.work / "pages", io.StringIO()
        with contextlib.redirect_stdout(shown):
            code = main(["views", "--document", str(document), "--out", str(out), "--no-simulator"])
        self.assertEqual(0, code)
        self.assertIn("table.html", shown.getvalue())
        self.assertNotIn("simulator.html", shown.getvalue())
        self.assertTrue((out / "table.html").exists())

    def test_a_document_that_is_not_xml_is_refused_with_its_name(self) -> None:
        import contextlib
        import io

        from sce_author.__main__ import main

        broken = self.write("broken.scxml", "<scxml")
        said = io.StringIO()
        with contextlib.redirect_stderr(said):
            code = main(["views", "--document", str(broken), "--out", str(self.work / "x"), "--no-simulator"])
        self.assertEqual(1, code)
        self.assertIn("broken.scxml", said.getvalue())


class OnlyAStatechartIsDrawn(Work):
    def test_a_document_of_another_kind_is_refused_by_name(self) -> None:
        text = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" '
                'sce:kind="transform" datamodel="ecmascript"/>')
        with self.assertRaises(ViewError) as caught:
            read_machine(self.write("t.scxml", text))
        self.assertIn("transform", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
