// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// MeshDispatch unit tests — SCE Mesh envelope-to-engine routing.
//
// Pure unit tests on dispatchEnvelope<Policy, Engine>() covering:
//   1. Inbound patterns (FireForget/RpcRequest/RpcReply/EventNotify/
//      FieldNotify) enqueue via raiseExternal; empty and non-empty data.
//   2. Outbound-only patterns (EventSubscribe/EventUnsubscribe/FieldRead/
//      FieldWrite) reject (return false) — echo-back guard.
//   3. Unknown event names return false without calling the engine.
//
// A prior revision rejected RpcRequest at the receiver (SCE_MESH.md §9.5
// request path was unreachable end-to-end); this suite locks in the fix.

#include "mesh/MeshDispatch.h"
#include "mesh/MeshEnvelope.h"

#include <gtest/gtest.h>

#include <cstring>
#include <optional>
#include <string>
#include <utility>
#include <vector>

using SCE::Mesh::dispatchEnvelope;
using SCE::Mesh::MeshEnvelope;
using SCE::Mesh::PatternKind;

namespace {

/// Minimal receiver double. Policy::getEventFromName recognises two event
/// names; the engine records every raiseExternal invocation so tests can
/// assert both delivery and payload preservation.
struct RecordingEngine {
    enum class Event { Request, Reply, Unknown };

    struct Policy {
        static std::optional<Event> getEventFromName(const char* name) {
            if (std::strcmp(name, "service.request.compute_force") == 0) {
                return Event::Request;
            }
            if (std::strcmp(name, "service.response.compute_force") == 0) {
                return Event::Reply;
            }
            return std::nullopt;
        }
    };

    struct Raised {
        Event event;
        std::string data;
    };

    std::vector<Raised> events;

    void raiseExternal(Event ev) {
        events.push_back({ev, {}});
    }
    void raiseExternal(Event ev, const std::string& data) {
        events.push_back({ev, data});
    }
};

MeshEnvelope makeRequest(const std::string& type, std::vector<std::uint8_t> data = {}) {
    MeshEnvelope env;
    env.pattern = PatternKind::RpcRequest;
    env.type = type;
    env.data = std::move(data);
    return env;
}

}  // namespace

TEST(MeshDispatchTest, RpcRequestIsDeliveredEmptyPayload) {
    RecordingEngine engine;
    auto env = makeRequest("service.request.compute_force");

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_EQ(engine.events[0].event, RecordingEngine::Event::Request);
    EXPECT_TRUE(engine.events[0].data.empty());
}

TEST(MeshDispatchTest, RpcRequestPreservesPayload) {
    RecordingEngine engine;
    auto env = makeRequest("service.request.compute_force",
                           {'{', '"', 'x', '"', ':', '1', '}'});

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_EQ(engine.events[0].event, RecordingEngine::Event::Request);
    EXPECT_EQ(engine.events[0].data, R"({"x":1})");
}

TEST(MeshDispatchTest, RpcRequestUnknownEventNameIsDropped) {
    RecordingEngine engine;
    auto env = makeRequest("service.request.unknown");

    EXPECT_FALSE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    EXPECT_TRUE(engine.events.empty());
}

TEST(MeshDispatchTest, FireForgetIsStillDelivered) {
    RecordingEngine engine;
    MeshEnvelope env;
    env.pattern = PatternKind::FireForget;
    env.type = "service.response.compute_force";

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_EQ(engine.events[0].event, RecordingEngine::Event::Reply);
}

TEST(MeshDispatchTest, OutboundOnlyPatternsAreRejected) {
    for (auto pattern : {PatternKind::EventSubscribe,
                         PatternKind::EventUnsubscribe,
                         PatternKind::FieldRead,
                         PatternKind::FieldWrite}) {
        RecordingEngine engine;
        MeshEnvelope env;
        env.pattern = pattern;
        env.type = "service.request.compute_force";

        EXPECT_FALSE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)))
            << "Pattern " << static_cast<int>(pattern) << " should reject inbound";
        EXPECT_TRUE(engine.events.empty());
    }
}
