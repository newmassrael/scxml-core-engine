# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The standard host features (docs/SCE_HOST_FEATURES.md) as a generated Python
machine sees them through the reference host: every scenario of
``tests/host_features/scenarios`` replayed against the machine it names, with
``sce_host_features.ReferenceHost`` registered as the invoker of every standard
type.

A scenario's ``host`` directives set what the host is like before a step; its
``expect.host_invokes`` and ``expect.host_state`` are what the host was asked and
what it holds afterwards. The expected values are derived from the specification,
not observed from the host.

The machines are the ones ``scripts/regen_static_datamodel_python.sh`` generates
beside this file.
"""
from __future__ import annotations

import dataclasses
import importlib
import itertools
import json
import re
import sys
from pathlib import Path
from typing import Dict, List, Optional, Sequence

import pytest

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "host-features"))

from sce_host_features import (  # noqa: E402 -- paths inserted above
    ANSWERED,
    FAILED,
    HELD,
    REFUSED,
    ExhaustiveProvider,
    HttpRequest,
    HttpResponse,
    OutcomeClass,
    Reading,
    ReferenceHost,
    ScriptedAnswer,
    ScriptedClock,
    ScriptedProvider,
    TransportFailure,
    Zone,
    serve,
)
from sce_runtime import AnswerSchema, EventMetadata  # noqa: E402

_SCENARIOS = _HERE.parents[4] / "tests" / "host_features" / "scenarios"
_CLASSES = _HERE.parents[4] / "tests" / "host_features" / "model_outcome_classes.json"

#: The request fields a scenario writes as numbers; every other is text.
_INTEGER_FIELDS = ("at_ms", "max_tokens", "temperature_milli")


def _answer_text(answer: dict) -> str:
    """The text a ``model`` answer gives: ``text``, or ``text_repeat`` as a text
    and a count."""
    if "text" in answer:
        return answer["text"]
    text, count = answer["text_repeat"]
    return text * count


def _script_model(provider: ScriptedProvider, d: dict) -> None:
    """Apply a ``model`` directive to the scripted provider: each member it
    names."""
    if "answers" in d:
        provider.answers.clear()
        provider.answers.extend(
            ScriptedAnswer(_answer_text(a), a.get("finish", "stop")) for a in d["answers"]
        )
    if "constrained" in d:
        provider.constrained = d["constrained"]
    if "name" in d:
        provider.name = d["name"]
    if "context" in d:
        provider.context = d["context"]
    if "hold" in d:
        provider.hold = d["hold"]


class _ScriptTransport:
    """Answers what a scenario says and keeps what it was given."""

    def __init__(self) -> None:
        self.answer: Optional[object] = None
        self.set = False
        self.log: List[HttpRequest] = []

    def directive(self, d: dict) -> None:
        self.set = True
        if d.get("hold"):
            self.answer = "hold"
            return
        if "fail" in d:
            self.answer = "fail"
            return
        if "body" in d:
            body = d["body"].encode("utf-8")
        elif "body_hex" in d:
            body = bytes.fromhex(d["body_hex"])
        elif "body_repeat" in d:
            text, count = d["body_repeat"]
            body = (text * count).encode("utf-8")
        else:
            body = b""
        headers = [(name, value) for name, value in d["headers"]]
        self.answer = HttpResponse(d["status"], headers, body)

    def exchange(self, request: HttpRequest):
        self.log.append(request)
        if self.answer == "hold":
            return None
        if self.answer == "fail" or not self.set:
            raise TransportFailure("the transport has no response")
        return self.answer


def _atomic_state(engine) -> str:
    leaves = engine.active_leaves
    if len(leaves) != 1:
        return f"<{len(leaves)} active atomic states>"
    return str(leaves[0])


def _read(policy, readers: Dict[str, str], variable: str):
    if variable in readers:
        return getattr(policy, readers[variable])()
    attribute = "v_" + re.sub(r"(?<=[a-z0-9])([A-Z])", r"_\1", variable).lower()
    assert hasattr(policy, attribute), f"the machine holds no variable {variable!r}"
    return getattr(policy, attribute)


def _plain(value):
    """``value`` as a scenario writes it: an enum member by its declared name, a
    byte string as its Latin-1 text, a record by the schema's field ids."""
    if hasattr(value, "sce_name"):
        return value.sce_name
    if isinstance(value, bytes):
        return value.decode("latin-1")
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {f.name: _plain(getattr(value, f.name)) for f in dataclasses.fields(value)}
    return value


def _same(got, want) -> bool:
    """Python says ``True == 1``, which JSON does not; a 64-bit integer is
    written as the text of its digits; a record is held field by field."""
    if isinstance(got, bool) or isinstance(want, bool):
        return isinstance(got, bool) and isinstance(want, bool) and got == want
    if isinstance(got, dict) and isinstance(want, dict):
        return got.keys() == want.keys() and all(_same(got[key], want[key]) for key in want)
    if isinstance(got, int) and isinstance(want, str):
        return str(got) == want
    return got == want


class _Rig:
    def __init__(self) -> None:
        self.clock = ScriptedClock()
        self.transport = _ScriptTransport()
        self.host = ReferenceHost(self.clock, self.transport)
        self.model = ScriptedProvider()
        self.host.set_model_provider(self.model)

    def directives(self, directives: dict) -> None:
        for name, value in directives.items():
            if name == "clock":
                reading, zone = self.clock.reading, self.clock.zone_now
                assert reading is not None and zone is not None
                self.clock.reading = Reading(
                    value.get("utc_ms", reading.utc_ms),
                    value.get("monotonic_ms", reading.monotonic_ms),
                )
                self.clock.zone_now = Zone(
                    value.get("zone_id", zone.id), value.get("utc_offset_s", zone.utc_offset_s)
                )
            elif name == "permission":
                self.host.set_permission(value)
            elif name == "user_decision":
                self.host.set_user_decision(value)
            elif name == "quota":
                self.host.set_quota(value)
            elif name == "fail":
                self.host.set_faults(value)
            elif name == "http":
                self.transport.directive(value)
            elif name == "model":
                _script_model(self.model, value)
            else:
                raise AssertionError(f"`{name}` is not a host directive")

    def starts(self) -> list:
        """What the host was started with since the last step, as a scenario writes
        it: the request as its record, the integer fields as numbers."""
        out = []
        for start in self.host.take_starts():
            request = {
                name: (int(text) if name in _INTEGER_FIELDS else text)
                for name, text in start.params.items()
            }
            out.append({"type": start.operation, "id": start.id, "request": request})
        return out

    def host_state(self, wanted: dict) -> dict:
        state = {}
        for name in wanted:
            if name == "store":
                state[name] = dict(self.host.store_entries())
            elif name == "waiting":
                state[name] = self.host.waiting()
            elif name == "shown":
                state[name] = self.host.shown()
            elif name == "cancelled":
                state[name] = list(self.host.cancelled)
            elif name == "http":
                state[name] = [
                    {
                        "method": r.method,
                        "url": r.url,
                        "headers": [{"name": n, "value": v} for n, v in r.headers],
                        "body": r.body,
                    }
                    for r in self.transport.log
                ]
            else:
                raise AssertionError(f"`{name}` is not host state")
        return state


def replay(machine: str) -> None:
    scenario = json.loads((_SCENARIOS / f"{machine}.json").read_text())
    assert scenario["machine"] == machine
    steps = scenario["steps"]
    assert steps, f"scenario {machine} has no steps, which judges nothing"
    module = importlib.import_module(f"integration.static_datamodel.{machine}_sm")
    engine = module.create_engine()
    rig = _Rig()
    rig.host.set_answer_schemas(getattr(module, "ANSWER_SCHEMAS", ()))
    serve(engine, rig.host)
    engine.initialize()
    policy = engine.policy
    readers = module.SCE_HOST_NAMES["readers"]
    for n, step in enumerate(steps):
        where = f"{machine} step {n}: {step.get('note', '')}"
        if "host" in step:
            rig.directives(step["host"])
        if "advance_ms" in step:
            engine.advance_time(step["advance_ms"])
        event_name = step.get("event")
        if event_name is not None:
            event = policy.resolve_event_by_name(event_name)
            assert event is not None, f"{where}: the machine's events do not match {event_name!r}"
            data = json.dumps(step["data"]) if "data" in step else ""
            engine.send_event(event, EventMetadata(data=data))
        expect = step["expect"]
        started = rig.starts()
        if "host_invokes" in expect:
            assert started == expect["host_invokes"], f"{where}: what the host was asked: {started!r}"
        if "host_state" in expect:
            got = rig.host_state(expect["host_state"])
            assert got == expect["host_state"], f"{where}: what the host holds: {got!r}"
        if "state" in expect:
            assert _atomic_state(engine) == expect["state"], where
        for variable, want in expect.get("variables", {}).items():
            got = _plain(_read(policy, readers, variable))
            assert _same(got, want), f"{where}: variable {variable!r} is {got!r}, not {want!r}"


def test_a_machine_asks_the_key_value_store_of_its_host() -> None:
    replay("static_host_kv")


def test_a_machine_reads_the_clock_of_its_host() -> None:
    replay("static_host_clock")


def test_a_machine_posts_notifications_through_its_host() -> None:
    replay("static_host_notify")


def test_a_machine_makes_an_http_exchange_through_its_host() -> None:
    replay("static_host_http")


def test_a_machine_asks_its_host_for_a_text_under_an_answer_schema() -> None:
    replay("static_host_model")


def _outcome_class(cls: dict) -> OutcomeClass:
    """What the exhaustive provider answers for a class of the table."""
    end = cls["end"]
    if end == "answer":
        return OutcomeClass(ANSWERED, text=cls["text"], finish=cls["finish"])
    if end == "failure":
        return OutcomeClass(FAILED, code=cls["error"])
    if end == "refusal":
        return OutcomeClass(REFUSED)
    assert end == "hold", f"`{end}` is not an end"
    return OutcomeClass(HELD)


def _run_sequence(table: dict, sequence: Sequence[int]) -> None:
    """Run ``static_host_model`` against the exhaustive provider answering the
    classes ``sequence`` names, one call a class, and hold what the machine
    publishes after each call to what its class says."""
    classes = table["classes"]
    named = [classes[i]["class"] for i in sequence]
    module = importlib.import_module("integration.static_datamodel.static_host_model_sm")
    engine = module.create_engine()
    host = ReferenceHost(ScriptedClock(), _ScriptTransport())
    host.set_answer_schemas(module.ANSWER_SCHEMAS)
    host.set_model_provider(ExhaustiveProvider([_outcome_class(classes[i]) for i in sequence]))
    serve(engine, host)
    engine.initialize()
    policy = engine.policy
    readers = module.SCE_HOST_NAMES["readers"]
    ask = table["ask"]
    event = policy.resolve_event_by_name(ask["event"])
    leave = policy.resolve_event_by_name(table["leave"])
    assert event is not None and leave is not None, "the machine's events do not match the table's"
    answer = None
    for n, i in enumerate(sequence):
        cls = classes[i]
        engine.send_event(event, EventMetadata(data=json.dumps(ask["data"])))
        then = cls.get("then")
        if then == "advance":
            engine.advance_time(table["deadline_ms"])
        elif then == "leave":
            engine.send_event(leave, EventMetadata(data=""))
        expect = cls["expect"]
        if "answer" in expect:
            answer = expect["answer"]
        at = f"{named}, call {n + 1}"
        assert _atomic_state(engine) == "idle", f"{at}: the machine is not idle"
        for variable in ("outcome", "why", "finish"):
            got = _plain(_read(policy, readers, variable))
            assert _same(got, expect[variable]), f"{at}: {variable!r} is {got!r}"
        got = _plain(_read(policy, readers, "answer"))
        assert _same(got, answer), f"{at}: 'answer' is {got!r}, not {answer!r}"
        assert _read(policy, readers, "calls") == n + 1, f"{at}: 'calls'"


def test_every_sequence_of_outcome_classes_is_read_as_its_classes_say() -> None:
    """The machine around a model is tested exhaustively (docs/adr/0087, section
    9.4): every sequence of outcome classes up to the table's bound, each call
    read as its class says, and every refusal code of the library either reached
    by a class or named as deferred with its reason."""
    table = json.loads(_CLASSES.read_text())
    module = importlib.import_module("integration.static_datamodel.static_host_model_sm")
    assert table["machine"] == "static_host_model"
    assert [s.id for s in module.ANSWER_SCHEMAS][:1] == [table["answer_schema"]]
    count = len(table["classes"])
    bound = table["bound"]
    assert count and bound, "a table that names nothing judges nothing"
    runs = 0
    for length in range(1, bound + 1):
        for sequence in itertools.product(range(count), repeat=length):
            _run_sequence(table, sequence)
            runs += 1
    assert runs == sum(count**k for k in range(1, bound + 1))
    codes = {c["expect"]["why"]["code"] for c in table["classes"] if c["expect"]["why"]}
    codes |= {d["code"] for d in table["deferred"]}
    assert len(codes) == 13, f"every refusal code is reached or deferred: {sorted(codes)}"


def _ask(prompt: str, schema: str = "", max_tokens: str = "8", temperature: str = "0") -> dict:
    return {
        "prompt": prompt,
        "answer_schema": schema,
        "max_tokens": max_tokens,
        "temperature_milli": temperature,
    }


def _model_host(provider) -> ReferenceHost:
    host = ReferenceHost(ScriptedClock(), _ScriptTransport())
    host.set_answer_schemas([AnswerSchema("urn:sce:schema:answer:1", "{}", 'root ::= "{}"')])
    host.set_model_provider(provider)
    return host


def _fields(outcome) -> dict:
    assert outcome is not None and outcome.kind == "done", outcome
    return json.loads(outcome.data)


def test_a_generation_the_operation_does_not_accept_is_refused_before_the_provider() -> None:
    provider = ScriptedProvider(constrained=True)
    provider.answers.append(ScriptedAnswer("kept"))
    host = _model_host(provider)
    for params in (
        _ask(""),
        _ask("p", max_tokens="0"),
        _ask("p", temperature="2001"),
        _ask("p", schema="urn:sce:schema:other:1"),
    ):
        outcome = host.start("sce-host-model-generate", "gen", params)
        assert outcome is not None and outcome.kind == "refuse", params
    kept = _fields(host.start("sce-host-model-generate", "gen", _ask("abc", "urn:sce:schema:answer:1")))
    assert (kept["text"], kept["constrained"], kept["prompt_tokens"]) == ("kept", True, 3)
    none = _fields(host.start("sce-host-model-generate", "gen", _ask("p")))
    assert none["error"] == "unavailable"


def test_an_answer_past_its_bound_or_its_context_fails_whole() -> None:
    provider = ScriptedProvider(context=10)
    provider.answers.extend([ScriptedAnswer("a" * 4097), ScriptedAnswer("kept")])
    host = _model_host(provider)
    assert _fields(host.start("sce-host-model-generate", "gen", _ask("p")))["error"] == "answer_too_large"
    past = _fields(host.start("sce-host-model-generate", "gen", _ask("abc", max_tokens="8")))
    assert past["error"] == "context_exceeded"
    assert _fields(host.start("sce-host-model-generate", "gen", _ask("abc", max_tokens="7")))["text"] == "kept"


def test_the_exhaustive_provider_answers_each_class_in_turn() -> None:
    host = _model_host(
        ExhaustiveProvider(
            [
                OutcomeClass(ANSWERED, text="t", finish="length"),
                OutcomeClass(FAILED, code="busy"),
                OutcomeClass(REFUSED),
                OutcomeClass(HELD),
            ]
        )
    )
    answer = _fields(host.start("sce-host-model-generate", "gen", _ask("p")))
    assert (answer["text"], answer["finish"], answer["model"]) == ("t", "length", "exhaustive")
    assert _fields(host.start("sce-host-model-generate", "gen", _ask("p")))["error"] == "busy"
    assert host.start("sce-host-model-generate", "gen", _ask("p")).kind == "refuse"
    assert host.start("sce-host-model-generate", "gen", _ask("p")).kind == "hold"
    assert _fields(host.start("sce-host-model-generate", "gen", _ask("p")))["error"] == "unavailable"


def test_every_scenario_of_the_directory_is_replayed_here() -> None:
    names = sorted(p.stem for p in _SCENARIOS.glob("*.json"))
    assert names == [
        "static_host_clock",
        "static_host_http",
        "static_host_kv",
        "static_host_model",
        "static_host_notify",
    ]


def test_the_system_clock_answers_a_monotonic_reading_that_does_not_decrease() -> None:
    from sce_host_features import SystemClock

    host = ReferenceHost(SystemClock(), _ScriptTransport())
    last = -1
    for _ in range(50):
        outcome = host.start("sce-host-clock-now", "now", {})
        assert outcome is not None
        fields = json.loads(outcome.data)
        assert fields["error"] == ""
        assert int(fields["monotonic_ms"]) >= last
        last = int(fields["monotonic_ms"])
    zone = host.start("sce-host-clock-zone", "zone", {})
    assert zone is not None and json.loads(zone.data)["error"] == ""


def test_an_operation_outside_the_standard_set_is_not_served_and_a_nul_key_is_refused() -> None:
    from sce_host_features import REFUSE

    host = ReferenceHost(ScriptedClock(), _ScriptTransport())
    assert host.start("x-app-host", "id", {}) is None
    refused = host.start("sce-host-kv-get", "id", {"key": "a\x00b"})
    assert refused is not None and refused.kind == REFUSE


def test_keys_are_ordered_by_their_utf8_bytes() -> None:
    host = ReferenceHost(ScriptedClock(), _ScriptTransport())
    for key in ("～", "\U0001f600"):
        host.start("sce-host-kv-put", "put", {"key": key, "value": "v"})
    walk = host.start("sce-host-kv-next-key", "walk", {"after": "", "prefix": ""})
    assert walk is not None and json.loads(walk.data)["key"] == "～"


def test_the_http_transport_carries_an_exchange_to_a_local_server() -> None:
    """The transport over the standard library's client: a local server answers,
    the host carries the request's headers and body to it, follows no redirect,
    and gives the response's fields in the order they were received."""
    import threading
    from http.server import BaseHTTPRequestHandler, HTTPServer

    from sce_host_features import HttpTransport, SystemClock

    seen: dict = {}

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args) -> None:  # noqa: D401 -- silence
            pass

        def _answer(self) -> None:
            length = int(self.headers.get("Content-Length", "0") or "0")
            seen["body"] = self.rfile.read(length).decode("utf-8") if length else ""
            seen["trace"] = self.headers.get("X-Trace")
            seen["method"] = self.command
            if self.path == "/moved":
                self.send_response(301)
                self.send_header("Location", "/elsewhere")
                self.send_header("Content-Length", "0")
                self.end_headers()
            elif self.path == "/missing":
                self.send_response(404)
                self.send_header("Content-Length", "0")
                self.end_headers()
            else:
                self.send_response(201)
                self.send_header("X-Second", "2")
                self.send_header("X-First", "1")
                self.send_header("Content-Length", "4")
                self.end_headers()
                self.wfile.write(b"made")

        do_GET = do_POST = _answer

    server = HTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        base = f"http://127.0.0.1:{server.server_address[1]}"
        host = ReferenceHost(SystemClock(), HttpTransport())

        def fetch(method: str, path: str, headers: str = "", body: str = "") -> dict:
            outcome = host.start(
                "sce-host-http-fetch",
                "fetch",
                {"method": method, "url": base + path, "headers": headers, "body": body},
            )
            assert outcome is not None
            return json.loads(outcome.data)

        made = fetch("POST", "/items", "X-Trace: 7\nContent-Type: text/plain", "payload")
        assert (made["error"], made["status"], made["body"]) == ("", 201, "made")
        names = [line.split(":")[0] for line in made["headers"].split("\n")]
        assert names.index("x-second") < names.index("x-first"), made["headers"]
        assert seen == {"body": "payload", "trace": "7", "method": "POST"}

        moved = fetch("GET", "/moved")
        assert moved["status"] == 301 and "location: /elsewhere" in moved["headers"]
        assert fetch("GET", "/missing")["status"] == 404
    finally:
        server.shutdown()
        server.server_close()
    assert fetch("GET", "/items")["error"] == "network"
