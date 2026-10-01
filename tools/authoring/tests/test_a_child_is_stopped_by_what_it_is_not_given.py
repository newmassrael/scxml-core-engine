"""The supervisor that plays a design stops a child by what the child is not given.

`process.run_isolated` is the only place this package starts a program that plays
code nobody has read. Each case here starts a small Python program that does one
thing a design could make an engine do, and holds the supervisor to what it owes:
stop it, say which limit did, leave nothing running, and never report as a clean
exit what was an ending.

No generator is involved, so these run in the domain-free job too.
"""

from __future__ import annotations

import os
import signal
import sys
import time
import unittest

from sce_author import process

QUICK = process.Limits(wall_seconds=2.0, cpu_seconds=1, memory_mb=256, output_mb=1,
                       open_files=64)

# ⚠ A case about ANY limit but the clock must not hand the clock a chance to win.
# Starting two interpreters on a host with a load average of 52 on 32 cores takes
# longer than a 2 s clock (measured 2026-10-02: a memory-limit case read as
# `wall-clock`), and a test that depends on how busy the machine is says nothing
# about the limit it is named for. These limits keep the limit under test short
# in processor time and the clock far away.
ROOMY = process.Limits(wall_seconds=60.0, cpu_seconds=1, memory_mb=256, output_mb=1,
                       open_files=64)


def python(code: str) -> list:
    return [sys.executable, "-c", code]


class TestAChildThatEndsOnItsOwn(unittest.TestCase):
    def test_what_it_prints_and_what_it_was_told_come_through(self):
        out = process.run_isolated(python("import sys; print(sys.stdin.read().upper())"),
                                   limits=QUICK, stdin_text="hello")
        self.assertTrue(out.ended_on_its_own, out)
        self.assertEqual("HELLO", out.stdout.strip())
        self.assertIsNone(out.stopped_by)

    def test_a_nonzero_exit_is_not_a_clean_ending(self):
        out = process.run_isolated(python("import sys; sys.exit(3)"), limits=QUICK)
        self.assertFalse(out.ended_on_its_own)
        self.assertIsNone(out.stopped_by)
        self.assertEqual(3, out.returncode)
        self.assertIn("status 3", out.said())

    def test_it_is_told_what_this_host_really_applied(self):
        out = process.run_isolated(python("pass"), limits=QUICK)
        self.assertEqual(process.isolation_level(), out.isolation)
        if sys.platform.startswith("linux"):
            self.assertEqual("process+rlimit", out.isolation)


class TestAChildThatTheLimitsStop(unittest.TestCase):
    def test_a_child_that_sleeps_past_the_clock_is_killed_at_it(self):
        started = time.monotonic()
        out = process.run_isolated(python("import time; time.sleep(60)"), limits=QUICK)
        self.assertEqual("wall-clock", out.stopped_by, out)
        self.assertLess(time.monotonic() - started, 10)
        self.assertIn("had not finished", out.said())

    @unittest.skipUnless(sys.platform.startswith("linux"), "the kernel limits are Linux's here")
    def test_a_child_that_spins_in_native_code_is_stopped_by_processor_time(self):
        """The case a Python-level timer cannot reach: the loop never returns to
        the interpreter. A string search over a large buffer stands in for the
        Lua machine's C code."""
        out = process.run_isolated(
            python("b = b'a' * 50_000_000\nwhile True:\n    b.find(b'z')"),
            limits=process.Limits(wall_seconds=20.0, cpu_seconds=1, memory_mb=512))
        self.assertEqual("cpu", out.stopped_by, out)
        self.assertLess(out.seconds, 15)

    @unittest.skipUnless(sys.platform.startswith("linux"), "the kernel limits are Linux's here")
    def test_a_child_that_asks_for_too_much_memory_is_stopped(self):
        out = process.run_isolated(python("x = bytearray(2 * 1024**3)"),
                                   limits=process.Limits(wall_seconds=60.0, cpu_seconds=30,
                                                         memory_mb=256))
        self.assertEqual("memory", out.stopped_by, out)

    @unittest.skipUnless(sys.platform.startswith("linux"), "the kernel limits are Linux's here")
    def test_a_child_that_prints_without_end_is_stopped_at_its_cap(self):
        out = process.run_isolated(
            python("import sys\nwhile True:\n    sys.stdout.write('x' * 65536)\n"
                   "    sys.stdout.flush()"),
            limits=ROOMY)
        self.assertEqual("output", out.stopped_by, out)
        self.assertLessEqual(len(out.stdout), 1024 * 1024)

    @unittest.skipUnless(sys.platform.startswith("linux"), "the kernel limits are Linux's here")
    def test_a_child_that_crashes_is_reported_as_one_not_as_a_limit(self):
        # The clock is generous because a host may spend seconds on a core dump
        # it was told not to take (measured 1.9 s here), and a crash that lost
        # the race with a short clock would read as a hang.
        out = process.run_isolated(python("import os, signal; os.kill(os.getpid(), signal.SIGSEGV)"),
                                   limits=process.Limits(wall_seconds=20.0, cpu_seconds=10))
        self.assertEqual("crash", out.stopped_by, out)
        self.assertIn("signal", out.said())


class TestWhatTheKillLeavesBehind(unittest.TestCase):
    def test_a_grandchild_the_child_started_does_not_outlive_it(self):
        """The child gets a session of its own so one kill reaches everything it
        started. A design's machine that forks must not leave a process behind."""
        code = ("import subprocess, sys, time\n"
                "p = subprocess.Popen(['sleep', '60'])\n"
                "print(p.pid, flush=True)\n"
                "time.sleep(60)\n")
        out = process.run_isolated(python(code), limits=QUICK)
        self.assertEqual("wall-clock", out.stopped_by, out)
        grandchild = int(out.stdout.split()[0])
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            try:
                os.kill(grandchild, 0)
            except ProcessLookupError:
                return
            time.sleep(0.05)
        os.kill(grandchild, signal.SIGKILL)
        self.fail("the process the child started was still running after the kill")


class TestWhatTheChildIsGiven(unittest.TestCase):
    def test_the_callers_environment_does_not_reach_it(self):
        os.environ["SCE_TEST_SECRET_TOKEN"] = "do-not-leak"
        try:
            out = process.run_isolated(
                python("import os; print(repr(os.environ.get('SCE_TEST_SECRET_TOKEN')))"),
                limits=QUICK)
        finally:
            del os.environ["SCE_TEST_SECRET_TOKEN"]
        self.assertEqual("None", out.stdout.strip(), out)

    def test_what_the_caller_names_does_reach_it(self):
        out = process.run_isolated(
            python("import os; print(os.environ.get('SCE_TEST_NAMED'))"), limits=QUICK,
            env={"SCE_TEST_NAMED": "kept"})
        self.assertEqual("kept", out.stdout.strip(), out)


class TestTheTrustedProgramsAreOnlyGivenAClock(unittest.TestCase):
    def test_a_program_that_ends_is_run_and_read_as_text(self):
        done = process.run(python("print('ok')"), timeout=10)
        self.assertEqual("ok", done.stdout.strip())

    def test_a_program_that_does_not_end_is_an_answer_not_a_hang(self):
        with self.assertRaises(process.ProcessTimeout) as caught:
            process.run(python("import time; time.sleep(60)"), timeout=1)
        self.assertIn("had not finished", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
