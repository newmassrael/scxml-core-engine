// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-14.6
//
// SCE Mesh §14.6 responder-set enforcement over live SOME/IP.
//
// §14.6 scopes which peers may answer a request: the `reply_from:`
// responder set, defaulting to the binding's own target. The spec once
// justified the same-target rule by saying cross-target pairing "would
// require an RPC routing table the engine does not maintain" — but
// `pending_rpcs_` IS that table, and it keyed on `correlation_id`
// alone, so the rule was stated and not enforced.
//
// The client router here has two SOME/IP RPC targets (`#alpha`,
// `#beta`), so codegen installs a response message handler on each
// service. Both handlers consult the same router-scoped `pending_rpcs_`
// table. The adjudication drives the beta handler with a reply carrying
// ALPHA's correlation_id — the literal "request to #A, reply from #B"
// shape the clause names.
//
// Both servers are raw vsomeip applications owned by this test, not
// generated SCE machines: the adjudication needs a server that replies
// with a correlation_id it did not receive, which a generated server
// never does (it echoes the one it was sent).
//
// Three stages, in one process, in this order:
//
//   Gate — beta replies with alpha's correlation_id. Must NOT
//     correlate: beta is not in #alpha's responder set.
//
//   Control — beta replies with its own correlation_id.
//     Establishes that the beta reply path is live and that the gate stage's
//     outcome is attributable to the responder set, not to a dead
//     transport leg.
//
//   Survival (why the check precedes the erase) — #alpha answers
//     late, under a raw event name. It must still be rewritten to the
//     registered reply-event name, proving the rejected reply did not
//     retire the entry.

#include "cross_target_client_sm.h"
#include "cross_target_client_transport.h"

#include "MeshTestUtils.h"
#include "SomeipTestUtils.h"
#include "mesh/MeshEnvelope.h"
#include "mesh/MeshEnvelopeCodec.h"

#include <vsomeip/vsomeip.hpp>

#include <atomic>
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

/// Captured correlation_id from the request the client issued to #alpha.
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

/// Which correlation_id the beta server stamps on its response.
enum class BetaReplyMode {
    ForeignCid,  // alpha's — the adjudication
    OwnCid,      // the one beta received — the control
};

int run_test() {
    wipe_stale_vsomeip_sockets();

    auto rt = vsomeip::runtime::get();

    // ── Client router (system under test) ────────────────────────────
    TestSenderEngine client_engine;
    SCE::Generated::cross_target_client::TransportRouter<TestSenderEngine> client_router({&client_engine});
    MESH_TEST_REQUIRE(client_router.init(), "client router init failed");

    // ── Raw alpha server: capture the correlation_id, defer the reply ─
    // The request message is stashed so the survival stage can answer it LATE, after
    // beta's forged reply has already consumed the correlation entry.
    CapturedCid alpha_cid;
    std::mutex alpha_msg_m;
    std::shared_ptr<vsomeip::message> alpha_pending_msg;
    auto alpha_app = rt->create_application("xt_alpha_app");
    MESH_TEST_REQUIRE(alpha_app->init(), "alpha app init failed");
    alpha_app->register_message_handler(
        kAlphaService, kAlphaInstance, kAlphaMethod,
        [&alpha_cid, &alpha_msg_m, &alpha_pending_msg](const std::shared_ptr<vsomeip::message> &msg) {
            auto pl = msg->get_payload();
            if (!pl) {
                return;
            }
            SCE::Mesh::MeshEnvelope env;
            if (!SCE::Mesh::decodeEnvelope(pl->get_data(), pl->get_length(), env)) {
                return;
            }
            {
                std::lock_guard<std::mutex> lk(alpha_msg_m);
                alpha_pending_msg = msg;
            }
            if (env.correlation_id) {
                alpha_cid.set(*env.correlation_id);
            }
            // Deliberately no response here: the first reply in flight
            // must be beta's forged one.
        });
    alpha_app->offer_service(kAlphaService, kAlphaInstance);
    std::thread alpha_thread([&alpha_app] { alpha_app->start(); });

    // ── Raw beta server: reply with whichever cid the stage selects ──
    std::atomic<BetaReplyMode> beta_mode{BetaReplyMode::ForeignCid};
    auto beta_app = rt->create_application("xt_beta_app");
    MESH_TEST_REQUIRE(beta_app->init(), "beta app init failed");
    beta_app->register_message_handler(
        kBetaService, kBetaInstance, kBetaMethod,
        [&beta_app, rt, &alpha_cid, &beta_mode](const std::shared_ptr<vsomeip::message> &msg) {
            auto pl = msg->get_payload();
            if (!pl) {
                return;
            }
            SCE::Mesh::MeshEnvelope in;
            if (!SCE::Mesh::decodeEnvelope(pl->get_data(), pl->get_length(), in)) {
                return;
            }

            SCE::Mesh::MeshEnvelope reply;
            reply.id = SCE::uuid::v7();
            reply.source = "beta";
            reply.type = "service.response.other_call";
            reply.pattern = SCE::Mesh::PatternKind::RpcReply;

            if (beta_mode.load() == BetaReplyMode::ForeignCid) {
                std::lock_guard<std::mutex> lk(alpha_cid.m);
                if (!alpha_cid.value) {
                    return;
                }
                reply.correlation_id = *alpha_cid.value;
            } else {
                reply.correlation_id = in.correlation_id;
            }

            const auto bytes = SCE::Mesh::encodeEnvelope(reply);
            auto response = rt->create_response(msg);
            auto payload = rt->create_payload();
            payload->set_data(bytes.data(), static_cast<vsomeip::length_t>(bytes.size()));
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

    // Let both offers propagate through the routing manager before the
    // client's blind sends go out (this deployment declares no
    // `outbound_buffer:`, so an unavailable target drops silently).
    std::this_thread::sleep_for(std::chrono::milliseconds(800));

    // ── Gate: a reply from outside the responder set ─────────────────
    // Request to #alpha registers cid_alpha in pending_rpcs_.
    MESH_TEST_REQUIRE(client_engine.mesh_send_cb_ != nullptr, "router did not install the mesh send callback");
    client_engine.mesh_send_cb_("#alpha", "service.request.compute_force", "", "", "");

    MESH_TEST_REQUIRE(alpha_cid.wait(std::chrono::seconds(5)),
                      "alpha server never observed the request — the SOME/IP leg to #alpha is dead, "
                      "so nothing downstream can be adjudicated");

    // Request to #beta gives beta a live request to respond to. Its
    // response will carry alpha's correlation_id.
    client_engine.mesh_send_cb_("#beta", "service.request.other_call", "", "", "");

    // Wait for EITHER the (forbidden) correlation or the §16.7 row 14
    // raise, so the stage does not burn its full timeout on the answer
    // it expects.
    (void)client_engine.received_.wait_for(
        [](const std::vector<ReceivedEvent> &evs) {
            for (const auto &e : evs) {
                if (e.type == "service.response.compute_force" || e.type == "error.communication") {
                    return true;
                }
            }
            return false;
        },
        std::chrono::seconds(5));

    bool foreign_correlated = false;
    bool undeclared_raised = false;
    {
        std::lock_guard<std::mutex> lk(client_engine.received_.m);
        for (const auto &e : client_engine.received_.events) {
            if (e.type == "service.response.compute_force") {
                foreign_correlated = true;
            }
            // The rejection must be loud: §16.7 row 14 rather than a
            // silent drop. Checking the reason string (not just the
            // event name) keeps an unrelated error.communication —
            // TRANSPORT_UNAVAILABLE from a flaky leg, say — from
            // satisfying this.
            if (e.type == "error.communication" && e.data.find("RPC_REPLY_FROM_UNDECLARED_PEER") != std::string::npos) {
                undeclared_raised = true;
            }
        }
    }

    // ── Control: the beta reply leg is live ─────────────────────────
    // Same beta leg, beta's own correlation_id. Proves the reply path is
    // live independent of the gate stage's outcome.
    client_engine.received_.clear();
    beta_mode.store(BetaReplyMode::OwnCid);
    client_engine.mesh_send_cb_("#beta", "service.request.other_call", "", "", "");

    const bool control_correlated = client_engine.received_.wait_for(
        [](const std::vector<ReceivedEvent> &evs) {
            for (const auto &e : evs) {
                if (e.type == "service.response.other_call") {
                    return true;
                }
            }
            return false;
        },
        std::chrono::seconds(5));

    if (!control_correlated) {
        std::fprintf(stderr, "FAIL: control stage — beta's reply carrying its OWN correlation_id "
                             "did not reach the client engine. The beta reply leg is dead, so "
                             "the gate stage carries no information either way.\n");
        return 1;
    }

    // ── Survival: does the genuine reply still correlate? ───────────
    // The correlation lookup ERASES the entry on a match. If beta's
    // forged reply consumed cid_alpha, alpha's own late reply — the
    // legitimate one, from the target the request was actually sent to —
    // has no entry left to match.
    //
    // The probe is the event NAME, not mere arrival: a matched entry
    // REWRITES env.type to the registered reply-event name. So alpha's
    // genuine reply goes out under a name the client never registered.
    // Arriving as `service.response.compute_force` proves the rewrite
    // happened; arriving under the raw name proves the entry was gone.
    constexpr const char *kRawReplyName = "alpha.raw.reply";
    client_engine.received_.clear();
    bool genuine_reply_sent = false;
    {
        std::lock_guard<std::mutex> lk(alpha_msg_m);
        if (alpha_pending_msg) {
            SCE::Mesh::MeshEnvelope genuine;
            genuine.id = SCE::uuid::v7();
            genuine.source = "alpha";
            genuine.type = kRawReplyName;
            genuine.pattern = SCE::Mesh::PatternKind::RpcReply;
            {
                std::lock_guard<std::mutex> ck(alpha_cid.m);
                genuine.correlation_id = alpha_cid.value;
            }
            const auto bytes = SCE::Mesh::encodeEnvelope(genuine);
            auto response = rt->create_response(alpha_pending_msg);
            auto payload = rt->create_payload();
            payload->set_data(bytes.data(), static_cast<vsomeip::length_t>(bytes.size()));
            response->set_payload(payload);
            alpha_app->send(response);
            genuine_reply_sent = true;
        }
    }
    MESH_TEST_REQUIRE(genuine_reply_sent, "alpha never stashed the request message, so the "
                                          "late-genuine-reply stage could not run");

    // Wait for EITHER outcome so the stage does not burn its full timeout
    // on the answer it is trying to distinguish.
    (void)client_engine.received_.wait_for(
        [kRawReplyName](const std::vector<ReceivedEvent> &evs) {
            for (const auto &e : evs) {
                if (e.type == "service.response.compute_force" || e.type == kRawReplyName) {
                    return true;
                }
            }
            return false;
        },
        std::chrono::seconds(3));

    bool genuine_correlated = false;
    bool genuine_arrived_raw = false;
    {
        std::lock_guard<std::mutex> lk(client_engine.received_.m);
        for (const auto &e : client_engine.received_.events) {
            if (e.type == "service.response.compute_force") {
                genuine_correlated = true;
            }
            if (e.type == kRawReplyName) {
                genuine_arrived_raw = true;
            }
        }
    }

    if (!undeclared_raised) {
        std::fprintf(stderr, "FAIL: the cross-target reply was rejected but no error.communication\n"
                             "  carrying RPC_REPLY_FROM_UNDECLARED_PEER reached the engine. A silent\n"
                             "  rejection is indistinguishable from a dropped packet — §16.7 row 14\n"
                             "  exists precisely so the author can observe it.\n");
        return 4;
    }

    if (foreign_correlated) {
        std::fprintf(stderr, "FAIL: a reply received on target #beta was correlated against the request\n"
                             "  issued to #alpha and dispatched under #alpha's reply-event name. The\n"
                             "  §14.6 responder-set gate is not enforced on the SOME/IP `<send>` RPC\n"
                             "  path — any peer holding a correlation_id can retire another peer's\n"
                             "  pending request.\n");
        return 2;
    }

    // The property the gate exists for: rejecting the forged reply must
    // LEAVE THE ENTRY LIVE. If the responder check ran after the erase,
    // #alpha's genuine reply would arrive under its raw name because the
    // correlation entry that rewrites it was already spent.
    if (!genuine_correlated || genuine_arrived_raw) {
        std::fprintf(stderr,
                     "FAIL: after the forged reply was rejected, #alpha's genuine reply was%s\n"
                     "  rewritten to the registered event name (raw-name observed: %s). The\n"
                     "  rejected reply retired the correlation entry, so the request can never\n"
                     "  be answered by the target it was actually sent to.\n",
                     genuine_correlated ? "" : " NOT", genuine_arrived_raw ? "yes" : "no");
        return 3;
    }

    std::printf("SCE Mesh §14.6 SOME/IP responder set: cross-target reply rejected, entry "
                "survived, genuine reply still correlated: PASS\n");
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
