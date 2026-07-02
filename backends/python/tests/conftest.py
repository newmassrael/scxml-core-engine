# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""pytest harness — W3C SCXML C.2 BasicHTTP Event I/O Processor test server.

Spawns a Python `http.server.HTTPServer` on port 8080 with the same echo
semantics as `tests/w3c/standalone_http_server.js`, so the AOT-generated
HTTP fixtures (test201 / test509-534 / test567 / test577) can round-trip
their `<send target="http://localhost:8080/test">` against a real socket
without requiring Node.js to be installed.

The W3C HTTP test fixtures all hardcode `http://localhost:8080/test`, so
the listener uses a fixed port (the spec test infrastructure makes the
same assumption across Rust / Go / C++ backends). Tests that don't need
HTTP simply don't take the fixture and pay no startup cost.
"""

from __future__ import annotations

import json
import sys
import threading
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import Optional

import pytest

# Make the runtime importable for every generated test wrapper.
_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parent / "runtime"))


_HTTP_PORT = 8080
_HTTP_PATH = "/test"
_HTTP_TIMEOUT_SECONDS = 3.0


class _W3CEchoHandler(BaseHTTPRequestHandler):
    """W3C SCXML C.2 — echo POST bodies back as JSON with the event name
    extracted per the BasicHTTP encoding rules (form-encoded params,
    JSON body, or plain text). Mirrors `standalone_http_server.js`
    1:1 so the same fixtures pass under Python and Node.js harnesses."""

    def do_POST(self) -> None:  # noqa: N802 — http.server naming
        if self.path != _HTTP_PATH:
            self._respond_json(404, {"status": "error", "message": "not found"})
            return
        length = int(self.headers.get("Content-Length", "0") or "0")
        body_bytes = self.rfile.read(length) if length > 0 else b""
        body = body_bytes.decode("utf-8", errors="replace")
        content_type = self.headers.get("Content-Type", "") or ""

        event_name = "event1"
        event_data = ""

        if "application/x-www-form-urlencoded" in content_type:
            params = urllib.parse.parse_qs(body, keep_blank_values=True)
            scxml_event_name = params.get("_scxmleventname", [""])[0]
            if scxml_event_name:
                event_name = scxml_event_name
            data_obj = {}
            for key, values in params.items():
                value = values[0] if values else ""
                # Coerce numeric strings to numbers (matches the Node.js
                # echo server's `Number(value)` heuristic — W3C test519
                # asserts that `param1` arrives as a number).
                try:
                    coerced = int(value)
                except ValueError:
                    try:
                        coerced = float(value)
                    except ValueError:
                        coerced = value
                data_obj[key] = coerced
            event_data = json.dumps(data_obj)
        elif body.startswith("{") or body.startswith("["):
            try:
                parsed = json.loads(body)
                if isinstance(parsed, dict) and "event" in parsed:
                    event_name = str(parsed["event"])
                event_data = body
            except json.JSONDecodeError:
                event_data = body
        elif body:
            event_name = "HTTP.POST"
            event_data = body

        try:
            response_data = json.loads(event_data) if event_data else ""
        except json.JSONDecodeError:
            response_data = event_data

        self._respond_json(
            200,
            {
                "status": "success",
                "event": event_name,
                "data": response_data,
            },
        )

    def log_message(self, format: str, *args) -> None:  # noqa: A002 — pinned API
        # Suppress request logs so pytest output stays clean.
        return

    def _respond_json(self, status: int, payload: dict) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(body)


@pytest.fixture(scope="session")
def w3c_http_server() -> str:
    """W3C SCXML C.2 — start the echo server for the test session and
    return its URL. Lazy: only tests that take this fixture spawn the
    server, so non-HTTP fixtures pay no socket / thread cost."""
    server = HTTPServer(("127.0.0.1", _HTTP_PORT), _W3CEchoHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{_HTTP_PORT}{_HTTP_PATH}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2.0)


def _make_http_callback():
    """W3C SCXML C.2 — build an HTTP dispatcher closure. POSTs the
    request as `application/x-www-form-urlencoded` with the event name
    in `_scxmleventname`, parses the JSON response, and returns an
    `HttpSendResponse` so the engine raises the echoed event."""
    from sce_runtime import HttpSendResponse

    def dispatch(request) -> Optional["HttpSendResponse"]:
        body_parts = []
        if request.event_name:
            body_parts.append(
                f"_scxmleventname={urllib.parse.quote_plus(request.event_name)}"
            )
        for key, values in (request.params or {}).items():
            if key == "_scxmleventname" and request.event_name:
                continue
            for value in values:
                body_parts.append(
                    f"{urllib.parse.quote_plus(key)}={urllib.parse.quote_plus(value)}"
                )

        if body_parts:
            body = "&".join(body_parts).encode("utf-8")
            content_type = "application/x-www-form-urlencoded"
        elif request.content:
            body = request.content.encode("utf-8")
            content_type = "text/plain"
        else:
            body = b""
            content_type = "application/x-www-form-urlencoded"

        req = urllib.request.Request(
            request.target,
            data=body,
            method="POST",
            headers={"Content-Type": content_type},
        )
        try:
            with urllib.request.urlopen(req, timeout=_HTTP_TIMEOUT_SECONDS) as resp:
                text = resp.read().decode("utf-8", errors="replace")
        except Exception:
            return None
        try:
            payload = json.loads(text)
        except json.JSONDecodeError:
            return None
        event_name = str(payload.get("event", ""))
        if not event_name:
            return None
        data = payload.get("data", "")
        if not isinstance(data, str):
            data = json.dumps(data)
        return HttpSendResponse(event_name=event_name, event_data=data)

    return dispatch


@pytest.fixture
def setup_http(w3c_http_server) -> "callable":
    """W3C SCXML C.2 — return a function that registers the HTTP
    dispatch callback on a freshly-created engine. Generated test
    wrappers for HTTP fixtures call this once between `create_engine()`
    and `initialize()`. The dependency on `w3c_http_server` keeps the
    listener alive for the duration of any HTTP test."""

    def _register(engine) -> None:
        engine.set_http_send_callback(_make_http_callback())

    return _register
