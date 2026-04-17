// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Shared test-only infrastructure for mesh transport runtime tests.
// Transport-agnostic — no zenoh.hxx or vsomeip.hpp dependency.
//
// Single source of truth for: thread-safe event capture, envelope
// construction, and the TestSenderEngine double that satisfies
// TransportRouter's template contract.

#pragma once

#include "common/Uuid.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/PatternKind.h"

#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <functional>
#include <mutex>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Test::Mesh {

constexpr auto kDefaultTimeout = std::chrono::seconds(5);

// ── Thread-safe event capture ────────────────────────────────────────

struct ReceivedEvent {
    std::string type;
    std::string data;
};

struct ReceivedEvents {
    std::mutex m;
    std::condition_variable cv;
    std::vector<ReceivedEvent> events;

    void push(ReceivedEvent ev) {
        {
            std::lock_guard<std::mutex> lock(m);
            events.push_back(std::move(ev));
        }
        cv.notify_all();
    }

    template <typename Pred>
    bool wait_for(Pred&& pred,
                  std::chrono::seconds timeout = kDefaultTimeout) {
        std::unique_lock<std::mutex> lock(m);
        return cv.wait_for(lock, timeout, [&] { return pred(events); });
    }

    void clear() {
        std::lock_guard<std::mutex> lock(m);
        events.clear();
    }
};

// ── Test sender engine double ────────────────────────────────────────
//
// Satisfies the API surface TransportRouter requires from SenderEngine:
//   - getPolicy() → Policy with getEventFromName(const char*)
//   - raiseExternal(event) / raiseExternal(event, data) / raiseExternal(EventWithMetadata)
//   - setMeshSendCallback(cb)
struct TestSenderEngine {
    using Event = std::string;

    struct Policy {
        static std::optional<Event> getEventFromName(const char* name) {
            return Event{name};
        }
    };

    struct EventWithMetadata {
        Event event;
        std::string data;
        std::string invokeId;
    };

    Policy policy_;
    Policy& getPolicy() { return policy_; }
    std::string currentEventInvokeId() const { return {}; }

    ReceivedEvents received_;
    void raiseExternal(const Event& event_name) {
        received_.push({event_name, ""});
    }
    void raiseExternal(const Event& event_name, const std::string& data) {
        received_.push({event_name, data});
    }
    void raiseExternal(const EventWithMetadata& meta) {
        received_.push({meta.event, meta.data});
    }

    using MeshSendCb = std::function<bool(const std::string&, const std::string&,
                                          const std::string&, const std::string&,
                                          const std::string&)>;
    MeshSendCb mesh_send_cb_;
    void setMeshSendCallback(MeshSendCb cb) { mesh_send_cb_ = std::move(cb); }
};

// ── Envelope factory ─────────────────────────────────────────────────

inline SCE::Mesh::MeshEnvelope make_envelope(
        const std::string& type,
        SCE::Mesh::PatternKind pattern,
        std::string_view data = {}) {
    SCE::Mesh::MeshEnvelope env;
    // Mirror the generated mesh send callback: stamp a fresh UUID v7 on
    // every envelope so successive fixtures from the same logical source
    // are distinguishable. SCE_MESH.md §10.5 dedup keys on
    // (env.source, env.id); a zero-id default would alias every test
    // envelope into a single dedup slot.
    env.id = SCE::uuid::v7();
    env.source = "test";
    env.type = type;
    env.pattern = pattern;
    env.datacontenttype = data.empty()
        ? SCE::Mesh::PayloadCodec::None
        : SCE::Mesh::PayloadCodec::Json;
    env.data.assign(data.begin(), data.end());
    return env;
}

}  // namespace SCE::Test::Mesh

// ── Test assertion macro ─────────────────────────────────────────────
#ifndef MESH_TEST_REQUIRE
#define MESH_TEST_REQUIRE(cond, msg)                                           \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__); \
            return 1;                                                          \
        }                                                                      \
    } while (0)
#endif
