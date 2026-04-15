// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh Session D: zenoh multi-pattern peer-mode runtime E2E.
//
// Exercises the generated send_zenoh dispatcher against a live zenoh
// peer in the same process. No zenoh router daemon is required — both
// peers open a zenoh::Session in peer mode with an explicit TCP
// locator for deterministic discovery.
//
// Coverage:
//   - FireForget:       send_zenoh → session.put       → raw subscriber  receives envelope
//   - RpcRequest:       send_zenoh → session.get       → raw queryable   replies via Query::reply
//                       → on_reply closure rewrites env.type = reply-event, pattern = RpcReply
//   - EventSubscribe:   send_zenoh → declare_subscriber → raw publisher put → subscriber callback
//   - FieldRead:        send_zenoh → session.get       → raw queryable   replies (FieldNotify)
//
// The receive path on the sender side is driven by the TransportRouter's
// `dispatchToSender` helper, which calls MeshDispatch::dispatchEnvelope
// against the sender engine that was bound in the router ctor. For this
// test we stand up a minimal TestSenderEngine that records every envelope
// it is asked to dispatch — that replaces the earlier pattern of reaching
// into the router's internal receive_callback_ member.

#include "brake_zenoh_multi_transport.h"

#include "mesh/MeshDispatch.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/PatternKind.h"

#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <functional>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <utility>
#include <vector>
#include <zenoh.hxx>

namespace {

constexpr const char* kMotorKey = "sce/brake/motor";
constexpr const char* kListen   = "tcp/127.0.0.1:17447";
constexpr auto        kTimeout  = std::chrono::seconds(5);

// Minimal capture buffer for envelopes the sender side observes. Populated
// by TestSenderEngine::raiseExternal, which is called through the normal
// MeshDispatch path from the router's dispatchToSender. Protected by a
// mutex because zenoh delivers replies/samples on its own runtime threads.
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

    // Wait until `predicate(envelopes)` is true or the timeout elapses.
    template <typename Pred>
    bool wait_for(Pred&& predicate) {
        std::unique_lock<std::mutex> lock(m);
        return cv.wait_for(lock, kTimeout, [&] { return predicate(envelopes); });
    }
};

// Received-event record — what the sender side observes after
// MeshDispatch rewrites env.type (for RpcReply/FieldNotify replies) and
// calls raiseExternal. The original MeshEnvelope is not available here
// because dispatchEnvelope only threads (event_name, data) through.
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
    bool wait_for(Pred&& pred) {
        std::unique_lock<std::mutex> lock(m);
        return cv.wait_for(lock, kTimeout, [&] { return pred(events); });
    }
    void clear() {
        std::lock_guard<std::mutex> lock(m);
        events.clear();
    }
};

// Test-only sender engine that satisfies the API surface TransportRouter
// requires from its SenderEngine template parameter:
//
//   - getPolicy() returning a Policy with getEventFromName(const char*)
//     → required by MeshDispatch::dispatchEnvelope
//   - raiseExternal(event)  /  raiseExternal(event, data)  /  raiseExternal(EventWithMetadata)
//     → called by dispatchEnvelope for inbound patterns (FireForget,
//       RpcRequest, RpcReply, EventNotify, FieldNotify); the metadata
//       overload preserves env.invoke_id as _event.invokeid per W3C SCXML
//       5.10.1 and is what production engines receive.
//   - setMeshSendCallback(cb)
//     → called by TransportRouter's ctor to install the outbound hook
//
// Policy::getEventFromName returns the event name itself (as const char*)
// so raiseExternal can record the post-rewrite type for assertions. This
// lets the test observe everything it needs without ever touching the
// router's internals (no more `router.receive_callback_ = ...`).
struct TestSenderEngine {
    using Event = std::string;

    struct Policy {
        static std::optional<Event> getEventFromName(const char* name) {
            return Event{name};
        }
    };
    // Mirror StaticExecutionEngine's metadata surface so dispatchEnvelope
    // routes the mock through the same raiseExternal(EventWithMetadata)
    // path as production engines — keeps test observations identical to
    // what the generated receivers would see on the wire.
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

zenoh::Session open_peer(const std::string& connect, const std::string& listen) {
    auto config = zenoh::Config::create_default();
    config.insert_json5("mode", "\"peer\"");
    if (!connect.empty()) {
        config.insert_json5("connect/endpoints", std::string("[\"") + connect + "\"]");
    }
    if (!listen.empty()) {
        config.insert_json5("listen/endpoints",  std::string("[\"") + listen  + "\"]");
    }
    // Disable multicast scouting — deterministic CI requires explicit locators.
    config.insert_json5("scouting/multicast/enabled", "false");
    return zenoh::Session::open(std::move(config));
}

SCE::Mesh::MeshEnvelope make_envelope(const std::string& type,
                                      SCE::Mesh::PatternKind pattern,
                                      std::string_view data = {}) {
    SCE::Mesh::MeshEnvelope env;
    env.id = {};  // test-only: deterministic zero id
    env.source = "peer";
    env.type = type;
    env.pattern = pattern;
    env.datacontenttype = data.empty()
        ? SCE::Mesh::PayloadCodec::None
        : SCE::Mesh::PayloadCodec::Json;
    env.data.assign(data.begin(), data.end());
    return env;
}

#define REQUIRE(cond, msg)                                                     \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__); \
            return 1;                                                          \
        }                                                                      \
    } while (0)

int run_test() {
    using namespace SCE::Generated::brake_zenoh_multi;
    using PK = SCE::Mesh::PatternKind;
    using RouterT = TransportRouter<TestSenderEngine>;

    // ── Sender side: TestSenderEngine + generated TransportRouter ──────
    //
    // The router ctor installs the outbound dispatch hook on the sender,
    // captures a const reference to it, and will call back through
    // `dispatchToSender` on transport callback threads. The test observes
    // round-trips through `sender.received_` — no direct access to the
    // router's internal state. This is the textbook pattern after
    // Session D's ctor-injection refactor.
    TestSenderEngine sender;
    RouterT router(sender);

    // ── Order matters: bring up the listener first, then the connector,
    //    so the TCP endpoint is accepting before the peer dials. ──────
    auto motor_session = open_peer(/*connect=*/"", /*listen=*/kListen);

    // Open the router's zenoh session directly with an explicit locator
    // so peer discovery is deterministic under CI. The default `init()`
    // config (generated from deploy.yaml) understands `mode: peer` but
    // not `connect: ["tcp/.."]` yet, so the test cannot just call
    // `router.init()`.
    //
    // TODO(Session E / deploy.yaml locator extension): add `connect` and
    // `listen` fields to the zenoh transport block in deploy.yaml, then
    // collapse this block to `router.init()` and remove the direct
    // member access.
    {
        auto config = zenoh::Config::create_default();
        config.insert_json5("mode", "\"peer\"");
        config.insert_json5("connect/endpoints", std::string("[\"") + kListen + "\"]");
        config.insert_json5("scouting/multicast/enabled", "false");
        // Test-only boundary crossing — see TODO above.
        router.zenoh_session_.emplace(zenoh::Session::open(std::move(config)));
    }

    // FireForget / FieldWrite receive point — plain subscriber on the
    // motor key. Record whatever arrives so the test can assert on it.
    CapturedEvents motor_inbox;
    auto motor_subscriber = motor_session.declare_subscriber(
        zenoh::KeyExpr(kMotorKey),
        [&motor_inbox](const zenoh::Sample& sample) {
            auto bytes = sample.get_payload().as_vector();
            SCE::Mesh::MeshEnvelope env;
            if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), env)) {
                motor_inbox.push(env);
            }
        },
        [] {});

    // RPC / FieldRead receive point — queryable on the same key. Echoes
    // request type back with a small JSON payload so the sender can
    // verify the reply round-trip.
    auto motor_queryable = motor_session.declare_queryable(
        zenoh::KeyExpr(kMotorKey),
        [](const zenoh::Query& query) {
            // Decode the request envelope so the reply can echo its type —
            // it models a real RPC server's behaviour (act on request data,
            // return structured response).
            std::string req_type = "unknown";
            if (auto payload = query.get_payload(); payload) {
                auto bytes = payload->get().as_vector();
                SCE::Mesh::MeshEnvelope req;
                if (SCE::Mesh::decodeEnvelope(bytes.data(), bytes.size(), req)) {
                    req_type = req.type;
                }
            }
            auto resp_env = make_envelope(
                req_type + ".response", PK::FireForget,
                std::string(R"({"result":42})"));
            auto bytes = SCE::Mesh::encodeEnvelope(resp_env);
            query.reply(zenoh::KeyExpr(kMotorKey), zenoh::Bytes(std::move(bytes)));
        },
        [] {});

    // Peer discovery on loopback completes in tens of milliseconds, but the
    // session-level handshake + subscription/queryable declaration
    // propagation have separate latencies. Use a generous initial barrier
    // to amortize CI noise; the per-step condition_variable waits still
    // keep the test fast once discovery has stabilized.
    std::this_thread::sleep_for(std::chrono::seconds(1));

    // ── 1. FireForget: send_zenoh → session.put → motor subscriber ─────
    {
        auto env = make_envelope("service.fire_forget.activate", PK::FireForget);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        REQUIRE(ok, "send_zenoh FireForget returned false");
        REQUIRE(motor_inbox.wait_for([](const auto& v) { return !v.empty(); }),
                "motor subscriber did not receive FireForget envelope");
        std::lock_guard<std::mutex> lk(motor_inbox.m);
        REQUIRE(motor_inbox.envelopes.front().type == "service.fire_forget.activate",
                "FireForget envelope type mismatch on motor side");
        REQUIRE(motor_inbox.envelopes.front().pattern == PK::FireForget,
                "FireForget envelope pattern not preserved");
        motor_inbox.envelopes.clear();
    }

    // ── 2. RpcRequest: send_zenoh → session.get → queryable → on_reply ─
    //
    // The sender's on_reply closure rewrites env.type to the paired
    // reply event "service.response.compute_force" (SCE_MESH.md §13
    // path B: inferred by topology convention from the request name)
    // and pattern to RpcReply. After MeshDispatch delivers the reply,
    // the sender engine sees the rewritten event name — we assert on
    // that.
    {
        auto env = make_envelope("service.request.compute_force", PK::RpcRequest);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        REQUIRE(ok, "send_zenoh RpcRequest returned false");

        REQUIRE(sender.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "service.response.compute_force";
                }),
                "sender engine did not see paired reply 'service.response.compute_force'");
        sender.received_.clear();
    }

    // ── 3. EventSubscribe: sender subscribes, receiver publishes ─────
    //
    // send_zenoh declares a subscriber on the motor key and stores the
    // handle in zenoh_subscribers_. When motor_session publishes, the
    // subscription callback rewrites pattern → EventNotify and dispatches
    // to the sender; the original envelope's type is preserved.
    {
        auto env = make_envelope("event.subscribe.status", PK::EventSubscribe);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        REQUIRE(ok, "send_zenoh EventSubscribe returned false");
        // Briefly yield so the subscription is visible to the other peer
        // before we publish — Zenoh propagates subscription declarations
        // asynchronously.
        std::this_thread::sleep_for(std::chrono::milliseconds(200));

        auto notify = make_envelope("event.notification.status", PK::FireForget);
        auto bytes = SCE::Mesh::encodeEnvelope(notify);
        motor_session.put(zenoh::KeyExpr(kMotorKey), zenoh::Bytes(std::move(bytes)));

        REQUIRE(sender.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "event.notification.status";
                }),
                "sender engine did not see 'event.notification.status' from subscriber");
        sender.received_.clear();
    }

    // ── 4. FieldRead: structural mirror of RpcRequest ──────────────────
    //
    // FieldRead uses session.get; the on_reply closure rewrites pattern
    // to FieldNotify but does not rewrite the type (no sce:reply-event
    // for FieldRead — the field name itself identifies the data). The
    // queryable echoes back "<req.type>.response", so the sender engine
    // should see `field.get.position.response`.
    {
        auto env = make_envelope("field.get.position", PK::FieldRead);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        REQUIRE(ok, "send_zenoh FieldRead returned false");
        REQUIRE(sender.received_.wait_for([](const auto& v) {
                    return !v.empty() && v.back().type == "field.get.position.response";
                }),
                "sender engine did not see FieldRead reply echo");
        sender.received_.clear();
    }

    // ── 5. EventUnsubscribe: handle map entry drops on erase ───────────
    {
        auto env = make_envelope("event.unsubscribe.status", PK::EventUnsubscribe);
        const bool ok = router.send_zenoh(env, kMotorKey, "#motor");
        REQUIRE(ok, "send_zenoh EventUnsubscribe returned false");
        // Direct read of zenoh_subscribers_ — subscription state has no
        // observable side effect from outside the router in the absence
        // of an inbound publish, so this test-only boundary crossing is
        // the only way to assert the handle was dropped.
        //
        // TODO(Session E): once the router exposes a public "is
        // subscribed to #X?" query (or tests move to publish-based
        // observation via a teardown publish), remove this direct
        // access. Keep confined to transport-level acid tests.
        REQUIRE(router.zenoh_subscribers_.count("#motor") == 0,
                "subscriber handle not erased on EventUnsubscribe");
    }

    router.shutdown();
    std::printf("SCE Mesh zenoh multi-pattern runtime E2E: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception& ex) {
        std::fprintf(stderr, "FAIL: uncaught exception: %s\n", ex.what());
        return 1;
    }
}
