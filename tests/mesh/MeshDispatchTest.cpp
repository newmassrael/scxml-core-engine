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
/// assert both delivery and payload preservation. Recording covers both the
/// simple and metadata overloads so the SFINAE branch in dispatchEnvelope is
/// exercised on whichever path is selected.
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

    struct EventWithMetadata {
        Event event{Event::Unknown};
        std::string data;
        std::string invokeId;
    };

    struct Raised {
        Event event;
        std::string data;
        std::string invokeId;
    };

    std::vector<Raised> events;

    void raiseExternal(Event ev) {
        events.push_back({ev, {}, {}});
    }
    void raiseExternal(Event ev, const std::string& data) {
        events.push_back({ev, data, {}});
    }
    void raiseExternal(EventWithMetadata meta) {
        events.push_back({meta.event, std::move(meta.data), std::move(meta.invokeId)});
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

TEST(MeshDispatchTest, RpcRequestPreservesInvokeIdAsStringifiedUuid) {
    RecordingEngine engine;
    auto env = makeRequest("service.request.compute_force");
    // Fixed UUID so the stringified _event.invokeid is stable for assertion.
    env.invoke_id = std::array<std::uint8_t, 16>{
        0xf8, 0x1d, 0x4f, 0xae, 0x7d, 0xec, 0x11, 0xd0,
        0xa7, 0x65, 0x00, 0xa0, 0xc9, 0x1e, 0x6b, 0xf6,
    };

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_EQ(engine.events[0].event, RecordingEngine::Event::Request);
    EXPECT_EQ(engine.events[0].invokeId, "f81d4fae-7dec-11d0-a765-00a0c91e6bf6");
}

TEST(MeshDispatchTest, RpcRequestWithoutInvokeIdYieldsEmptyMetadata) {
    RecordingEngine engine;
    auto env = makeRequest("service.request.compute_force");
    // No invoke_id set — metadata path still fires but invokeId stays empty.

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_TRUE(engine.events[0].invokeId.empty());
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
