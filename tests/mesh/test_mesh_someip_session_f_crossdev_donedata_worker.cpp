// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5c — SOME/IP donedata two-host roundtrip (worker).
//
// Listener half. Hosts THREE worker machines (param / content / nested)
// in one process inside the `sce-mesh-worker` netns (172.16.10.2),
// mirroring the custom_tcp + zenoh donedata fixtures' three-router
// shape. vsomeip 3.x supports multiple applications per process; one
// of them runs the routing manager (selected by config's `routing`
// field — `worker_session_f_donedata_content_sce` here, the
// alphabetically-middle name) and the other two are routing-manager-
// clients sharing the same RM. Each TransportRouter creates its own
// SomeipScxmlInvokeEndpoint with its own vsomeip::application instance
// so service offers go out as three distinct (service-id, instance)
// pairs over the same SD multicast group.
//
// Service IDs (RFC F.X-1 hybrid counter + author-pin allocator, lex-sorted
// across all §9.6 SOMEIP scxml-invoke participants in this deploy):
//   parent_session_f_donedata          → 0x8100 (parent-side service)
//   worker_session_f_donedata_content  → 0x8101 (reliable port 30587)
//   worker_session_f_donedata_nested   → 0x8102 (reliable port 30588)
//   worker_session_f_donedata_param    → 0x8103 (reliable port 30586)
// Counter scheme is collision-free up to the 128-slot ceiling of the
// invoke sub-range [0x8100, 0x817F]; the deploy-time validator surfaces
// overflow / pin-out-of-range / pin-vs-pin collisions explicitly rather
// than the FNV-1a low-byte birthday-paradox shape that preceded RFC F.X-1.

#include "worker_session_f_donedata_param_sm.h"
#include "worker_session_f_donedata_param_transport.h"
#include "worker_session_f_donedata_content_sm.h"
#include "worker_session_f_donedata_content_transport.h"
#include "worker_session_f_donedata_nested_sm.h"
#include "worker_session_f_donedata_nested_transport.h"

#include "SomeipTestUtils.h"

#include <chrono>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <thread>

#ifndef VSOMEIP_CONFIG_PATH
#error "VSOMEIP_CONFIG_PATH must be defined by CMake (path to vsomeip_session_f_donedata_crossdev_worker.json)"
#endif

namespace {
volatile std::sig_atomic_t g_signalled = 0;
void on_signal(int) { g_signalled = 1; }
}  // namespace

int main() {
    setenv("VSOMEIP_CONFIGURATION", VSOMEIP_CONFIG_PATH, 1);

    SCE::Test::Mesh::wipe_stale_vsomeip_sockets();

    std::signal(SIGTERM, on_signal);
    std::signal(SIGINT, on_signal);

    using ParamWorker = SCE::Generated::worker_session_f_donedata_param::worker_session_f_donedata_param;
    using ContentWorker = SCE::Generated::worker_session_f_donedata_content::worker_session_f_donedata_content;
    using NestedWorker = SCE::Generated::worker_session_f_donedata_nested::worker_session_f_donedata_nested;

    ParamWorker w_param;
    w_param.initialize();
    SCE::Generated::worker_session_f_donedata_param::TransportRouter<ParamWorker> r_param({&w_param});
    if (!r_param.init()) {
        std::fprintf(stderr, "FAIL: param router.init() returned false\n");
        return 10;
    }

    ContentWorker w_content;
    w_content.initialize();
    SCE::Generated::worker_session_f_donedata_content::TransportRouter<ContentWorker> r_content({&w_content});
    if (!r_content.init()) {
        std::fprintf(stderr, "FAIL: content router.init() returned false\n");
        return 20;
    }

    NestedWorker w_nested;
    w_nested.initialize();
    SCE::Generated::worker_session_f_donedata_nested::TransportRouter<NestedWorker> r_nested({&w_nested});
    if (!r_nested.init()) {
        std::fprintf(stderr, "FAIL: nested router.init() returned false\n");
        return 30;
    }

    // Sync barrier for run_two_host_fixture.sh. Three vsomeip apps are
    // up; their three Offers are queued for the next SD cycle and will
    // multicast across the veth as soon as the orchestrator's settle
    // window expires.
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
