# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Python's reader of the W3C BasicHTTP fixture endpoint.

W3C SCXML C.2.3: the endpoint is owned by ``basic_http_test_endpoint.h``, a C
header because the C11 AOT runners must include it. This module does not
restate the path; it READS it from that header, so a Python fixture server and
a compiled runner cannot come to disagree about where the listener answers.

The port has no default. The listener this module serves runs in the process
of the machines it answers (the pybind11 W3C suite's), so it binds the port
``SCE_W3C_HTTP_PORT`` names, or 0 for one the kernel assigns, and the location
those machines publish is read from the listener once it has bound.

Raises rather than guessing. A server that quietly bound another port after
being told one would take the port another tree is using, and the collision
would surface as a test failure in whichever tree lost it.
"""

from __future__ import annotations

import os
import re
from pathlib import Path

_HEADER = Path(__file__).resolve().parent / "basic_http_test_endpoint.h"

_PATH_RE = re.compile(r'^#define\s+SCE_W3C_HTTP_TEST_PATH\s+"([^"]+)"', re.MULTILINE)


def endpoint_path() -> str:
    """The path the fixture listener answers on."""
    try:
        text = _HEADER.read_text(encoding="utf-8")
    except OSError as exc:
        raise RuntimeError(
            f"the BasicHTTP fixture endpoint header is unreadable: {_HEADER} ({exc})"
        ) from exc
    path = _PATH_RE.search(text)
    if not path:
        raise RuntimeError(
            f"{_HEADER} no longer declares SCE_W3C_HTTP_TEST_PATH - the endpoint "
            "owner moved or was renamed"
        )
    return path.group(1)


def in_process_port() -> int:
    """The port an in-process fixture listener is told to bind: the one
    ``SCE_W3C_HTTP_PORT`` names, or 0 for one the kernel assigns."""
    raw = os.environ.get("SCE_W3C_HTTP_PORT", "")
    if raw:
        if not raw.isdigit() or not 1 <= int(raw) <= 65535:
            raise RuntimeError(f'SCE_W3C_HTTP_PORT="{raw}" is not a TCP port')
        return int(raw)
    return 0
