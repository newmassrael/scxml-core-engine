// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh two-process orchestration smoke — worker half.
//
// Binds Server to "127.0.0.1:0" (kernel-assigned ephemeral), announces
// the resolved endpoint to the orchestrator via the stderr handshake
// line ("LISTEN_ENDPOINT=host:port"), then blocks until SIGTERM. The
// receive callback records arrivals in an atomic counter; on graceful
// shutdown the worker writes the count to stderr so the harness can
// sanity-check that the parent's envelope actually landed.

#include "mesh/MeshEnvelope.h"
#include "mesh/transports/CustomTcpTransport.h"

#include <atomic>
#include <csignal>
#include <cstdio>
#include <cstdlib>

namespace {
std::atomic<int> g_received{0};
volatile std::sig_atomic_t g_signalled{0};
void on_signal(int) { g_signalled = 1; }
}  // namespace

int main() {
    // Install SIGTERM handler so the orchestrator's tear-down step is
    // graceful. Default action (process termination) also works but
    // would mask the received-count diagnostic the test reads back.
    std::signal(SIGTERM, on_signal);
    std::signal(SIGINT, on_signal);

    SCE::Mesh::CustomTcp::Server server(
        "127.0.0.1:0",
        [](const SCE::Mesh::MeshEnvelope& env) {
            if (env.type == "smoke.ping") {
                g_received.fetch_add(1, std::memory_order_release);
            }
        });
    if (!server.valid()) {
        std::fprintf(stderr, "worker: Server bind on 127.0.0.1:0 failed\n");
        return 1;
    }

    auto ep = server.local_endpoint();
    if (!ep) {
        std::fprintf(stderr, "worker: local_endpoint() returned nullopt after bind\n");
        return 2;
    }

    // Handshake lines the orchestrator (run_two_process_fixture.sh)
    // greps out of our stderr. `LISTEN_ENDPOINT=` carries the port;
    // `LISTEN_READY` is the barrier the orchestrator polls for before
    // parsing — multi-peer workers fan out multiple LISTEN lines, and
    // the barrier is what tells the script "all listeners announced"
    // uniformly across single- and multi-peer callers. Both lines
    // flushed together so the orchestrator's grep sees them atomically.
    std::fprintf(stderr, "LISTEN_ENDPOINT=%s\n", ep->c_str());
    std::fprintf(stderr, "LISTEN_READY\n");
    std::fflush(stderr);

    // Wait for SIGTERM. `pause` returns -1 when interrupted by any
    // unblocked signal, which is what the orchestrator sends after the
    // parent exits. The receive callback runs on the Server's reader
    // thread independently of this main-thread loop.
    while (!g_signalled) {
        pause();
    }

    std::fprintf(stderr, "worker: received %d envelope(s) before shutdown\n",
                 g_received.load(std::memory_order_acquire));
    return g_received.load(std::memory_order_acquire) >= 1 ? 0 : 3;
}
