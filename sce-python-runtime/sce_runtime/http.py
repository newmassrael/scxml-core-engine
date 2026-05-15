# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML C.2 BasicHTTP Event I/O Processor — request/response dataclasses.

Mirrors `sce-rust-runtime/src/http.rs` and `sce-go-runtime/http.go` 1:1.

The Python engine never links against an HTTP library. Generated state
machines call `Engine.perform_http_send(...)` which delegates to a
user-supplied callback registered via `Engine.set_http_send_callback`.
The pytest harness wires up a callback that POSTs to a Python
`http.server`-based echo server (W3C SCXML C.2 test fixture); production
callers inject their own dispatcher (requests, httpx, urllib, etc.).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Dict, List


@dataclass
class HttpSendRequest:
    """W3C SCXML C.2 — outbound HTTP send payload passed to the
    `on_http_send` callback.

    Matches the field set of `sce_rust_runtime.HttpSendRequest` and
    `sce.HttpSendRequest` (Go) so cross-backend harnesses can share
    fixture logic. `params` is a multi-map (each key maps to a list
    of values) because `<param>` may repeat with the same name."""

    target: str = ""
    event_name: str = ""
    content: str = ""
    params: Dict[str, List[str]] = field(default_factory=dict)
    send_id: str = ""


@dataclass
class HttpSendResponse:
    """W3C SCXML C.2 — inbound HTTP echo response returned by the
    `on_http_send` callback.

    When the callback returns an `HttpSendResponse` whose `event_name`
    resolves on the running policy's event enum, the engine raises
    that event onto the external queue with `event_data` bound as
    `_event.data` (W3C SCXML 5.10.3). Returning `None` from the
    callback indicates fire-and-forget dispatch (no inbound event)."""

    event_name: str = ""
    event_data: str = ""
