#!/usr/bin/env bash
# SCE Mesh two-process fixture orchestrator (SCE_MESH.md §16.8.3 test harness).
#
# Handshake protocol between this script and the worker / parent binaries:
#
#   1. Worker binds its Server to "127.0.0.1:0" (ephemeral), reads its
#      Server::local_endpoint() back via getsockname, then writes one
#      line of the form
#
#          LISTEN_ENDPOINT=host:port
#
#      to stderr and flushes. The worker then blocks until SIGTERM.
#
#   2. This script reads the first matching LISTEN_ENDPOINT= line from
#      the worker's stderr (default 5s handshake timeout). It exports the
#      endpoint as MESH_PEER_ENDPOINT and launches the parent binary with
#      any extra args the caller passed through. The parent reads
#      MESH_PEER_ENDPOINT at startup and uses it as the connect target —
#      typically via TransportRouter::init(PortOverride) so the Client
#      dials the kernel-assigned ephemeral port instead of the deploy.yaml
#      "host:0" placeholder.
#
#   3. When the parent exits the worker is SIGTERM'd, joined, and the
#      parent's exit code is propagated as this script's exit code.
#
# Failures at each stage emit a diagnostic to stderr before exiting non-zero:
#
#     exit 64  — misuse (wrong argv count)
#     exit 65  — handshake timeout (worker never emitted LISTEN_ENDPOINT=)
#     exit N   — parent exit code (N != 0 passes through verbatim)
#
# Worker SIGTERM exit status is intentionally discarded — the worker's
# role is purely to listen for the parent; its exit signal is the
# orchestration mechanism, not a test result.

WORKER_BIN="$1"
PARENT_BIN="$2"
if [[ -z "$WORKER_BIN" || -z "$PARENT_BIN" ]]; then
    echo "usage: $0 <worker_bin> <parent_bin> [parent args...]" >&2
    exit 64
fi
shift 2

HANDSHAKE_TIMEOUT_MS="${SCE_TWO_PROCESS_HANDSHAKE_MS:-5000}"
HANDSHAKE_POLL_MS=100
HANDSHAKE_ITERS=$(( HANDSHAKE_TIMEOUT_MS / HANDSHAKE_POLL_MS ))

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
WORKER_STDERR="$TMPDIR/worker.stderr"

"$WORKER_BIN" 2> "$WORKER_STDERR" &
WORKER_PID=$!

MESH_PEER_ENDPOINT=""
for _ in $(seq 1 "$HANDSHAKE_ITERS"); do
    if [[ -s "$WORKER_STDERR" ]]; then
        MESH_PEER_ENDPOINT=$(grep -m1 '^LISTEN_ENDPOINT=' "$WORKER_STDERR" 2>/dev/null \
                             | head -1 | cut -d= -f2-)
        if [[ -n "$MESH_PEER_ENDPOINT" ]]; then
            break
        fi
    fi
    # Worker may have crashed before announcing — short-circuit the wait
    # loop so the timeout diagnostic lands promptly rather than 5s late.
    if ! kill -0 "$WORKER_PID" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

if [[ -z "$MESH_PEER_ENDPOINT" ]]; then
    echo "orchestrator: worker never emitted LISTEN_ENDPOINT= within ${HANDSHAKE_TIMEOUT_MS}ms" >&2
    echo "orchestrator: worker stderr follows --" >&2
    cat "$WORKER_STDERR" >&2 || true
    kill -TERM "$WORKER_PID" 2>/dev/null || true
    wait "$WORKER_PID" 2>/dev/null || true
    exit 65
fi

echo "orchestrator: worker listening on $MESH_PEER_ENDPOINT (pid $WORKER_PID)" >&2

export MESH_PEER_ENDPOINT
"$PARENT_BIN" "$@"
PARENT_EXIT=$?

# Brief grace so the kernel TCP buffer drains to the worker's reader
# thread before SIGTERM arrives. Parent's send() returns as soon as the
# data is copied into the socket buffer; the worker's receive callback
# fires whenever the reader thread next picks it up — a window that is
# sub-millisecond in practice but non-zero. Without this grace, a worker
# that asserts on its received-count before SIGTERM can lose the race.
sleep 0.3

# Tear down the worker. SIGTERM first; if it ignores the signal, escalate
# to SIGKILL after 2s so a stuck worker cannot hang the whole test run.
kill -TERM "$WORKER_PID" 2>/dev/null || true
for _ in $(seq 1 20); do
    if ! kill -0 "$WORKER_PID" 2>/dev/null; then
        break
    fi
    sleep 0.1
done
kill -KILL "$WORKER_PID" 2>/dev/null || true
wait "$WORKER_PID" 2>/dev/null || true

if [[ $PARENT_EXIT -ne 0 ]]; then
    echo "orchestrator: parent exited $PARENT_EXIT; worker stderr follows --" >&2
    cat "$WORKER_STDERR" >&2 || true
fi

exit $PARENT_EXIT
