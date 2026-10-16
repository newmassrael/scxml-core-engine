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

import importlib
import json
import re
import sys
from pathlib import Path
from typing import Dict, List, Optional

import pytest

_HERE = Path(__file__).resolve().parent
# The tests directory, so this package is importable by the name it has there.
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "host-features"))

from sce_host_features import (  # noqa: E402 -- paths inserted above
    HttpRequest,
    HttpResponse,
    Reading,
    ReferenceHost,
    ScriptedClock,
    TransportFailure,
    Zone,
    serve,
)
from sce_runtime import EventMetadata  # noqa: E402

_SCENARIOS = _HERE.parents[4] / "tests" / "host_features" / "scenarios"


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


def _same(got, want) -> bool:
    """Python says ``True == 1``, which JSON does not."""
    if isinstance(got, bool) or isinstance(want, bool):
        return isinstance(got, bool) and isinstance(want, bool) and got == want
    if isinstance(got, int) and isinstance(want, str):
        return str(got) == want
    return got == want


class _Rig:
    def __init__(self) -> None:
        self.clock = ScriptedClock()
        self.transport = _ScriptTransport()
        self.host = ReferenceHost(self.clock, self.transport)

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
            else:
                raise AssertionError(f"`{name}` is not a host directive")

    def starts(self) -> list:
        """What the host was started with since the last step, as a scenario writes
        it: the request as its record, ``at_ms`` being the one integer field."""
        out = []
        for start in self.host.take_starts():
            request = {
                name: (int(text) if name == "at_ms" else text) for name, text in start.params.items()
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
            got = _read(policy, readers, variable)
            assert _same(got, want), f"{where}: variable {variable!r} is {got!r}, not {want!r}"


def test_a_machine_asks_the_key_value_store_of_its_host() -> None:
    replay("static_host_kv")


def test_a_machine_reads_the_clock_of_its_host() -> None:
    replay("static_host_clock")


def test_a_machine_posts_notifications_through_its_host() -> None:
    replay("static_host_notify")


def test_a_machine_makes_an_http_exchange_through_its_host() -> None:
    replay("static_host_http")


def test_every_scenario_of_the_directory_is_replayed_here() -> None:
    names = sorted(p.stem for p in _SCENARIOS.glob("*.json"))
    assert names == ["static_host_clock", "static_host_http", "static_host_kv", "static_host_notify"]


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
