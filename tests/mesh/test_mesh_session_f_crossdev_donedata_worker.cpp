// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6.2 wire-18 cross-device donedata — worker half.
//
// Hosts three worker machines (`param`, `content`, `nested`) on one
// logical ecu. Each machine gets its own TransportRouter whose
// device-shared Server binds to `"127.0.0.1:0"`, so the kernel
// assigns three independent ephemeral ports. Their `host:port` values
// are read back via `custom_tcp_local_endpoint()` (Stage B0) and
// announced as three separate `LISTEN_ENDPOINT_<peer>=...` lines
// before the shared `LISTEN_READY` barrier; the orchestrator
// (run_two_process_fixture.sh) exports each as
// `MESH_PEER_ENDPOINT_<peer>`, and the parent's gtest binary feeds
// all three into a single `PortOverride::peer_connect_endpoints`
// map before its own `init()`.
//
// The three routers pump in the same loop because wire-14 arrival is
// transport-callback-driven (device-shared Server threads) and the
// pump only needs to drain the per-peer staging queue. Sequencing is
// irrelevant: the parent SCXML runs one invoke at a time, but
// wire-ready workers stay idle until they see their own wire-14, so a
// naive round-robin pump is free of priority inversion.

#include "worker_session_f_donedata_param_sm.h"
#include "worker_session_f_donedata_param_transport.h"
#include "worker_session_f_donedata_content_sm.h"
#include "worker_session_f_donedata_content_transport.h"
#include "worker_session_f_donedata_nested_sm.h"
#include "worker_session_f_donedata_nested_transport.h"

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

    using ParamWorker = SCE::Generated::worker_session_f_donedata_param::worker_session_f_donedata_param;
    using ContentWorker = SCE::Generated::worker_session_f_donedata_content::worker_session_f_donedata_content;
    using NestedWorker = SCE::Generated::worker_session_f_donedata_nested::worker_session_f_donedata_nested;

    ParamWorker w_param;
    w_param.initialize();
    SCE::Generated::worker_session_f_donedata_param::TransportRouter<ParamWorker> r_param({&w_param});
    if (!r_param.init()) {
        std::fprintf(stderr, "worker: param router init() failed\n");
        return 10;
    }
    auto ep_param = r_param.custom_tcp_local_endpoint();
    if (!ep_param) {
        std::fprintf(stderr, "worker: param custom_tcp_local_endpoint nullopt\n");
        return 11;
    }

    ContentWorker w_content;
    w_content.initialize();
    SCE::Generated::worker_session_f_donedata_content::TransportRouter<ContentWorker> r_content({&w_content});
    if (!r_content.init()) {
        std::fprintf(stderr, "worker: content router init() failed\n");
        return 20;
    }
    auto ep_content = r_content.custom_tcp_local_endpoint();
    if (!ep_content) {
        std::fprintf(stderr, "worker: content custom_tcp_local_endpoint nullopt\n");
        return 21;
    }

    NestedWorker w_nested;
    w_nested.initialize();
    SCE::Generated::worker_session_f_donedata_nested::TransportRouter<NestedWorker> r_nested({&w_nested});
    if (!r_nested.init()) {
        std::fprintf(stderr, "worker: nested router init() failed\n");
        return 30;
    }
    auto ep_nested = r_nested.custom_tcp_local_endpoint();
    if (!ep_nested) {
        std::fprintf(stderr, "worker: nested custom_tcp_local_endpoint nullopt\n");
        return 31;
    }

    // Multi-peer handshake: one LISTEN_ENDPOINT_<peer>= line per
    // router, then the shared LISTEN_READY barrier. All four lines
    // flushed together so the orchestrator's grep of the stderr file
    // sees a consistent snapshot (see run_two_process_fixture.sh).
    std::fprintf(stderr, "LISTEN_ENDPOINT_worker_session_f_donedata_param=%s\n",
                 ep_param->c_str());
    std::fprintf(stderr, "LISTEN_ENDPOINT_worker_session_f_donedata_content=%s\n",
                 ep_content->c_str());
    std::fprintf(stderr, "LISTEN_ENDPOINT_worker_session_f_donedata_nested=%s\n",
                 ep_nested->c_str());
    std::fprintf(stderr, "LISTEN_READY\n");
    std::fflush(stderr);

    while (!g_signalled) {
        r_param.pumpScxmlInvokeRequests();
        r_content.pumpScxmlInvokeRequests();
        r_nested.pumpScxmlInvokeRequests();
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    return 0;
}
