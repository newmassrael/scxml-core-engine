#!/usr/bin/env bash
# Create the §9.6 two-host test network — two netns connected by a veth
# pair so the parent and worker each see a distinct IP, distinct ARP cache,
# distinct routing table, and distinct multicast domain. Mirrors the
# tc8-harness mock_dut/env/setup-netns.sh layout (the load-bearing
# precedent for "two computers on one Linux host" in this org's test
# infra) but uses sce-mesh-* names so the SCE fixtures can coexist with a
# tc8-harness checkout on the same machine.
#
# Why netns + veth instead of loopback aliases:
#   * SOME/IP-SD relies on UDP multicast to 224.244.224.245; loopback
#     alone refuses multicast routing in vsomeip's default config, so the
#     parent and worker would never discover each other when SD is on.
#     veth links carry multicast end-to-end like a real Ethernet wire.
#   * Zenoh peer-mesh discovery convergence behaves differently when both
#     peers share the same routing table. netns gives each side its own
#     table, mirroring the production "two ECUs on one CAN/Ethernet
#     segment" topology the §9.6 cross-device classifier targets.
#   * vsomeip's per-`network` routing manager UNIX socket lives under
#     /tmp; running both sides under one PID namespace + one filesystem
#     would force the two routing managers to share that socket. With
#     distinct vsomeip.json `network` fields per side (configured by the
#     fixture, not this script) plus distinct netns the parent and worker
#     each own a private RM domain — the textbook two-ECU pattern.
#
# Idempotent: tears down any prior state before creating.
#
# Envvars (all optional):
#   PARENT_NS, WORKER_NS    netns names           (default: sce-mesh-parent / sce-mesh-worker)
#   VETH_P, VETH_W          veth interface names  (default: veth-sce-p / veth-sce-w)
#   PARENT_IP, WORKER_IP    CIDR addresses        (default: 172.16.10.1/24 / 172.16.10.2/24)
#   MCAST_ROUTE             multicast dest        (default: 224.0.0.0/4 — covers SOME/IP-SD)
set -euo pipefail

PARENT_NS=${PARENT_NS:-sce-mesh-parent}
WORKER_NS=${WORKER_NS:-sce-mesh-worker}
VETH_P=${VETH_P:-veth-sce-p}
VETH_W=${VETH_W:-veth-sce-w}
PARENT_IP=${PARENT_IP:-172.16.10.1/24}
WORKER_IP=${WORKER_IP:-172.16.10.2/24}
MCAST_ROUTE=${MCAST_ROUTE:-224.0.0.0/4}

die() { echo "setup_crossdev_netns: $*" >&2; exit 1; }

[[ $EUID -eq 0 ]] || die "must run as root (try: sudo $0)"

# Tear down previous state. Deleting the netns implicitly removes the
# veth peer that lives inside it; the explicit `ip link del` below
# handles a half-finished prior run where one veth ended up in the root
# namespace before we could move it.
ip netns delete "$WORKER_NS" 2>/dev/null || true
ip netns delete "$PARENT_NS" 2>/dev/null || true
ip link del "$VETH_P" 2>/dev/null || true

ip netns add "$PARENT_NS"
ip netns add "$WORKER_NS"

ip link add "$VETH_P" type veth peer name "$VETH_W"
ip link set "$VETH_P" netns "$PARENT_NS"
ip link set "$VETH_W" netns "$WORKER_NS"

ip -n "$PARENT_NS" link set lo up
ip -n "$WORKER_NS" link set lo up
ip -n "$PARENT_NS" link set "$VETH_P" up
ip -n "$WORKER_NS" link set "$VETH_W" up
ip -n "$PARENT_NS" addr add "$PARENT_IP" dev "$VETH_P"
ip -n "$WORKER_NS" addr add "$WORKER_IP" dev "$VETH_W"

# Multicast route is the load-bearing piece for SOME/IP-SD: vsomeip's
# default SD destination is 224.244.224.245:30490, which falls inside
# 224.0.0.0/4. Without a route the kernel returns ENETUNREACH on the
# first sendto() and the routing manager flips into "no peers" state.
ip -n "$PARENT_NS" route add "$MCAST_ROUTE" dev "$VETH_P"
ip -n "$WORKER_NS" route add "$MCAST_ROUTE" dev "$VETH_W"

# Reachability sanity. A failure here means veth pair, addressing, or
# the kernel's net.ipv4.ip_forward state is off — easier to surface now
# than as an obscure vsomeip routing manager log later.
ip netns exec "$PARENT_NS" ping -c 1 -W 1 "${WORKER_IP%/*}" >/dev/null \
    || die "parent ($PARENT_NS) → worker (${WORKER_IP%/*}) ping failed"

cat <<INFO
sce mesh netns ready:
  $PARENT_NS: $PARENT_IP on $VETH_P
  $WORKER_NS: $WORKER_IP on $VETH_W

build with -DSCE_ENABLE_NETNS_TESTS=ON to register the §9.6 two-host
fixtures (mesh_someip_scxml_invoke_crossdev, mesh_zenoh_scxml_invoke_crossdev).

For sudoless ctest runs, configure passwordless sudo once:
  sudo visudo
  # Add: ${SUDO_USER:-$USER} ALL=(ALL) NOPASSWD: ALL
After that, plain 'ctest -R mesh_.*_scxml_invoke_crossdev' works as the
regular user — the orchestrator self-elevates only when needed and skips
gracefully (code 77 = ctest "Skipped") when neither root nor passwordless
sudo is available, so a fresh checkout never sees a Failed result here.
Alternative without sudoers config: rerun the whole test under sudo:
  sudo ctest -R mesh_.*_scxml_invoke_crossdev

teardown:
  sudo $(dirname "$(readlink -f "$0")")/cleanup_crossdev_netns.sh
INFO
