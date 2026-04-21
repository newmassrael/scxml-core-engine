// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// MeshDispatch unit tests — SCE Mesh envelope-to-engine routing.
//
// Pure unit tests on dispatchEnvelope<Policy, Engine>() covering:
//   1. Inbound patterns (FireForget/RpcRequest/RpcReply/EventNotify/
//      FieldNotify/FieldRead/FieldWrite) enqueue via raiseExternal;
//      empty and non-empty data. FieldRead/FieldWrite are server-role
//      inbound (SCE_MESH.md §8.3).
//   2. Transport-control patterns (EventSubscribe/EventUnsubscribe)
//      reject (return false) — echo-back guard.
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
using SCE::Mesh::RpcStatus;

namespace {

/// Minimal receiver double. Policy::getEventFromName recognises two event
/// names; the engine records every raiseExternal invocation so tests can
/// assert both delivery and payload preservation. Recording covers both the
/// simple and metadata overloads so the SFINAE branch in dispatchEnvelope is
/// exercised on whichever path is selected.
struct RecordingEngine {
    enum class Event { Request, Reply, ErrorExecution, Unknown };

    struct Policy {
        static std::optional<Event> getEventFromName(const char* name) {
            if (std::strcmp(name, "service.request.compute_force") == 0) {
                return Event::Request;
            }
            if (std::strcmp(name, "service.response.compute_force") == 0) {
                return Event::Reply;
            }
            if (std::strcmp(name, "error.execution") == 0) {
                return Event::ErrorExecution;
            }
            return std::nullopt;
        }
    };

    struct EventWithMetadata {
        Event event{Event::Unknown};
        std::string data;
        std::string invokeId;
        // Additional metadata accepted by MeshDispatch's ChildEvent path.
        std::string type;
        std::string originType;
        std::string origin;
        std::string sendId;
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
    // Event subscribe/unsubscribe are transport control messages handled by
    // the sender router's subscription bookkeeping — they never reach an
    // engine. Echo-back of these patterns indicates a misconfigured transport
    // and MeshDispatch must reject them so the misconfiguration surfaces.
    //
    // FieldRead/FieldWrite were previously in this set but were promoted to
    // inbound-valid when server-side FieldAccess landed (SCE_MESH.md §8.3):
    // the server's queryable / `register_message_handler` receives the
    // getter/setter request and dispatches it to the engine so the matching
    // `<transition event="field.get.X">` / `<transition event="field.set.X">`
    // fires. Covered by `DispatchesFieldAccessInboundOnServerRole` below.
    for (auto pattern : {PatternKind::EventSubscribe,
                         PatternKind::EventUnsubscribe}) {
        RecordingEngine engine;
        MeshEnvelope env;
        env.pattern = pattern;
        env.type = "service.request.compute_force";

        EXPECT_FALSE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)))
            << "Pattern " << static_cast<int>(pattern) << " should reject inbound";
        EXPECT_TRUE(engine.events.empty());
    }
}

TEST(MeshDispatchTest, InvokeErrorRaisesErrorExecutionOnParent) {
    // SCE_MESH.md §9.6.2 wire 20 (`InvokeError`, Child → Parent): the parent's
    // MeshDispatch translates the envelope into a local `error.execution`
    // raise carrying the `rpc_error_message` via `EventWithMetadata::data`.
    // This is the receiver half of the §9.6 line 1396 round-trip that closes
    // the SESSION_F silent-broken window — Session F sub-item 2.
    RecordingEngine engine;
    MeshEnvelope env;
    env.pattern = PatternKind::InvokeError;
    env.type = "error.invoke";  // routing is on pattern, not type
    env.rpc_status = RpcStatus::Unimplemented;
    env.rpc_error_message = "SESSION_F_NOT_IMPLEMENTED";
    env.invoke_id = std::array<std::uint8_t, 16>{
        0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12,
        0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77, 0x66,
    };

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_EQ(engine.events[0].event, RecordingEngine::Event::ErrorExecution);
    EXPECT_EQ(engine.events[0].data, "SESSION_F_NOT_IMPLEMENTED");
    EXPECT_EQ(engine.events[0].invokeId, "0182b14d-a35c-7012-b4de-f0429a887766");
}

TEST(MeshDispatchTest, InvokeErrorWithoutReasonStillRaises) {
    // A wire-20 envelope missing `rpc_error_message` is still dispatched —
    // the empty reason surfaces as empty `EventWithMetadata::data`. Authors'
    // `<transition event="error.execution">` still observes the raise.
    RecordingEngine engine;
    MeshEnvelope env;
    env.pattern = PatternKind::InvokeError;
    env.type = "error.invoke";

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_EQ(engine.events[0].event, RecordingEngine::Event::ErrorExecution);
    EXPECT_TRUE(engine.events[0].data.empty());
}

TEST(MeshDispatchTest, InvokeStartIsRejectedAtDispatchLayer) {
    // SCE_MESH.md §9.6.2 wire 14 (`InvokeStart`, Parent → Child): handled by
    // the TransportRouter's inbound branch (it answers with a wire-20
    // InvokeError inline, without going through engine dispatch). An envelope
    // that reaches MeshDispatch means the upstream branch missed it — we
    // fail-closed drop (same shape as the EventSubscribe echo guard), so no
    // silent fallthrough into raise.
    RecordingEngine engine;
    MeshEnvelope env;
    env.pattern = PatternKind::InvokeStart;
    env.type = "scxml";
    env.invoke_id = std::array<std::uint8_t, 16>{
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x70, 0x01,
        0x90, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    };

    EXPECT_FALSE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    EXPECT_TRUE(engine.events.empty());
}

TEST(MeshDispatchTest, DispatchesFieldAccessInboundOnServerRole) {
    // Server-role FieldRead/FieldWrite are inbound requests produced by the
    // transport layer after decoding a SOME/IP `register_message_handler`
    // message or a Zenoh queryable query. They must reach the engine so the
    // matching `<transition event="field.get.X">` / `<transition
    // event="field.set.X">` fires (SCE_MESH.md §8.3).
    for (auto pattern : {PatternKind::FieldRead, PatternKind::FieldWrite}) {
        RecordingEngine engine;
        auto env = makeRequest("service.request.compute_force");
        env.pattern = pattern;

        EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)))
            << "Pattern " << static_cast<int>(pattern) << " must dispatch inbound";
        ASSERT_EQ(engine.events.size(), 1u);
    }
}
