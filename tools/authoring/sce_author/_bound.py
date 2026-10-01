"""Bind this process to its limits, then become the program it was asked to run.

    python -I -S _bound.py <limits-json> -- <program> <args...>

This is the tail of `process.py`, kept as its own file so that it can be started
by path with no package around it: `-I -S` leaves it nothing to import but the
standard library, and it imports almost none of that.

Why a launcher and not `preexec_fn`. The supervisor starts children from worker
threads, and Python's own documentation says `preexec_fn` is not safe in the
presence of threads: the child, forked from a process whose other threads may
hold a lock, can deadlock before it reaches `exec`. Everything `preexec_fn` did
(the kernel's limits, no core dump, die-with-the-parent, no new privileges) is
applied here instead, by the child itself, in a fresh interpreter with no other
thread, and then `execvp` replaces it. Limits and the two `prctl` settings
survive `execve`; the program that follows is bound by them from its first
instruction.

⚠ On Linux a limit that cannot be applied is a refusal to start (status 125 and
a line on stderr beginning `sce-bound:`), never a child that runs unbound while
the supervisor reports it bound. Elsewhere the limits the kernel does not
enforce are skipped, and `process.isolation_level` says so.
"""

import ctypes
import json
import os
import resource
import signal
import sys

NOT_STARTED = 125

_PR_SET_PDEATHSIG = 1
_PR_SET_NO_NEW_PRIVS = 38

_MEBIBYTE = 1024 * 1024


def _refuse(why: str) -> None:
    sys.stderr.write(f"sce-bound: {why}\n")
    sys.stderr.flush()
    os._exit(NOT_STARTED)


def _limit(which: str, soft: int, hard: int, strict: bool) -> None:
    try:
        resource.setrlimit(getattr(resource, which), (soft, hard))
    except (ValueError, OSError, AttributeError) as exc:
        if strict:
            _refuse(f"could not apply {which}: {exc}")


def _prctl(strict: bool) -> None:
    try:
        libc = ctypes.CDLL(None, use_errno=True)
        if libc.prctl(_PR_SET_PDEATHSIG, signal.SIGKILL, 0, 0, 0) != 0 and strict:
            _refuse("could not ask to die with the supervisor")
        if libc.prctl(_PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 and strict:
            _refuse("could not forbid new privileges")
    except (OSError, AttributeError) as exc:
        if strict:
            _refuse(f"prctl is not available: {exc}")


def main(argv: list) -> None:
    if len(argv) < 4 or argv[2] != "--":
        _refuse("usage: _bound.py <limits-json> -- <program> [args...]")
    limits = json.loads(argv[1])
    program = argv[3:]
    strict = sys.platform.startswith("linux")
    cpu = int(limits["cpu_seconds"])
    # The soft limit ends a runaway with SIGXCPU, which says what happened; the
    # hard one, two seconds later, is the kernel's own SIGKILL for a child that
    # ignores the signal.
    _limit("RLIMIT_CPU", cpu, cpu + 2, strict)
    _limit("RLIMIT_AS", int(limits["memory_mb"]) * _MEBIBYTE, int(limits["memory_mb"]) * _MEBIBYTE,
           strict)
    _limit("RLIMIT_FSIZE", int(limits["output_mb"]) * _MEBIBYTE,
           int(limits["output_mb"]) * _MEBIBYTE, strict)
    _limit("RLIMIT_NOFILE", int(limits["open_files"]), int(limits["open_files"]), strict)
    _limit("RLIMIT_CORE", 0, 0, strict)
    if sys.platform.startswith("linux"):
        _prctl(strict)
    try:
        os.execvp(program[0], program)
    except OSError as exc:
        _refuse(f"could not start {program[0]}: {exc}")


if __name__ == "__main__":
    main(sys.argv)
