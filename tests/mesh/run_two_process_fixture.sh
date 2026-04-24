#!/usr/bin/env bash
# SCE Mesh two-process fixture orchestrator (SCE_MESH.md §16.8.3 test harness).
#
# Handshake protocol between this script and the worker / parent binaries:
#
#   1. Worker binds one or more Servers (each on "127.0.0.1:0" for an
#      ephemeral kernel port), reads each local_endpoint back via
#      getsockname, then writes one line per listener to stderr:
#
#          LISTEN_ENDPOINT=host:port            (single-peer backward-compat)
#        or
#          LISTEN_ENDPOINT_<peer>=host:port     (multi-peer fan-out)
#
#      Worker finally writes a sync barrier:
#
#          LISTEN_READY
#
#      and flushes. The worker then blocks until SIGTERM. The barrier
#      lets the orchestrator distinguish "all listeners announced" from
#      "partial announcement still buffered", which matters when a
#      multi-peer worker fans out >1 listen line across separate
#      `fprintf`s.
#
#   2. This script polls the worker's stderr until `LISTEN_READY`
#      appears (default 5s timeout). It then parses every
#      `LISTEN_ENDPOINT(|_<peer>)=` line and exports each as an env
#      var:
#
#          LISTEN_ENDPOINT=ep          → MESH_PEER_ENDPOINT=ep
#          LISTEN_ENDPOINT_<peer>=ep   → MESH_PEER_ENDPOINT_<peer>=ep
#
#      The parent binary is then launched with any extra args the
#      caller passed through. The parent reads whichever env vars it
#      needs at startup and feeds them into
#      `TransportRouter::init(PortOverride)` so the Client(s) dial the
#      kernel-assigned ephemeral port(s) instead of the deploy.yaml
#      `"127.0.0.1:0"` placeholders.
#
#   3. When the parent exits the worker is SIGTERM'd, joined, and the
#      parent's exit code is propagated as this script's exit code.
#
# Failures at each stage emit a diagnostic to stderr before exiting non-zero:
#
#     exit 64  — misuse (wrong argv count)
#     exit 65  — handshake timeout (worker never emitted LISTEN_READY)
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

# Poll for LISTEN_READY. The barrier guarantees every LISTEN_ENDPOINT*=
# line the worker intended to emit has already been flushed when we
# start parsing the file; before the barrier, grep could race the
# worker between multi-peer fanout writes and observe only a subset.
SAW_READY=false
for _ in $(seq 1 "$HANDSHAKE_ITERS"); do
    if [[ -s "$WORKER_STDERR" ]] \
       && grep -q '^LISTEN_READY$' "$WORKER_STDERR" 2>/dev/null; then
        SAW_READY=true
        break
    fi
    # Worker may have crashed before announcing — short-circuit the wait
    # loop so the timeout diagnostic lands promptly rather than 5s late.
    if ! kill -0 "$WORKER_PID" 2>/dev/null; then
        break
    fi
    sleep 0.1
done

if [[ "$SAW_READY" != "true" ]]; then
    echo "orchestrator: worker never emitted LISTEN_READY within ${HANDSHAKE_TIMEOUT_MS}ms" >&2
    echo "orchestrator: worker stderr follows --" >&2
    cat "$WORKER_STDERR" >&2 || true
    kill -TERM "$WORKER_PID" 2>/dev/null || true
    wait "$WORKER_PID" 2>/dev/null || true
    exit 65
fi

# Parse every LISTEN_ENDPOINT*= line into the corresponding env var.
# `declare -A` is unnecessary — a straight export per match keeps bash
# 3.x compatibility and sidesteps the associative-array/ordering
# question (ctest invokes this script under whatever shell CMake picked
# at `add_test` registration, typically `bash` on Linux; nothing in the
# protocol depends on export order).
while IFS='=' read -r key value; do
    case "$key" in
        LISTEN_ENDPOINT)
            export MESH_PEER_ENDPOINT="$value"
            echo "orchestrator: worker listening on $value (pid $WORKER_PID)" >&2
            ;;
        LISTEN_ENDPOINT_*)
            peer="${key#LISTEN_ENDPOINT_}"
            export "MESH_PEER_ENDPOINT_${peer}=$value"
            echo "orchestrator: peer '$peer' listening on $value" >&2
            ;;
    esac
done < <(grep '^LISTEN_ENDPOINT\(_[A-Za-z0-9_]*\)\?=' "$WORKER_STDERR" 2>/dev/null)

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
