"""Reach a generated module that lives in another process.

`verify` and `compare` used to import the module a design was lowered to and call
it: `module.create_engine()`, `engine.send_event(...)`, `engine.advance_time(...)`.
That put a design's machine in the server's own process, where a loop in it kept
the call from ever returning and a crash in it took the server with it. They now
call the same names on objects of this module, which look like the originals and
are not: each is a reference to an object held by `worker`, a process that
`process.Session` binds to the kernel's limits and a clock.

    module = sandbox.load_module(built_dir, document)
    engine = module.create_engine()          # a reference, not an engine
    recorder = module.new_recorder()
    engine.register_event_processor("x-host", recorder)
    engine.initialize()
    engine.send_event(engine.policy.get_event_from_name("go"))
    recorder.take()                           # copies of the sends, as plain data

What comes back is data or a reference. Plain values are copied, a dataclass is
copied field by field, and anything else (an engine, an enumeration member) stays
in the worker and is held here by a number. An attribute that is a method comes
back as one that can be called. That is the whole surface; it is enough because
the callers only ever read results and pass them back.

⚠ A stopped worker is not an exception the callers' `except Exception` should
eat. `WorkerStopped` is a `VerifyError` so the verifier turns it into a refusal
and nothing else catches it by accident, but the comparer says so explicitly: a
machine that did not answer is not a machine that raised.

⚠ Every exchange has a clock. A worker that does not answer in
`Limits.wall_seconds` is killed with everything it started, and every later call
on anything it held raises the same `WorkerStopped`.
"""

from __future__ import annotations

import builtins
import json
import pathlib
import sys
import threading
import weakref

from . import process, wire
from .errors import VerifyError

#: Longest the worker may take to import a module. Importing starts the Lua
#: machine, which is slow on a loaded host; it is not a step of a design.
LOAD_SECONDS = 120.0

_PACKAGE_ROOT = str(pathlib.Path(__file__).resolve().parents[1])

#: Built-in exceptions a design's code can raise that the callers already tell
#: apart. Anything else is re-raised as a stand-in named like the original, so a
#: report that prints `type(exc).__name__` prints what the design raised.
_RECOGNISED = frozenset({
    "TypeError", "ValueError", "KeyError", "IndexError", "AttributeError", "LookupError",
    "ArithmeticError", "ZeroDivisionError", "OverflowError", "AssertionError", "RuntimeError",
    "RecursionError", "NotImplementedError", "MemoryError", "StopIteration", "NameError",
    "UnicodeError", "OSError", "SyntaxError", "ImportError"})


class WorkerStopped(VerifyError):
    """The process that plays the design was stopped, and why.

    Not about one example or one step: the machine it held is gone. `cause` is
    always `environment` here, because what stopped it was the clock, the memory,
    the output cap or a crash; whether the design provoked it is not something a
    process that is no longer there can say."""

    cause = "environment"

    def __init__(self, stopped: process.SessionStopped) -> None:
        super().__init__(f"the process that plays the design stopped: {stopped.said}")
        self.stopped_by = stopped.stopped_by
        self.said = stopped.said


class RemoteError(Exception):
    """The design raised something the callers have no name for."""


_STAND_INS: dict = {}


def _stand_in(name: str, base):
    """An exception class called `name`, so `type(exc).__name__` reads as it did."""
    key = (name, base)
    if key not in _STAND_INS:
        bases = (RemoteError,) if base is None else (RemoteError, base)
        _STAND_INS[key] = type(name, bases, {})
    return _STAND_INS[key]


def _raise_remote(error: dict):
    message = error.get("message", "")
    mro = error.get("mro", [])
    if "sce_author.errors.VerifyError" in mro or "sce_author.wire.WireError" in mro:
        return VerifyError(message)
    for name in mro:
        module, _, cls = name.rpartition(".")
        if module == "builtins" and cls in _RECOGNISED:
            base = getattr(builtins, cls)
            if error.get("type") == cls:
                return base(message)
            return _stand_in(error.get("type", cls), base)(message)
    return _stand_in(error.get("type", "Exception"), None)(message)


class _Lease:
    """The parent's hold on one object in the worker.

    The object stays in the worker for as long as something here can still ask
    for it, and that is not one thing: a `Remote`, and every `RemoteMethod` found
    on it, share one lease, so a method outlives a `Remote` that was only a
    stepping stone (`load().rec()`). The last one to let go releases the number,
    and the release rides on the next request. Held in the worker's own table by
    number, never by identity, so there is nothing here for a reference cycle to
    keep alive."""

    __slots__ = ("worker", "number", "__weakref__")

    def __init__(self, worker: "Worker", number: int) -> None:
        self.worker, self.number = worker, number

    def __del__(self):
        try:
            self.worker.forget(self.number)
        except Exception:  # noqa: BLE001 - interpreter shutdown
            pass


class Remote:
    """An object in the worker. Attributes come back as data, references or
    methods; calling one is an exchange. Never copied: two `Remote`s are equal
    when they name the same object of the same worker."""

    __slots__ = ("_lease", "_methods", "__weakref__")

    def __init__(self, lease: _Lease) -> None:
        object.__setattr__(self, "_lease", lease)
        object.__setattr__(self, "_methods", {})

    @property
    def _worker(self) -> "Worker":
        return self._lease.worker

    @property
    def _number(self) -> int:
        return self._lease.number

    def __getattr__(self, name: str):
        # Reached only for a name that is not one of the slots or properties. A
        # slot that was never set (an object whose constructor failed) must not
        # come back here asking for itself.
        if (name.startswith("__") and name.endswith("__")) or name in ("_lease", "_methods"):
            raise AttributeError(name)
        known = self._methods.get(name)
        if known is not None:
            return known
        found = self._worker.request("getattr", ref=self._number, name=name)
        if isinstance(found, RemoteMethod):
            found._lease = self._lease
            # A method of this object stays a method: asking again is an exchange
            # that can only give the same answer. Were the attribute ever to
            # stop being callable, calling it fails with the design's own
            # TypeError rather than quietly meaning something else.
            self._methods[name] = found
        return found

    def __call__(self, *args, **kwargs):
        return self._worker.request("call", ref=self._number, args=self._worker.arguments(args),
                                    kwargs=self._worker.arguments(kwargs))

    def __eq__(self, other) -> bool:
        return (isinstance(other, Remote) and other._worker is self._worker
                and other._number == self._number)

    def __hash__(self) -> int:
        return hash((id(self._worker), self._number))

    def __repr__(self) -> str:
        return f"<remote object {self._number}>"


class RemoteModule(Remote):
    """The generated module. Besides its own attributes it can make a sink for the
    sends a machine makes, which has to live where the machine does."""

    __slots__ = ()

    def new_recorder(self) -> Remote:
        return self._worker.request("recorder")

    def procedure(self, name: str, **arguments):
        """Run one of this package's procedures (`procedures.PROCEDURES`) next to
        the module, with plain arguments, and get its plain result in one
        exchange. For a loop too chatty to run a step at a time across the
        boundary."""
        return self._worker.request("procedure", ref=self._number, name=name,
                                    args=self._worker.arguments(arguments))

    def close(self) -> None:
        self._worker.close()


class RemoteMethod:
    """A method, function or class of a remote object.

    ⚠ It keeps the object it belongs to alive, through the owner's lease.
    Without that, `load().rec()` lets the temporary module go the moment the
    method is found, the release of its number rides on the very request that
    calls the method, and the worker drops the module before it runs the call."""

    __slots__ = ("_lease", "_name")

    def __init__(self, lease: _Lease, name: str) -> None:
        self._lease, self._name = lease, name

    @property
    def _worker(self) -> "Worker":
        return self._lease.worker

    @property
    def _number(self) -> int:
        return self._lease.number

    def __call__(self, *args, **kwargs):
        return self._worker.request("invoke", ref=self._number, name=self._name,
                                    args=self._worker.arguments(args),
                                    kwargs=self._worker.arguments(kwargs))

    def __repr__(self) -> str:
        return f"<remote method {self._name} of {self._number}>"


class Worker:
    """One supervised worker process, and the references into it."""

    def __init__(self, limits: process.Limits | None = None) -> None:
        # Decided when the worker starts, not when this module is imported, so a
        # caller (or a test) can change `process.SESSION_LIMITS` and mean it.
        self.session = process.Session(
            [sys.executable, "-m", "sce_author.worker"],
            limits=limits or process.SESSION_LIMITS, env={"PYTHONPATH": _PACKAGE_ROOT})
        self._leases: "weakref.WeakValueDictionary" = weakref.WeakValueDictionary()
        self._unheld: set = set()
        self._counter = 0
        self._lock = threading.Lock()

    # -- references --------------------------------------------------------

    def _lease(self, number: int) -> _Lease:
        """The hold on `number`, made if there is none. A number the parent was
        about to give up is held again: the worker has just handed out the same
        object under it, so the pending release is withdrawn."""
        lease = self._leases.get(number)
        if lease is None:
            lease = _Lease(self, number)
            self._leases[number] = lease
        self._unheld.discard(number)
        return lease

    def _reference(self, tag: str, data: dict):
        if tag == "$m":
            number, name = data["$m"]
            return RemoteMethod(self._lease(number), name)
        number = data["$ref"]
        return (RemoteModule if data.get("k") == "module" else Remote)(self._lease(number))

    def forget(self, number: int) -> None:
        self._unheld.add(number)

    def arguments(self, value):
        """`value` ready to send: plain data as it is, references by number."""
        def hook(obj):
            if isinstance(obj, Remote):
                return {"$ref": obj._number}
            if isinstance(obj, RemoteMethod):
                return {"$m": [obj._number, obj._name]}
            raise wire.WireError(
                f"a {type(obj).__name__} cannot be sent to the process that plays the design: "
                f"only plain values and objects that process holds can")
        return wire.encode(value, hook)

    # -- exchanges ---------------------------------------------------------

    def request(self, operation: str, timeout: float | None = None, **fields):
        with self._lock:
            self._counter += 1
            number = self._counter
        release = []
        while self._unheld:
            try:
                release.append(self._unheld.pop())
            except KeyError:
                break
        message = {"id": number, "op": operation, "release": release, **fields}
        line = json.dumps(message, separators=(",", ":"), allow_nan=False).encode("utf-8")
        try:
            raw = self.session.exchange(line, timeout)
        except process.SessionStopped as stopped:
            raise WorkerStopped(stopped) from None
        try:
            reply = json.loads(raw)
            if not isinstance(reply, dict) or reply.get("id") != number:
                raise ValueError("an answer to another request")
        except ValueError as exc:
            # A process that answers in something that is not the protocol is not
            # one to go on asking.
            raise WorkerStopped(self.session.fail(f"it answered outside the protocol: {exc}")) \
                from None
        if "error" in reply:
            raise _raise_remote(reply["error"])
        try:
            return wire.decode(reply.get("ok"), self._reference)
        except (wire.WireError, KeyError, TypeError, ValueError) as exc:
            raise WorkerStopped(self.session.fail(f"it sent a value the protocol cannot carry: {exc}")) \
                from None

    def close(self) -> None:
        self.session.close()


def load_module(built: pathlib.Path, document: pathlib.Path, *,
                limits: process.Limits | None = None) -> RemoteModule:
    """Start a worker and have it import what the generator wrote for `document`.

    The worker is closed when the enclosing `process.reaping()` block ends, or
    by `module.close()`."""
    worker = Worker(limits)
    try:
        return worker.request("load", LOAD_SECONDS, built=str(built), document=str(document))
    except BaseException:
        worker.close()
        raise


def state_of(obj) -> dict:
    """A copy of an object's attributes, as `vars(obj)` would give for a local one."""
    if isinstance(obj, Remote):
        return obj._worker.request("vars", ref=obj._number)
    return dict(vars(obj))
