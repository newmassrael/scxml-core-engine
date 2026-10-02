"""Load what the product generated for a document, and record what it sends.

This is the small half of `verify` that touches generated code: importing a
generated module as a package, finding which of the files the generator wrote IS
the document, reading the names the module publishes, and the sink a host
registers to hear the machine's sends. It is its own module so that the process
which PLAYS a design (`worker`, `scenario_play`) imports these and nothing of
the verifier: a design's machine is started in a process that holds the
minimum, and the server that serves the client holds none of it.

⚠ Nothing here is called in the server process any more. `verify` and `compare`
reach a generated module through `sandbox`, which starts `worker` in a child
under limits; `load` runs in that child. The server's own `sys.modules` and
`sys.path` stay as they were.
"""

from __future__ import annotations

import importlib.util
import pathlib
import re
import sys

from .errors import VerifyError


def default_runtime() -> pathlib.Path:
    """Where the product's Python runtime sits in this tree, by default.

    Derived the way the generator's location is, and for the same reason: a
    constant would be a home this package is not entitled to have.
    """
    root = pathlib.Path(__file__).resolve().parents[3]
    return root / "backends" / "python" / "runtime"


class SendRecorder:
    """Every host-served send the machine made, in the order it made them."""

    def __init__(self) -> None:
        self.sends: list = []

    def __call__(self, request):
        self.sends.append(request)
        # W3C SCXML 6.2.5 lets a handler answer with the events the act
        # produced. A verifier answers with none: inventing a reply would
        # drive the machine on words no record contains, and the verdict
        # would be about a run the examples did not describe.
        return []

    def take(self) -> list:
        """The sends since the last call, and reset. One case, one reading."""
        taken, self.sends = self.sends, []
        return taken


#: The target a `<send>` names to reach the session that invoked this one. The
#: engine delivers it itself, through the SCXML Event I/O Processor, so a route
#: to it is never one a host serves (W3C SCXML C.1).
PARENT_TARGET = "#_parent"

#: How many scheduled instants one move of virtual time may stop at before the
#: move is given up. A timer that re-arms at zero delay never ends, and a
#: heartbeat every millisecond over an hour is not worth playing.
MAX_TIME_STOPS = 50_000


class Unplayable(Exception):
    """The engine will not play a design any further, and the words say what it did.

    They start at the verb, so the caller puts its own subject in front: a
    scenario says which step, a comparison says which drive. A design's machine
    that does this does it on any host, which is what separates it from a machine
    that ran out of time or memory."""


def advance(engine, ms: int, max_stops: int = MAX_TIME_STOPS) -> None:
    """Move virtual time forward by `ms`, one scheduled instant at a time.

    W3C SCXML 6.2: a delay is measured from when its `<send>` executes. The
    engine's clock belongs to the host and `advance_time(ms)` sets it to the
    end of the move before it runs what fell due, so a timer armed while
    handling a deadline in the middle of a long move is dated from the end
    of the move. Measured: the retry machine passed three moves of 200 ms
    and failed one of 600 ms. The engine says how far the next deadline is
    (`time_until_next_scheduled_ms`), the product's own answer to a host
    that would otherwise guess a step size, so the move is cut there and
    the same time passes the same way however a caller splits it.

    This is the one place that rule is written: a scenario and a comparison both
    move time through it, and a comparison that jumped where a scenario walked
    told the owner that two drafts of one prose differ when they do not.

    A run that ends, or a machine that has finished, owes the rest no
    deadlines."""
    remaining, stops = ms, 0
    while remaining > 0 and engine.is_running and not engine.reached_final:
        due = engine.time_until_next_scheduled_ms()
        step = remaining if due is None or due > remaining else due
        engine.advance_time(step)
        remaining -= step
        stops += 1
        if stops > max_stops:
            raise Unplayable(
                f"lets {ms} ms pass and the design has a deadline at more than {max_stops} "
                f"of its instants (a timer that re-arms at zero delay never ends)")


def endless_macrostep(engine, policy) -> str | None:
    """What the engine stopped, when it stopped a macrostep that would not end.

    W3C SCXML 3.13: a macrostep may not terminate, and the engine stops one after
    a ceiling. Every other reading of the machine says it is fine (it runs, it
    names a state, the call returned), which is how an endless chain used to pass
    an example that says the machine waits, and how a comparison judged it equal
    to one that waits. None when no macrostep was stopped."""
    if not engine.truncated_macrosteps():
        return None
    state = engine.last_truncated_macrostep_state()
    where = f" in `{policy.get_state_name(state)}`" if state is not None else ""
    return f"a macrostep{where} that did not reach a stable configuration (W3C SCXML 3.13)"


def endless_event_chain(engine, policy) -> str | None:
    """What the engine stopped, when it handed a call back with external events
    still queued: a machine that answers each event by sending itself the next.

    Every macrostep of such a machine ends, so `endless_macrostep` never sees it,
    and the engine call did not return: the driver's only bound was its
    processor-time limit, which stopped the run after 25 s and called it the
    machine's doing (`environment`) when a design that does this does it on every
    host (measured 2026-10-02). The engine takes at most
    `max_external_events_per_call` events in one call and counts the call it
    handed back, so the verdict is the same on any host and arrives at once.
    None when no call was handed back."""
    if not engine.truncated_event_chains():
        return None
    last = engine.last_truncated_event()
    still = f", still taking `{policy.get_event_name(last)}`" if last is not None else ""
    return (f"a chain of external events that did not end: one call took "
            f"{engine.max_external_events_per_call()} of them and more were queued{still}")


def unanswered_error(engine, policy) -> str | None:
    """What the engine raised that no state answered, as a clause after "the engine".

    W3C SCXML 3.12.2: a failed `<send>` or expression raises `error.*` on the
    internal queue, and a machine that has no transition for it carries on. An
    entry block that meets an unresolvable `<send>` ends there (W3C SCXML 4.9), so
    a timer armed after it is never armed, and the machine then does something its
    document does not say. Every other reading of the run (it runs, it names a
    state, the call returned) says it is fine, which is how a machine that raised
    was classed with one that did not. None when nothing went unanswered."""
    failures = engine.unhandled_error_events() + engine.error_cascade_events()
    if not failures:
        return None
    last = engine.last_unhandled_error()
    if last is None:
        last = engine.last_error_cascade_event()
    name = policy.get_event_name(last) if last is not None else "error"
    return f"raised `{name}` and no state answered it"


def decisions_behind_unanswered_error(engine) -> tuple:
    """The open decisions the ROUTE of the `<send>` behind an unanswered error
    rested on: the ids the generated machine raised the route's failure with, for
    a send whose route (`typeexpr`, `targetexpr`) is chosen at run time from data
    the specification leaves open (W3C SCXML 6.2.4).

    An unanswered failure of such a route is the owner's question showing
    through, not a fault the design could have avoided: the draft wrote a route
    because a send needs one, and nobody has said where it goes. Empty when
    nothing went unanswered and when the error was not a failure of such a route
    (the same send failing in its event name or its delay is the draft's).
    Empty says the machine named no question, not that none exists: the caller
    treats the error as it treats every unanswered error, and does not guess a
    decision from the shape of the failure."""
    if engine.last_unhandled_error() is None:
        return ()
    return tuple(engine.last_unhandled_error_rests_on())


def route_decision_clause(decisions: tuple) -> str:
    """The words every consumer of a run uses to say a failure was a send's route
    resting on an open decision, so the owner is asked the question and the draft
    is not mended for a fault it does not have. One spelling: the scenario driver,
    a comparison and a verification all say it, and a second sentence would drift
    from the first."""
    asked = ", ".join(f"`{decision}`" for decision in decisions)
    return (f"the send that failed chose where it goes from data the specification "
            f"leaves open (open decision {asked})")


def stopped_run(engine, policy) -> str | None:
    """Why the engine's run is not the design's behaviour, as a clause after "the
    engine": it stopped a macrostep that would not end, it handed a call back
    from a chain of external events that would not end, or it raised an error
    nothing answered. None when none happened.

    ⚠ The one place that says so, for every consumer that reads a verdict off a
    run: a scenario, a comparison and a verification. Each of them used to carry
    some of these rules and none carried all, and the review that found one gap
    found the next in the path beside it. A consumer that plays a design asks
    here, after it starts the machine and after every step it takes."""
    endless = endless_macrostep(engine, policy)
    if endless is not None:
        return f"stopped {endless}"
    chain = endless_event_chain(engine, policy)
    if chain is not None:
        return f"stopped {chain}"
    unanswered = unanswered_error(engine, policy)
    if unanswered is None:
        return None
    # A failure of a route that rests on an open decision is the owner's to
    # answer, and a comparison or a verification that only said "raised and no
    # state answered it" sent the draft back to be mended for a fault it does not
    # have. Only the machine's own naming of the question counts (no guessing).
    decisions = decisions_behind_unanswered_error(engine)
    if decisions:
        return f"{unanswered}, and {route_decision_clause(decisions)}"
    return unanswered


def host_names(module, part: str) -> dict:
    """The names the generated module gives one part of the document's
    surface, keyed by the id the document wrote.

    ⚠ Read from the module, never re-derived. This used to be `_snake`, a
    copy of the generator's casing rule, and a copy is the half that stops
    agreeing: it missed the `.` and `-` the generator folds (`screen-rules`
    reads through `screen_rules`), and a keyword the generator escapes
    (`pass` reads through `pass_`). The generator publishes what it chose as
    `SCE_HOST_NAMES`, from the same calls it spells the definitions with.
    """
    table = getattr(module, "SCE_HOST_NAMES", None)
    if not isinstance(table, dict) or not isinstance(table.get(part), dict):
        raise VerifyError(
            f"the generated module carries no SCE_HOST_NAMES[{part!r}], so the "
            f"names it gives the document's {part} are not known. It was "
            f"generated by a code generator older than this verifier; rebuild "
            f"it, or name a current one with --codegen")
    return table[part]


def _plain(name: str) -> str:
    """A name with its separators gone: `door-with-auto-close` and
    `door_with_auto_close` are the same document."""
    return re.sub(r"[^a-z0-9]", "", name.lower())


def generated_module_of(emitted: list, document: pathlib.Path) -> pathlib.Path:
    """Which of the files the generator wrote IS the document.

    ⚠ The generator names a statechart's module `<stem>_sm.py` and any other
    kind's `<stem>.py`, and writes the module of every child session a design
    starts, or of every document it imports, beside it. This used to compare
    the document's stem with each file's whole stem, which is true for the
    second naming and never for the first, and then take the first file the
    directory listing returned for a statechart: right while there was one
    file, and while there were two a coin toss that answered for the child.
    A lone file is the document's whatever it is called; several and none of
    them the document's is refused, because guessing here means judging a
    machine nobody wrote."""
    wanted = _plain(document.stem)
    for path in emitted:
        stem = path.stem[: -len("_sm")] if path.stem.endswith("_sm") else path.stem
        if _plain(stem) == wanted:
            return path
    if len(emitted) == 1:
        return emitted[0]
    raise VerifyError(
        f"{document.name}: the generator wrote {', '.join(sorted(p.name for p in emitted))} "
        f"and none of them is named for the document, so there is no telling which to run")


def load(into: pathlib.Path, document: pathlib.Path):
    """Import what was generated, as a package so its own imports resolve."""
    # ⚠ A generated statechart imports the product's own runtime; a generated
    # pure computation does not, which is why nothing needed this until one
    # was driven. Without it the import died with `No module named
    # 'sce_runtime'` -- a traceback out of a verifier, about the verifier's
    # environment rather than about the document it was asked to judge.
    runtime = default_runtime()
    if runtime.is_dir() and str(runtime) not in sys.path:
        sys.path.insert(0, str(runtime))
    emitted = [p for p in into.glob("*.py") if p.name != "__init__.py"]
    if not emitted:
        raise VerifyError(f"{document}: the generator wrote no python")
    main = generated_module_of(emitted, document)
    (into / "__init__.py").write_text("", encoding="utf-8")
    sys.path.insert(0, str(into.parent))
    # ⚠ A statechart that starts a child session imports the child's module by its
    # bare name (`import child_sm`, at the moment it starts the child). The
    # generator writes a design's modules into one flat directory and expects a
    # host to put that directory on its path; this loader is that host. It is safe
    # here, and not in a long-lived server, because a process loads ONE design: a
    # scenario's child process, a verification's worker, a draft's worker, so no
    # second design's `child_sm` can answer for the first's.
    if str(into) not in sys.path:
        sys.path.insert(0, str(into))
    parent_spec = importlib.util.spec_from_file_location(
        into.name, into / "__init__.py", submodule_search_locations=[str(into)])
    parent = importlib.util.module_from_spec(parent_spec)
    sys.modules[into.name] = parent
    parent_spec.loader.exec_module(parent)
    spec = importlib.util.spec_from_file_location(
        f"{into.name}.{main.stem}", main, submodule_search_locations=[str(into)])
    module = importlib.util.module_from_spec(spec)
    sys.modules[f"{into.name}.{main.stem}"] = module
    # ⚠ An answer, not a traceback. The generator reported success and wrote
    # code that does not import -- measured 2026-09-23, a document with a
    # multi-target transition came back `target=State.A B`, and `verify` died
    # on the SyntaxError with nothing saying whose fault it was. What failed
    # is the product's lowering of a document it accepted; the report says
    # that, and where.
    try:
        spec.loader.exec_module(module)
    except Exception as exc:  # noqa: BLE001 - any import failure is the same answer
        where = (f"{main.name}:{exc.lineno}" if isinstance(exc, SyntaxError)
                 and exc.lineno else main.name)
        raise VerifyError(
            f"the product generated python for {document.name} that "
            f"does not import ({where}: {type(exc).__name__}: {exc}). The "
            f"generator accepted the document and wrote code it cannot run; "
            f"that is a defect in the generator's lowering, not a verdict on "
            f"the document") from exc
    return module
