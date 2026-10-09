"""The one place this package starts another program.

Two kinds of program are started, and they are held to different standards.

`run` starts the product's own code generator, a program this tree builds and
trusts to end. It gets a clock and nothing else.

`run_isolated` and `Session` start something that plays a DESIGN: code an AI
client wrote, which can loop for ever inside the Lua machine, re-send itself
without end, or grow until the host is out of memory. Measured 2026-10-01, a
`while (true)` in a `<script>` ignored a `SIGALRM` handler set to fire at 3 s,
because the loop is inside Lua's C code and no Python-level timer runs; the
only things that stop it are a watchdog outside the process and the operating
system's own limits. So a design is never run in the caller's process: it runs
in a child of its own, in a session of its own, under limits the kernel
enforces, and a clock outside it.

`run_isolated` runs a child to its end and reads what it printed. `Session`
keeps one child alive and speaks to it line by line, each exchange under a clock
of its own: it is what `sandbox` builds on, for the consumers (`verify`,
`compare`) that drive one machine through many steps and cannot afford a process
per step.

What the supervisor will and will not say:

    It reports WHICH limit stopped a child, and whether that is a fact about the
    design or about the machine is the caller's reading of it: running out of
    clock or memory is the machine's (another machine may play the same example
    to its end), a crash is reported as a crash.

    It reports the isolation it really applied, as a name, and not the isolation
    it hoped for. On a host that cannot enforce a limit it says so, and where
    Linux cannot apply a limit the child does not start (see `_bound`).

    It does not claim a namespace, a cgroup or a seccomp filter it did not set
    up. Those are further layers, and they cannot be assumed: a namespace
    sandbox needs unprivileged user namespaces, which a host may forbid (AppArmor
    on a current Ubuntu does, so `bwrap` and `unshare -Urn` fail there), while a
    cgroup scope needs a user systemd. A deployment that needs one asks for it by
    name and is refused where it is absent.

Children are started through `_bound.py`, which applies the limits to itself and
then `exec`s the program, instead of through `preexec_fn`: the supervisor starts
children from worker threads, and `preexec_fn` is documented as unsafe there.

⚠ `PR_SET_PDEATHSIG` follows the THREAD that started the child, not the process:
a child started from a worker thread that then exits is killed with it. The
callers here wait for every child before their threads end.
"""

from __future__ import annotations

import contextvars
import dataclasses
import json
import os
import pathlib
import select
import signal
import subprocess
import sys
import tempfile
import threading
import time
from dataclasses import dataclass


class ProcessTimeout(Exception):
    """A trusted program did not finish in the time it was given."""


def run(argv: list, *, cwd=None, timeout: float | None = None,
        stdin_text: str | None = None, env: dict | None = None,
        own_group: bool = False) -> subprocess.CompletedProcess:
    """Run a program this tree trusts to end, and read what it printed as text.

    `env` is what is added to this process's environment for the program (a caller that tells
    the program of a file by a variable, as `revision_gate.command_reviser` does); nothing is
    removed, and left out the program is started with the environment as it is.

    `own_group` starts the program in a session of its own and, when it runs past `timeout`,
    stops everything it started and not only the program itself. A caller whose program is a
    client that starts programs in turn needs it: `subprocess.run` kills the direct child on a
    timeout, and what that child started goes on running, and writing, after the caller has said
    it stopped (a reviser that outlived its timeout edited the design the gate was judging). The
    group is signalled while the program is still uncollected, so its number is still its own;
    what a program leaves running after it EXITS is not stopped, because by then its number
    belongs to nobody.

    A clock is the only protection: the product's generator is code this
    repository builds, and what it is handed is a document, not a program.

    `stdin_text` is what the program reads on its standard input, for a text
    too large for a command line (a specification). ⚠ Text in both directions is
    UTF-8, named rather than left to the locale: every program started here is
    this repository's Rust, which writes and reads UTF-8, and a console code
    page (a Korean Windows one is not UTF-8) would otherwise turn a specification
    into other characters on the way in.

    ⚠ For a Workbench generation (`SCE_AUTHOR_WORK`) the folder the program runs in is the one
    folder it may open files in (`SCE_FILE_ROOT`, read by the generator): what it is handed was
    written by a model, and a document that names a file elsewhere (an import, a template, an
    include) is a way to ask the machine about its files. The generator holds that where it opens
    a file, so it holds for a place nobody listed; the staged folder is the one this is run in."""
    added = dict(env or {})
    if os.environ.get("SCE_AUTHOR_WORK") and cwd is not None:
        added["SCE_FILE_ROOT"] = str(cwd)
    environment = {**os.environ, **added} if added else None
    if own_group:
        return _run_in_own_group(argv, cwd, timeout, stdin_text, environment)
    try:
        return subprocess.run(argv, capture_output=True, text=True, encoding="utf-8",
                              cwd=cwd, timeout=timeout, input=stdin_text, env=environment)
    except subprocess.TimeoutExpired as exc:
        raise ProcessTimeout(
            f"{os.path.basename(str(argv[0]))} had not finished after {timeout:.0f} s") from exc


def _run_in_own_group(argv: list, cwd, timeout: float | None, stdin_text: str | None,
                      environment: dict | None) -> subprocess.CompletedProcess:
    """`run` for a program started in a session of its own, whose whole group is stopped when it
    runs past its clock or this call is interrupted."""
    child = subprocess.Popen(argv, stdin=subprocess.PIPE if stdin_text is not None else None,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                             encoding="utf-8", cwd=cwd, env=environment, start_new_session=True)
    try:
        out, err = child.communicate(input=stdin_text, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        _end_group(child)
        raise ProcessTimeout(
            f"{os.path.basename(str(argv[0]))} had not finished after {timeout:.0f} s") from exc
    except BaseException:
        _end_group(child)
        raise
    return subprocess.CompletedProcess(argv, child.returncode, out, err)


def _end_group(child: subprocess.Popen) -> None:
    """Stop everything the program started, collect it and close its pipes. Only while the program
    is still uncollected: once it is collected its number belongs to nobody, and a signal to it
    could reach a stranger."""
    if child.returncode is None:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
    for pipe in (child.stdin, child.stdout, child.stderr):
        if pipe is not None:
            pipe.close()
    child.wait()


@dataclass(frozen=True)
class Limits:
    """What a child may use. Each is enforced by the kernel except the clock,
    which the supervisor enforces from outside."""

    #: The backstop. A verdict that depends on it is about the machine, and says so.
    #: For a `Session` it is the clock of ONE exchange, not of the session.
    wall_seconds: float = 30.0
    cpu_seconds: int = 25
    memory_mb: int = 2048
    output_mb: int = 16
    open_files: int = 256

    def record(self) -> dict:
        """The bounds as the observation trace repeats them."""
        return dataclasses.asdict(self)


#: What a `Session` may use. Processor time is cumulative over the whole life of
#: the child, which plays thousands of steps, so it is far larger than a single
#: example's; one runaway step is stopped by the exchange clock long before.
SESSION_LIMITS = Limits(wall_seconds=30.0, cpu_seconds=900, memory_mb=2048,
                        output_mb=16, open_files=256)


@dataclass
class Outcome:
    """What running a child came to."""

    returncode: int | None
    stdout: str
    stderr: str
    #: None when the child ended on its own. Otherwise which limit stopped it:
    #: `wall-clock`, `cpu`, `memory`, `output`, `crash` for a signal that no
    #: limit explains, or `not-started` when the limits could not be applied.
    stopped_by: str | None
    seconds: float
    isolation: str

    @property
    def ended_on_its_own(self) -> bool:
        return self.stopped_by is None and self.returncode == 0

    def said(self) -> str:
        """One line for a person: what happened to the child."""
        return _said(self.stopped_by, self.returncode, self.stderr, self.seconds)


def _said(stopped_by: str | None, returncode: int | None, stderr: str, seconds: float) -> str:
    took = f"after {seconds:.1f} s"
    if stopped_by == "wall-clock":
        return f"it had not finished {took} and was stopped"
    if stopped_by == "cpu":
        return f"it used more processor time than it was allowed ({took}) and was stopped"
    if stopped_by == "memory":
        return f"it asked for more memory than it was allowed ({took})"
    if stopped_by == "output":
        return f"it wrote more than it was allowed to ({took}) and was stopped"
    if stopped_by == "not-started":
        tail = (stderr.strip().splitlines() or ["no output"])[-1][:200]
        return f"it did not start, because its limits could not be applied: {tail}"
    if returncode is not None and returncode < 0:
        return f"it was ended by signal {-returncode} {took}"
    tail = (stderr.strip().splitlines() or ["no output"])[-1][:200]
    return f"it exited with status {returncode} {took}: {tail}"


def isolation_level() -> str:
    """What this module really applies on this host, as a name.

    `process+rlimit`: a session and process group of its own, the kernel's
    limits on processor time, address space, file size and open files, no core
    dump, no new privileges, and a clock outside the child. Anywhere the kernel
    does not enforce those, the name says so rather than borrowing the Linux one."""
    if sys.platform.startswith("linux"):
        return "process+rlimit"
    return f"process (kernel limits are not enforced on {sys.platform})"


#: The levels `isolation_level` can name, weakest first. An operator names one to
#: say the weakest they accept for running a stranger's design, and a stronger one
#: (a namespace, a cgroup) goes at the end when this module can apply it.
ISOLATION_LEVELS = ("process", "process+rlimit")


def isolation_rank(level: str) -> int:
    """Where a reported level sits in `ISOLATION_LEVELS`, or -1 for a name nobody
    defined. A description ranks as the name it begins with: `process (kernel
    limits are not enforced on darwin)` is `process`, which is the point of
    ranking it, since it reads like the stronger one and is not."""
    name = level.split(" ", 1)[0]
    return ISOLATION_LEVELS.index(name) if name in ISOLATION_LEVELS else -1


#: What a child is given of the environment: enough to find programs and speak
#: UTF-8, and nothing else. A design's machine has no business with the
#: caller's credentials, tokens or paths.
_KEPT = ("PATH", "LANG", "LC_ALL", "LC_CTYPE")


def _environment(extra: dict | None) -> dict:
    kept = {key: os.environ[key] for key in _KEPT if key in os.environ}
    kept["PYTHONIOENCODING"] = "utf-8"
    kept.update(extra or {})
    return kept


_BOUND = str(pathlib.Path(__file__).with_name("_bound.py"))

#: The status `_bound` exits with when it could not apply a limit.
NOT_STARTED = 125


def _bounded(argv: list, limits: Limits) -> list:
    """`argv`, behind the launcher that binds the child to `limits` first."""
    return [sys.executable, "-I", "-S", _BOUND, json.dumps(limits.record()), "--", *argv]


def _stopped_by(returncode: int | None, killed_for_time: bool, stderr: str,
                output_full: bool = False) -> str | None:
    # CPython ignores SIGXFSZ, so a program that writes past its file-size limit
    # is not killed by the signal: its write fails with EFBIG and it raises. Both
    # forms are the same event, and it is the FIRST thing that happened. Measured
    # 2026-10-02, a child that had printed its traceback for it could then spend
    # a second or two failing to flush its standard output on the way out, and
    # was ended by the processor limit or the clock afterwards; naming that end
    # would say the cause was time when the cause had already been said.
    ended_cleanly = returncode == 0 and not killed_for_time
    if ended_cleanly:
        return None
    # `output_full`: the file it wrote reached the very size it was allowed. That
    # needs no message to read, and a child that was ended afterwards (it may
    # spin in CPython's handling of the failed write before it raises) was ended
    # for what it did at the cap.
    if returncode == -signal.SIGXFSZ or "File too large" in stderr or output_full:
        return "output"
    if killed_for_time:
        return "wall-clock"
    if returncode is None:
        return None
    if returncode == NOT_STARTED and "sce-bound:" in stderr:
        return "not-started"
    if returncode == -signal.SIGXCPU:
        return "cpu"
    if "MemoryError" in stderr or "not enough memory" in stderr:
        return "memory"
    if returncode < 0:
        # SIGKILL with no word about a limit, SIGSEGV, SIGABRT: the kernel's
        # address-space limit makes some allocations fail as a crash and not as
        # an exception, so a bare signal is reported as one.
        return "crash"
    return None


class _Leader:
    """A child that leads a process group of its own, and the one way its group ends.

    The child is started in a session of its own so that one signal reaches
    everything it started. That signal is addressed by the leader's pid, and a
    pid is free for another process from the moment its owner is collected, so
    the order is fixed here and nowhere else: wait for the leader to END without
    collecting it, signal the group, and only then collect. `communicate()`,
    `wait()` and `poll()` all collect, which is why a group used to be killed
    only on the paths that killed it: a child that ended on its own was collected
    first and what it had started was left running (measured 2026-10-02).

    `pidfd_open` is what waits without collecting. Where a host has none, the
    leader is waited for and collected, and the group is signalled anyway: the
    most that host allows, and `isolation_level` already says such a host is not
    the one the limits are written for."""

    def __init__(self, child: subprocess.Popen) -> None:
        self.child = child
        self._ended = False
        try:
            self._pidfd: int | None = os.pidfd_open(child.pid)
        except (AttributeError, OSError):
            self._pidfd = None

    def ended(self, timeout: float | None) -> bool:
        """Whether the leader ended within `timeout` seconds, collecting nothing
        where the host can wait that way."""
        if self._pidfd is None:
            try:
                self.child.wait(timeout)
            except subprocess.TimeoutExpired:
                return False
            return True
        # poll(), not select(): a server with many sessions holds descriptors
        # past what select() can address.
        poller = select.poll()
        poller.register(self._pidfd, select.POLLIN)
        return bool(poller.poll(None if timeout is None else max(0.0, timeout) * 1000))

    def end(self) -> None:
        """Kill everything the leader started, collect it, release the handle.

        Done once: after the leader is collected its pid belongs to nobody, so a
        second signal to it could reach a stranger. A caller that collected the
        child itself before this runs gets the signal anyway (the group outlives
        its leader for as long as anything is in it), which is the best that
        order allows."""
        if self._ended:
            return
        self._ended = True
        try:
            os.killpg(self.child.pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
        self.child.wait()
        if self._pidfd is not None:
            os.close(self._pidfd)
            self._pidfd = None


def run_isolated(argv: list, *, limits: Limits = Limits(), stdin_text: str = "",
                 cwd=None, env: dict | None = None) -> Outcome:
    """Run a program that plays a design, in a child no design can outlive.

    The child gets a session of its own (so a kill reaches everything it
    started), the kernel's limits, a scrubbed environment, and a clock outside
    it. Its output goes to files under a limit of their own and is read after it
    ends, so a child that prints without end fills a file to its cap and stops,
    and the supervisor never holds more than it chose to read.

    Whatever the child started is ended with it, whichever way it ended: on its
    own, by a limit, or by the clock. What it was told is a file too, so there is
    no pipe to feed and `communicate()`, which collects the child before its group
    can be signalled, is not needed (see `_Leader`)."""
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="sce_run_") as scratch:
        in_path = os.path.join(scratch, "stdin")
        out_path = os.path.join(scratch, "stdout")
        err_path = os.path.join(scratch, "stderr")
        with open(in_path, "wb") as told:
            told.write(stdin_text.encode("utf-8"))
        with open(in_path, "rb") as given, open(out_path, "wb") as out, \
                open(err_path, "wb") as err:
            child = subprocess.Popen(
                _bounded(argv, limits), stdin=given, stdout=out, stderr=err, cwd=cwd,
                env=_environment(env), start_new_session=True)
            leader = _Leader(child)
            try:
                killed_for_time = not leader.ended(limits.wall_seconds)
            finally:
                leader.end()
        cap = limits.output_mb * 1024 * 1024
        output_full = os.path.getsize(out_path) >= cap
        with open(out_path, "rb") as out, open(err_path, "rb") as err:
            stdout = out.read(cap).decode("utf-8", errors="replace")
            stderr = err.read(1024 * 1024).decode("utf-8", errors="replace")
    returncode = child.returncode
    return Outcome(
        returncode=returncode, stdout=stdout, stderr=stderr,
        stopped_by=_stopped_by(returncode, killed_for_time, stderr, output_full),
        seconds=time.monotonic() - started, isolation=isolation_level())


class SessionStopped(Exception):
    """A session's child is gone, and why. Every later exchange raises the same."""

    def __init__(self, stopped_by: str, said: str, isolation: str) -> None:
        super().__init__(said)
        #: `wall-clock`, `cpu`, `memory`, `output`, `crash`, `not-started`, or
        #: `exited` for a child that ended without a limit explaining it.
        self.stopped_by = stopped_by
        self.said = said
        self.isolation = isolation


_REAPERS: contextvars.ContextVar = contextvars.ContextVar("sce_session_reapers", default=None)


class reaping:
    """Close every `Session` opened inside this block when it ends.

        with process.reaping():
            ...   # a worker started here does not outlive the call

    The callers that open sessions are long calls with many exits, and a design's
    machine left running in the background after its answer was given is the
    thing this module exists to prevent."""

    def __enter__(self):
        self._sessions: list = []
        self._token = _REAPERS.set(self._sessions)
        return self

    def __exit__(self, *_):
        _REAPERS.reset(self._token)
        for session in reversed(self._sessions):
            session.close()
        return False


class Session:
    """One supervised child, spoken to a line at a time.

    The child reads a line from its standard input and answers with a line on
    its standard output. Each exchange has a clock; a child that does not answer
    in time, answers with more than it may, or dies, is killed (with everything
    it started) and every later exchange raises the same `SessionStopped`: a
    session that has once been stopped is not trusted again, and the caller
    decides what its work is worth without the rest of it."""

    def __init__(self, argv: list, *, limits: Limits = SESSION_LIMITS, cwd=None,
                 env: dict | None = None) -> None:
        self.limits = limits
        self._lock = threading.Lock()
        self._buffer = b""
        self._stopped: SessionStopped | None = None
        self._scratch = tempfile.TemporaryDirectory(prefix="sce_session_")
        self._stderr_path = os.path.join(self._scratch.name, "stderr")
        self._stderr = open(self._stderr_path, "wb")
        self._started = time.monotonic()
        self.child = subprocess.Popen(
            _bounded(argv, limits), stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=self._stderr, cwd=cwd, env=_environment(env), start_new_session=True,
            bufsize=0)
        self._leader = _Leader(self.child)
        reapers = _REAPERS.get()
        if reapers is not None:
            reapers.append(self)

    @property
    def stopped(self) -> SessionStopped | None:
        return self._stopped

    def exchange(self, line: bytes, timeout: float | None = None) -> bytes:
        """Send one line, and return the one line that answers it."""
        with self._lock:
            if self._stopped is not None:
                raise self._stopped
            deadline = time.monotonic() + (timeout if timeout is not None
                                           else self.limits.wall_seconds)
            try:
                self.child.stdin.write(line + b"\n")
            except (BrokenPipeError, OSError):
                raise self._stop(False) from None
            cap = self.limits.output_mb * 1024 * 1024
            descriptor = self.child.stdout.fileno()
            while True:
                found = self._buffer.find(b"\n")
                if found >= 0:
                    answer, self._buffer = self._buffer[:found], self._buffer[found + 1:]
                    return answer
                if len(self._buffer) > cap:
                    raise self._stop(False, "output")
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise self._stop(True)
                ready, _, _ = select.select([descriptor], [], [], remaining)
                if not ready:
                    continue
                chunk = os.read(descriptor, 65536)
                if not chunk:
                    raise self._stop(False)
                self._buffer += chunk

    def fail(self, why: str) -> SessionStopped:
        """Stop a child that answered in a way the protocol cannot use. A process
        that has once spoken outside it is not one to go on asking."""
        with self._lock:
            return self._stop(False, forced="protocol", said=f"it {why}")

    def _stop(self, for_time: bool, forced: str | None = None,
              said: str | None = None) -> SessionStopped:
        """Kill the child and everything it started, and say why. Idempotent."""
        if self._stopped is not None:
            return self._stopped
        self._leader.end()
        self._stderr.flush()
        try:
            with open(self._stderr_path, "rb") as handle:
                stderr = handle.read(1024 * 1024).decode("utf-8", errors="replace")
        except OSError:
            stderr = ""
        seconds = time.monotonic() - self._started
        returncode = self.child.returncode
        stopped_by = forced or _stopped_by(returncode, for_time, stderr) or "exited"
        self._stopped = SessionStopped(
            stopped_by, said or _said(stopped_by, returncode, stderr, seconds),
            isolation_level())
        return self._stopped

    def close(self) -> None:
        """End the child and everything it started, and release what it held. Safe
        to call twice.

        The group is ended whether or not the child still runs. A worker that
        answered and ended on its own has started nothing the session knows of,
        and the point is that it does not have to have: what it left running is
        this session's to end, and `poll()` here used to collect the worker and
        skip the group (measured 2026-10-02)."""
        with self._lock:
            self._leader.end()
            for stream in (self.child.stdin, self.child.stdout, self._stderr):
                try:
                    stream.close()
                except (OSError, ValueError):
                    pass
            self._scratch.cleanup()
            if self._stopped is None:
                self._stopped = SessionStopped("closed", "the session was closed",
                                               isolation_level())

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()
        return False
