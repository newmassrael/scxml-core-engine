// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Zenoh-specific test utilities layered on top of MeshTestUtils.h.
// Adds: peer-mode session factory, raw envelope capture, deterministic
// peer-mesh convergence barrier.

#pragma once

#include "MeshTestUtils.h"
#include "common/Uuid.h"

#include <condition_variable>
#include <mutex>
#include <stdexcept>
#include <string>
#include <zenoh.hxx>

namespace SCE::Test::Mesh {

// ── Raw envelope capture (transport-level, pre-MeshDispatch) ─────────

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

// ── Zenoh peer-mode session factory ──────────────────────────────────

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

// ── Deterministic peer-mesh convergence barrier ──────────────────────
//
// Blocks until a liveliness token declared on `peer_session` is
// observed by a subscriber on `local_session`. Once the PUT sample
// arrives, zenoh's peer-mesh routing state is known to have propagated
// between the two sessions — the same invariant every sleep_for-based
// "peer discovery stabilization" was imprecisely waiting for.
//
// Uses `history = true` on the subscriber so the token declaration can
// interleave with the subscriber declaration in either order without
// racing: zenoh replays tokens that existed before the subscriber came
// up, and forwards future PUT samples live.
//
// The RAII-scoped subscriber and token are undeclared on return;
// ephemeral handshake state leaves no residue on either session.
//
// For tests that use a generated TransportRouter on one endpoint,
// both session arguments must be raw test-side sessions. In zenoh
// peer mode the router participates as a relay peer: once a liveliness
// token declared on one of the raw sessions is observed on the other
// raw session, every pairwise edge sharing the listen endpoint has
// converged — including the router's link to each raw session.
inline void wait_for_peer_ready(
        zenoh::Session& local_session,
        zenoh::Session& peer_session,
        std::chrono::milliseconds timeout = std::chrono::seconds(2)) {
    const std::string key = "sce/test/handshake/" +
        SCE::uuid::to_string(SCE::uuid::v7());

    std::mutex m;
    std::condition_variable cv;
    bool seen = false;

    zenoh::Session::LivelinessSubscriberOptions sub_opts;
    sub_opts.history = true;

    auto sub = local_session.liveliness_declare_subscriber(
        zenoh::KeyExpr(key),
        [&](const zenoh::Sample& sample) {
            if (sample.get_kind() == ::Z_SAMPLE_KIND_PUT) {
                std::lock_guard<std::mutex> lk(m);
                seen = true;
                cv.notify_all();
            }
        },
        []() noexcept {},
        std::move(sub_opts));

    auto token = peer_session.liveliness_declare_token(zenoh::KeyExpr(key));

    std::unique_lock<std::mutex> lk(m);
    if (!cv.wait_for(lk, timeout, [&] { return seen; })) {
        throw std::runtime_error(
            "wait_for_peer_ready: zenoh liveliness token not observed "
            "within timeout — peer-mesh routing did not converge");
    }
}

}  // namespace SCE::Test::Mesh
