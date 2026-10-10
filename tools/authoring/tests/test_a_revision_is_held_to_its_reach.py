"""A revision is held to the reach of what changed by a loop its caller runs.

`scxml_revision_check` is a report: nothing refuses a design for it, and the model that revises runs
in the owner's client. What can be held is the RESULT, so `revision_gate.hold` judges, hands what is
out of reach back to the reviser, judges again, and says `within-reach` only on the judgment of the
design as it stands (`docs/adr/0013-a-revision-is-held-to-its-reach-by-a-loop-its-caller-runs.md`).

The loop is held first on judgments made by `revision.join` and revisers that are functions, so that
every way it can end is reached on purpose; then on the committed design and the product's own
generator, with the command line a client runner would be started by.
"""

from __future__ import annotations

import json
import os
import pathlib
import shlex
import shutil
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

from sce_author import revision, revision_gate
from sce_author.__main__ import main
from sce_author.mcp import call_tool
from sce_author.revision_gate import (GateError, Outcome, ReviserError, command_reviser, guarded,
                                      hold, judge, keep_baseline)
from sce_author.verify import _default_codegen
from tests.test_a_revised_design_is_checked_against_what_the_specification_changed import (
    DOCUMENT, EVENT, FIXTURES, MANIFEST, PROSE, body, fixture_sidecar_text, words)


def judged(moved: dict[str, list[str]] | None = None, *, retired_cited: tuple[str, ...] = (),
           carried: tuple[str, ...] = ("R1", "R2", "R3"), seen: bool = True) -> dict:
    """A judgment as `revision.join` makes it: every id in `carried` carries over unless `moved`
    names the places it moved at (then it is `moved-without-reason`), and the ids in `retired_cited`
    were dropped by the specification and are still cited."""
    lines = []
    for id_ in carried if seen else ():
        if moved and id_ in moved:
            lines.append({"requirement": id_, "evidence": "changed", "moved": moved[id_], "gone": 1})
        else:
            lines.append({"requirement": id_, "evidence": "unchanged"})
    for id_ in retired_cited:
        lines.append({"requirement": id_, "evidence": "changed", "moved": [f"states.{id_}"], "gone": 0})
    return revision.join(
        {"requirements": {"carried": list(carried), "changed": [], "new": [],
                          "retired": list(retired_cited)}},
        {"requirements": lines, "unclaimed": {"added": [], "gone": 0}})


class Script:
    """The judgments a gate will be given, one per call, and a count of how many it asked for."""

    def __init__(self, *judgments: dict | tuple[None, str]):
        self.judgments = list(judgments)
        self.asked = 0

    def __call__(self) -> tuple[dict | None, str]:
        self.asked += 1
        held = self.judgments.pop(0)
        return held if isinstance(held, tuple) else (held, "")


class Reviser:
    """A reviser that only records what it was asked, and does nothing else."""

    def __init__(self, fails_with: str | None = None):
        self.requests: list[str] = []
        self.fails_with = fails_with

    def __call__(self, request: str) -> None:
        self.requests.append(request)
        if self.fails_with is not None:
            raise ReviserError(self.fails_with)


class ARevisionIsHeldToItsReach(unittest.TestCase):
    def test_a_revision_already_within_reach_is_not_revised(self):
        reviser, script = Reviser(), Script(judged())
        outcome = hold(script, reviser)
        self.assertEqual(revision_gate.WITHIN_REACH, outcome.status)
        self.assertTrue(outcome.within_reach)
        self.assertEqual((0, 1, []), (outcome.revisions, script.asked, reviser.requests))

    def test_a_violation_is_handed_back_and_the_design_judged_again(self):
        reviser = Reviser()
        script = Script(judged({"R2": ["states.a.transitions[0]"]}), judged())
        outcome = hold(script, reviser)
        self.assertEqual(revision_gate.WITHIN_REACH, outcome.status)
        self.assertEqual((1, 2), (outcome.revisions, script.asked))
        self.assertEqual(1, len(reviser.requests))
        request = reviser.requests[0]
        self.assertIn("R2 (moved-without-reason): moved at states.a.transitions[0]", request)
        self.assertIn("put back what the design had at these places", request)
        self.assertIn("attempt 1 of 3", request)
        self.assertIn("# Revision report", request, "the page the owner reads is what the reviser reads")
        self.assertNotIn("R1 (", request, "a requirement that carries over is not listed to change")

    def test_a_requirement_the_specification_dropped_is_told_to_lose_its_citations(self):
        reviser = Reviser()
        hold(Script(judged(retired_cited=("R9",)), judged()), reviser)
        self.assertIn("R9 (retired-still-cited)", reviser.requests[0])
        self.assertIn("remove its id from every `sce:req`", reviser.requests[0])

    def test_progress_over_several_rounds_is_not_a_stall(self):
        # The same requirement, the same kind, fewer places each time: the reviser is getting there.
        two = ["states.a", "states.b"]
        script = Script(judged({"R2": two}), judged({"R2": two[:1]}), judged())
        outcome = hold(script, Reviser())
        self.assertEqual(revision_gate.WITHIN_REACH, outcome.status)
        self.assertEqual(2, outcome.revisions)

    def test_a_revision_that_changes_nothing_stalls_and_is_not_asked_again(self):
        reviser = Reviser()
        stuck = judged({"R2": ["states.a"]})
        outcome = hold(Script(stuck, stuck), reviser, rounds=5)
        self.assertEqual(revision_gate.STALLED, outcome.status)
        self.assertFalse(outcome.within_reach)
        self.assertEqual(1, len(reviser.requests), "asked once, then the same answer ended it")
        self.assertIn("R2 (moved-without-reason)", outcome.reason)

    def test_the_rounds_are_bounded_and_the_last_judgment_is_the_outcome(self):
        reviser = Reviser()
        # A different place each time, so it is never a stall: only the rounds end it.
        script = Script(*(judged({"R2": [f"states.s{n}"]}) for n in range(3)))
        outcome = hold(script, reviser, rounds=2)
        self.assertEqual(revision_gate.OUTSIDE_REACH, outcome.status)
        self.assertEqual((2, 3, 2), (outcome.revisions, script.asked, len(reviser.requests)))
        self.assertEqual("outside-reach", outcome.result["verdict"])
        self.assertIn("after 2 revision(s)", outcome.reason)

    def test_no_rounds_judges_once_and_never_asks(self):
        reviser = Reviser()
        outcome = hold(Script(judged({"R2": ["states.a"]})), reviser, rounds=0)
        self.assertEqual(revision_gate.OUTSIDE_REACH, outcome.status)
        self.assertEqual([], reviser.requests)

    def test_a_verdict_that_saw_nothing_is_not_a_pass(self):
        # Every requirement uncited: no violation, so the verdict reads `within-reach`, and it
        # compared nothing. A gate that took it for a pass would call an unchecked design checked.
        nothing = judged(seen=False)
        self.assertEqual(("within-reach", 0), (nothing["verdict"], nothing["summary"]["seen"]))
        reviser = Reviser()
        outcome = hold(Script(nothing), reviser)
        self.assertEqual(revision_gate.NOT_JUDGED, outcome.status)
        self.assertFalse(outcome.within_reach)
        self.assertIn("compared nothing", outcome.reason)
        self.assertEqual([], reviser.requests)

    def test_a_judgment_the_product_refused_is_not_judged_and_says_why(self):
        reviser = Reviser()
        outcome = hold(Script((None, "the design cannot be read")), reviser)
        self.assertEqual((revision_gate.NOT_JUDGED, "the design cannot be read", None),
                         (outcome.status, outcome.reason, outcome.result))
        self.assertEqual([], reviser.requests)

    def test_a_judgment_that_stops_being_possible_after_a_revision_is_not_judged(self):
        outcome = hold(Script(judged({"R2": ["states.a"]}), (None, "the design cannot be read")),
                       Reviser())
        self.assertEqual(revision_gate.NOT_JUDGED, outcome.status)
        self.assertEqual(1, outcome.revisions)

    def test_a_reviser_that_fails_ends_the_gate_with_its_own_words(self):
        script = Script(judged({"R2": ["states.a"]}), judged())
        outcome = hold(script, Reviser(fails_with="the reviser exited with status 3: boom"))
        self.assertEqual((revision_gate.REVISER_FAILED, "the reviser exited with status 3: boom"),
                         (outcome.status, outcome.reason))
        self.assertEqual(1, script.asked, "a design nobody revised is not judged again as if revised")
        self.assertEqual(1, outcome.revisions, "the call that failed was a call")

    def test_a_number_of_rounds_that_is_not_one_is_refused(self):
        for rounds in (-1, revision_gate.MAX_ROUNDS + 1, True, "3", 2.5, None):
            with self.subTest(rounds=rounds), self.assertRaises(GateError):
                hold(Script(judged()), Reviser(), rounds=rounds)

    def test_an_outcome_says_in_json_how_it_ended(self):
        outcome = hold(Script(judged({"R2": ["states.a"]}), judged()), Reviser())
        shown = outcome.as_json()
        self.assertEqual(("within-reach", True), (shown["status"], shown["within_reach"]))
        self.assertEqual([{"round": 0, "verdict": "outside-reach", "seen": 3,
                           "violations": [{"requirement": "R2", "kind": "moved-without-reason",
                                           "moved": ["states.a"]}]},
                          {"round": 1, "verdict": "within-reach", "seen": 3, "violations": []}],
                         shown["rounds"])
        self.assertIsInstance(outcome, Outcome)
        json.dumps(shown)


class ACommandIsAReviser(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.design = pathlib.Path(temporary.name) / "design.scxml"
        self.design.write_text("before\n", encoding="utf-8")

    def command(self, code: str) -> str:
        return shlex.join([sys.executable, "-c", code])

    def test_it_is_told_of_the_design_and_of_the_request(self):
        code = ("import os, pathlib; d = pathlib.Path(os.environ['SCE_REVISION_DESIGN']);"
                "d.write_text('asked: ' + pathlib.Path(os.environ['SCE_REVISION_REQUEST']).read_text())")
        command_reviser(self.command(code), self.design)("put R2 back\n")
        self.assertEqual("asked: put R2 back\n", self.design.read_text(encoding="utf-8"))

    def test_a_command_that_exits_non_zero_has_failed_its_round_and_its_words_are_kept(self):
        code = "import sys; print('boom', file=sys.stderr); sys.exit(3)"
        with self.assertRaises(ReviserError) as caught:
            command_reviser(self.command(code), self.design)("x")
        self.assertIn("status 3", str(caught.exception))
        self.assertIn("boom", str(caught.exception))

    def test_a_command_that_cannot_be_started_has_failed_its_round(self):
        with self.assertRaises(ReviserError) as caught:
            command_reviser("/nonexistent/reviser-binary", self.design)("x")
        self.assertIn("could not be started", str(caught.exception))

    def test_a_command_that_runs_too_long_is_stopped(self):
        with self.assertRaises(ReviserError) as caught:
            command_reviser(self.command("import time; time.sleep(30)"), self.design, timeout_s=1)("x")
        self.assertIn("ran past 1 seconds", str(caught.exception))

    def test_a_timeout_stops_what_the_command_started_and_not_only_the_command(self):
        # A client runner starts programs. `subprocess.run` kills the command on a timeout and
        # leaves what it started running, and one of those wrote to the design after the gate had
        # said the round was over (a review of the gate, 2026-10-09).
        late = ("import os, pathlib, time; time.sleep(2); "
                "pathlib.Path(os.environ['SCE_REVISION_DESIGN']).write_text('written after the timeout')")
        code = ("import subprocess, sys, time; "
                f"subprocess.Popen([sys.executable, '-c', {late!r}]); time.sleep(60)")
        with self.assertRaises(ReviserError):
            command_reviser(self.command(code), self.design, timeout_s=1)("x")
        time.sleep(3.5)
        self.assertEqual("before\n", self.design.read_text(encoding="utf-8"),
                         "a process the reviser started went on running after the timeout")

    def test_a_host_that_cannot_stop_a_group_does_not_start_the_command_and_says_so(self):
        # A host with neither a POSIX group to signal nor Windows' tree to end: a command started
        # there could not be stopped with what it started, and its first timeout would be an
        # AttributeError in place of the round's failure (a review of the gate, 2026-10-09). The
        # marker proves nothing was started.
        marker = self.design.parent / "started"
        code = f"import pathlib; pathlib.Path({str(marker)!r}).write_text('started')"
        self.addCleanup(setattr, os, "killpg", os.killpg)
        del os.killpg
        with self.assertRaises(ReviserError) as caught:
            command_reviser(self.command(code), self.design)("x")
        self.assertIn("could not be started", str(caught.exception))
        self.assertIn("process group", str(caught.exception))
        self.assertFalse(marker.exists(), "the command ran on a host that cannot stop its group")

    def test_on_windows_the_command_is_started_in_a_group_of_its_own_and_its_tree_is_ended_with_taskkill(self):
        # What this holds is the command that is issued, not that Windows ends a tree with it: it was
        # written from what `taskkill` documents and has not been run there.
        from sce_author import process

        self.assertEqual({"creationflags": 0x00000200}, process._own_group_options(True))
        self.assertEqual({"start_new_session": True}, process._own_group_options(False))

        class Child:
            returncode = None
            pid = 4242
            stdin = stdout = stderr = None
            waited = False

            def wait(self, timeout=None):
                self.waited = True

            def kill(self):
                raise AssertionError("taskkill was there to end the tree")

        issued = []
        child = Child()
        with mock.patch.object(process.subprocess, "run", lambda argv, **kw: issued.append(argv)):
            process._end_group(child, True)
        self.assertEqual([["taskkill", "/F", "/T", "/PID", "4242"]], issued)
        self.assertTrue(child.waited)

        # A program already collected is not signalled: its number belongs to nobody now.
        issued.clear()
        child.returncode = 0
        with mock.patch.object(process.subprocess, "run", lambda argv, **kw: issued.append(argv)):
            process._end_group(child, True)
        self.assertEqual([], issued)

    def test_when_taskkill_cannot_be_run_the_command_itself_is_killed(self):
        from sce_author import process

        class Child:
            returncode = None
            pid = 7
            stdin = stdout = stderr = None
            killed = False

            def wait(self, timeout=None):
                pass

            def kill(self):
                self.killed = True

        def missing(argv, **kw):
            raise FileNotFoundError(argv[0])

        child = Child()
        with mock.patch.object(process.subprocess, "run", missing):
            process._end_group(child, True)
        self.assertTrue(child.killed)

    def test_a_taskkill_that_hangs_is_given_a_time_and_then_the_command_itself_is_killed(self):
        from sce_author import process

        class Child:
            returncode = None
            pid = 9
            stdin = stdout = stderr = None
            killed = False

            def wait(self, timeout=None):
                pass

            def kill(self):
                self.killed = True

        seen = {}

        def hung(argv, **kw):
            seen.update(kw)
            raise process.subprocess.TimeoutExpired(argv, kw["timeout"])

        child = Child()
        with mock.patch.object(process.subprocess, "run", hung):
            process._end_group(child, True)
        self.assertEqual(process._TASKKILL_WITHIN, seen.get("timeout"))
        self.assertTrue(child.killed)

    def _alive(self, process, ends_on_kill):
        """A program `taskkill` did not end: it lives until its own kill, if that ends it, and a wait
        with no time never returns for it (here that is an error, not a hang)."""

        class Alive:
            returncode = None
            pid = 11
            stdin = stdout = stderr = None
            killed = False
            ended = False

            def wait(self, timeout=None):
                if self.ended:
                    return 0
                if timeout is None:
                    raise AssertionError("waited without a time for a program that is still running")
                raise process.subprocess.TimeoutExpired("program", timeout)

            def kill(self):
                self.killed = True
                self.ended = ends_on_kill

        return Alive()

    def test_a_taskkill_that_answers_with_a_failure_while_the_command_lives_is_not_waited_on(self):
        # `taskkill` can refuse (access denied) and still return: its answer is read, not only whether
        # it could be run, and the command is asked about itself, which is the evidence.
        from sce_author import process

        child = self._alive(process, ends_on_kill=True)

        def refused(argv, **kw):
            return process.subprocess.CompletedProcess(argv, 1, b"", b"ERROR: Access is denied.")

        with mock.patch.object(process.subprocess, "run", refused):
            process._end_group(child, True)
        self.assertTrue(child.killed)

    def test_a_command_that_cannot_be_ended_is_said_so_and_not_waited_on(self):
        from sce_author import process

        child = self._alive(process, ends_on_kill=False)

        def refused(argv, **kw):
            return process.subprocess.CompletedProcess(argv, 1, b"", b"")

        with mock.patch.object(process.subprocess, "run", refused):
            with self.assertRaises(OSError) as caught:
                process._end_group(child, True)
        self.assertIn("11", str(caught.exception))
        self.assertTrue(child.killed)

    def _held_pipe(self):
        """An output pipe that cannot be closed while something still holds it, as a reader thread of
        Windows' `communicate` holds it while a child that outlived its parent keeps the other end open.
        Its close waits for a release that never comes (here: until the test ends)."""
        release = threading.Event()
        self.addCleanup(release.set)

        class Held:
            closed = False

            def close(self):
                release.wait(30)
                self.closed = True

        return Held()

    def test_a_pipe_a_child_of_the_command_still_holds_is_not_waited_on_when_the_command_is_ended(self):
        from sce_author import process

        class Child:
            returncode = 0
            pid = 12
            stdin = None
            stderr = None
            waited = False

            def wait(self, timeout=None):
                self.waited = True

        child = Child()
        child.stdout = self._held_pipe()
        started = time.monotonic()
        with mock.patch.object(process, "_CLOSE_WITHIN", 0.3):
            left = process._end_group(child, False)
        self.assertLess(time.monotonic() - started, 5.0, "ending the command waited on a pipe still held")
        self.assertEqual(["stdout"], left)
        self.assertTrue(child.waited)

    def test_the_round_still_fails_for_its_clock_and_says_which_pipe_was_left(self):
        from sce_author import process

        class Child:
            returncode = None
            pid = 13
            stdin = stderr = None

            def communicate(self, input=None, timeout=None):
                raise process.subprocess.TimeoutExpired("program", timeout)

            def wait(self, timeout=None):
                return 0

        child = Child()
        child.stdout = self._held_pipe()
        with mock.patch.object(process.subprocess, "Popen", lambda *a, **k: child), \
                mock.patch.object(process.os, "killpg", lambda *a: None), \
                mock.patch.object(process, "_CLOSE_WITHIN", 0.3):
            with self.assertRaises(process.ProcessTimeout) as caught:
                process._run_in_own_group(["x"], None, 1.0, None, None)
        self.assertIn("had not finished", str(caught.exception))
        self.assertIn("stdout", str(caught.exception))

    def test_a_command_that_finishes_is_read_as_before_when_it_has_a_session_of_its_own(self):
        code = "import sys; print('said'); print('and', file=sys.stderr); sys.exit(0)"
        command_reviser(self.command(code), self.design)("x")   # no error: exit 0 is a finished round

    def test_an_empty_command_is_refused_before_anything_runs(self):
        for command in ("", "   "):
            with self.subTest(command=command), self.assertRaises(GateError):
                command_reviser(command, self.design)


@unittest.skipUnless(_default_codegen().exists(),
                     "the record is the product's; build sce-codegen first")
class ARevisedCommittedDesignIsHeldToItsReach(unittest.TestCase):
    """The same loop on the committed design, judged by the product's own comparison."""

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name) / "tree"
        (self.root / "spec").mkdir(parents=True)
        (self.root / "design").mkdir()
        shutil.copy(FIXTURES / MANIFEST, self.root / "spec" / MANIFEST)
        self.design = self.root / "design" / DOCUMENT
        shutil.copy(FIXTURES / DOCUMENT, self.design)
        self.original = self.design.read_text(encoding="utf-8")
        self.accepted_copy = pathlib.Path(temporary.name) / "accepted.scxml"
        self.accepted_copy.write_text(self.original, encoding="utf-8")
        prose = self.root / "spec" / "prose.md"
        prose.write_text(PROSE, encoding="utf-8")
        sidecar = self.root / "spec" / "requirements.sidecar.json"
        sidecar.write_text(fixture_sidecar_text(), encoding="utf-8")
        self.record = self.root / "acceptance.json"
        accepted = call_tool("scxml_accept", {
            "document": str(self.design), "manifest": str(self.root / "spec" / MANIFEST),
            "sidecar": str(sidecar), "variant": "base", "root": str(self.root),
            "out": str(self.record), "sources": [str(prose)]})
        self.assertFalse(accepted.get("isError"), accepted)
        evidence = call_tool("scxml_acceptance_delta", {"record": str(self.record),
                                                         "root": str(self.root)})
        self.ids = [r["requirement"] for r in body(evidence)["requirements"]]
        # The revision that went wrong: a transition a requirement cites is renamed, though the
        # words of every requirement are carried.
        self.assertEqual(1, self.original.count(EVENT), "the fixture changed under the test")
        self.design.write_text(self.original.replace(EVENT, 'event="tcp_up"'), encoding="utf-8")
        self.delta = words(carried=self.ids)

    def judging(self):
        return judge(self.delta, self.record, self.root, self.design)

    def restoring(self, request: str) -> None:
        self.design.write_text(self.accepted_copy.read_text(encoding="utf-8"), encoding="utf-8")

    def test_the_judgment_is_the_one_the_tool_gives(self):
        result, refusal = self.judging()
        self.assertEqual("", refusal)
        shown = call_tool("scxml_revision_check", {"delta": self.delta, "record": str(self.record),
                                                    "root": str(self.root)})
        self.assertEqual(body(shown), result)
        self.assertEqual("outside-reach", result["verdict"])

    def test_a_delta_of_another_list_is_refused_and_not_judged_as_this_ones(self):
        # The join is by requirement id, and an id means nothing outside the list that issued it:
        # a delta of another list that happens to name the same ids would join this design's
        # evidence and give a verdict about nothing. The gate refuses it, as the tool does.
        other = {**self.delta, "from_manifest_sha256": "0" * 64}
        with self.assertRaises(revision.RevisionError) as caught:
            judge(other, self.record, self.root, self.design)
        self.assertIn("not the same list", str(caught.exception))

    def forged_record(self) -> pathlib.Path:
        """The acceptance record of the design as it is NOW (broken): a baseline a reviser would
        like the gate to judge against, because against it the broken design has not moved."""
        forged = self.accepted_copy.parent / "forged-acceptance.json"
        made = call_tool("scxml_accept", {
            "document": str(self.design), "manifest": str(self.root / "spec" / MANIFEST),
            "sidecar": str(self.root / "spec" / "requirements.sidecar.json"), "variant": "base",
            "root": str(self.root), "out": str(forged),
            "sources": [str(self.root / "spec" / "prose.md")]})
        self.assertFalse(made.get("isError"), made)
        return forged

    def kept(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        return keep_baseline(self.record, pathlib.Path(directory.name))

    def test_the_baseline_is_a_private_read_only_copy_of_the_bytes_that_were_hashed(self):
        baseline = self.kept()
        self.assertEqual(self.record.read_bytes(), baseline.copy.read_bytes())
        self.assertNotEqual(self.record, baseline.copy)
        self.assertFalse(baseline.copy.stat().st_mode & 0o222, "the copy can be written to")
        self.assertEqual("", baseline.moved())

    def test_a_reviser_that_replaces_the_record_has_failed_its_round_and_no_design_passes(self):
        # Before the fix the gate said `within-reach` here: the design was left as broken as it was
        # and the record it is judged against was replaced by the record of that broken design.
        forged, baseline = self.forged_record(), self.kept()

        def replaces_the_baseline(request: str) -> None:
            shutil.copy(forged, self.record)

        outcome = hold(lambda: judge(self.delta, baseline.copy, self.root, self.design),
                       guarded(replaces_the_baseline, baseline))
        self.assertEqual(revision_gate.REVISER_FAILED, outcome.status)
        self.assertFalse(outcome.within_reach)
        self.assertIn("changed the acceptance record", outcome.reason)
        self.assertIn(str(self.record), outcome.reason)

    def test_a_record_replaced_after_it_was_read_is_not_what_is_judged_against(self):
        # Without the guard at all: the judgment reads the copy, so replacing the source moves
        # nothing, and the broken design is still outside its reach.
        forged, baseline = self.forged_record(), self.kept()
        shutil.copy(forged, self.record)
        result, refusal = judge(self.delta, baseline.copy, self.root, self.design)
        self.assertEqual(("", "outside-reach"), (refusal, result["verdict"]))
        self.assertNotEqual("", baseline.moved())

    def test_a_record_the_reviser_removed_is_a_record_that_moved(self):
        baseline = self.kept()
        self.record.unlink()
        self.assertIn("cannot be read after the reviser ran", baseline.moved())

    def test_a_reviser_that_leaves_the_record_alone_is_not_failed_for_it(self):
        baseline = self.kept()
        outcome = hold(lambda: judge(self.delta, baseline.copy, self.root, self.design),
                       guarded(self.restoring, baseline))
        self.assertEqual(revision_gate.WITHIN_REACH, outcome.status)

    def test_a_design_put_back_by_the_reviser_is_within_reach_after_one_revision(self):
        requests: list[str] = []

        def revise(request: str) -> None:
            requests.append(request)
            self.restoring(request)

        outcome = hold(self.judging, revise)
        self.assertEqual((revision_gate.WITHIN_REACH, 1), (outcome.status, outcome.revisions))
        self.assertIn("3.DoIP-127 (moved-without-reason): moved at", requests[0])
        self.assertIn("transitions[", requests[0], "the product's own places are what the reviser is told")
        self.assertEqual(self.original, self.design.read_text(encoding="utf-8"))

    def test_a_reviser_that_only_says_it_is_done_is_judged_by_the_design_it_left(self):
        outcome = hold(self.judging, lambda request: print("done: within reach"))
        self.assertEqual(revision_gate.STALLED, outcome.status)
        self.assertEqual("outside-reach", outcome.result["verdict"])

    def test_a_design_the_product_cannot_read_is_not_judged(self):
        outcome = hold(lambda: judge(self.delta, self.record, self.root,
                                     self.root / "design" / "missing.scxml"), self.restoring)
        self.assertEqual(revision_gate.NOT_JUDGED, outcome.status)
        self.assertTrue(outcome.reason)

    def run_cli(self, reviser: str, *extra: str) -> tuple[int, dict]:
        delta = self.root / "delta.json"
        delta.write_text(json.dumps(self.delta), encoding="utf-8")
        out = self.root / "outcome.json"
        code = main(["revise-gate", "--record", str(self.record), "--root", str(self.root),
                     "--delta", str(delta), "--design", str(self.design), "--reviser", reviser,
                     "--out", str(out), *extra])
        return code, json.loads(out.read_text(encoding="utf-8")) if out.exists() else {}

    def test_the_command_line_exits_zero_only_when_the_design_is_within_reach(self):
        restore = ("import os, pathlib, sys;"
                   "pathlib.Path(os.environ['SCE_REVISION_DESIGN']).write_text("
                   "pathlib.Path(sys.argv[1]).read_text())")
        code, shown = self.run_cli(shlex.join([sys.executable, "-c", restore, str(self.accepted_copy)]))
        self.assertEqual((0, "within-reach", True), (code, shown["status"], shown["within_reach"]))
        self.assertEqual(2, len(shown["rounds"]))

    def test_the_command_line_exits_one_for_a_revision_that_stays_out_of_reach(self):
        code, shown = self.run_cli(shlex.join([sys.executable, "-c", "pass"]))
        self.assertEqual((1, "stalled", False), (code, shown["status"], shown["within_reach"]))

    def test_the_command_line_fails_a_reviser_that_replaces_the_record_it_is_judged_against(self):
        forged = self.forged_record()
        replace = ("import pathlib, shutil, sys; shutil.copy(sys.argv[1], sys.argv[2])")
        code, shown = self.run_cli(shlex.join([sys.executable, "-c", replace, str(forged),
                                               str(self.record)]))
        self.assertEqual((2, "reviser-failed", False), (code, shown["status"], shown["within_reach"]))
        self.assertIn("changed the acceptance record", shown["reason"])

    def test_the_command_line_exits_two_for_a_reviser_that_fails(self):
        code, shown = self.run_cli(shlex.join([sys.executable, "-c", "raise SystemExit(7)"]))
        self.assertEqual((2, "reviser-failed"), (code, shown["status"]))
        self.assertIn("status 7", shown["reason"])


if __name__ == "__main__":
    unittest.main()
