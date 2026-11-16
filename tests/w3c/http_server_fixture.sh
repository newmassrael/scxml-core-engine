#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Lifecycle fixture for the W3C SCXML C.2 BasicHTTP corpus' standalone
# Node server. ctest FIXTURES_SETUP runs the `start` arm before any
# `w3c_c_aot_<N>` test whose registry harness is `http`; FIXTURES_CLEANUP
# runs the `stop` arm afterwards. Both arms are idempotent so partial-run states
# (server already up, pid file orphaned) do not break the next ctest
# invocation.
#
# The runners' environment is fixed when the tree is configured, before this
# server has a port, so the port it bound reaches them through a file: the
# `start` arm writes the port the server's `[READY] port=<n>` line names into
# <portfile>, and each runner is given SCE_W3C_HTTP_PORT_FILE=<portfile>
# (tests/w3c/basic_http_test_endpoint.h). <port> is 0 for one the kernel
# assigns, which is what a tree configured with no SCE_W3C_HTTP_PORT passes, so
# no run depends on a fixed port being free.
#
# Usage:
#   http_server_fixture.sh start <node> <server.js> <port> <path> <pidfile> <portfile>
#   http_server_fixture.sh stop  <pidfile> <portfile>

set -euo pipefail

cmd="${1:-}"
shift || true

if [[ "$cmd" == "start" ]]; then
    node_bin="$1"; server_js="$2"; port="$3"; access_path="$4"; pidfile="$5"; portfile="$6"
    logfile="${pidfile%.pid}.log"

    # A server this fixture left up is reused only when the port it wrote is
    # still there to hand on; otherwise it is stopped and started again.
    if [[ -f "$pidfile" ]] && kill -0 "$(cat "$pidfile")" 2>/dev/null; then
        if [[ -s "$portfile" ]]; then
            exit 0
        fi
        kill -TERM "$(cat "$pidfile")" 2>/dev/null || true
    fi
    rm -f "$pidfile" "$logfile" "$portfile"

    nohup "$node_bin" "$server_js" "${port:-0}" "$access_path" \
        > "$logfile" 2>&1 &
    echo $! > "$pidfile"

    for _ in $(seq 1 50); do
        bound="$(sed -n 's/^\[READY\] port=\([0-9]\{1,5\}\)$/\1/p' "$logfile" 2>/dev/null || true)"
        if [[ -n "$bound" ]]; then
            # Written whole and then moved, so a runner never reads half a port.
            printf '%s\n' "$bound" > "$portfile.tmp"
            mv "$portfile.tmp" "$portfile"
            exit 0
        fi
        if ! kill -0 "$(cat "$pidfile")" 2>/dev/null; then
            break
        fi
        sleep 0.1
    done

    kill -TERM "$(cat "$pidfile")" 2>/dev/null || true
    rm -f "$pidfile" "$portfile"
    echo "ERROR: http server did not report [READY] port=<n> within 5s; its log names the address it could not hold" >&2
    [[ -f "$logfile" ]] && cat "$logfile" >&2
    exit 1

elif [[ "$cmd" == "stop" ]]; then
    pidfile="$1"
    portfile="${2:-}"
    if [[ -n "$portfile" ]]; then
        rm -f "$portfile"
    fi
    if [[ -f "$pidfile" ]]; then
        pid="$(cat "$pidfile")"
        if kill -0 "$pid" 2>/dev/null; then
            kill -TERM "$pid" 2>/dev/null || true
            for _ in $(seq 1 20); do
                kill -0 "$pid" 2>/dev/null || break
                sleep 0.1
            done
            kill -KILL "$pid" 2>/dev/null || true
        fi
        rm -f "$pidfile"
    fi
    exit 0
fi

echo "Unknown command: ${cmd:-<empty>}" >&2
exit 2
