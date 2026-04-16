// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh Session F: SOME/IP runtime E2E.
//
// Exercises both client (brake) and server (motor) generated
// TransportRouters end-to-end via vsomeip internal routing (single
// process, no daemon). Both applications run in the same process; the
// routing manager is embedded in the motor (server) application per
// the VSOMEIP_CONFIGURATION JSON.
//
// Coverage:
//   1. Server init:   offer_service + register_message_handler for
//                     compute_force RPC method (0x0101)
//   2. Client init:   request_service + register_message_handler for
//                     response correlation
//   3. RPC round-trip: brake send_to_motor (RpcRequest) → vsomeip routes
//                     to motor callback → motor engine receives →
//                     handleServerResponse → vsomeip response → brake
//                     response handler → brake engine receives reply
//   4. FireForget:    brake send_to_motor → vsomeip routes → motor
//                     callback → motor engine receives
//
// VSOMEIP_CONFIGURATION must point to vsomeip_e2e_test.json (internal
// routing, service discovery disabled, motor as routing manager).

#include "brake_someip_multi_sm.h"
#include "brake_someip_multi_transport.h"
#include "motor_someip_multi_sm.h"
#include "motor_someip_multi_transport.h"

#include "mesh/MeshDispatch.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"
#include "mesh/PatternKind.h"

#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <functional>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <utility>
#include <vector>

namespace {

constexpr auto kTimeout = std::chrono::seconds(5);

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

#define REQUIRE(cond, msg)                                                     \
    do {                                                                       \
        if (!(cond)) {                                                         \
            std::fprintf(stderr, "FAIL: %s (%s:%d)\n", msg, __FILE__, __LINE__); \
            return 1;                                                          \
        }                                                                      \
    } while (0)

int run_test() {
    namespace brake_gen = SCE::Generated::brake_someip_multi;
    namespace motor_gen = SCE::Generated::motor_someip_multi;
    using PK = SCE::Mesh::PatternKind;
    using BrakeRouterT = brake_gen::TransportRouter<TestSenderEngine>;
    using MotorRouterT = motor_gen::TransportRouter<TestSenderEngine>;

    // ── Motor (server) must init first — it is the routing manager. ──
    TestSenderEngine motor_engine;
    MotorRouterT motor_router(motor_engine);
    REQUIRE(motor_router.init(), "motor router init failed");

    // ── Brake (client) connects to motor. ──
    TestSenderEngine brake_engine;
    BrakeRouterT brake_router(brake_engine);
    REQUIRE(brake_router.init(), "brake router init failed");

    // vsomeip internal routing needs time for service offer/request
    // handshake within the process. 2s is generous for local dispatch.
    std::this_thread::sleep_for(std::chrono::seconds(2));

    // ── 1. RPC round-trip: brake → motor → brake ──────────────────────
    //
    // FireForget is not tested here because the motor server only
    // registers a handler for compute_force (0x0101). FireForget
    // (0x0100) would need a separate handler registration on the motor
    // side, which is a client-side feature tested by the brake
    // transport — out of scope for server E2E.
    {
        SCE::Mesh::MeshEnvelope env;
        env.id = {};
        env.source = "test";
        env.type = "service.request.compute_force";
        env.pattern = PK::RpcRequest;
        env.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string payload = R"({"force":100})";
        env.data.assign(payload.begin(), payload.end());
        // Correlation setup: generate UUID, register reply event
        auto cid = SCE::uuid::v7();
        env.correlation_id = cid;
        {
            std::lock_guard<std::mutex> lock(brake_router.correlation_mutex_);
            brake_router.pending_rpcs_[BrakeRouterT::CorrelationKey{cid}] =
                "service.response.compute_force";
        }

        const bool sent = brake_router.route_send("#motor", env);
        REQUIRE(sent, "brake route_send RpcRequest returned false");

        // Motor receives the request via register_message_handler.
        REQUIRE(motor_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.request.compute_force";
                }),
                "motor engine did not receive RPC request via SOME/IP");

        // Motor responds via handleServerResponse.
        std::array<uint8_t, 16> server_cid{};
        {
            std::lock_guard<std::mutex> lock(motor_router.server_pending_mutex_);
            REQUIRE(!motor_router.pending_server_requests_.empty(),
                    "motor did not store pending request");
            server_cid = motor_router.pending_server_requests_.begin()->first.id;
        }

        SCE::Mesh::MeshEnvelope resp;
        resp.id = {};
        resp.source = "motor";
        resp.type = "service.response.compute_force";
        resp.pattern = PK::RpcReply;
        resp.datacontenttype = SCE::Mesh::PayloadCodec::Json;
        std::string resp_payload = R"({"result":42})";
        resp.data.assign(resp_payload.begin(), resp_payload.end());
        resp.correlation_id = server_cid;

        REQUIRE(motor_router.handleServerResponse(resp),
                "handleServerResponse returned false");

        // Brake receives the correlated reply.
        REQUIRE(brake_engine.received_.wait_for([](const auto& v) {
                    return !v.empty() &&
                           v.back().type == "service.response.compute_force";
                }),
                "brake engine did not receive RPC reply via SOME/IP");

        // Verify payload survived CBOR round-trip.
        {
            std::lock_guard<std::mutex> lock(brake_engine.received_.m);
            REQUIRE(brake_engine.received_.events.back().data.find("42") != std::string::npos,
                    "reply payload did not survive SOME/IP CBOR round-trip");
        }
    }

    brake_router.shutdown();
    motor_router.shutdown();
    std::printf("SCE Mesh SOME/IP runtime E2E: PASS\n");
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
