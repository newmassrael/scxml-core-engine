// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5b — Zenoh scxml-invoke two-host roundtrip (worker).
//
// Listener half of the §9.6 cross-device fixture for Zenoh. Runs
// inside the `sce-mesh-worker` netns (172.16.10.2). The deploy.yaml
// ecu_worker.transports.zenoh.connect points at the parent's listen
// address (tcp/172.16.10.1:17449) so init() opens a Zenoh peer session
// that immediately tries to dial the parent. If the parent is not yet
// listening (which is the normal case under run_two_host_fixture.sh's
// worker-first launch order) Zenoh's connect logic retries in the
// background, so init() returns successfully and the worker can emit
// LISTEN_READY without waiting for the parent process.
//
// After init the worker loops pumpScxmlInvokeRequests until SIGTERM.
// The codegen-side WorkerSessionHost stages incoming wire-14
// envelopes; the pump drains them, instantiates the trivial-final
// child, and publishes wire-15 InvokeStarted + wire-18 InvokeDone on
// the worker's Publisher.

#include "scxml_invoke_zenoh_crossdev_worker_sm.h"
#include "scxml_invoke_zenoh_crossdev_worker_transport.h"

#include <chrono>
#include <csignal>
#include <cstdio>
#include <thread>

namespace {
volatile std::sig_atomic_t g_signalled = 0;

void on_signal(int) {
    g_signalled = 1;
}
}  // namespace

int main() {
    std::signal(SIGTERM, on_signal);
    std::signal(SIGINT, on_signal);

    using WorkerEngine = SCE::Generated::scxml_invoke_zenoh_crossdev_worker::scxml_invoke_zenoh_crossdev_worker;
    WorkerEngine worker;
    worker.initialize();
    SCE::Generated::scxml_invoke_zenoh_crossdev_worker::TransportRouter<WorkerEngine> worker_router({&worker});
    if (!worker_router.init()) {
        std::fprintf(stderr, "FAIL: worker_router.init() returned false\n");
        return 1;
    }

    // Sync barrier for run_two_host_fixture.sh. The orchestrator polls
    // this stderr for `LISTEN_READY`, then sleeps 500 ms (the SCE_TWO_HOST_
    // SETTLE_MS default) before launching the parent — ample time for
    // worker-side Subscriber declarations to be registered locally so
    // the parent observes them as soon as its own listen binds.
    std::fprintf(stderr, "LISTEN_READY\n");
    std::fflush(stderr);

    while (!g_signalled) {
        worker_router.pumpScxmlInvokeRequests();
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    return 0;
}
