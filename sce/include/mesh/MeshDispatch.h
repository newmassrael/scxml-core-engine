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
// Pattern dispatch:
//   Inbound (enqueue to engine): FireForget, RpcReply, EventNotify, FieldNotify
//   Outbound-only (reject):      RpcRequest, EventSubscribe/Unsubscribe, FieldRead/Write
//
// Returns false for outbound-only patterns (misconfigured echo-back) and
// unknown pattern kinds (forward compatibility).

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
    case PatternKind::FireForget:
    // Inbound notifications are unidirectional — same delivery as FireForget.
    // The pattern discriminator enables transport-level routing (e.g. SOME/IP
    // event group subscription) but the engine dispatch is identical.
    case PatternKind::EventNotify:
    case PatternKind::FieldNotify:
    // RPC reply: correlation matching is done by TransportRouter before calling
    // dispatchEnvelope. By the time we get here, env.type is already set to the
    // reply-event name from the correlation table. Delivery is same as FireForget.
    case PatternKind::RpcReply: {
        auto ev = Policy::getEventFromName(env.type.c_str());
        if (!ev) return false;
        if (env.data.empty()) {
            engine.raiseExternal(*ev);
        } else {
            engine.raiseExternal(*ev, std::string(env.data.begin(), env.data.end()));
        }
        return true;
    }
    // Outbound-only patterns: these are sent by the engine, not received.
    // If they arrive here it means a misconfigured transport is echoing
    // back our own sends — return false so caller can log or drop.
    case PatternKind::RpcRequest:
    case PatternKind::EventSubscribe:
    case PatternKind::EventUnsubscribe:
    case PatternKind::FieldRead:
    case PatternKind::FieldWrite:
        return false;
    }
    return false;  // unknown pattern kind — forward compatibility
}

}  // namespace SCE::Mesh
