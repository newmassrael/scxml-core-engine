// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5b — SOME/IP scxml-invoke two-host roundtrip (worker).
//
// Listener half of the §9.6 cross-device fixture. Runs inside the
// `sce-mesh-worker` netns (172.16.10.2). Hosts a TransportRouter for
// scxml_invoke_someip_crossdev_worker, then writes `LISTEN_READY` to
// stderr so the orchestrator can launch the parent + apply its 500 ms
// SOME/IP-SD convergence settle. The pump loop drains incoming wire-14
// envelopes; the codegen-side WorkerSessionHost instantiates the
// trivial-final child in response, observes isFinal()==true after
// initialize(), and publishes wire-15 InvokeStarted + wire-18 InvokeDone
// on the worker's own ScxmlInvokeEndpoint.
//
// Per-binary VSOMEIP_CONFIGURATION:
//   Worker-side vsomeip.json (vsomeip_scxml_invoke_crossdev_worker.json)
//   defines its own routing manager identity + unicast 172.16.10.2.
//   setenv() before init() guarantees vsomeip's app factory picks up the
//   right config when ctest launches this binary via `ip netns exec`.

#include "scxml_invoke_someip_crossdev_worker_sm.h"
#include "scxml_invoke_someip_crossdev_worker_transport.h"

#include "SomeipTestUtils.h"

#include <chrono>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <thread>

#ifndef VSOMEIP_CONFIG_PATH
#error "VSOMEIP_CONFIG_PATH must be defined by CMake (path to vsomeip_scxml_invoke_crossdev_worker.json)"
#endif

namespace {
volatile std::sig_atomic_t g_signalled = 0;
void on_signal(int) { g_signalled = 1; }
}  // namespace

int main() {
    setenv("VSOMEIP_CONFIGURATION", VSOMEIP_CONFIG_PATH, 1);

    // Mirrors the parent half — wipe any stale per-network sockets a
    // crashed prior run might have left in /tmp.
    SCE::Test::Mesh::wipe_stale_vsomeip_sockets();

    std::signal(SIGTERM, on_signal);
    std::signal(SIGINT, on_signal);

    using WorkerEngine =
        SCE::Generated::scxml_invoke_someip_crossdev_worker::scxml_invoke_someip_crossdev_worker;
    WorkerEngine worker;
    worker.initialize();
    SCE::Generated::scxml_invoke_someip_crossdev_worker::TransportRouter<WorkerEngine>
        worker_router({&worker});
    if (!worker_router.init()) {
        std::fprintf(stderr, "FAIL: worker_router.init() returned false\n");
        return 1;
    }

    // Sync barrier for run_two_host_fixture.sh. The orchestrator polls
    // this stderr for `LISTEN_READY`, then sleeps 500 ms (the SCE_TWO_HOST_
    // SETTLE_MS default) so SOME/IP-SD multicast Offer/Find packets can
    // round-trip across the veth pair before launching the parent.
    std::fprintf(stderr, "LISTEN_READY\n");
    std::fflush(stderr);

    while (!g_signalled) {
        worker_router.pumpScxmlInvokeRequests();
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    return 0;
}
