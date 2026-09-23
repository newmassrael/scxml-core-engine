"""A transition written on one region can restart every region.

W3C SCXML 3.13 takes a transition's domain from the nearest proper ancestor
that is a compound state or the `<scxml>` element. A `<parallel>` is neither,
so it is stepped over: a transition written on a region -- or from one region
into another -- exits the whole `<parallel>` and enters it again, and every
OTHER region restarts at its initial state. The document reads "this latch
forgets"; the machine does "every latch forgets".

⚠ THE POINT IS THAT NOTHING ELSE SAYS SO. The document is valid SCXML, it
generates on every backend, and a platform's tests pass it whenever they never
move one region's input between another region's edges. Measured 2026-09-23 on
one component: a model-written document reset all four latches on any latch's
ERROR and failed a shipped case; the reference document, which passed every
shipped case, reset them on any change of a fault input -- and its author had
already written this very trap down as a lesson.

So `check` refuses it, and says the two ways out: `type="internal"` when only
the region should restart (the source is then the domain), or target the
`<parallel>` itself when all of it should.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import check, read_document
from sce_author.pack import load_pack

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

HEAD = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="watch" sce:kind="statechart">
"""


def document(body: str) -> str:
    return HEAD + body + "\n</scxml>\n"


# Two regions. The approach region forgets what it saw when its detector
# faults -- written on the REGION, which is the trap.
ON_THE_REGION = document("""
  <parallel id="watch">
    <state id="approach" initial="unseen">
      <transition event="obstacle.fault" target="unseen"/>
      <state id="unseen">
        <transition event="train.approaching" target="seen"/>
      </state>
      <state id="seen">
        <transition event="train.cleared" target="unseen">
          <send event="signal.dark" type="x-sce-host"/>
        </transition>
      </state>
    </state>
    <state id="barrier" initial="up">
      <state id="up"><transition event="barrier.down" target="down"/></state>
      <state id="down"><transition event="barrier.up" target="up"/></state>
    </state>
  </parallel>""")


class WhatTheDocumentSays(unittest.TestCase):
    """The detector, read off the document alone."""

    def reentries(self, text: str):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "d.scxml"
            path.write_text(text, encoding="utf-8")
            return [(s, p, below) for s, _, _, p, below, _ in read_document(path).reentries]

    def restarted(self, text: str):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "d.scxml"
            path.write_text(text, encoding="utf-8")
            return [r for *_, r in read_document(path).reentries]

    def test_a_transition_on_a_region_restarts_the_parallel(self):
        self.assertEqual([("approach", "watch", True)], self.reentries(ON_THE_REGION))
        self.assertEqual([("barrier",)], self.restarted(ON_THE_REGION),
                         "the region that restarts without being named")

    def test_a_joint_move_that_names_every_region_restarts_none(self):
        """Measured 2026-09-23: a document moved two latches together with one
        transition naming a state in each, under a compound state that kept
        the domain there. The pair is exited and re-entered, and every region
        of it lands where the transition says -- nothing restarts unasked."""
        joint = document("""
  <parallel id="watch">
    <state id="pair" initial="both">
      <parallel id="both">
        <state id="left" initial="l_up">
          <state id="l_up">
            <transition event="train.approaching" target="l_down r_down"/>
          </state>
          <state id="l_down"/>
        </state>
        <state id="right" initial="r_up">
          <state id="r_up"/>
          <state id="r_down"/>
        </state>
      </parallel>
    </state>
    <state id="other" initial="o"><state id="o"/></state>
  </parallel>""")
        self.assertEqual([], self.reentries(joint))

    def test_internal_restarts_only_the_region(self):
        """The discriminator: one attribute, and the source is the domain."""
        internal = ON_THE_REGION.replace(
            'event="obstacle.fault" target="unseen"',
            'event="obstacle.fault" target="unseen" type="internal"')
        self.assertEqual([], self.reentries(internal))

    def test_a_transition_inside_one_region_stays_in_it(self):
        """Between two states of one region the region is the domain."""
        inside = ON_THE_REGION.replace(
            '<transition event="obstacle.fault" target="unseen"/>', "")
        self.assertEqual([], self.reentries(inside))

    def test_targeting_the_parallel_is_a_restart_that_says_so(self):
        says_so = ON_THE_REGION.replace(
            'event="obstacle.fault" target="unseen"', 'event="obstacle.fault" target="watch"')
        self.assertEqual([], self.reentries(says_so))

    def test_a_transition_between_regions_restarts_them_all(self):
        across = ON_THE_REGION.replace(
            '<transition event="obstacle.fault" target="unseen"/>', "").replace(
            '<state id="up"><transition event="barrier.down" target="down"/></state>',
            '<state id="up"><transition event="barrier.down" target="seen"/></state>')
        self.assertEqual([("up", "watch", False)], self.reentries(across))

    def test_nested_parallels_are_reported_once_at_the_widest(self):
        nested = document("""
  <parallel id="outer">
    <parallel id="inner">
      <state id="left" initial="a">
        <transition event="obstacle.fault" target="a"/>
        <state id="a"><transition event="train.approaching" target="b"/></state>
        <state id="b"/>
      </state>
      <state id="right" initial="c"><state id="c"/></state>
    </parallel>
    <state id="other" initial="d"><state id="d"/></state>
  </parallel>""").replace('initial="watch"', 'initial="outer"')
        self.assertEqual([("left", "outer", True)], self.reentries(nested))


class WhatCheckSays(unittest.TestCase):
    """And `check` refuses it, naming both ways out."""

    def test_the_refusal_names_the_parallel_and_the_fix(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            for name in ("interface-model.yaml", "conventions.yaml"):
                shutil.copy(CROSSING / name, root / name)
            (root / "d.scxml").write_text(ON_THE_REGION, encoding="utf-8")
            binding = root / "b.yaml"
            binding.write_text(yaml.safe_dump({
                "version": 1, "document": "d.scxml",
                "inputs": {
                    "approaching": {"address": "plant/in/train-approach",
                                    "becomes": "APPROACHING", "event": "train.approaching"},
                    "cleared": {"address": "plant/in/train-approach",
                                "becomes": "CLEAR", "event": "train.cleared"},
                    "fault": {"address": "plant/in/obstacle",
                              "becomes": "FAULT", "event": "obstacle.fault"},
                },
                "outputs": {"roadSignal": {
                    "address": "plant/out/road-signal", "field": "value",
                    "sent": {"processor": "x-sce-host"},
                    "when_nothing_sent": "signal.dark",
                    "map": {"signal.dark": "DARK"}}},
            }), encoding="utf-8")
            found = [f for f in check(load_pack(root), binding)
                     if "re-enters" in f.detail]
        self.assertEqual(1, len(found), found)
        self.assertEqual("state approach", found[0].where)
        for words in ("<parallel id='watch'>", "'barrier' restarts", 'type="internal"',
                      "target 'watch' itself"):
            self.assertIn(words, found[0].detail)


if __name__ == "__main__":
    unittest.main()
