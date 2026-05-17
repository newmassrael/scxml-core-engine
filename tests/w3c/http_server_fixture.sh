#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Lifecycle fixture for the W3C SCXML C.2 BasicHTTP corpus' standalone
# Node server. ctest FIXTURES_SETUP runs the `start` arm before any
# `w3c_c_aot_<N>` test marked NEEDS_HTTP; FIXTURES_CLEANUP runs the
# `stop` arm afterwards. Both arms are idempotent so partial-run states
# (server already up, pid file orphaned) do not break the next ctest
# invocation.
#
# Usage:
#   http_server_fixture.sh start <node> <server.js> <port> <path> <pidfile>
#   http_server_fixture.sh stop  <pidfile>

set -euo pipefail

cmd="${1:-}"
shift || true

if [[ "$cmd" == "start" ]]; then
    node_bin="$1"; server_js="$2"; port="$3"; access_path="$4"; pidfile="$5"
    logfile="${pidfile%.pid}.log"

    if [[ -f "$pidfile" ]] && kill -0 "$(cat "$pidfile")" 2>/dev/null; then
        exit 0
    fi
    rm -f "$pidfile" "$logfile"

    nohup "$node_bin" "$server_js" "$port" "$access_path" \
        > "$logfile" 2>&1 &
    echo $! > "$pidfile"

    for _ in $(seq 1 50); do
        if grep -q "\[READY\]" "$logfile" 2>/dev/null; then
            exit 0
        fi
        sleep 0.1
    done

    kill -TERM "$(cat "$pidfile")" 2>/dev/null || true
    rm -f "$pidfile"
    echo "ERROR: http server did not reach [READY] within 5s" >&2
    [[ -f "$logfile" ]] && cat "$logfile" >&2
    exit 1

elif [[ "$cmd" == "stop" ]]; then
    pidfile="$1"
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
