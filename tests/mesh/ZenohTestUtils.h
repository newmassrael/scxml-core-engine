// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Shared test-only infrastructure for zenoh mesh runtime tests.
//
// Single source of truth for: thread-safe event capture, envelope
// construction, peer-mode session factory, and the TestSenderEngine
// double that satisfies TransportRouter's template contract.

#pragma once

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
#include <zenoh.hxx>

namespace SCE::Test::Mesh {

constexpr auto kDefaultTimeout = std::chrono::seconds(5);

// ── Thread-safe event capture ────────────────────────────────────────

// Post-dispatch event record — what the sender side observes after
// MeshDispatch rewrites env.type and calls raiseExternal.
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

// Raw envelope capture — records full MeshEnvelope objects before
// MeshDispatch rewriting. Used for transport-level assertions.
struct CapturedEvents {
    std::mutex m;
    std::condition_variable cv;
    std::vector<SCE::Mesh::MeshEnvelope> envelopes;

    void push(const SCE::Mesh::MeshEnvelope& env) {
        {
            std::lock_guard<std::mutex> lock(m);
            envelopes.push_back(env);
        }
        cv.notify_all();
    }

    template <typename Pred>
    bool wait_for(Pred&& predicate,
                  std::chrono::seconds timeout = kDefaultTimeout) {
        std::unique_lock<std::mutex> lock(m);
        return cv.wait_for(lock, timeout,
                           [&] { return predicate(envelopes); });
    }
};

// ── Test sender engine double ────────────────────────────────────────
//
// Satisfies the API surface TransportRouter requires from SenderEngine:
//   - getPolicy() → Policy with getEventFromName(const char*)
//   - raiseExternal(event) / raiseExternal(event, data) / raiseExternal(EventWithMetadata)
//   - setMeshSendCallback(cb)
//
// Policy::getEventFromName returns the event name itself so
// raiseExternal can record the post-rewrite type for assertions.
struct TestSenderEngine {
    using Event = std::string;

    struct Policy {
        static std::optional<Event> getEventFromName(const char* name) {
            return Event{name};
        }
    };

    // Mirror StaticExecutionEngine's metadata surface so dispatchEnvelope
    // routes through the same raiseExternal(EventWithMetadata) path as
    // production engines.
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

// ── Factories ────────────────────────────────────────────────────────

inline zenoh::Session open_peer(const std::string& connect,
                                const std::string& listen) {
    auto config = zenoh::Config::create_default();
    config.insert_json5("mode", "\"peer\"");
    if (!connect.empty()) {
        config.insert_json5("connect/endpoints",
                            std::string("[\"") + connect + "\"]");
    }
    if (!listen.empty()) {
        config.insert_json5("listen/endpoints",
                            std::string("[\"") + listen + "\"]");
    }
    config.insert_json5("scouting/multicast/enabled", "false");
    return zenoh::Session::open(std::move(config));
}

inline SCE::Mesh::MeshEnvelope make_envelope(
        const std::string& type,
        SCE::Mesh::PatternKind pattern,
        std::string_view data = {}) {
    SCE::Mesh::MeshEnvelope env;
    env.id = {};
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
// Returns 1 from the enclosing function on failure (must be a macro).
#ifndef MESH_TEST_REQUIRE
#define MESH_TEST_REQUIRE(cond, msg)                                           \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__); \
            return 1;                                                          \
        }                                                                      \
    } while (0)
#endif
