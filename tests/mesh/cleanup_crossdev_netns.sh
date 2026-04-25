#!/usr/bin/env bash
# Remove the §9.6 two-host test netns created by setup_crossdev_netns.sh.
# Idempotent — safe to run when the netns are already absent.
set -euo pipefail

PARENT_NS=${PARENT_NS:-sce-mesh-parent}
WORKER_NS=${WORKER_NS:-sce-mesh-worker}

[[ $EUID -eq 0 ]] || {
    echo "cleanup_crossdev_netns: must run as root (try: sudo $0)" >&2
    exit 1
}

ip netns delete "$WORKER_NS" 2>/dev/null || true
ip netns delete "$PARENT_NS" 2>/dev/null || true
echo "sce mesh netns removed ($PARENT_NS, $WORKER_NS)"
