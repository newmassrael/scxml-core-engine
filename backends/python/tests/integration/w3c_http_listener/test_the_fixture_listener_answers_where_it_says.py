# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The BasicHTTP fixture listener of this suite's conftest.

It runs in the pytest process, beside the machines it serves, so it is told no
port: unless SCE_W3C_HTTP_PORT names one it binds a port the kernel assigns,
and the location it yields, which each machine publishes as its BasicHTTP
'location', names the port it bound (`tests/w3c/basic_http_test_endpoint.h`).
A fixed port made every HTTP test of this suite fail to start on a host where
another program held it.
"""

import json
import os
import urllib.parse
import urllib.request
from pathlib import Path


def _ephemeral_port_range() -> range:
    """The range Linux assigns a port bound to 0 from."""
    low, high = Path("/proc/sys/net/ipv4/ip_local_port_range").read_text().split()
    return range(int(low), int(high) + 1)


def test_the_listener_binds_the_port_it_is_told_or_one_the_kernel_assigns(
    w3c_http_server,
) -> None:
    port = urllib.parse.urlsplit(w3c_http_server).port
    named = os.environ.get("SCE_W3C_HTTP_PORT", "")
    if named:
        assert port == int(named), (
            f"SCE_W3C_HTTP_PORT names {named}, and the listener's location says {port}"
        )
    else:
        assert port in _ephemeral_port_range(), (
            f"no port was named, so the listener should hold one the kernel assigned; "
            f"its location says {port}, outside the range the kernel assigns from"
        )


def test_the_listener_answers_at_the_location_it_yields(w3c_http_server) -> None:
    request = urllib.request.Request(
        w3c_http_server,
        data=b"_scxmleventname=probe&value=1",
        method="POST",
        headers={"Content-Type": "application/x-www-form-urlencoded"},
    )
    with urllib.request.urlopen(request, timeout=3.0) as response:
        payload = json.loads(response.read().decode("utf-8"))
    assert payload["event"] == "probe", (
        "the location a machine is handed is not where this listener answers"
    )
    assert payload["data"]["value"] == 1
