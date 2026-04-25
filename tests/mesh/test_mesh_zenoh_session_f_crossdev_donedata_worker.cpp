// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §9.6 Session 5c — Zenoh donedata two-host roundtrip (worker).
//
// Listener half. Hosts THREE worker machines (param / content / nested)
// in a single process inside the `sce-mesh-worker` netns (172.16.10.2),
// mirroring the custom_tcp 2-process donedata fixture's three-router
// shape. Each TransportRouter opens its own Zenoh peer session that
// connects to tcp/172.16.10.1:17450; their codegen-derived
// `<machine>_scxml_invoke` Subscriber declarations all propagate to the
// parent peer once the TCP link establishes. The pump loop drains
// incoming wire-14 envelopes; the codegen-side WorkerSessionHost
// instantiates the trivial-final + donedata child in response, observes
// isFinal()==true after initialize(), and publishes wire-15
// InvokeStarted + wire-18 InvokeDone (with the donedata payload) on the
// router's own Publisher.
//
// Three sessions in one process: Zenoh's runtime allows independent
// peer sessions to coexist (they each manage their own background
// I/O threads and gossip-derived link tables). The custom_tcp 2-proc
// equivalent does the same with three Servers — same multiplexing
// pattern, different transport.

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

    // Sync barrier for run_two_host_fixture.sh. Three sessions are up;
    // their Subscriber declarations are queued locally and will fan out
    // to the parent peer as soon as the parent's listen socket binds.
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
