"""The one place this package starts another program.

Two kinds of program are started, and they are held to different standards.

`run` starts the product's own code generator, a program this tree builds and
trusts to end. It gets a clock and nothing else.

`run_isolated` starts something that plays a DESIGN: code an AI client wrote,
which can loop for ever inside the Lua machine, re-send itself without end, or
grow until the host is out of memory. Measured 2026-10-01, a `while (true)` in a
`<script>` ignored a `SIGALRM` handler set to fire at 3 s, because the loop is
inside Lua's C code and no Python-level timer runs; the only things that stop it
are a watchdog outside the process and the operating system's own limits. So a
design is never run in the caller's process: it runs in a child of its own, in a
session of its own, under limits the kernel enforces, and a clock outside it.

What the supervisor will and will not say:

    It reports WHICH limit stopped a child, and whether that is a fact about the
    design or about the machine is the caller's reading of it: running out of
    clock or memory is the machine's (another machine may play the same example
    to its end), a crash is reported as a crash.

    It reports the isolation it really applied, as a name, and not the isolation
    it hoped for. On a host that cannot enforce a limit it says so.

    It does not claim a namespace, a cgroup or a seccomp filter it did not set
    up. Those are further layers, and they cannot be assumed: a namespace
    sandbox needs unprivileged user namespaces, which a host may forbid (AppArmor
    on a current Ubuntu does, so `bwrap` and `unshare -Urn` fail there), while a
    cgroup scope needs a user systemd. A deployment that needs one asks for it by
    name and is refused where it is absent.

⚠ `PR_SET_PDEATHSIG` follows the THREAD that started the child, not the process:
a child started from a worker thread that then exits is killed with it. The
callers here wait for every child before their threads end.
"""

from __future__ import annotations

import ctypes
import dataclasses
import os
import resource
import signal
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass


class ProcessTimeout(Exception):
    """A trusted program did not finish in the time it was given."""


def run(argv: list, *, cwd=None, timeout: float | None = None) -> subprocess.CompletedProcess:
    """Run a program this tree trusts to end, and read what it printed as text.

    A clock is the only protection: the product's generator is code this
    repository builds, and what it is handed is a document, not a program."""
    try:
        return subprocess.run(argv, capture_output=True, text=True, cwd=cwd, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        raise ProcessTimeout(
            f"{os.path.basename(str(argv[0]))} had not finished after {timeout:.0f} s") from exc


@dataclass(frozen=True)
class Limits:
    """What a child may use. Each is enforced by the kernel except the clock,
    which the supervisor enforces from outside."""

    #: The backstop. A verdict that depends on it is about the machine, and says so.
    wall_seconds: float = 30.0
    cpu_seconds: int = 25
    memory_mb: int = 2048
    output_mb: int = 16
    open_files: int = 256

    def record(self) -> dict:
        """The bounds as the observation trace repeats them."""
        return dataclasses.asdict(self)


@dataclass
class Outcome:
    """What running a child came to."""

    returncode: int | None
    stdout: str
    stderr: str
    #: None when the child ended on its own. Otherwise which limit stopped it:
    #: `wall-clock`, `cpu`, `memory`, `output`, or `crash` for a signal that no
    #: limit explains.
    stopped_by: str | None
    seconds: float
    isolation: str

    @property
    def ended_on_its_own(self) -> bool:
        return self.stopped_by is None and self.returncode == 0

    def said(self) -> str:
        """One line for a person: what happened to the child."""
        took = f"after {self.seconds:.1f} s"
        if self.stopped_by == "wall-clock":
            return f"it had not finished {took} and was stopped"
        if self.stopped_by == "cpu":
            return f"it used more processor time than it was allowed ({took}) and was stopped"
        if self.stopped_by == "memory":
            return f"it asked for more memory than it was allowed ({took})"
        if self.stopped_by == "output":
            return f"it wrote more than it was allowed to ({took}) and was stopped"
        if self.returncode is not None and self.returncode < 0:
            return f"it was ended by signal {-self.returncode} {took}"
        tail = (self.stderr.strip().splitlines() or ["no output"])[-1][:200]
        return f"it exited with status {self.returncode} {took}: {tail}"


def isolation_level() -> str:
    """What `run_isolated` really applies on this host, as a name.

    `process+rlimit`: a session and process group of its own, the kernel's
    limits on processor time, address space, file size and open files, no core
    dump, no new privileges, and a clock outside the child. Anywhere the kernel
    does not enforce those, the name says so rather than borrowing the Linux one."""
    if sys.platform.startswith("linux"):
        return "process+rlimit"
    return f"process (kernel limits are not enforced on {sys.platform})"


_PR_SET_PDEATHSIG = 1
_PR_SET_NO_NEW_PRIVS = 38


def _prctl() -> None:
    """Die with the supervisor; never gain privileges. Linux only, best effort:
    a host without `prctl` simply does not get these two."""
    if not sys.platform.startswith("linux"):
        return
    try:
        libc = ctypes.CDLL(None, use_errno=True)
        libc.prctl(_PR_SET_PDEATHSIG, signal.SIGKILL, 0, 0, 0)
        libc.prctl(_PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)
    except (OSError, AttributeError):
        pass


def _applying(limits: Limits):
    mebibyte = 1024 * 1024

    def apply() -> None:
        # The hard limit sits a little above the soft one: the soft limit ends a
        # runaway with SIGXCPU, which says what happened, and the hard one is the
        # kernel's own SIGKILL for a child that ignores it.
        resource.setrlimit(resource.RLIMIT_CPU, (limits.cpu_seconds, limits.cpu_seconds + 2))
        resource.setrlimit(resource.RLIMIT_AS, (limits.memory_mb * mebibyte,) * 2)
        resource.setrlimit(resource.RLIMIT_FSIZE, (limits.output_mb * mebibyte,) * 2)
        resource.setrlimit(resource.RLIMIT_NOFILE, (limits.open_files,) * 2)
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        _prctl()

    return apply


#: What a child is given of the environment: enough to find programs and speak
#: UTF-8, and nothing else. A design's machine has no business with the
#: caller's credentials, tokens or paths.
_KEPT = ("PATH", "LANG", "LC_ALL", "LC_CTYPE")


def _environment(extra: dict | None) -> dict:
    kept = {key: os.environ[key] for key in _KEPT if key in os.environ}
    kept["PYTHONIOENCODING"] = "utf-8"
    kept.update(extra or {})
    return kept


def _stopped_by(returncode: int | None, killed_for_time: bool, stderr: str) -> str | None:
    if killed_for_time:
        return "wall-clock"
    if returncode is None or returncode == 0:
        return None
    if returncode == -signal.SIGXCPU:
        return "cpu"
    # CPython ignores SIGXFSZ, so a program that writes past its file-size limit
    # is not killed by the signal: its write fails with EFBIG and it exits on the
    # exception. Both forms are the same event.
    if returncode == -signal.SIGXFSZ or "File too large" in stderr:
        return "output"
    if "MemoryError" in stderr or "not enough memory" in stderr:
        return "memory"
    if returncode < 0:
        # SIGKILL with no word about a limit, SIGSEGV, SIGABRT: the kernel's
        # address-space limit makes some allocations fail as a crash and not as
        # an exception, so a bare signal is reported as one.
        return "crash"
    return None


def run_isolated(argv: list, *, limits: Limits = Limits(), stdin_text: str = "",
                 cwd=None, env: dict | None = None) -> Outcome:
    """Run a program that plays a design, in a child no design can outlive.

    The child gets a session of its own (so a kill reaches everything it
    started), the kernel's limits, a scrubbed environment, and a clock outside
    it. Its output goes to files under a limit of their own and is read after it
    ends, so a child that prints without end fills a file to its cap and stops,
    and the supervisor never holds more than it chose to read."""
    started = time.monotonic()
    killed_for_time = False
    with tempfile.TemporaryDirectory(prefix="sce_run_") as scratch:
        out_path = os.path.join(scratch, "stdout")
        err_path = os.path.join(scratch, "stderr")
        with open(out_path, "wb") as out, open(err_path, "wb") as err:
            child = subprocess.Popen(
                argv, stdin=subprocess.PIPE, stdout=out, stderr=err, cwd=cwd,
                env=_environment(env), start_new_session=True,
                preexec_fn=_applying(limits))
            try:
                child.communicate(input=stdin_text.encode("utf-8"),
                                  timeout=limits.wall_seconds)
            except subprocess.TimeoutExpired:
                killed_for_time = True
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                child.wait()
        cap = limits.output_mb * 1024 * 1024
        with open(out_path, "rb") as out, open(err_path, "rb") as err:
            stdout = out.read(cap).decode("utf-8", errors="replace")
            stderr = err.read(1024 * 1024).decode("utf-8", errors="replace")
    returncode = child.returncode
    return Outcome(
        returncode=returncode, stdout=stdout, stderr=stderr,
        stopped_by=_stopped_by(returncode, killed_for_time, stderr),
        seconds=time.monotonic() - started, isolation=isolation_level())
