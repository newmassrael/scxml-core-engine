// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Zenoh-specific test utilities layered on top of MeshTestUtils.h.
// Adds: peer-mode session factory, raw envelope capture.

#pragma once

#include "MeshTestUtils.h"

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

}  // namespace SCE::Test::Mesh
