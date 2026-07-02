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

    void push(const SCE::Mesh::MeshEnvelope &env) {
        {
            std::lock_guard<std::mutex> lock(m);
            envelopes.push_back(env);
        }
        cv.notify_all();
    }

    template <typename Pred> bool wait_for(Pred &&predicate, std::chrono::seconds timeout = kDefaultTimeout) {
        std::unique_lock<std::mutex> lock(m);
        return cv.wait_for(lock, timeout, [&] { return predicate(envelopes); });
    }
};

// ── Zenoh peer-mode session factory ──────────────────────────────────

inline zenoh::Session open_peer(const std::string &connect, const std::string &listen) {
    auto config = zenoh::Config::create_default();
    config.insert_json5("mode", "\"peer\"");
    if (!connect.empty()) {
        config.insert_json5("connect/endpoints", std::string("[\"") + connect + "\"]");
    }
    if (!listen.empty()) {
        config.insert_json5("listen/endpoints", std::string("[\"") + listen + "\"]");
    }
    config.insert_json5("scouting/multicast/enabled", "false");
    return zenoh::Session::open(std::move(config));
}

// Tighter than `MeshTestUtils.h::kDefaultTimeout` (5 s) because a wedged
// zenoh listen endpoint should fail fast rather than extend every
// migrated handshake site by the full default. Observed convergence on
// this system is under 100 ms; 2 s gives a ~20× safety margin while
// still surfacing a broken endpoint in well under one ctest job slot.
constexpr auto kHandshakeTimeout = std::chrono::seconds(2);

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
//
// Scope: pub/sub tests only. Routers whose generated init() declares a
// queryable as a first-class entity (any `<invoke type="sce:mesh-rpc">`
// server, any `field.get`/`field.set` server, any pool target) should
// prefer `wait_for_queryable` below — matching is a direct signal on
// the route state rather than an indirect liveliness proxy, and
// typically converges in under 10 ms instead of ~50 ms. The liveliness
// helper remains correct but is retained only for pub/sub routes whose
// subscriber is declared on-demand inside `send_zenoh` (no init-time
// matchable resource exists to probe).
inline void wait_for_peer_ready(zenoh::Session &local_session, zenoh::Session &peer_session,
                                std::chrono::milliseconds timeout = kHandshakeTimeout) {
    const std::string key = "sce/test/handshake/" + SCE::uuid::to_string(SCE::uuid::v7());

    std::mutex m;
    std::condition_variable cv;
    bool seen = false;

    zenoh::Session::LivelinessSubscriberOptions sub_opts;
    sub_opts.history = true;

    auto sub = local_session.liveliness_declare_subscriber(
        zenoh::KeyExpr(key),
        [&](const zenoh::Sample &sample) {
            if (sample.get_kind() == ::Z_SAMPLE_KIND_PUT) {
                std::lock_guard<std::mutex> lk(m);
                seen = true;
                cv.notify_all();
            }
        },
        []() noexcept {}, std::move(sub_opts));

    auto token = peer_session.liveliness_declare_token(zenoh::KeyExpr(key));

    std::unique_lock<std::mutex> lk(m);
    if (!cv.wait_for(lk, timeout, [&] { return seen; })) {
        throw std::runtime_error("wait_for_peer_ready: zenoh liveliness token not observed "
                                 "within timeout — peer-mesh routing did not converge");
    }
}

// ── Queryable-matching convergence barrier ───────────────────────────
//
// Blocks until a queryable matching `key` has propagated into
// `session`'s routing state. Uses zenoh's native matching signal
// (`Session::declare_querier` + `Querier::declare_matching_listener`)
// so the convergence condition is exactly the invariant each caller
// actually needs — "can a `session.get(key, …)` from this side reach
// a queryable?" — rather than the indirect liveliness proxy
// `wait_for_peer_ready` relies on.
//
// Ordering inside the helper matters: the listener is installed before
// the synchronous `get_matching_status()` read so that a matching
// transition occurring between "install" and "check" cannot slip past
// the check without the callback recording it. The reverse order would
// produce a deadlock if the queryable was already declared before
// `wait_for_queryable` was called — the matching-status change event
// fires only on transitions, so an already-matching state never fires.
//
// The throwaway querier is scoped to this function: it is undeclared
// on return, leaving no residue in `session`. The matching listener is
// bound to the querier and dies with it.
//
// For generated routers that do not expose a raw zenoh session, pass a
// raw probe peer that shares the same listen endpoint — zenoh peer
// mesh routing makes one peer's matching status representative of any
// co-located peer's.
inline void wait_for_queryable(zenoh::Session &session, std::string_view key,
                               std::chrono::milliseconds timeout = kHandshakeTimeout) {
    auto querier = session.declare_querier(zenoh::KeyExpr(std::string(key)));

    std::mutex m;
    std::condition_variable cv;
    bool seen = false;

    auto listener = querier.declare_matching_listener(
        [&](const zenoh::MatchingStatus &status) {
            if (!status.matching) {
                return;
            }
            std::lock_guard<std::mutex> lk(m);
            seen = true;
            cv.notify_all();
        },
        []() noexcept {});

    // Fast path: matching may already be live. The listener above only
    // fires on subsequent transitions, so read status directly to cover
    // queryables that were declared before this helper was called.
    if (querier.get_matching_status().matching) {
        return;
    }

    std::unique_lock<std::mutex> lk(m);
    if (!cv.wait_for(lk, timeout, [&] { return seen; })) {
        throw std::runtime_error(std::string("wait_for_queryable: no matching queryable for '") + std::string(key) +
                                 "' within timeout");
    }
}

}  // namespace SCE::Test::Mesh
