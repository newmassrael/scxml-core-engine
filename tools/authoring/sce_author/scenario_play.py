"""Play ONE scenario into a design, in a process of its own, and print what was seen.

This is the half of the scenario driver that touches the engine. It runs as a
child of `scenario_driver` (never in the process that serves a client), one
child per scenario, under the limits `process.run_isolated` sets, so that a
design that loops, re-sends itself or grows without end costs exactly the one
example it was playing: the supervisor stops the child and the rest keep their
verdicts. The caller never imports the engine or the Lua machine.

    python -m sce_author.scenario_play < request.json > result.json

The request names the generated module, the design, the scenario and what the
caller knows from the generator's manifest. The result is one JSON object:
`run` (the scenario's observations, or why it was refused and whether that is
the design's doing), and, because only a process that has loaded the module can
know them, `unreadable` and `data_unavailable`.

⚠ It reports what it saw and nothing else. The places where a driver is
tempted to fill a hole are closed on purpose.

    A run during which the engine raised an `error.*` event that no state
    answered is refused, not observed (W3C SCXML 3.12.2). The machine did not
    do what its document says: an entry block that meets an unresolvable
    `<send>` ends there (W3C SCXML 4.9), so a timer armed after it is never
    armed and the machine then "fails" an example about timing it was never
    allowed to keep. The refusal names the error and, when the design holds
    open decisions, which, because an open route is the usual cause.

    A run during which the engine stopped a macrostep that would not end is
    refused too (W3C SCXML 3.13). Every other reading of such a machine says it
    is fine, and it used to pass an example that says the machine waits.

    Virtual time moves one scheduled instant at a time, never in one jump. The
    engine dates a timer from the end of the move that fires it, so an example
    that says 600 ms and one that says 200 ms three times were two different
    runs of one machine.

    An input is delivered under the name the example gives it. The generated
    engines carry an event as the descriptor the design declares, and W3C SCXML
    3.12.1 lets `request.new` match a transition on `request`, so the shorter
    member used to be what `_event.name` reported. The longer name now rides
    with the event (`EventMetadata.name`) and is what the machine reads however
    it reads it: in a guard, through a helper function, through a variable, by
    a computed key.

    A send the driver has no sink for is refused, not dropped. It observes
    sends to host-served processors, and BasicHTTP sends it detects and
    refuses; a send to a mesh peer either raises an unanswered error or
    reaches nobody, and both end the run the same way.

    A send to `#_parent` is observed only when the interface the examples were
    accepted with routes an output through it. Without that the run has no
    parent, as a machine nothing invoked has none (W3C SCXML C.1); the driver
    never invents a caller. The run stops at the first send that reaches for
    one, found by the send itself and not by the `error.communication` it would
    raise (a machine that finishes in the same macrostep never handles that
    error, and its example then failed on an output nobody could send). When the
    specification leaves open who the caller is, the run is BLOCKED by that
    decision (`cause: decision`, naming it), which is neither a defect of the
    design nor a fact about the machine.

    An output declared with a route (`via`) counts only when the design sent
    it through exactly that route, and a design that sent it through another
    one is refused with both routes named. The interface is what the owner
    accepted; a design that leaves by a different door is not what the
    examples describe, and counting the event anyway would pass it.

    A design the engine cannot start is refused for every example, with what
    the engine said, never a traceback for the whole call.

What the machine sent while it started (the initial state's `<onentry>`) is
part of the first step's observation, since no step has run before it.
"""

from __future__ import annotations

import json
import pathlib
import sys
import xml.etree.ElementTree as ET

from .errors import VerifyError
from .lowering import (PARENT_TARGET, SendRecorder, Unplayable, advance,
                       decisions_behind_unanswered_error, endless_event_chain,
                       endless_macrostep, host_names as _host_names, load,
                       unanswered_error)


class _HttpSeen:
    """BasicHTTP sends the machine made. The driver does not observe them, so
    a run that made one is refused rather than reported without it."""

    def __init__(self) -> None:
        self.requests: list = []

    def __call__(self, request):
        self.requests.append(request)
        return None


class _Refusal(Exception):
    """One run ended because what it saw would not be the design's behaviour.
    Its text is what the owner reads.

    `cause` says whether another machine would refuse the same run: `design`
    when what the design did made the example unplayable, `decision` when the
    design leaves open a question the example needs answered. This process
    refuses only for those reasons; a refusal for the machine's sake (time,
    memory, a crash) is the supervisor's to make, because by then this process
    is gone."""

    def __init__(self, why: str, cause: str = "design") -> None:
        super().__init__(why)
        self.cause = cause


def _scalar(value) -> bool:
    return isinstance(value, (bool, int, float, str))


def _payload_of(event_data: str) -> dict:
    """The fields a sent event carried, as the machine's own serialisation of
    its `<param>`s and namelist (W3C SCXML 5.10). Anything that is not an
    object of plain values carries no field a scenario could name."""
    if not event_data:
        return {}
    try:
        value = json.loads(event_data)
    except ValueError:
        return {}
    if not isinstance(value, dict):
        return {}
    return {name: field for name, field in value.items() if _scalar(field)}


def _resolve_event(policy, name: str):
    """`(event, declared)`: the policy's event for a name and the name the
    design declares it under, falling back through dot-token prefixes the way
    the engine does for a name it does not declare: a transition on `door`
    answers `door.open` (W3C SCXML 3.12.1). `(None, None)` when no prefix of
    the name is declared."""
    parts = name.split(".")
    while parts:
        declared = ".".join(parts)
        event = policy.get_event_from_name(declared)
        if event is not None:
            return event, declared
        parts.pop()
    return None, None


class _ObservingParent:
    """The parent session a design that sends to `#_parent` needs, which a driver
    does not have: it records what the machine sends there, in the order it sent
    it and among the host-served sends, and answers nothing.

    ⚠ It exists only when the interface the examples were accepted with says an
    output leaves through `#_parent` (its `via`). A design that sends to its
    parent because a send needs a target, while the owner has not said who the
    caller is, would otherwise be played against a parent nobody chose, and the
    examples would pass or fail on the driver's invention. The engine gets one
    only on the owner's say-so, and a run without one is a run that says why.

    The engine delivers what a design sends to its parent by `append`ing
    `(event name, data)` to the queue it was given, which is all this has."""

    def __init__(self, sink: SendRecorder) -> None:
        self._sink = sink

    def append(self, entry) -> None:
        name, data = entry
        self._sink.sends.append(_ParentSend(name, data if isinstance(data, str) else ""))


class _ParentWatch:
    """Stands where a parent would be when the owner has named none, to learn
    whether the machine reaches for one. It records the event names it is handed
    and nothing else, and the run ends at the first of them.

    Without it a machine that sends to a parent that is not there raises
    `error.communication` (W3C SCXML C.1), and what that does to the run depends
    on what the machine does next: an error no state answers is counted and the
    run is refused, but a machine that finishes in the same macrostep never
    handles it, and the example then fails on an output that was never sent. The
    failure would read as the design's when the design was waiting for a caller.
    The send itself is the thing to detect, not the error it would raise.

    ⚠ The machine is run with this in place of no parent only up to the first
    send that reaches it, where the run stops, so nothing after it (which a real
    parent could have changed) is ever read as the design's behaviour."""

    def __init__(self) -> None:
        self.received: list[str] = []

    def append(self, entry) -> None:
        self.received.append(entry[0])


class _ParentSend:
    """What a send to the parent looks like to the code that reads host-served
    sends: the event, its serialised payload, and the route it took.

    The route is the engine's, not the interface's: the engine delivers to
    `#_parent` through the SCXML Event I/O Processor and no other (W3C SCXML
    C.1), so that is the type reported. An interface that names another type for
    its parent route is then refused with both routes named, where echoing the
    interface's own type back would make the check agree with itself."""

    def __init__(self, event_name: str, event_data: str) -> None:
        from sce_runtime.io_processors import SCXML_EVENT_PROCESSOR_URI

        self.event_name = event_name
        self.event_data = event_data
        self.processor_type = SCXML_EVENT_PROCESSOR_URI
        self.target = PARENT_TARGET


class _Machine:
    """One design, loaded, and one scenario to play into it."""

    def __init__(self, request: dict) -> None:
        self.routes = {name: tuple(route) for name, route in request["routes"].items()}
        self.parent_sends = request.get("parent_sends") or []
        # Whether the interface sends an output through the parent, which is the
        # owner's word that there is a caller to give the machine.
        self.routes_to_parent = any(via[1] == PARENT_TARGET for via in self.routes.values())
        self.data = request["data"]
        self.declared = request["declared"]
        self.opened = request["opened"]
        self.max_time_stops = request["max_time_stops"]
        self.manifest_unreadable = request["manifest_unreadable"]
        self.document = pathlib.Path(request["document"])
        self.readers: dict = {}
        self.data_unavailable = ""
        self.unreadable: dict = {}
        self.module = load(pathlib.Path(request["built"]), self.document)
        if self.data:
            try:
                self.readers = _host_names(self.module, "readers")
            except VerifyError as exc:
                self.data_unavailable = str(exc)
        if not self.data_unavailable:
            self.unreadable = {name: self._why_unreadable(name) for name in self.data
                               if name not in self.readers}

    def _why_unreadable(self, name: str) -> str:
        """Why the generated module has no reader for a data item the
        interface names, in the generator's words when it gave them."""
        record = next((r for r in self.manifest_unreadable if r.get("var") == name), None)
        if record:
            return (f"the generator gives `{name}` no reader ({record.get('reason')} in "
                    f"{record.get('language')}), and a reader exists in every backend or in none")
        declared = {node.get("id") for node in ET.parse(self.document).getroot().iter()
                    if node.tag.rsplit("}", 1)[-1] == "data" and node.get("id")}
        if name not in declared:
            return f"the design declares no data item called `{name}`"
        # Said as far as it is known: the generated module has no reader and
        # the generator's manifest gives no reason, which is the case of an
        # item declared with no initial value.
        return (f"the generated module has no reader for `{name}`, and the generator gives no "
                f"reason")

    def run(self, scenario: dict) -> dict:
        """The scenario's run: one observation per step, or why it was refused."""
        try:
            return {"scenario": scenario["id"], "observations": self._play(scenario["steps"])}
        except _Refusal as exc:
            return {"scenario": scenario["id"],
                    "refused": {"why": str(exc), "cause": exc.cause}}

    def _no_caller(self, watch: _ParentWatch, index: int) -> _Refusal:
        """The refusal of a run whose machine reached for a parent nobody named.

        A machine nothing invoked has no parent (W3C SCXML C.1), and the driver
        does not invent one: a parent chosen by the driver would pass or fail the
        example on words nobody wrote. So the run stops where the machine reaches
        for it, and says whose question that is. With a question open on the send
        it is the owner's, and the example is BLOCKED by it (`decision`). With
        none recorded it is the design's, which sent to a caller it never asked
        about, and the words say how to record one."""
        reached = list(dict.fromkeys(watch.received))
        named = ", ".join(f"`{event}`" for event in reached)
        # A computed event name (`eventexpr`) is a site the manifest lists
        # without one, so it belongs to whichever event was sent.
        decisions = list(dict.fromkeys(
            decision for site in self.parent_sends
            if site.get("event") is None or site["event"] in reached
            for decision in site.get("decisions", [])))
        if decisions:
            asked = ", ".join(f"`{decision}`" for decision in decisions)
            return _Refusal(
                f"by step {index} the design sent {named} to its parent, and the "
                f"specification leaves open who that is (open decision {asked}). The machine "
                f"was given no parent, and the driver does not invent one, so the example is "
                f"blocked by that decision: once the owner names the caller, give the output "
                f"a route through `#_parent` and the example can be played",
                cause="decision")
        return _Refusal(
            f"by step {index} the design sent {named} to its parent, and neither the "
            f"interface nor the specification's decisions say who that is. The machine "
            f"was given no parent, and the driver does not invent one. Either the "
            f"interface routes the output through `#_parent` (its `via`), or the design "
            f"records the open question on the send (`sce:unresolved`) so the example is "
            f"blocked by it and not failed")

    def _play(self, steps: list) -> list:
        from sce_runtime.event import EventMetadata

        sink, http = SendRecorder(), _HttpSeen()
        watch = None
        try:
            engine = self.module.create_engine()
            # Before `initialize()`: a send in the initial state's `<onentry>` is
            # the first thing that reaches a parent.
            if self.routes_to_parent:
                engine.policy._parent_queue = _ObservingParent(sink)
            elif self.parent_sends:
                watch = _ParentWatch()
                engine.policy._parent_queue = watch
            for processor in self.declared:
                engine.register_event_processor(processor, sink)
            engine.set_http_send_callback(http)
            engine.initialize()
        except Exception as exc:  # noqa: BLE001 - a design that cannot start is the answer
            # An answer for the example, not a traceback for the whole call: the
            # generated parent of a design that starts a child session imports
            # the child's module by a bare name this loader does not put on the
            # path, so it dies here with ModuleNotFoundError.
            raise _Refusal(f"the engine could not start the design: "
                           f"{type(exc).__name__}: {exc}") from exc
        policy = engine.policy
        if watch is not None and watch.received:
            raise self._no_caller(watch, 0)
        observations = []
        for index, step in enumerate(steps):
            try:
                if "send" in step:
                    event, declared = _resolve_event(policy, step["send"])
                    if event is None:
                        raise _Refusal(f"step {index} sends `{step['send']}`, and the design "
                                       f"names no event that answers it, so the example's "
                                       f"input reaches nothing in it")
                    # W3C SCXML 5.10: the machine is told the name the event
                    # arrived under. The event itself is the enumeration member
                    # of the descriptor the design declares (3.12.1), so a
                    # longer name rides beside it, whatever a guard or a helper
                    # function does with `_event.name`.
                    carried = step["send"] if declared != step["send"] else ""
                    engine.send_event(event, EventMetadata(data=step.get("payload") or "",
                                                           name=carried))
                elif "advance_ms" in step:
                    self._advance(engine, step["advance_ms"], index)
            except _Refusal:
                raise
            except Exception as exc:  # noqa: BLE001 - the engine failing is the report
                raise _Refusal(f"the engine raised {type(exc).__name__} at step {index}: {exc}")
            if watch is not None and watch.received:
                raise self._no_caller(watch, index)
            self._check_unobserved(engine, policy, http, index)
            observations.append(self._observe(engine, policy, sink, index))
        return observations

    def _advance(self, engine, ms: int, index: int) -> None:
        """Move virtual time forward by `ms`, the way `lowering.advance` moves it:
        one scheduled instant at a time, never in one jump, so the same time
        passes the same way however an example splits it."""
        try:
            advance(engine, ms, self.max_time_stops)
        except Unplayable as exc:
            raise _Refusal(f"step {index} {exc}, so the example cannot be played to "
                           f"its end") from exc

    def _check_unobserved(self, engine, policy, http: _HttpSeen, index: int) -> None:
        """Refuse the run when it did something this driver cannot report."""
        if http.requests:
            request = http.requests[0]
            raise _Refusal(f"at step {index} the design sent `{request.event_name}` over "
                           f"BasicHTTP to `{request.target}`; this driver observes sends to "
                           f"host-served processors and does not observe HTTP")
        endless = endless_macrostep(engine, policy)
        if endless is not None:
            raise _Refusal(f"by step {index} the engine stopped {endless}, so where the "
                           f"machine stands is not the design's behaviour")
        chain = endless_event_chain(engine, policy)
        if chain is not None:
            raise _Refusal(f"by step {index} the engine stopped {chain}, so where the "
                           f"machine stands is not the design's behaviour")
        unanswered = unanswered_error(engine, policy)
        if unanswered is not None:
            decisions = decisions_behind_unanswered_error(engine)
            if decisions:
                # The send that failed chose its route from data the specification
                # leaves open: the failure is the owner's question showing through.
                # Another machine fails the same way, and so does any design that
                # leaves the question open, so the example is blocked by it and
                # not failed by the design (`cause: decision`).
                asked = ", ".join(f"`{decision}`" for decision in decisions)
                raise _Refusal(
                    f"by step {index} the engine {unanswered}, and the send that failed chose "
                    f"where it goes from data the specification leaves open (open decision "
                    f"{asked}). The machine was given no answer, and the driver does not invent "
                    f"one, so the example is blocked by that decision: once the owner answers "
                    f"it, the example can be played.{self.opened}", cause="decision")
            raise _Refusal(f"by step {index} the engine {unanswered}, so what the machine did "
                           f"from there is not the design's behaviour.{self.opened}")

    def _observe(self, engine, policy, sink: SendRecorder, index: int) -> dict:
        outbound = []
        for request in sink.take():
            via = self.routes.get(request.event_name)
            route = (request.processor_type, request.target)
            if via is not None and route != via:
                raise _Refusal(
                    f"at step {index} the design sent `{request.event_name}` through "
                    f"`{route[0]}` to `{route[1]}`, and the interface says it leaves through "
                    f"`{via[0]}` to `{via[1]}`")
            sent = {"event": request.event_name}
            payload = _payload_of(request.event_data)
            if payload:
                sent["payload"] = payload
            outbound.append(sent)
        return {
            "outbound": outbound,
            "finished": bool(engine.reached_final),
            "configuration": sorted(policy.get_state_name(s)
                                    for s in engine.active_configuration()),
            "data": self._read_data(policy),
        }

    def _read_data(self, policy) -> dict:
        values = {}
        for name in self.data:
            method = self.readers.get(name)
            reader = getattr(policy, method, None) if method else None
            if reader is None:
                continue
            try:
                value = reader()
            except Exception:  # noqa: BLE001 - an unreadable name is a gap, said by the judge
                continue
            if _scalar(value):
                values[name] = value
        return values


def play(request: dict) -> dict:
    """The result for one request. Never raises for what a design does."""
    scenario = request["scenario"]
    try:
        machine = _Machine(request)
    except VerifyError as exc:
        # The generator reported success and wrote code that does not import:
        # the same on every machine, and the product's to repair.
        return {"run": {"scenario": scenario["id"],
                        "refused": {"why": str(exc), "cause": "design"}},
                "unreadable": {}, "data_unavailable": ""}
    return {"run": machine.run(scenario), "unreadable": machine.unreadable,
            "data_unavailable": machine.data_unavailable}


def main() -> int:
    print(json.dumps(play(json.load(sys.stdin)), ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
