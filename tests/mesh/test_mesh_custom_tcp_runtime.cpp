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
        if (SCE::Mesh::CustomTcp::detail::parse_endpoint("127.0.0.1:0", addr)) {
            std::fprintf(stderr, "FAIL: parse_endpoint accepted port 0\n");
            return 103;
        }
        if (SCE::Mesh::CustomTcp::detail::parse_endpoint("127.0.0.1:65536", addr)) {
            std::fprintf(stderr, "FAIL: parse_endpoint accepted port out of range\n");
            return 104;
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
