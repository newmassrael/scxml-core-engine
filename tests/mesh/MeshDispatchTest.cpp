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
        static std::optional<Event> getEventFromName(const char *name) {
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

    // SCE_MESH.md §mesh-10.7 wires six `_event` fields from an inbound
    // envelope, so the double records all of them. Recording only
    // event/data/invokeId (the earlier shape) made the origin, origintype and
    // sendid rows of that table unobservable — every assertion about them
    // would have passed vacuously.
    struct Raised {
        Event event;
        std::string data;
        std::string invokeId;
        std::string origin;
        std::string originType;
        std::string sendId;
    };

    std::vector<Raised> events;

    void raiseExternal(Event ev) {
        events.push_back({ev, {}, {}, {}, {}, {}});
    }

    void raiseExternal(Event ev, const std::string &data) {
        events.push_back({ev, data, {}, {}, {}, {}});
    }

    void raiseExternal(EventWithMetadata meta) {
        events.push_back({meta.event, std::move(meta.data), std::move(meta.invokeId), std::move(meta.origin),
                          std::move(meta.originType), std::move(meta.sendId)});
    }

    // SCE_MESH.md §9.6.4 step 4: the parent identifies the `<invoke>` a
    // wire-16 ChildEvent belongs to before the event may take part in
    // transition selection. A real parent answers from `activeInvokes_`;
    // the double answers from a set the test controls.
    std::vector<std::string> activeChildSessions;

    bool hasActiveChildSession(const std::string &childSessionId) const {
        for (const auto &id : activeChildSessions) {
            if (id == childSessionId) {
                return true;
            }
        }
        return false;
    }
};

MeshEnvelope makeRequest(const std::string &type, std::vector<std::uint8_t> data = {}) {
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
    auto env = makeRequest("service.request.compute_force", {'{', '"', 'x', '"', ':', '1', '}'});

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
        0xf8, 0x1d, 0x4f, 0xae, 0x7d, 0xec, 0x11, 0xd0, 0xa7, 0x65, 0x00, 0xa0, 0xc9, 0x1e, 0x6b, 0xf6,
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
    for (auto pattern : {PatternKind::EventSubscribe, PatternKind::EventUnsubscribe}) {
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
        0x01, 0x82, 0xb1, 0x4d, 0xa3, 0x5c, 0x70, 0x12, 0xb4, 0xde, 0xf0, 0x42, 0x9a, 0x88, 0x77, 0x66,
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
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x70, 0x01, 0x90, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    };

    EXPECT_FALSE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    EXPECT_TRUE(engine.events.empty());
}

// SCE_MESH.md §9.6.4 — a wire-16 ChildEvent is enqueued on the parent only
// while the invoke that produced it is still active. `child_session_id` is
// the discriminator the section's step 4 matches on and the same key the
// parent's `<finalize>` lookup uses.
namespace {

MeshEnvelope makeChildEvent(const std::string &childSessionId) {
    MeshEnvelope env;
    env.pattern = PatternKind::ChildEvent;
    env.type = "service.request.compute_force";
    env.child_session_id = childSessionId;
    env.subject = "child_send_7";
    env.invoke_id = std::array<std::uint8_t, 16>{
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x70, 0x08, 0x90, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
    };
    return env;
}

}  // namespace

TEST(MeshDispatchTest, ChildEventFromActiveInvokeIsDelivered) {
    RecordingEngine engine;
    engine.activeChildSessions.push_back("dev_a:parent:remote_inv");

    EXPECT_TRUE((
        dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(makeChildEvent("dev_a:parent:remote_inv"), engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    // §mesh-9.6.3: the invoke correlation reaches the parent as _event.invokeid.
    EXPECT_FALSE(engine.events[0].invokeId.empty());
}

TEST(MeshDispatchTest, ChildEventFromRetiredInvokeIsDiscarded) {
    // §mesh-9.6.4: "If the parent has already exited the invoking state when
    // the event arrives (step 4 fails), the event is discarded silently
    // (finalize not executed, transition not considered)." Exiting the
    // invoking state erases the `activeInvokes_` entry, so a late wire-16 —
    // one the child emitted before its wire-19 cancel landed — must not reach
    // the parent's queue, where it would otherwise be offered to transition
    // selection in whatever state the parent moved on to.
    RecordingEngine engine;
    engine.activeChildSessions.push_back("dev_a:parent:other_inv");

    EXPECT_FALSE((
        dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(makeChildEvent("dev_a:parent:remote_inv"), engine)));
    EXPECT_TRUE(engine.events.empty()) << "a ChildEvent whose invoke is no longer active was enqueued on the "
                                          "parent — §9.6.4's late-event discard is not enforced";
}

TEST(MeshDispatchTest, InboundEnvelopeCarriesMeshOriginFromSource) {
    // SCE_MESH.md §mesh-10.7: `_event.origin` = `mesh://<envelope.source>` for
    // a distributed event. The section's whole point is surface compatibility
    // — `<transition cond="_event.origin == 'mesh://chassis'">` must select
    // the same way whether the event arrived locally or over any transport —
    // so an empty origin is not a cosmetic gap: the guard silently never fires.
    for (auto pattern : {PatternKind::FireForget, PatternKind::EventNotify, PatternKind::FieldNotify,
                         PatternKind::RpcRequest, PatternKind::RpcReply}) {
        RecordingEngine engine;
        auto env = makeRequest("service.request.compute_force");
        env.pattern = pattern;
        env.source = "chassis";

        EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
        ASSERT_EQ(engine.events.size(), 1u);
        EXPECT_EQ(engine.events[0].origin, "mesh://chassis")
            << "pattern " << static_cast<int>(pattern) << " delivered without the §10.7 origin URI";
    }
}

TEST(MeshDispatchTest, InboundEnvelopeCarriesScxmlProcessorOriginType) {
    // §mesh-10.7 pins `_event.origintype` to the W3C SCXML processor URI for
    // inter-SCXML mesh traffic. Every envelope reaching dispatchEnvelope is
    // SCE's own CBOR MeshEnvelope (§mesh-7.5), i.e. SCE↔SCE by construction —
    // raw bus traffic never takes this path — so the URI is unconditional here.
    RecordingEngine engine;
    auto env = makeRequest("service.request.compute_force");
    env.pattern = PatternKind::FireForget;
    env.source = "chassis";

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_EQ(engine.events[0].originType, "http://www.w3.org/TR/scxml/#SCXMLEventProcessor");
}

TEST(MeshDispatchTest, InboundEnvelopeCarriesSubjectAsSendId) {
    // §mesh-10.7: `_event.sendid` = envelope `subject`, "or unset if not
    // <send>-originated". Both halves are asserted — an implementation that
    // stamped a placeholder when subject is absent would break `<cancel>`
    // matching against a sendid the author never issued.
    RecordingEngine withSubject;
    auto env = makeRequest("service.request.compute_force");
    env.pattern = PatternKind::FireForget;
    env.source = "chassis";
    env.subject = "send_42";
    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, withSubject)));
    ASSERT_EQ(withSubject.events.size(), 1u);
    EXPECT_EQ(withSubject.events[0].sendId, "send_42");

    RecordingEngine withoutSubject;
    auto bare = makeRequest("service.request.compute_force");
    bare.pattern = PatternKind::FireForget;
    bare.source = "chassis";
    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(bare, withoutSubject)));
    ASSERT_EQ(withoutSubject.events.size(), 1u);
    EXPECT_TRUE(withoutSubject.events[0].sendId.empty());
}

TEST(MeshDispatchTest, ChildEventKeepsSessionUriOriginNotMeshUri) {
    // §mesh-9.6.3 L1463-1466 governs the wire-16 ChildEvent arm and overrides
    // the generic §mesh-10.7 row: the parent's `<finalize>`/autoforward match
    // compares `activeInvokes_[id].sessionId == _event.origin`, and that
    // sessionId is the §mesh-9.6.1 child session id. Rewriting this arm to
    // `mesh://<source>` for table uniformity would silently stop every remote
    // finalize from matching, so the divergence is locked in deliberately.
    RecordingEngine engine;
    engine.activeChildSessions.push_back("dev_a:parent:remote_inv");
    auto env = makeChildEvent("dev_a:parent:remote_inv");
    env.source = "child_machine";

    EXPECT_TRUE((dispatchEnvelope<RecordingEngine::Policy, RecordingEngine>(env, engine)));
    ASSERT_EQ(engine.events.size(), 1u);
    EXPECT_EQ(engine.events[0].origin, "dev_a:parent:remote_inv");
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
