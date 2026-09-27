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
    an answer arriving INSIDE the window is met at its first moment
                                                        (the discriminator)
    an answer never met in the window fails, read at its end
    read as the first announcement, the first round that WRITES an expected
    position answers: a held position announces when it is written
                                                        (the discriminator)
    and a window nothing writes in is a wait that went unanswered
    uncertainty CARRIES: a setup step's window widens every later reading,
    and a deadline the slack may put inside a window is not guessed at
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

    def test_an_answer_that_arrives_inside_the_window_is_met(self):
        """⚠ The discriminator. The window opens at 400 ms, when the signal is
        still dark, and the flashing it expects arrives at 500 -- inside. A
        harness that waits UP TO a timeout returns at that notification, so
        the record says FLASHING held at some moment in the window, and it
        did. Read as one unknown instant, the case could only be withheld."""
        result = self.run_cases([detected("FLASHING", {"min": 400, "max": 700})])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def declare_host(self):
        """The pack states what the host announces -- a first-announcement
        reading rests on it."""
        path = self.tmp / "conventions.yaml"
        conventions = yaml.safe_load(path.read_text(encoding="utf-8"))
        conventions["host"] = {"writes": "every-round"}
        path.write_text(yaml.safe_dump(conventions), encoding="utf-8")

    def test_a_first_announcement_reading_needs_the_host_stated(self):
        """Which positions a round announces is the host's behaviour; a pack
        that reads cases that way and does not say is refused, not guessed."""
        case = detected("DARK", {"min": 0, "max": 1000})
        case["observed"] = "first"
        result = self.run_cases([case])
        self.assertFalse(result.ran)
        self.assertIn("host.writes", result.refusal)

    def test_read_as_the_first_announcement_an_early_one_is_the_answer(self):
        """⚠ The discriminator for `observed: first`. The same record as the
        case above, read the way a harness that returns at its FIRST
        notification reads it: the host announces the dark signal in the
        drive's own round, at 0 ms, and that is the answer -- wrong in value
        and too early for a window that opens at 400. Read as 'any moment in
        the window', the same run passes."""
        self.declare_host()
        case = detected("FLASHING", {"min": 400, "max": 700})
        case["observed"] = "first"
        result = self.run_cases([case])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 1, 0), (result.passed, result.failed, result.unjudged),
                         details(result))
        failed = dict((a, (w, g)) for a, w, g in result.results[0].failures)
        self.assertEqual(("between 400 and 700 ms after the drive", "at 0 ms"),
                         failed["(first announcement)"])
        self.assertIn(SIGNAL, failed)

    def test_read_as_the_first_announcement_a_prompt_answer_passes(self):
        self.declare_host()
        case = detected("DARK", {"min": 0, "max": 1000})
        case["observed"] = "first"
        result = self.run_cases([case])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def hold_until_sent(self):
        """The signal's position is written only when the machine sends to
        it: a round that sends nothing hands the rule a value its map lacks,
        and a `hold_last` rule writes nothing then."""
        self.binding["outputs"] = {"roadSignal": {
            **BINDING["outputs"]["roadSignal"],
            "when_nothing_sent": "signal.unchanged", "hold_last": True}}

    def test_a_position_that_holds_is_first_announced_when_it_is_written(self):
        """⚠ The discriminator for WHICH round announces. Nothing writes the
        signal until the delayed send at 500 ms, so that is the first
        announcement, inside a window that opens at 400. Read as 'the drive's
        round announces', the same run failed as arriving at 0 ms -- and a
        document written to announce late could not be told from one that
        announced early."""
        self.declare_host()
        self.hold_until_sent()
        case = detected("FLASHING", {"min": 400, "max": 700})
        case["observed"] = "first"
        result = self.run_cases([case])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_a_held_position_written_before_the_window_arrived_early(self):
        self.declare_host()
        self.hold_until_sent()
        case = detected("FLASHING", {"min": 600, "max": 900})
        case["observed"] = "first"
        result = self.run_cases([case])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 1, 0), (result.passed, result.failed, result.unjudged),
                         details(result))
        failed = dict((a, (w, g)) for a, w, g in result.results[0].failures)
        self.assertEqual(("between 600 and 900 ms after the drive", "at 500 ms"),
                         failed["(first announcement)"])

    def test_a_wait_nothing_announces_in_fails(self):
        """A window that closes before the delayed send: nothing writes the
        position in it, so the harness waited out the window unanswered."""
        self.declare_host()
        self.hold_until_sent()
        case = detected("DARK", {"min": 0, "max": 300})
        case["observed"] = "first"
        result = self.run_cases([case])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 1, 0), (result.passed, result.failed, result.unjudged),
                         details(result))
        failed = dict((a, (w, g)) for a, w, g in result.results[0].failures)
        self.assertEqual(("between 0 and 300 ms after the drive", "none by 300 ms"),
                         failed["(first announcement)"])

    def test_an_answer_never_met_in_the_window_fails_at_its_end(self):
        """Nothing fires before 400 ms, so FLASHING is never met: the harness
        waited the whole window, and what it read at the end is the verdict."""
        result = self.run_cases([detected("FLASHING", {"min": 100, "max": 400})])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 1, 0), (result.passed, result.failed, result.unjudged),
                         details(result))

    def test_a_deadline_the_slack_may_put_inside_the_window_is_not_guessed(self):
        """The setup step leaves up to 300 ms of slack; the reading's window
        closes 100 ms before the deadline the engine sees. Whether it fell
        inside the window the record read in is not known."""
        result = self.run_cases([
            {"name": "detected, then occupied and read",
             "given": {APPROACH: "OCCUPIED"}, "drove": [APPROACH],
             "elapsed_ms": {"min": 100, "max": 200}, "expect": {SIGNAL: "FLASHING"},
             "before": [{"given": {APPROACH: "APPROACHING"}, "drove": [APPROACH],
                         "elapsed_ms": {"min": 0, "max": 300}}]},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 0, 1), (result.passed, result.failed, result.unjudged),
                         details(result))
        self.assertIn("whether it fell inside is not known", result.results[0].refusal)

    def test_a_setup_windows_uncertainty_carries_into_the_reading(self):
        """The setup step says only that 0 to 300 ms passed; the reading is
        300 ms after that. The engine stands at 300 and the deadline is 200
        ms on -- but real time may already be 600 ms in. An exact-number
        reading of the same record would have passed DARK."""
        result = self.run_cases([
            {"name": "detected, then occupied and read",
             "given": {APPROACH: "OCCUPIED"}, "drove": [APPROACH],
             "elapsed_ms": 300, "expect": {SIGNAL: "DARK"},
             "before": [{"given": {APPROACH: "APPROACHING"}, "drove": [APPROACH],
                         "elapsed_ms": {"min": 0, "max": 300}}]},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((0, 0, 1), (result.passed, result.failed, result.unjudged),
                         details(result))
        self.assertIn("not known", result.results[0].refusal)

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
