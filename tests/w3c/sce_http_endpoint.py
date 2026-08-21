# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Python's reader of the W3C BasicHTTP fixture endpoint.

W3C SCXML C.2.3: the endpoint is owned by ``basic_http_test_endpoint.h``, a C
header because the C11 AOT runners must include it. This module does not
restate the port; it READS it from that header, so a Python fixture server and
a compiled runner cannot come to disagree about where the listener answers.

``SCE_W3C_HTTP_PORT`` in the environment wins, which is how a second checkout is
given a port of its own -- the listener is machine-global and only one process
per host can hold it.

Raises rather than guessing. A server that quietly bound the default after being
told otherwise would take the port another tree is using, and the collision
would surface as a test failure in whichever tree lost it.
"""

from __future__ import annotations

import os
import re
from pathlib import Path

_HEADER = Path(__file__).resolve().parent / "basic_http_test_endpoint.h"

_PORT_RE = re.compile(r"^#define\s+SCE_W3C_HTTP_DEFAULT_PORT\s+(\d+)", re.MULTILINE)
_PATH_RE = re.compile(r'^#define\s+SCE_W3C_HTTP_TEST_PATH\s+"([^"]+)"', re.MULTILINE)


def _read_header() -> tuple[int, str]:
    try:
        text = _HEADER.read_text(encoding="utf-8")
    except OSError as exc:
        raise RuntimeError(
            f"the BasicHTTP fixture endpoint header is unreadable: {_HEADER} ({exc})"
        ) from exc
    port = _PORT_RE.search(text)
    path = _PATH_RE.search(text)
    if not port or not path:
        raise RuntimeError(
            f"{_HEADER} no longer declares SCE_W3C_HTTP_DEFAULT_PORT and "
            "SCE_W3C_HTTP_TEST_PATH - the endpoint owner moved or was renamed"
        )
    return int(port.group(1)), path.group(1)


def endpoint_port() -> int:
    """The port the fixture listener binds and the runners address."""
    raw = os.environ.get("SCE_W3C_HTTP_PORT", "")
    if raw:
        if not raw.isdigit() or not 1 <= int(raw) <= 65535:
            raise RuntimeError(f'SCE_W3C_HTTP_PORT="{raw}" is not a TCP port')
        return int(raw)
    return _read_header()[0]


def endpoint_path() -> str:
    """The path the fixture listener answers on."""
    return _read_header()[1]


def endpoint_url() -> str:
    """The published BasicHTTP location: ``http://localhost:<port><path>``."""
    return f"http://localhost:{endpoint_port()}{endpoint_path()}"
