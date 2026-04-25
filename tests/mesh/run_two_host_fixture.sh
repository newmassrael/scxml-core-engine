#!/usr/bin/env bash
# SCE Mesh §9.6 two-host fixture orchestrator (netns + veth boundary).
#
# Sister to run_two_process_fixture.sh — same worker-first handshake but
# launches each binary inside a Linux network namespace via
# `ip netns exec`. Used by fixtures whose transport (SOME/IP with SD
# enabled, Zenoh peer-mesh) relies on a real network stack — multicast,
# distinct IPs, real ARP — that loopback alone cannot exercise.
#
# Usage:
#   run_two_host_fixture.sh <parent_netns> <worker_netns> <worker_bin> <parent_bin>
#
# Handshake (worker-first, mirrors run_two_process_fixture.sh):
#   1. Worker launches inside <worker_netns>. It opens its transport
#      (vsomeip routing manager + scxml-invoke app, or zenoh peer
#      session + scxml-invoke endpoint) and writes `LISTEN_READY\n` to
#      stderr once ready to receive wire-14. Worker then loops its
#      pumpScxmlInvokeRequests() pump until SIGTERM.
#   2. The orchestrator polls worker stderr for LISTEN_READY
#      (default 5 s timeout). On success it sleeps
#      SCE_TWO_HOST_SETTLE_MS (default 500 ms) — the same convergence
#      window the 1-process Session 4b/5 fixtures already absorb after
#      both routers `init()`. SOME/IP-SD needs the multicast Offer to
#      land on the peer; Zenoh peer-mesh needs the subscriber
#      declaration to fan out across the gossip-derived link table.
#   3. Parent launches inside <parent_netns> and runs to natural
#      completion. The parent's exit code becomes the test result.
#      Worker is SIGTERM'd after the parent exits.
#
# No LISTEN_ENDPOINT line is required. The two transports here use
# fixed addresses baked into vsomeip.json / deploy.yaml (172.16.10.1/2
# from setup_crossdev_netns.sh), not kernel-ephemeral ports — there is
# nothing to hand from worker to parent at startup.
#
# Skip behaviour (exit 77, the ctest SKIP_RETURN_CODE this fixture is
# registered with): the orchestrator surfaces this when running as
# non-root or when either netns is missing. That keeps a default `ctest`
# in a fresh checkout from failing red just because the developer hasn't
# run setup_crossdev_netns.sh yet.
#
# Failure exit codes:
#   exit 64 — misuse (wrong argv count)
#   exit 65 — handshake timeout (worker never emitted LISTEN_READY)
#   exit 77 — skip (no root / netns missing)
#   exit N  — parent exit code (N != 0 passes through verbatim)

set -uo pipefail

PARENT_NS="${1:-}"
WORKER_NS="${2:-}"
WORKER_BIN="${3:-}"
PARENT_BIN="${4:-}"
if [[ -z "$PARENT_NS" || -z "$WORKER_NS" || -z "$WORKER_BIN" || -z "$PARENT_BIN" ]]; then
    echo "usage: $0 <parent_netns> <worker_netns> <worker_bin> <parent_bin>" >&2
    exit 64
fi

if [[ $EUID -ne 0 ]]; then
    # Self-elevate when passwordless sudo is configured. After
    # `sudo visudo` adds `<user> ALL=(ALL) NOPASSWD: ALL`, plain
    # `ctest` runs as the regular user and the orchestrator re-execs
    # itself under sudo here so worker/parent spawning, stderr
    # parsing, and SIGTERM cleanup all run under the same root UID
    # (signaling a root child from non-root would silently fail at
    # teardown). `sudo -n` exits non-zero if a password would be
    # prompted, which falls through to the skip path below.
    if sudo -n true 2>/dev/null; then
        exec sudo -E "$0" "$@"
    fi
    echo "two_host_fixture: skipping — needs root for 'ip netns exec'." >&2
    echo "two_host_fixture:   Recommended: configure passwordless sudo once:" >&2
    echo "two_host_fixture:     sudo visudo" >&2
    echo "two_host_fixture:     # Add: $USER ALL=(ALL) NOPASSWD: ALL" >&2
    echo "two_host_fixture:   After that, plain 'ctest' works without sudo." >&2
    echo "two_host_fixture:   Alternative: rerun the whole test under sudo:" >&2
    echo "two_host_fixture:     sudo ctest -R mesh_.*_scxml_invoke_crossdev" >&2
    exit 77
fi

# `ip netns list` formats as "<name> (id: N)" or "<name>" depending on
# version; awk's first field is the bare name on either layout.
have_netns() {
    ip netns list 2>/dev/null | awk '{print $1}' | grep -qx "$1"
}
if ! have_netns "$PARENT_NS"; then
    echo "two_host_fixture: skipping — netns '$PARENT_NS' missing; "\
"run tests/mesh/setup_crossdev_netns.sh as root first" >&2
    exit 77
fi
if ! have_netns "$WORKER_NS"; then
    echo "two_host_fixture: skipping — netns '$WORKER_NS' missing; "\
"run tests/mesh/setup_crossdev_netns.sh as root first" >&2
    exit 77
fi

HANDSHAKE_TIMEOUT_MS="${SCE_TWO_HOST_HANDSHAKE_MS:-5000}"
HANDSHAKE_POLL_MS=100
HANDSHAKE_ITERS=$(( HANDSHAKE_TIMEOUT_MS / HANDSHAKE_POLL_MS ))
SETTLE_MS="${SCE_TWO_HOST_SETTLE_MS:-500}"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
WORKER_STDERR="$TMPDIR/worker.stderr"
PARENT_STDERR="$TMPDIR/parent.stderr"

ip netns exec "$WORKER_NS" "$WORKER_BIN" 2> "$WORKER_STDERR" &
WORKER_PID=$!

SAW_READY=false
for _ in $(seq 1 "$HANDSHAKE_ITERS"); do
    if [[ -s "$WORKER_STDERR" ]] \
       && grep -q '^LISTEN_READY$' "$WORKER_STDERR" 2>/dev/null; then
        SAW_READY=true
        break
    fi
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

# SD / peer-mesh convergence window. The 1-process Session 4b fixture
# uses 500 ms after init() before parent.initialize(); same convergence
# physics applies across the veth wire (multicast Offer must reach the
# peer, subscriber declarations must propagate). Configurable for
# slower hardware (raspi etc.).
sleep "$(awk "BEGIN { printf \"%.3f\", ${SETTLE_MS}/1000 }")"

ip netns exec "$PARENT_NS" "$PARENT_BIN" 2> "$PARENT_STDERR"
PARENT_EXIT=$?

# Drain the kernel buffer for the worker's last receive callback before
# SIGTERM lands. The 1-process fixtures depend on the in-process callback
# completing before main() returns; here the worker is in a separate
# netns so the callback runs on its own thread — the same race window
# run_two_process_fixture.sh papers over with this 0.3 s grace.
sleep 0.3

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
    echo "orchestrator: parent exited $PARENT_EXIT" >&2
    echo "orchestrator: parent stderr follows --" >&2
    cat "$PARENT_STDERR" >&2 || true
    echo "orchestrator: worker stderr follows --" >&2
    cat "$WORKER_STDERR" >&2 || true
fi

exit $PARENT_EXIT
