# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A W3C run whose BasicHTTP listener cannot bind reports ERRORS and fails.

Usage: expect_unbindable_listener_errors.py <w3c_test_cli> <test id>

The test id must be one the conformance registry runs under its ``http``
harness. This holds a port, points ``SCE_W3C_HTTP_PORT`` at it, runs that one
test on both engines, and asserts the run says what happened: the test is
listed as an error on the Interpreter AND on AOT, and the process exits 1.

The run this guards against reported "Total 380 / Passed 380 / Errors 0" and
exited 0 on a host where another account held the port: the runner caught the
listener's exception, logged it, and made no report, so every HTTP test fell
out of the counts and nothing was left to fail.

The port is held on the wildcard address (both families where the host has
dual-stack IPv6), so whichever address ``localhost`` resolves to first, the
listener's bind collides. The holder sets no SO_REUSEPORT, so the listener's
own SO_REUSEPORT cannot share it either.
"""

from __future__ import annotations

import os
import re
import socket
import subprocess
import sys
import tempfile


def hold_a_port() -> socket.socket:
    """A listening socket on a port the kernel assigned, on every address."""
    if socket.has_dualstack_ipv6():
        return socket.create_server(("", 0), family=socket.AF_INET6, dualstack_ipv6=True)
    return socket.create_server(("", 0))


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    cli, test_id = argv[1], argv[2]

    with hold_a_port() as holder, tempfile.TemporaryDirectory() as artifacts:
        port = holder.getsockname()[1]
        env = dict(os.environ)
        env["SCE_W3C_HTTP_PORT"] = str(port)
        # The CLI writes its XML report into the directory it runs in, so it
        # runs in a directory of its own rather than beside another suite's.
        run = subprocess.run(
            [cli, test_id, "--no-save-failed-logs"],
            cwd=artifacts,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=240,
            check=False,
        )
    output = run.stdout.decode("utf-8", errors="replace")

    problems = []
    if run.returncode != 1:
        problems.append(f"exit status {run.returncode}, expected 1 (a run with errors)")
    for engine in ("Interpreter", "AOT"):
        listed = re.compile(r"Error Tests \(" + engine + r"\):[^\n]*\b" + re.escape(test_id) + r"\b")
        if not listed.search(output):
            problems.append(f"test {test_id} is not listed under 'Error Tests ({engine})'")

    if problems:
        print(output)
        print(f"With port {port} held, the run of test {test_id} did not report the listener:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1

    print(f"With port {port} held, test {test_id} was an ERROR on both engines and the run exited 1.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
