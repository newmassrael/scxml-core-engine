"""A time a record knows only as a window judges what the window cannot move.

A harness that waits "up to" a timeout, or whose steps last as long as the
platform takes, does not know the instant it observed -- and a document with
a pending delay answers differently on either side of its deadline. Measured
2026-09-27: fourteen cases of one component carried no time at all and could
not be judged; one number chosen for them would have judged them against a
moment no run is known to have reached. `elapsed_ms: {min, max}` states what
IS known, and `verify` judges every case whose answer is the same anywhere in
it.

Asserted here, against a signal that starts flashing 500 ms after a train is
detected:

    a window wholly past the deadline sees it fired; wholly before, not
    a window the deadline falls inside gives up the run from there
                                                        (the discriminator)
    uncertainty CARRIES: a setup step's window widens every later reading
    nothing pending, nothing carried: the slack resets
    a clock input is not handed one end of a window
    a window whose min is above its max is refused where it is read
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.errors import PackError
from sce_author.pack import load_pack
from sce_author.verify import verify
from tests.test_a_statechart_is_driven_not_called import (
    BINDING, CROSSING, DELAYED, EXAMPLES, codegen_is_built)

APPROACH = "plant/in/train-approach"
SIGNAL = "plant/out/road-signal.value"


def detected(expect: str | None = None, elapsed=None, **extra) -> dict:
    case = {"name": "a train is detected", "given": {APPROACH: "APPROACHING"},
            "drove": [APPROACH], **extra}
    if elapsed is not None:
        case["elapsed_ms"] = elapsed
    if expect is not None:
        case["expect"] = {SIGNAL: expect}
    return case


def details(result):
    return [(r.name, r.refusal, r.failures) for r in result.results]


@unittest.skipUnless(codegen_is_built(), "the product's generator is not built")
class ATimeKnownOnlyAsAWindow(unittest.TestCase):
    OCCUPIED = {"address": APPROACH, "becomes": "OCCUPIED", "event": "train.occupied"}

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(DELAYED, encoding="utf-8")
        self.binding = {**BINDING, "activation": "on-change",
                        "inputs": {**BINDING["inputs"], "occupied": self.OCCUPIED}}

    def run_cases(self, cases):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump({**EXAMPLES, "cases": cases}), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(self.binding), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    def test_a_window_wholly_past_the_deadline_sees_it_fired(self):
        result = self.run_cases([detected("FLASHING", {"min": 600, "max": 900})])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_a_window_wholly_before_the_deadline_sees_it_pending(self):
        result = self.run_cases([detected("DARK", {"min": 100, "max": 400})])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_a_deadline_inside_the_window_gives_up_the_run_from_there(self):
        """⚠ The discriminator. Read at its earliest end the window says
        DARK and at its latest FLASHING; neither is the record's, so the case
        is withheld -- and the next one too, whose machine is one of two."""
        result = self.run_cases([
            detected("DARK", {"min": 400, "max": 700}),
            {"name": "then it clears", "given": {APPROACH: "CLEAR"},
             "drove": [APPROACH], "elapsed_ms": 50,
             "expect": {SIGNAL: "DARK"}},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 0, 2), (result.passed, result.failed, result.unjudged),
                         details(result))
        self.assertIn("whether it fired is not known", result.results[0].refusal)

    def test_a_setup_windows_uncertainty_carries_into_the_reading(self):
        """The setup step says only that 0 to 300 ms passed; the reading is
        300 ms after that. The engine stands at 300 and the deadline is 200
        ms on -- but real time may already be 600 ms in. An exact-number
        reading of the same record would have passed DARK."""
        result = self.run_cases([
            {"name": "detected, then read",
             "given": {APPROACH: "APPROACHING"}, "drove": [APPROACH],
             "elapsed_ms": 300, "expect": {SIGNAL: "DARK"},
             "before": [{"given": {APPROACH: "APPROACHING"}, "drove": [APPROACH],
                         "elapsed_ms": {"min": 0, "max": 300}}]},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 0, 1), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_with_nothing_pending_the_slack_resets(self):
        """A wide window over a machine waiting on nothing moves nothing, and
        the next delay starts from a known clock again."""
        result = self.run_cases([
            {"name": "clear for a while", "given": {APPROACH: "CLEAR"},
             "drove": [APPROACH], "elapsed_ms": {"min": 0, "max": 5000},
             "expect": {SIGNAL: "DARK"}},
            detected("FLASHING", 600),
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((2, 0, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_a_window_whose_min_is_above_its_max_is_refused(self):
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump(
            {**EXAMPLES, "cases": [detected("DARK", {"min": 700, "max": 400})]}),
            encoding="utf-8")
        with self.assertRaises(PackError) as caught:
            load_pack(self.tmp)
        self.assertIn("min 700", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
