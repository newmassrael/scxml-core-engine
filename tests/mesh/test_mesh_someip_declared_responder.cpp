// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-14.6
//
// SCE Mesh §14.6 declared responder set — the positive half.
//
// `test_mesh_someip_cross_target_reply.cpp` pins the default: with no
// `reply_from:`, a reply arriving on #beta for a request sent to #alpha
// is rejected. This file pins the other direction — once the deployment
// DECLARES `reply_from: ["#alpha", "#beta"]` on the #alpha binding, that
// exact reply must be accepted and dispatched under #alpha's
// reply-event name.
//
// The two files share `cross_target_client.scxml` byte-for-byte and
// differ only in deploy.yaml, which is the point: whether a broker may
// answer is a platform-engineering decision, not an SCXML one (§1). If
// the gate were keyed on anything other than the declared set, one of
// the two tests would fail.
//
// The beta server is a raw vsomeip application because the test needs a
// server that replies with a correlation_id it did not receive — a
// generated server always echoes the one it was sent.

#include "cross_target_client_sm.h"
#include "cross_target_client_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <vsomeip/vsomeip.hpp>

#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <mutex>
#include <optional>
#include <string>
#include <thread>

namespace {

using namespace SCE::Test::Mesh;

constexpr vsomeip::service_t kAlphaService = 0x3000;
constexpr vsomeip::instance_t kAlphaInstance = 0x0001;
constexpr vsomeip::method_t kAlphaMethod = 0x0101;

constexpr vsomeip::service_t kBetaService = 0x3001;
constexpr vsomeip::instance_t kBetaInstance = 0x0001;
constexpr vsomeip::method_t kBetaMethod = 0x0102;

/// Correlation id the client stamped on its request to #alpha.
struct CapturedCid {
    std::mutex m;
    std::condition_variable cv;
    std::optional<SCE::uuid::Bytes> value;

    void set(SCE::uuid::Bytes v) {
        {
            std::lock_guard<std::mutex> lk(m);
            value = v;
        }
        cv.notify_all();
    }

    bool wait(std::chrono::seconds timeout) {
        std::unique_lock<std::mutex> lk(m);
        return cv.wait_for(lk, timeout, [this] { return value.has_value(); });
    }
};

int run_test() {
    wipe_stale_vsomeip_sockets();

    auto rt = vsomeip::runtime::get();

    TestSenderEngine client_engine;
    SCE::Generated::cross_target_client::TransportRouter<TestSenderEngine> client_router({&client_engine});
    MESH_TEST_REQUIRE(client_router.init(), "client router init failed");

    // Alpha takes the request and stays silent — the only reply in
    // flight must be the one beta sends on alpha's behalf.
    CapturedCid alpha_cid;
    auto alpha_app = rt->create_application("xt_alpha_app");
    MESH_TEST_REQUIRE(alpha_app->init(), "alpha app init failed");
    alpha_app->register_message_handler(kAlphaService, kAlphaInstance, kAlphaMethod,
                                        [&alpha_cid](const std::shared_ptr<vsomeip::message> &msg) {
                                            auto pl = msg->get_payload();
                                            if (!pl) {
                                                return;
                                            }
                                            SCE::Mesh::MeshEnvelope env;
                                            if (!SCE::Mesh::decodeEnvelope(pl->get_data(), pl->get_length(), env)) {
                                                return;
                                            }
                                            if (env.correlation_id) {
                                                alpha_cid.set(*env.correlation_id);
                                            }
                                        });
    alpha_app->offer_service(kAlphaService, kAlphaInstance);
    std::thread alpha_thread([&alpha_app] { alpha_app->start(); });

    // Beta answers alpha's correlation id — the broker shape the
    // deployment declared.
    auto beta_app = rt->create_application("xt_beta_app");
    MESH_TEST_REQUIRE(beta_app->init(), "beta app init failed");
    beta_app->register_message_handler(kBetaService, kBetaInstance, kBetaMethod,
                                       [&beta_app, rt, &alpha_cid](const std::shared_ptr<vsomeip::message> &msg) {
                                           SCE::Mesh::MeshEnvelope reply;
                                           reply.id = SCE::uuid::v7();
                                           reply.source = "beta";
                                           reply.type = "service.response.other_call";
                                           reply.pattern = SCE::Mesh::PatternKind::RpcReply;
                                           {
                                               std::lock_guard<std::mutex> lk(alpha_cid.m);
                                               if (!alpha_cid.value) {
                                                   return;
                                               }
                                               reply.correlation_id = *alpha_cid.value;
                                           }
                                           const auto bytes = SCE::Mesh::encodeEnvelope(reply);
                                           auto response = rt->create_response(msg);
                                           auto payload = rt->create_payload();
                                           payload->set_data(bytes.data(),
                                                             static_cast<vsomeip::length_t>(bytes.size()));
                                           response->set_payload(payload);
                                           beta_app->send(response);
                                       });
    beta_app->offer_service(kBetaService, kBetaInstance);
    std::thread beta_thread([&beta_app] { beta_app->start(); });

    struct Teardown {
        std::shared_ptr<vsomeip::application> a, b;
        std::thread *at, *bt;

        ~Teardown() {
            a->stop_offer_service(kAlphaService, kAlphaInstance);
            b->stop_offer_service(kBetaService, kBetaInstance);
            a->stop();
            b->stop();
            if (at->joinable()) {
                at->join();
            }
            if (bt->joinable()) {
                bt->join();
            }
        }
    } teardown{alpha_app, beta_app, &alpha_thread, &beta_thread};

    // Let both offers propagate before the client's blind sends go out
    // (no `outbound_buffer:` in this deployment).
    std::this_thread::sleep_for(std::chrono::milliseconds(800));

    MESH_TEST_REQUIRE(client_engine.mesh_send_cb_ != nullptr, "router did not install the mesh send callback");
    client_engine.mesh_send_cb_("#alpha", "service.request.compute_force", "", "", "");
    MESH_TEST_REQUIRE(alpha_cid.wait(std::chrono::seconds(5)),
                      "alpha server never observed the request — the SOME/IP leg to #alpha is dead");

    // Beta's response to this request carries alpha's correlation id.
    client_engine.mesh_send_cb_("#beta", "service.request.other_call", "", "", "");

    const bool correlated = client_engine.received_.wait_for(
        [](const std::vector<ReceivedEvent> &evs) {
            for (const auto &e : evs) {
                if (e.type == "service.response.compute_force") {
                    return true;
                }
            }
            return false;
        },
        std::chrono::seconds(5));

    if (!correlated) {
        std::fprintf(stderr, "FAIL: #beta is a declared member of #alpha's `reply_from:` set, but its\n"
                             "  reply carrying #alpha's correlation_id was not correlated. Cross-target\n"
                             "  reply is declared-and-unrealised, which is the shape the transport\n"
                             "  capability gate exists to prevent.\n");
        return 1;
    }

    std::printf("SCE Mesh §14.6 declared responder set: cross-target reply from a declared "
                "peer correlated: PASS\n");
    return 0;
}

}  // namespace

int main() {
    try {
        return run_test();
    } catch (const std::exception &ex) {
        std::fprintf(stderr, "FAIL: unexpected exception: %s\n", ex.what());
        return 1;
    } catch (...) {
        std::fprintf(stderr, "FAIL: unknown exception\n");
        return 1;
    }
}
