"""Several drafts of one specification are compared at every level, and the
comparison says only what it measured.

An author asked twice writes two files. Measured 2026-09-29 on six small
specifications, five drafts each: no two of thirty drafts were byte-identical,
the behaviour the prose decided came out alike, and what moved was names and
the list of open questions. `compare` reports each level separately so the
owner can tell those apart.

⚠ The two refusals to claim agreement are the reason this file exists. A draft
that cannot be driven is named rather than classed, and drives that move
nothing judge nothing -- the first analysis of that measurement reported five
drafts as behaving alike because each had failed with the same error.
"""

from __future__ import annotations

import json
import pathlib
import tempfile
import unittest

from sce_author.compare import CompareError, compare
from sce_author.verify import _default_codegen

HEAD = ('<?xml version="1.0" encoding="UTF-8"?>\n'
        '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
        'xmlns:sce="http://sce.dev/ext" version="1.0" name="gate" initial="shut">\n')

# Open on request, close by itself after 20 s; a request while open restarts
# the count. The same prose, drafted three ways below.
RESTARTS = HEAD + """
  <state id="shut"><transition event="request" target="opening"/></state>
  <state id="opening"><transition event="reached" target="open"/></state>
  <state id="open">
    <onentry><send id="timer" event="expire" delay="20s"/></onentry>
    <onexit><cancel sendid="timer"/></onexit>
    <transition event="request" target="open"/>
    <transition event="expire" target="closing"/>
  </state>
  <state id="closing"><transition event="reached" target="shut"/></state>
</scxml>
"""

# The same machine with its input called something else and its states
# renamed: every level that reads names moves, and the behaviour does not.
RENAMED = HEAD.replace('initial="shut"', 'initial="closed"') + """
  <!-- the author's note, which only the bytes see -->
  <state id="closed"><transition event="request.open" target="rising"/></state>
  <state id="rising"><transition event="reached" target="up"/></state>
  <state id="up">
    <onentry><send id="t" event="expire" delay="20s"/></onentry>
    <onexit><cancel sendid="t"/></onexit>
    <transition event="request.open" target="up"/>
    <transition event="expire" target="lowering"/>
  </state>
  <state id="lowering"><transition event="reached" target="closed"/></state>
</scxml>
"""

# The other reading of the prose: a request while open does not restart it.
IGNORES = RESTARTS.replace('    <transition event="request" target="open"/>\n', "")

# Moves only on a field of the event's data, which no drive carries.
NEEDS_DATA = HEAD + """
  <datamodel><data id="n" expr="0"/></datamodel>
  <state id="shut">
    <transition event="coin" cond="_event.data.value &gt; 0" target="open"/>
  </state>
  <state id="open"><transition event="coin" cond="_event.data.value &lt; 0" target="shut"/></state>
</scxml>
"""


def _start_at_a(text: str) -> str:
    return text.replace('initial="shut"', 'initial="a"')


# Waits for `go`. The settled draft the next two are compared with.
SETTLES = _start_at_a(HEAD) + """
  <state id="a"><transition event="go" target="b"/></state>
  <state id="b"/>
</scxml>
"""

# The eventless self-transition is enabled again every time it is taken, so the
# macrostep never reaches a stable configuration (W3C SCXML 3.13).
NEVER_SETTLES = _start_at_a(HEAD) + """
  <state id="a"><transition target="a"/><transition event="go" target="b"/></state>
  <state id="b"/>
</scxml>
"""

# The same, but only after an input: the first observation is a settled one.
NEVER_SETTLES_AFTER_GO = _start_at_a(HEAD) + """
  <state id="a"><transition event="go" target="b"/></state>
  <state id="b"><transition target="b"/></state>
</scxml>
"""

# Three retries 200 ms apart, written two ways. Each is armed when the last one
# fires, or all three are armed at the start. Walked deadline to deadline they
# are one machine; moved by a single jump of 600 ms the first is dated from the
# end of the jump and the second is not, and they part.
RETRIES_REARMED = _start_at_a(HEAD).replace('initial="a"', 'initial="w0"') + """
  <state id="w0"><onentry><send event="r" delay="200ms"/></onentry>
    <transition event="r" target="w1"/></state>
  <state id="w1"><onentry><send event="r" delay="200ms"/></onentry>
    <transition event="r" target="w2"/></state>
  <state id="w2"><onentry><send event="r" delay="200ms"/></onentry>
    <transition event="r" target="done"/></state>
  <final id="done"/>
</scxml>
"""

RETRIES_ARMED_TOGETHER = _start_at_a(HEAD).replace('initial="a"', 'initial="w0"') + """
  <state id="w0"><onentry>
      <send event="r" delay="200ms"/><send event="r" delay="400ms"/>
      <send event="r" delay="600ms"/></onentry>
    <transition event="r" target="w1"/></state>
  <state id="w1"><transition event="r" target="w2"/></state>
  <state id="w2"><transition event="r" target="done"/></state>
  <final id="done"/>
</scxml>
"""

# A heartbeat: a deadline at every 100 ms, forever.
HEARTBEAT = _start_at_a(HEAD) + """
  <state id="a"><onentry><send event="beat" delay="100ms"/></onentry>
    <transition event="beat" target="a"/></state>
</scxml>
"""


@unittest.skipUnless(_default_codegen().exists(),
                     "the comparison asks the product; build sce-codegen first")
class DraftsAreComparedAtEveryLevel(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.dir = pathlib.Path(temporary.name)

    def drafts(self, **texts) -> list:
        paths = []
        for name, text in texts.items():
            path = self.dir / f"{name}.scxml"
            path.write_text(text, encoding="utf-8")
            paths.append(path)
        return paths

    def test_a_renamed_draft_is_one_behaviour_and_the_rename_is_reported(self):
        report = compare(self.drafts(a=RESTARTS, b=RENAMED))
        levels = report["levels"]
        for level in ("bytes", "canonical", "logic", "table", "page", "vocabulary"):
            self.assertEqual(len(levels[level]), 2, level)
        behaviour = report["behaviour"]
        self.assertEqual(behaviour["verdict"], "judged")
        self.assertEqual(behaviour["classes"], [["a.scxml", "b.scxml"]])
        self.assertEqual(behaviour["renames"], {"b.scxml": {"request": "request.open"}})
        # The bound is part of the verdict, with each scheduled delay and the
        # millisecond either side of it among the advances.
        self.assertEqual(behaviour["bound"]["advances_ms"], [1000, 19999, 20000, 20001])
        self.assertTrue(all(n > 1 for n in behaviour["distinct_observations"].values()))

    def test_drafts_that_part_are_told_apart_by_a_minimal_witness(self):
        report = compare(self.drafts(a=RESTARTS, b=IGNORES))
        behaviour = report["behaviour"]
        self.assertEqual(behaviour["classes"], [["a.scxml"], ["b.scxml"]])
        (witness,) = behaviour["witnesses"]
        self.assertEqual(witness["drafts"], ["a.scxml", "b.scxml"])
        # Every step is needed: open, wait just short of the deadline, ask
        # again, and let time pass. Which advance ends it is whichever the
        # seeded drive happened to hold, so only its presence is asserted.
        self.assertEqual(witness["drive"][:4], ["request", "reached", "19999 ms", "request"])
        self.assertEqual(len(witness["drive"]), 5)
        self.assertTrue(witness["drive"][4].endswith(" ms"))
        self.assertEqual(witness["first_then"][0], ("open",))
        self.assertEqual(witness["second_then"][0], ("closing",))

    def test_drives_that_move_nothing_judge_nothing(self):
        report = compare(self.drafts(a=NEEDS_DATA, b=NEEDS_DATA.replace("coin", "token")))
        behaviour = report["behaviour"]
        self.assertEqual(behaviour["verdict"], "not judged")
        self.assertEqual(behaviour["distinct_observations"], {"a.scxml": 1, "b.scxml": 1})
        self.assertNotIn("classes", behaviour)
        self.assertIn("moved nothing", behaviour["why"])

    def test_a_draft_the_product_refuses_is_named_and_left_out(self):
        broken = RESTARTS.replace('target="opening"', 'target="nowhere"')
        report = compare(self.drafts(a=RESTARTS, b=RENAMED, c=broken))
        self.assertIn("c.scxml", report["refused"])
        self.assertEqual(report["behaviour"]["classes"], [["a.scxml", "b.scxml"]])

    def test_annotations_move_the_bytes_and_not_the_logic(self):
        annotated = RESTARTS.replace(
            '<state id="shut">',
            '<state id="shut" sce:unresolved="start" '
            'sce:unresolved-reason="the prose does not say where it starts">')
        report = compare(self.drafts(a=RESTARTS, b=annotated))
        levels = report["levels"]
        self.assertEqual(len(levels["bytes"]), 2)
        self.assertEqual(len(levels["logic"]), 1)
        self.assertEqual(len(levels["table"]), 1)
        # What the owner is asked is exactly what moved.
        self.assertEqual(report["open_sets"], {"a.scxml": [], "b.scxml": ["unresolved:start"]})

    def test_a_draft_whose_macrostep_never_ends_is_not_a_behaviour(self):
        """Measured 2026-10-02 against `b4a8f8c674`: this draft and a settled one
        were reported as behaving alike, because the engine stops such a
        macrostep and what is left looks like a machine that waits. The scenario
        driver already refuses it (W3C SCXML 3.13); a comparison that judges it
        says the opposite of the verdict on the same draft."""
        for when, never_settles in (("at the start", NEVER_SETTLES),
                                    ("after an input", NEVER_SETTLES_AFTER_GO)):
            with self.subTest(when):
                behaviour = compare(self.drafts(loop=never_settles, settled=SETTLES),
                                    drives=40, steps=12)["behaviour"]
                self.assertEqual("not judged", behaviour["verdict"])
                self.assertEqual(["loop.scxml"], list(behaviour["undriven"]))
                self.assertIn("stable configuration", behaviour["undriven"]["loop.scxml"])
                self.assertNotIn("classes", behaviour)
                # The reason given is the true one: the settled draft did move, so
                # "the drives moved nothing" would say something false about it.
                self.assertIn("undriven", behaviour["why"])
                self.assertNotIn("moved nothing", behaviour["why"])

    def test_a_draft_that_never_settles_does_not_hide_the_ones_that_do(self):
        renamed = SETTLES.replace('"go"', '"proceed"')
        behaviour = compare(self.drafts(loop=NEVER_SETTLES, one=SETTLES, two=renamed),
                            drives=40, steps=12)["behaviour"]
        self.assertEqual("judged", behaviour["verdict"])
        self.assertEqual([["one.scxml", "two.scxml"]], behaviour["classes"])
        self.assertEqual(["loop.scxml"], list(behaviour["undriven"]))

    def test_time_is_walked_from_deadline_to_deadline_not_jumped(self):
        """The engine dates a timer from the end of the move that fires it, so a
        machine that re-arms inside one long move is a different run from the
        same time passing in pieces. `scenario_play` walks to each deadline; a
        comparison that jumps tells the owner that two drafts of one prose
        differ when they do not, on a drive no host would make."""
        behaviour = compare(self.drafts(rearmed=RETRIES_REARMED, together=RETRIES_ARMED_TOGETHER),
                            drives=40, steps=12)["behaviour"]
        self.assertEqual("judged", behaviour["verdict"])
        self.assertEqual([["rearmed.scxml", "together.scxml"]], behaviour["classes"], behaviour)

    def test_a_deadline_at_too_many_instants_is_not_played_to_its_end(self):
        import importlib
        from unittest import mock

        with mock.patch.object(importlib.import_module("sce_author.compare"),
                               "MAX_TIME_STOPS", 5):
            behaviour = compare(self.drafts(beats=HEARTBEAT, settled=SETTLES),
                                drives=40, steps=12)["behaviour"]
        self.assertEqual("not judged", behaviour["verdict"])
        self.assertEqual(["beats.scxml"], list(behaviour["undriven"]))
        self.assertIn("more than 5 of its instants", behaviour["undriven"]["beats.scxml"])

    def test_one_draft_is_not_a_comparison(self):
        with self.assertRaises(CompareError):
            compare(self.drafts(a=RESTARTS))

    def test_a_remote_client_compares_drafts_handed_over_as_text(self):
        # The owner's assistant over HTTP has no path to give: each draft
        # arrives as text under the name the owner knows it by.
        from sce_author.mcp import call_tool

        answer = call_tool("compare", {"documents_text": [
            {"name": "first.scxml", "text": RESTARTS},
            {"name": "second.scxml", "text": IGNORES},
        ]}, remote=True)
        self.assertFalse(answer.get("isError"), answer)
        report = json.loads(answer["content"][0]["text"])
        self.assertEqual(report["documents"], ["first.scxml", "second.scxml"])
        self.assertEqual(report["behaviour"]["classes"], [["first.scxml"], ["second.scxml"]])
        self.assertIn("witness", report["summary"])


class TheReproducibilityCorpusIsWellFormed(unittest.TestCase):
    """The harness is run by hand, against a model; its corpus is not, and a
    corpus that names a case that does not exist would fail only after the
    first half hour of drafting."""

    def test_every_case_it_names_exists_and_states_its_kind(self):
        import sys
        eval_dir = pathlib.Path(__file__).resolve().parent.parent / "eval"
        sys.path.insert(0, str(eval_dir))
        try:
            import reproducibility
        finally:
            sys.path.remove(str(eval_dir))
        cases = reproducibility.load_cases()
        self.assertGreaterEqual(len(cases), 2)
        self.assertTrue(all(c.get("kind") and c.get("prose") for c in cases))


if __name__ == "__main__":
    unittest.main()
