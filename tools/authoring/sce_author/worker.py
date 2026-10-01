"""The process that plays a design on behalf of `sandbox`.

    python -m sce_author.worker

It reads one JSON request per line from its standard input and answers each with
one JSON line on a private copy of its standard output. It is the only place the
generated module and the runtime beneath it are imported when `verify` or
`compare` drive a design, and it runs under the limits `process.Session` binds
it to; the server that started it never loads what it loads.

The protocol is small and generic on purpose: the verifier and the comparer
decide WHAT to ask of a machine, and this process knows nothing of statecharts.
An object that is not plain data stays here, in a table, and the parent is given
a number for it; a request names an object by that number.

    load       {built, document}            import the generated module; its reference
    getattr    {ref, name}                  an attribute (a method comes back as a `$m`)
    invoke     {ref, name, args, kwargs}    call a method, function or class of an object
    call       {ref, args, kwargs}          call an object
    vars       {ref}                        a copy of an object's attributes
    recorder   {}                           a sink for the machine's host-served sends
    procedure  {ref, name, args}            one of `procedures.PROCEDURES` on a module
    ping       {}

Every request may carry `release`, the numbers the parent no longer holds, and
they are dropped before the request is read.

⚠ Whatever the design prints must not reach the protocol. The reply goes to a
duplicate of the original standard output, and file descriptor 1 is then pointed
at standard error, so a `print` in generated code lands in the (capped) error
file, never in the middle of an answer.
"""

from __future__ import annotations

import inspect
import json
import os
import sys

from . import lowering, procedures, wire

#: The longest message text sent back for an exception. A design can raise an
#: error whose text is the whole of its memory.
MAX_MESSAGE = 4000


class _Method:
    """A callable named on its owner: sent as `{"$m": [owner, name]}`."""

    def __init__(self, owner: int, name: str) -> None:
        self.owner, self.name = owner, name


class Table:
    """The objects the parent holds numbers for. One number per object, for as long
    as the parent holds it: asking twice for the same object gives the same number,
    so identity survives the crossing (an enumeration member is still itself)."""

    def __init__(self) -> None:
        self._objects: dict = {}
        self._numbers: dict = {}
        self._next = 1

    def number(self, obj) -> int:
        found = self._numbers.get(id(obj))
        if found is not None and self._objects.get(found) is obj:
            return found
        number = self._next
        self._next += 1
        self._objects[number] = obj
        self._numbers[id(obj)] = number
        return number

    def get(self, number):
        try:
            return self._objects[number]
        except KeyError:
            raise wire.WireError(
                f"the parent named object {number}, which this process does not hold") from None

    def release(self, number) -> None:
        obj = self._objects.pop(number, None)
        if obj is not None and self._numbers.get(id(obj)) == number:
            del self._numbers[id(obj)]


class Worker:
    def __init__(self) -> None:
        self.table = Table()

    # -- values ------------------------------------------------------------

    def _hook(self, obj):
        if isinstance(obj, _Method):
            return {"$m": [obj.owner, obj.name]}
        copy = wire.snapshot(obj, self._hook)
        if copy is not None:
            return copy
        kind = "module" if inspect.ismodule(obj) else "object"
        return {"$ref": self.table.number(obj), "r": type(obj).__name__, "k": kind}

    def _encode(self, value):
        return wire.encode(value, self._hook)

    def _resolve(self, tag, data):
        if tag == "$ref":
            return self.table.get(data["$ref"])
        number, name = data["$m"]
        return getattr(self.table.get(number), name)

    def _decode(self, data):
        return wire.decode(data, self._resolve)

    # -- operations --------------------------------------------------------

    def load(self, request):
        import pathlib

        return lowering.load(pathlib.Path(request["built"]), pathlib.Path(request["document"]))

    def getattr_(self, request):
        owner = self.table.get(request["ref"])
        name = request["name"]
        value = getattr(owner, name)
        if inspect.isroutine(value) or inspect.isclass(value):
            # Named on its owner and not registered: a bound method is a new
            # object at every access, and a table that kept each would grow.
            return _Method(request["ref"], name)
        return value

    def invoke(self, request):
        owner = self.table.get(request["ref"])
        function = getattr(owner, request["name"])
        return function(*self._decode(request.get("args", [])),
                        **self._decode(request.get("kwargs", {})))

    def call(self, request):
        return self.table.get(request["ref"])(*self._decode(request.get("args", [])),
                                              **self._decode(request.get("kwargs", {})))

    def vars_(self, request):
        return dict(vars(self.table.get(request["ref"])))

    def recorder(self, request):
        return lowering.SendRecorder()

    def procedure(self, request):
        """Run one of this package's procedures (`procedures.PROCEDURES`) on a
        module the parent holds, with plain arguments, and return its result."""
        try:
            function = procedures.PROCEDURES[request["name"]]
        except KeyError:
            raise wire.WireError(f"no procedure called {request['name']!r}") from None
        return function(self.table.get(request["ref"]), **self._decode(request.get("args", {})))

    def ping(self, request):
        return None

    OPERATIONS = {"load": "load", "getattr": "getattr_", "invoke": "invoke", "call": "call",
                  "vars": "vars_", "recorder": "recorder", "procedure": "procedure",
                  "ping": "ping"}

    # -- one request -------------------------------------------------------

    def answer(self, request: dict) -> dict:
        for number in request.get("release", ()):
            self.table.release(number)
        reply = {"id": request.get("id")}
        try:
            operation = self.OPERATIONS.get(request.get("op"))
            if operation is None:
                raise wire.WireError(f"no operation called {request.get('op')!r}")
            reply["ok"] = self._encode(getattr(self, operation)(request))
        except BaseException as exc:  # noqa: BLE001 - a design may raise anything
            if isinstance(exc, KeyboardInterrupt):
                raise
            reply.pop("ok", None)
            reply["error"] = error_of(exc)
        return reply


def error_of(exc: BaseException) -> dict:
    """An exception as data: its name, every class it descends from, and its text.
    The parent re-raises the most specific class it recognises and a stand-in
    named like the original for the rest."""
    return {
        "type": type(exc).__name__,
        "mro": [f"{cls.__module__}.{cls.__name__}" for cls in type(exc).__mro__],
        "message": str(exc)[:MAX_MESSAGE],
    }


def serve(stdin, protocol) -> None:
    worker = Worker()
    for raw in stdin:
        raw = raw.strip()
        if not raw:
            continue
        try:
            request = json.loads(raw)
            reply = worker.answer(request)
        except (ValueError, TypeError) as exc:
            reply = {"id": None, "error": error_of(wire.WireError(f"unreadable request: {exc}"))}
        try:
            line = json.dumps(reply, separators=(",", ":"), allow_nan=False)
        except (ValueError, TypeError) as exc:
            line = json.dumps({"id": reply.get("id"),
                               "error": error_of(wire.WireError(f"unsendable answer: {exc}"))},
                              separators=(",", ":"))
        protocol.write(line.encode("utf-8") + b"\n")


def main() -> int:
    # The protocol channel is a private duplicate of standard output; fd 1 itself
    # is pointed at standard error before any design code is imported.
    protocol = os.fdopen(os.dup(1), "wb", buffering=0)
    os.dup2(2, 1)
    sys.stdout = sys.stderr
    serve(sys.stdin.buffer, protocol)
    return 0


if __name__ == "__main__":
    sys.exit(main())
