"""Run a scenario set against one design on the Python lowering, and write down
what was seen.

The product owns the examples (`sce-codegen scenarios`) and the verdicts
(`sce-codegen judge-scenarios`). What sits between them is a driver: a program
that stands a machine up, plays each scenario into it and records what came
out. This is one such driver, over the same Python lowering `verify` and
`compare` drive, and what it writes is an observation trace
(`schemas/sce-observation-trace.v1.schema.json`) naming itself `Python
lowering`, because a verdict is about an engine and the examples could pass on
one runtime and fail on another.

⚠ It reports what it saw and nothing else. Three places where a driver is
tempted to fill a hole are closed on purpose.

    A run during which the engine raised an `error.*` event that no state
    answered is refused, not observed (W3C SCXML 3.12.2). The machine did not
    do what its document says: an entry block that meets an unresolvable
    `<send>` ends there (W3C SCXML 4.9), so a timer armed after it is never
    armed and the machine then "fails" an example about timing it was never
    allowed to keep. The refusal names the error and, when the design holds
    open decisions, which, because an open route is the usual cause.

    A send the driver has no sink for is refused, not dropped. It observes
    sends to host-served processors, and BasicHTTP sends it detects and
    refuses; a send to a parent session or a mesh peer either raises an
    unanswered error or reaches nobody, and both end the run the same way.

    An output declared with a route (`via`) counts only when the design sent
    it through exactly that route, and a design that sent it through another
    one is refused with both routes named. The interface is what the owner
    accepted; a design that leaves by a different door is not what the
    examples describe, and counting the event anyway would pass it.

What the machine sent while it started (the initial state's `<onentry>`) is
part of the first step's observation, since no step has run before it.

The run is bounded by the steps the scenarios write, in virtual time: it
never waits on a clock.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import xml.etree.ElementTree as ET

from .verify import (SendRecorder, VerifyError, _default_codegen, _host_names, _scratch,
                     generate, load, scenario_judgement, scenario_set_reading)

ENGINE_NAME = "Python lowering"
RECORD = "sce-observation-trace"
VERSION = 1


class ScenarioDriverError(VerifyError):
    """The scenario set cannot be driven at all. Never a verdict."""


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
    Its text is what the owner reads."""


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
    """The policy's event for a name, falling back through dot-token prefixes
    the way the engine does for a name it does not declare: a transition on
    `door` answers `door.open` (W3C SCXML 3.12.1)."""
    event = policy.get_event_from_name(name)
    parts = name.split(".")
    while event is None and len(parts) > 1:
        parts.pop()
        event = policy.get_event_from_name(".".join(parts))
    return event


#: How many open decisions a refusal quotes. The first few name the cause; a
#: design with dozens would bury the sentence they sit in.
OPEN_DECISIONS_QUOTED = 4


def _open_decisions(manifest: dict) -> str:
    """The design's open decisions, as the generator reported them, in the
    words the author wrote for each; empty when there are none.

    Each distinct sentence once: a decision that blocks several sends is
    reported at every one of them, and the same words three times say nothing
    the first did not."""
    found = list(dict.fromkeys(
        f"{record.get('id')}: {str(record.get('reason')).rstrip('.')}"
        for record in manifest.get("unresolved") or []
        if record.get("node_type") != "kind-basis"))
    if not found:
        return ""
    quoted = "; ".join(found[:OPEN_DECISIONS_QUOTED])
    more = len(found) - OPEN_DECISIONS_QUOTED
    return (f" The design holds {len(found)} open decision(s): {quoted}"
            + (f"; and {more} more." if more > 0 else "."))


class _Design:
    """One design, generated to Python once and run from scratch per scenario."""

    def __init__(self, document: pathlib.Path, codegen: pathlib.Path,
                 into: pathlib.Path, serves: tuple, routes: dict, data: list) -> None:
        self.routes = routes
        self.data = data
        self.refusal = ""
        self.module = None
        self.readers: dict = {}
        self.data_unavailable = ""
        self.unreadable: dict = {}
        self.opened = ""
        self.build = generate(document, codegen, into, serves=serves)
        if self.build.refusal:
            self.refusal = self.build.refusal
            return
        kind = (self.build.manifest.get("document_kind") or {}).get("name")
        if kind != "statechart":
            self.refusal = (f"{document.name} is a {kind or 'document of no known kind'}, and a "
                            f"scenario drives a statechart")
            return
        self.module = load(into, document)
        if data:
            try:
                self.readers = _host_names(self.module, "readers")
            except VerifyError as exc:
                self.data_unavailable = str(exc)
        self.unreadable = ({} if self.data_unavailable else
                           {name: self._why_unreadable(name, document) for name in data
                            if name not in self.readers})
        self.opened = _open_decisions(self.build.manifest)

    def _why_unreadable(self, name: str, document: pathlib.Path) -> str:
        """Why the generated module has no reader for a data item the
        interface names, in the generator's words when it gave them."""
        record = self.build.unreadable(name)
        if record:
            return (f"the generator gives `{name}` no reader ({record.get('reason')} in "
                    f"{record.get('language')}), and a reader exists in every backend or in none")
        declared = {node.get("id") for node in ET.parse(document).getroot().iter()
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
        if self.refusal:
            return {"scenario": scenario["id"], "refused": {"why": self.refusal}}
        try:
            return {"scenario": scenario["id"], "observations": self._play(scenario["steps"])}
        except _Refusal as exc:
            return {"scenario": scenario["id"], "refused": {"why": str(exc)}}

    def _play(self, steps: list) -> list:
        from sce_runtime.event import EventMetadata

        sink, http = SendRecorder(), _HttpSeen()
        engine = self.module.create_engine()
        for processor in self.build.declared:
            engine.register_event_processor(processor, sink)
        engine.set_http_send_callback(http)
        engine.initialize()
        policy = engine.policy
        observations = []
        for index, step in enumerate(steps):
            try:
                if "send" in step:
                    event = _resolve_event(policy, step["send"])
                    if event is None:
                        raise _Refusal(f"step {index} sends `{step['send']}`, and the design "
                                       f"names no event that answers it, so the example's "
                                       f"input reaches nothing in it")
                    payload = step.get("payload")
                    engine.send_event(event, EventMetadata(data=payload) if payload else None)
                elif "advance_ms" in step:
                    engine.advance_time(step["advance_ms"])
            except _Refusal:
                raise
            except Exception as exc:  # noqa: BLE001 - the engine failing is the report
                raise _Refusal(f"the engine raised {type(exc).__name__} at step {index}: {exc}")
            self._check_unobserved(engine, policy, http, index)
            observations.append(self._observe(engine, policy, sink, index))
        return observations

    def _check_unobserved(self, engine, policy, http: _HttpSeen, index: int) -> None:
        """Refuse the run when it did something this driver cannot report."""
        if http.requests:
            request = http.requests[0]
            raise _Refusal(f"at step {index} the design sent `{request.event_name}` over "
                           f"BasicHTTP to `{request.target}`; this driver observes sends to "
                           f"host-served processors and does not observe HTTP")
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


def drive(scenario_set: pathlib.Path, document: pathlib.Path,
          codegen: pathlib.Path | None = None) -> dict:
    """The observation trace of every runnable scenario in the set, played
    against the design `document` on the Python lowering.

    The set is read for what a driver needs and no more; whether it is a
    usable set is `sce-codegen scenarios`' answer, and `judge-scenarios`
    judges nothing from one that is not."""
    scenario_set, document = pathlib.Path(scenario_set), pathlib.Path(document)
    raw = scenario_set.read_bytes()
    try:
        spec = json.loads(raw)
        interface = spec["interface"]
        scenarios = spec["scenarios"]
        outputs = interface.get("outputs") or []
    except (ValueError, KeyError, AttributeError, TypeError) as exc:
        raise ScenarioDriverError(f"{scenario_set}: not a scenario set a driver can read "
                                  f"({type(exc).__name__}: {exc})") from exc
    routes = {output["name"]: (output["via"]["type"], output["via"]["target"])
              for output in outputs if output.get("via")}
    serves = tuple(dict.fromkeys(route[0] for route in routes.values()))
    data = list(interface.get("data") or [])
    codegen = pathlib.Path(codegen) if codegen else _default_codegen()
    with _scratch() as scratch:
        design = _Design(document, codegen, scratch / "design", serves, routes, data)
        runs = [design.run(scenario) for scenario in scenarios
                if scenario.get("status", "runnable") == "runnable"]
    generator = (design.build.manifest or {}).get("generator")
    detail = "generated Python on the product's Python runtime"
    trace = {
        "record": RECORD,
        "v": VERSION,
        "engine": {"name": ENGINE_NAME,
                   "detail": f"{detail}, generator {generator}" if generator else detail},
        "design": {"path": str(document),
                   "sha256": hashlib.sha256(document.read_bytes()).hexdigest()},
        "scenario_set": {"sha256": hashlib.sha256(raw).hexdigest()},
        "observes": {"outbound": True, "finished": True, "configuration": True,
                     "data": ({"unavailable": design.data_unavailable}
                              if design.data_unavailable else True)},
        "runs": runs,
    }
    if design.unreadable:
        trace["unreadable"] = design.unreadable
    return trace


def judge(scenario_set: pathlib.Path, trace: dict, codegen: pathlib.Path | None = None) -> list:
    """The product's judgement of `trace` against the set: the summary record
    first, then one record per scenario, then each failed check, gap and
    problem, as `sce-codegen judge-scenarios` wrote them.

    The verdicts are the product's and are passed through untouched. A
    judgement computed here would be a second answer to a question that has
    one, free to disagree with the first."""
    with _scratch() as scratch:
        written = scratch / "observation-trace.json"
        written.write_text(json.dumps(trace, ensure_ascii=False), encoding="utf-8")
        report, refusal = scenario_judgement(scenario_set, written, codegen)
    return _records(report, refusal, "the trace could not be judged")


def _records(report: str, refusal: str, what: str) -> list:
    """The records of a product run, or its refusal in the product's words."""
    if refusal:
        reasons = [d.get("message") or d.get("unparsed")
                   for d in json.loads(refusal).get("diagnostics") or []]
        raise ScenarioDriverError(f"{what}: " + ("; ".join(r for r in reasons if r) or refusal))
    return json.loads(report)["records"]


def run(scenario_set: pathlib.Path, document: pathlib.Path,
        codegen: pathlib.Path | None = None) -> dict:
    """Drive the set against the design, and have the product judge what was
    seen: `{"trace": ..., "judgement": [records]}`."""
    trace = drive(scenario_set, document, codegen)
    return {"trace": trace, "judgement": judge(scenario_set, trace, codegen)}


def read_set(scenario_set: pathlib.Path, specification: pathlib.Path | None = None,
             codegen: pathlib.Path | None = None, cwd: pathlib.Path | None = None) -> list:
    """The product's reading of the set: whether it is usable, and each
    problem it found, as `sce-codegen scenarios` wrote them. With the
    specification every quote is checked word for word; without it none is,
    and the summary says so."""
    report, refusal = scenario_set_reading(scenario_set, specification, codegen, cwd=cwd)
    return _records(report, refusal, "the file is not a scenario set")


def answer(read: list, played: dict | None) -> dict:
    """What the owner is told: the set's reading, and, when the set was usable
    and was played, the product's verdicts with the engine they are about.

    Every field is the product's record passed through, grouped. The words
    around them say only what the records leave unsaid: which engine, and
    that a pass is not a claim about the design."""
    summary = next((r for r in read if r.get("kind") == "scenario-set"), {})
    problems = [r for r in read if r.get("kind") == "problem"]
    reply = {"set": {"usable": summary.get("usable"),
                     "origin": summary.get("origin"),
                     "scenarios": summary.get("scenarios"),
                     "runnable": summary.get("runnable"),
                     "quotes_checked": summary.get("quotes_checked"),
                     "problems": problems}}
    if played is None:
        reply["verdict"] = "set not usable"
        reply["says"] = ("The examples have problems, so nothing was run: a verdict from a set "
                         "with problems is a verdict about nothing. Fix the problems listed and "
                         "ask again.")
        return reply
    records = played["judgement"]
    judgement = next((r for r in records if r.get("kind") == "judgement"), {})
    reply["verdict"] = "judged" if judgement.get("judged") else "nothing judged"
    reply["engine"] = judgement.get("engine")
    reply["counts"] = {name: judgement.get(name) for name in
                       ("scenarios", "pass", "fail", "not-judged", "blocked",
                        "awaiting-decision")}
    reply["scenarios"] = [{k: r[k] for k in ("id", "verdict", "reason", "requirements", "bound")
                           if k in r} for r in records if r.get("kind") == "verdict"]
    said = {r["reason"] for r in reply["scenarios"] if r.get("reason")}
    for kind in ("failure", "gap", "problem"):
        found = [r for r in records if r.get("kind") == kind]
        if kind == "gap":
            # A gap about a whole scenario says what its verdict's reason
            # already says. One about a step or a check says more.
            found = [r for r in found if r.get("step") is not None or r.get("check")
                     or r.get("why") not in said]
        if found:
            reply[f"{kind}s"] = found
    reply["means"] = judgement.get("means")
    return reply
