// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.8.3 reference transport runtime verification.
//
// Two layers of evidence:
//   1. Engine-level FireForget E2E (brake's TransportRouter → custom_tcp
//      TCP loopback → motor's TransportRouter → motor.step()). Proves
//      the routed path including codegen wiring, envelope build, framing,
//      decode, dispatchToSender, and engine state advancement.
//
//   2. Wire-level FIFO ordering assertion (§10.4 conformance). The motor
//      router is torn down and replaced by a thin CustomTcp::Server
//      double whose receive callback records `env.type` strings into a
//      vector. 100 uniquely-numbered envelopes are sent through a fresh
//      Client; the test asserts the recorded sequence equals the send
//      sequence exactly. The engine path cannot validate this directly
//      because motor.scxml's transitions are state-conditional, so a
//      terminal-state check would tolerate certain out-of-order
//      deliveries that produce the same final state.

#include "brake_sm.h"
#include "motor_sm.h"
#include "brake_transport.h"
#include "motor_transport.h"

#include "common/Uuid.h"
#include "mesh/transports/CustomTcpTransport.h"
#include "mesh/MeshEnvelope.h"

#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

// Set by CMake (configure_file substitutes the same value into
// deploy_custom_tcp.yaml.in). Keep the codegen-baked port and the
// in-test recording server's port in lockstep so a CMake override
// flows through both halves of the test atomically.
#ifndef SCE_TEST_CUSTOM_TCP_PORT
#  error "SCE_TEST_CUSTOM_TCP_PORT must be defined by the build system"
#endif
#define SCE_STRINGIFY_INNER(x) #x
#define SCE_STRINGIFY(x) SCE_STRINGIFY_INNER(x)
static constexpr const char* kTestEndpoint =
    "127.0.0.1:" SCE_STRINGIFY(SCE_TEST_CUSTOM_TCP_PORT);

namespace {

constexpr auto kPollInterval = std::chrono::milliseconds(10);
constexpr int kWaitIters = 200;       // 2s budget for the engine path
constexpr int kFifoCount = 100;
constexpr int kFifoWaitIters = 500;   // 5s budget for 100-envelope drain

template <typename Predicate>
bool waitFor(Predicate p, int iters = kWaitIters) {
    for (int i = 0; i < iters; ++i) {
        if (p()) return true;
        std::this_thread::sleep_for(kPollInterval);
    }
    return false;
}

// Thread-safe receive-order recorder for the wire-level FIFO check.
struct OrderRecorder {
    std::mutex m;
    std::condition_variable cv;
    std::vector<std::string> events;

    void push(std::string ev) {
        {
            std::lock_guard<std::mutex> lock(m);
            events.push_back(std::move(ev));
        }
        cv.notify_all();
    }

    bool waitForCount(std::size_t n, std::chrono::seconds timeout) {
        std::unique_lock<std::mutex> lock(m);
        return cv.wait_for(lock, timeout, [&] { return events.size() >= n; });
    }
};

}  // namespace

int main() {
    // ── 0. Endpoint parser strictness (regression guard) ──────
    // Tests the from_chars-based parse — drops here if a future
    // change re-introduces the std::stoi tolerant parsing that
    // would silently accept trailing garbage in a port. The check
    // is embedded in the runtime test (rather than its own ctest)
    // to keep the harness minimal; parse_endpoint is internal so
    // it has no dedicated unit-test surface.
    {
        sockaddr_in addr{};
        if (!SCE::Mesh::CustomTcp::detail::parse_endpoint(kTestEndpoint, addr)) {
            std::fprintf(stderr, "FAIL: parse_endpoint rejected a valid endpoint\n");
            return 100;
        }
        if (SCE::Mesh::CustomTcp::detail::parse_endpoint("127.0.0.1:8080abc", addr)) {
            std::fprintf(stderr, "FAIL: parse_endpoint accepted trailing garbage\n");
            return 101;
        }
        if (SCE::Mesh::CustomTcp::detail::parse_endpoint("127.0.0.1:8080 ", addr)) {
            std::fprintf(stderr, "FAIL: parse_endpoint accepted trailing whitespace\n");
            return 102;
        }
        // Port 0 is the BSD-sockets ephemeral-port sentinel. parse_endpoint
        // must surface it as sin_port=0 so that Server's bind() delegates
        // port assignment to the kernel and Server::local_endpoint() reads
        // the actual port back. A future regression that re-introduces the
        // strict [1, 65535] check would fire here.
        {
            sockaddr_in ephem{};
            if (!SCE::Mesh::CustomTcp::detail::parse_endpoint("127.0.0.1:0", ephem)) {
                std::fprintf(stderr, "FAIL: parse_endpoint rejected ephemeral port 0\n");
                return 103;
            }
            if (ntohs(ephem.sin_port) != 0) {
                std::fprintf(stderr,
                             "FAIL: parse_endpoint accepted :0 but sin_port=%u\n",
                             ntohs(ephem.sin_port));
                return 103;
            }
        }
        // Negative ports still rejected — from_chars parses "-1" into an int
        // successfully, so the `port < 0` guard is what actually protects
        // against the sin_port=htons(uint16_t(-1)) reinterpretation.
        if (SCE::Mesh::CustomTcp::detail::parse_endpoint("127.0.0.1:-1", addr)) {
            std::fprintf(stderr, "FAIL: parse_endpoint accepted negative port\n");
            return 104;
        }
        if (SCE::Mesh::CustomTcp::detail::parse_endpoint("127.0.0.1:65536", addr)) {
            std::fprintf(stderr, "FAIL: parse_endpoint accepted port out of range\n");
            return 105;
        }
    }

    // ── 0b. Ephemeral bind + local_endpoint readback ─────────
    // Proves that `Server` bound with "127.0.0.1:0" delegates port
    // assignment to the kernel and `local_endpoint()` surfaces the
    // kernel's choice. The two-process cross-device harness (Stage
    // A4) relies on exporting this endpoint to the peer process at
    // runtime rather than baking a static port into codegen.
    {
        SCE::Mesh::CustomTcp::Server ephem_server(
            "127.0.0.1:0",
            [](const SCE::Mesh::MeshEnvelope&) {});
        if (!ephem_server.valid()) {
            std::fprintf(stderr, "FAIL: ephemeral Server bind failed on 127.0.0.1:0\n");
            return 106;
        }
        auto ep = ephem_server.local_endpoint();
        if (!ep) {
            std::fprintf(stderr, "FAIL: local_endpoint returned nullopt on valid server\n");
            return 107;
        }
        // Round-trip the readback through parse_endpoint: the returned
        // string must itself be a well-formed endpoint, and the decoded
        // port must be non-zero (the kernel assigned a real ephemeral
        // port, not the literal :0 sentinel we requested). Reusing the
        // parser avoids bespoke string arithmetic and pins the readback
        // format to whatever parse_endpoint accepts.
        sockaddr_in verify{};
        if (!SCE::Mesh::CustomTcp::detail::parse_endpoint(*ep, verify)) {
            std::fprintf(stderr, "FAIL: local_endpoint returned unparseable '%s'\n",
                         ep->c_str());
            return 108;
        }
        if (ntohs(verify.sin_port) == 0) {
            std::fprintf(stderr,
                         "FAIL: local_endpoint returned ephemeral sentinel '%s'\n",
                         ep->c_str());
            return 109;
        }
        // After explicit shutdown the listen fd closes, so the readback
        // must switch to nullopt — guards against a post-teardown read
        // silently returning a stale cached endpoint.
        ephem_server.shutdown();
        if (ephem_server.local_endpoint().has_value()) {
            std::fprintf(stderr, "FAIL: local_endpoint returned a value after shutdown\n");
            return 110;
        }
    }

    // ── 0c. Client::set_connect_endpoint runtime override ───
    // TransportRouter::init(PortOverride) routes each peer's declared
    // override here before the Client's lazy connect. Contract under
    // test: success pre-connect, rejection after a socket is open,
    // rejection after shutdown. The second block then sends through
    // the override endpoint to prove the swap actually redirected the
    // dial (a no-op override would have connected to the decoy port).
    {
        SCE::Mesh::CustomTcp::Client client("127.0.0.1:1", nullptr);
        if (!client.set_connect_endpoint("127.0.0.1:65500")) {
            std::fprintf(stderr, "FAIL: set_connect_endpoint rejected pre-connect override\n");
            return 111;
        }
        // Re-assignment before any send() is also legal — the last
        // assignment wins, so the harness can retry the override after
        // picking up a fresher endpoint from the peer's export.
        if (!client.set_connect_endpoint("127.0.0.1:65501")) {
            std::fprintf(stderr, "FAIL: second pre-connect set_connect_endpoint failed\n");
            return 112;
        }
        client.shutdown();
        if (client.set_connect_endpoint("127.0.0.1:65502")) {
            std::fprintf(stderr, "FAIL: set_connect_endpoint accepted post-shutdown override\n");
            return 113;
        }
    }
    {
        SCE::Mesh::CustomTcp::Server target_server(
            "127.0.0.1:0",
            [](const SCE::Mesh::MeshEnvelope&) {});
        if (!target_server.valid()) {
            std::fprintf(stderr, "FAIL: override-target server bind failed\n");
            return 114;
        }
        auto target_ep = target_server.local_endpoint();
        if (!target_ep) {
            std::fprintf(stderr, "FAIL: override-target local_endpoint returned nullopt\n");
            return 115;
        }
        // Decoy endpoint that must not be where the send lands. Port 1
        // is privileged on Linux so unprivileged bind to it fails; a
        // no-op override would leave connect dialing :1 and send would
        // return false.
        SCE::Mesh::CustomTcp::Client client("127.0.0.1:1", nullptr);
        if (!client.set_connect_endpoint(*target_ep)) {
            std::fprintf(stderr, "FAIL: set_connect_endpoint pre-connect rejected\n");
            return 116;
        }
        SCE::Mesh::MeshEnvelope env;
        env.id = SCE::uuid::v7();
        env.source = "override_route_test";
        env.type = "override.ping";
        env.pattern = SCE::Mesh::PatternKind::FireForget;
        env.datacontenttype = SCE::Mesh::PayloadCodec::None;
        if (!client.send(env)) {
            std::fprintf(stderr, "FAIL: send via overridden endpoint returned false\n");
            return 117;
        }
        // Once the Client connected, the override setter must latch
        // off — a second override would either race the live socket
        // or silently drop a reconnect hint, both worse than failing
        // loudly.
        if (client.set_connect_endpoint("127.0.0.1:9999")) {
            std::fprintf(stderr, "FAIL: set_connect_endpoint accepted post-connect override\n");
            return 118;
        }
    }

    // ── 1. Engine-level FireForget E2E ────────────────────────
    {
        SCE::Generated::brake::brake brake;
        SCE::Generated::motor::motor motor;

        using BrakeRouter = SCE::Generated::brake::TransportRouter<SCE::Generated::brake::brake>;
        using MotorRouter = SCE::Generated::motor::TransportRouter<SCE::Generated::motor::motor>;
        BrakeRouter brake_router({&brake});
        MotorRouter motor_router({&motor});

        if (!motor_router.init()) {
            std::fprintf(stderr, "FAIL: motor router init() returned false (listen bind failed?)\n");
            return 1;
        }

        brake.initialize();
        motor.initialize();

        if (motor.getCurrentState() != SCE::Generated::motor::State::Running) {
            std::fprintf(stderr,
                         "FAIL: motor did not enter 'running' on initialize (state=%d)\n",
                         static_cast<int>(motor.getCurrentState()));
            return 2;
        }

        // brake_press → onentry → <send target="#motor" event="brake.activate"/>
        // → router send_to_motor → custom_tcp client.send → TCP write
        // → motor server reader thread → decodeEnvelope → dispatchToSession
        // → motor.raiseExternal(brake.activate)
        brake.processEvent(SCE::Generated::brake::Event::Brake_press);

        bool delivered = waitFor([&] {
            motor.step();
            return motor.getCurrentState() == SCE::Generated::motor::State::Stopped;
        });

        if (!delivered) {
            std::fprintf(stderr,
                         "FAIL: motor did not transition to 'stopped' (state=%d). "
                         "TCP delivery or dispatch likely broken.\n",
                         static_cast<int>(motor.getCurrentState()));
            return 3;
        }
        // motor_router goes out of scope here, releasing the listen
        // port for the FIFO check below. The immediate re-bind on the
        // same port only succeeds because Server() sets SO_REUSEADDR on
        // the listen socket — a kernel-level TIME_WAIT on the just-
        // closed socket would otherwise reject the bind for ~60s. If a
        // future change drops SO_REUSEADDR, this test fails at the
        // second Server's `valid()` check rather than producing a flaky
        // EADDRINUSE deep in the harness.
    }

    // ── 2. Wire-level FIFO ordering ────────────────────────────
    // Stand up a recording server on the SAME port the engine test
    // just released. Each inbound envelope's `type` is appended to
    // `events` in arrival order; the assertion is exact equality
    // between send and receive sequences.
    OrderRecorder recorder;
    SCE::Mesh::CustomTcp::Server recording_server(
        kTestEndpoint,
        [&recorder](const SCE::Mesh::MeshEnvelope& env) {
            recorder.push(env.type);
        });
    if (!recording_server.valid()) {
        std::fprintf(stderr, "FAIL: recording server bind failed (port reuse race?)\n");
        return 4;
    }

    SCE::Mesh::CustomTcp::Client client(kTestEndpoint, nullptr);
    std::vector<std::string> sent;
    sent.reserve(kFifoCount);

    for (int i = 0; i < kFifoCount; ++i) {
        SCE::Mesh::MeshEnvelope env;
        // Stamp a fresh UUID v7 per envelope — matches the
        // generated mesh-send callback and the other test
        // fixtures (MeshTestUtils::make_envelope). custom_tcp's
        // inbound path bypasses the DedupRouter because
        // `supplies_dedup = true` in transport.rs, so the id
        // value does not affect delivery here; the consistency
        // matters for fixture uniformity and any future tooling
        // (e.g. wire captures) that keys on `env.id`.
        env.id = SCE::uuid::v7();
        env.source = "fifo_test";
        // Sequence-numbered event names: each envelope is unique, so the
        // recorder's vector is a literal log of arrival order. A reorder
        // surfaces as a positional mismatch in the assertion below.
        env.type = "seq." + std::to_string(i);
        env.pattern = SCE::Mesh::PatternKind::FireForget;
        env.datacontenttype = SCE::Mesh::PayloadCodec::None;
        sent.push_back(env.type);
        if (!client.send(env)) {
            std::fprintf(stderr, "FAIL: client.send returned false at iter %d\n", i);
            return 5;
        }
    }

    // Wait for all envelopes to land. The reader thread runs
    // independently of this thread so we cannot rely on a happens-before
    // relation between send() return and recorder.push().
    if (!recorder.waitForCount(kFifoCount, std::chrono::seconds(5))) {
        std::lock_guard<std::mutex> lock(recorder.m);
        std::fprintf(stderr,
                     "FAIL: only %zu of %d envelopes received within 5s\n",
                     recorder.events.size(), kFifoCount);
        return 6;
    }

    // Exact-order comparison. TCP's per-stream FIFO + length-prefix
    // framing should preserve the send order verbatim; any deviation
    // is a regression in the framing layer.
    {
        std::lock_guard<std::mutex> lock(recorder.m);
        if (recorder.events.size() != sent.size()) {
            std::fprintf(stderr,
                         "FAIL: received %zu events, sent %zu\n",
                         recorder.events.size(), sent.size());
            return 7;
        }
        for (std::size_t i = 0; i < sent.size(); ++i) {
            if (recorder.events[i] != sent[i]) {
                std::fprintf(stderr,
                             "FAIL: FIFO violation at index %zu: sent='%s' received='%s'\n",
                             i, sent[i].c_str(), recorder.events[i].c_str());
                return 8;
            }
        }
    }

    std::printf("SCE Mesh custom_tcp runtime verification: PASS (FireForget E2E + FIFO 100/100)\n");
    return 0;
}
