// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 3 cross-device lifecycle — worker half.
//
// Hosts a TransportRouter for `worker_session_f_wired` whose device
// declaration is `transports.custom_tcp.listen: "127.0.0.1:0"`, so
// `init()` binds the device-shared Server on a kernel-ephemeral port.
// The actual port is read back via `custom_tcp_local_endpoint()` (the
// Stage B0 getter) and announced on stderr as
// `LISTEN_ENDPOINT=host:port` so the orchestrator
// (run_two_process_fixture.sh) can export it to the parent process as
// `MESH_PEER_ENDPOINT`. The parent uses that value to populate
// `PortOverride::peer_connect_endpoints["worker_session_f_wired"]`
// before its own `init()` — redirecting its `p2c_to_worker_session_f_wired_`
// Client away from the deploy.yaml `"127.0.0.1:0"` placeholder to the
// live ephemeral port.
//
// Worker-side `c2p_to_parent_session_f_wired_` (wire-15/18 reply
// channel) does NOT need a PortOverride here: the parent's listen is
// a static CMake-configurable port baked into deploy.yaml, so the
// codegen-baked connect endpoint resolves directly. This is the
// "parent static, worker ephemeral" shape documented in
// deploy_session_f_crossdev_lifecycle.yaml.in — one-directional
// ephemeral avoids the bilateral post-init endpoint update that
// current `PortOverride` (init-time only) cannot express.
//
// Shutdown: SIGTERM from the orchestrator flips the shared flag; the
// pump loop drains once more and exits. `TransportRouter::~` then
// tears down the Server, accept thread, and reader threads in RAII
// order.

#include "common/TestScriptEngine.h"
#include "worker_session_f_wired_sm.h"
#include "worker_session_f_wired_transport.h"

#include <chrono>
#include <csignal>
#include <cstdio>
#include <thread>

namespace {
volatile std::sig_atomic_t g_signalled = 0;
void on_signal(int) { g_signalled = 1; }
}  // namespace

int main() {
    std::signal(SIGTERM, on_signal);
    std::signal(SIGINT, on_signal);

    using WorkerEngine = SCE::Generated::worker_session_f_wired::worker_session_f_wired;
    WorkerEngine worker;
    SCE::Test::inject_build_engine(worker);
    worker.initialize();
    SCE::Generated::worker_session_f_wired::TransportRouter<WorkerEngine> router({&worker});
    if (!router.init()) {
        std::fprintf(stderr, "worker: TransportRouter::init() returned false\n");
        return 1;
    }

    // Read back the kernel-assigned ephemeral port so the orchestrator
    // can export it to the parent. A `nullopt` here would mean the
    // device-shared Server never became valid — likely a parse failure
    // on the deploy.yaml listen string, which would have failed init()
    // already. Defensive but not expected to trigger in practice.
    auto ep = router.custom_tcp_local_endpoint();
    if (!ep) {
        std::fprintf(stderr,
                     "worker: custom_tcp_local_endpoint() returned nullopt "
                     "after init — Server bind likely failed\n");
        return 2;
    }
    std::fprintf(stderr, "LISTEN_ENDPOINT=%s\n", ep->c_str());
    std::fprintf(stderr, "LISTEN_READY\n");
    std::fflush(stderr);

    // Pump wire-14/17/19 until SIGTERM. The parent's wire-14 arrives
    // via the device-shared Server callback and lands in the
    // `tcp_invoke_inbound_queue_worker_session_f_wired_` staging
    // buffer; `pumpScxmlInvokeRequests()` drains it under the paired
    // mutex, which is the only safe entry into `WorkerSessionHost`.
    while (!g_signalled) {
        router.pumpScxmlInvokeRequests();
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    return 0;
}
