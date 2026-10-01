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
    refuses; a send to a parent session or a mesh peer either raises an
    unanswered error or reaches nobody, and both end the run the same way.

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
from .lowering import SendRecorder, host_names as _host_names, load


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
    when what the design did made the example unplayable. This process refuses
    only for that reason; a refusal for the machine's sake (time, memory, a
    crash) is the supervisor's to make, because by then this process is gone."""

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


class _Machine:
    """One design, loaded, and one scenario to play into it."""

    def __init__(self, request: dict) -> None:
        self.routes = {name: tuple(route) for name, route in request["routes"].items()}
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

    def _play(self, steps: list) -> list:
        from sce_runtime.event import EventMetadata

        sink, http = SendRecorder(), _HttpSeen()
        try:
            engine = self.module.create_engine()
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
            self._check_unobserved(engine, policy, http, index)
            observations.append(self._observe(engine, policy, sink, index))
        return observations

    def _advance(self, engine, ms: int, index: int) -> None:
        """Move virtual time forward by `ms`, one scheduled instant at a time.

        W3C SCXML 6.2: a delay is measured from when its `<send>` executes. The
        engine's clock belongs to the host and `advance_time(ms)` sets it to the
        end of the move before it runs what fell due, so a timer armed while
        handling a deadline in the middle of a long move is dated from the end
        of the move. Measured: the retry machine passed three moves of 200 ms
        and failed one of 600 ms. The engine says how far the next deadline is
        (`time_until_next_scheduled_ms`), the product's own answer to a host
        that would otherwise guess a step size, so the move is cut there and
        the same time passes the same way however an example splits it.

        A run that ends, or a machine that has finished, owes the rest no
        deadlines."""
        remaining, stops = ms, 0
        while remaining > 0 and engine.is_running and not engine.reached_final:
            due = engine.time_until_next_scheduled_ms()
            step = remaining if due is None or due > remaining else due
            engine.advance_time(step)
            remaining -= step
            stops += 1
            if stops > self.max_time_stops:
                raise _Refusal(
                    f"step {index} lets {ms} ms pass and the design has a deadline at more "
                    f"than {self.max_time_stops} of its instants (a timer that re-arms at zero "
                    f"delay never ends), so the example cannot be played to its end")

    def _check_unobserved(self, engine, policy, http: _HttpSeen, index: int) -> None:
        """Refuse the run when it did something this driver cannot report."""
        if http.requests:
            request = http.requests[0]
            raise _Refusal(f"at step {index} the design sent `{request.event_name}` over "
                           f"BasicHTTP to `{request.target}`; this driver observes sends to "
                           f"host-served processors and does not observe HTTP")
        if engine.truncated_macrosteps():
            # W3C SCXML 3.13: a macrostep may not terminate, and the engine
            # stops one after a ceiling. Every other reading of the machine
            # says it is fine (it runs, it names a state, the call returned),
            # which is how an endless chain used to pass an example that says
            # the machine waits.
            state = engine.last_truncated_macrostep_state()
            where = f" in `{policy.get_state_name(state)}`" if state is not None else ""
            raise _Refusal(f"by step {index} the engine stopped a macrostep{where} that did not "
                           f"reach a stable configuration (W3C SCXML 3.13), so where the "
                           f"machine stands is not the design's behaviour")
        failures = engine.unhandled_error_events() + engine.error_cascade_events()
        if failures:
            last = engine.last_unhandled_error()
            if last is None:
                last = engine.last_error_cascade_event()
            name = policy.get_event_name(last) if last is not None else "error"
            raise _Refusal(f"by step {index} the engine raised `{name}` and no state answered "
                           f"it, so what the machine did from there is not the design's "
                           f"behaviour.{self.opened}")

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
