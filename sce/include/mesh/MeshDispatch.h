// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh MeshDispatch — shared envelope-to-engine dispatch helper.
//
// Single source of truth for pattern-based envelope delivery. Used by:
//   - ShmChannel::drain()                     (receiver side)
//   - TransportRouter::onIncoming()           (receiver side)
//   - TransportRouter::route_send() local     (same-process shortcut)
//
// Adding a new pattern: add a case to the switch in dispatchEnvelope().
// The function returns false for unhandled patterns so callers can log
// or drop as appropriate.

#pragma once

#include "mesh/MeshEnvelope.h"

#include <string>

namespace SCE::Mesh {

/// Dispatch a MeshEnvelope to a state machine engine based on pattern kind.
///
/// FireForget: resolve event name via Policy::getEventFromName and enqueue
/// via engine.raiseExternal(). Other patterns: return false (Session C/D).
///
/// @tparam Policy  Receiver's generated StatePolicy (provides getEventFromName)
/// @tparam Engine  StaticExecutionEngine<Policy>
/// @return true if the event was dispatched to the engine
template <typename Policy, typename Engine>
bool dispatchEnvelope(const MeshEnvelope& env, Engine& engine) {
    switch (env.pattern) {
    case PatternKind::FireForget: {
        auto ev = Policy::getEventFromName(env.type.c_str());
        if (!ev) return false;
        if (env.data.empty()) {
            engine.raiseExternal(*ev);
        } else {
            engine.raiseExternal(*ev, std::string(env.data.begin(), env.data.end()));
        }
        return true;
    }
    default:
        // Stub: non-FireForget patterns not yet realized.
        // Session C/D will add RpcRequest, PubSub, etc.
        return false;
    }
}

}  // namespace SCE::Mesh
