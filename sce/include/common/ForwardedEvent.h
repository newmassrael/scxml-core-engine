// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// This file is part of SCE (SCXML Core Engine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact newmassrael@gmail.com)
//
// Commercial License:
//   Individual: $5000 cumulative
//   Enterprise: Contact for pricing
//   Contact: https://github.com/newmassrael
//
// Full terms: https://github.com/newmassrael/scxml-core-engine/blob/main/LICENSE

#pragma once

#include <string>

namespace SCE::Common {

/**
 * @brief One external event addressed by NAME rather than by a machine-local
 *        `Event` enum (§scxml-6.4 autoforward carrier).
 *
 * A data carrier, not an implementation of any clause: the §-form citations
 * that belong to the autoforward contract live on the functions that satisfy
 * it (`StaticExecutionEngine::raiseExternal`), not on these fields.
 *
 * Autoforward is the one path where an event must leave the machine that
 * owns its enum: the parent hands a copy to an invoked child whose `Event`
 * enum is an unrelated type (local invoke), or to a child hosted on another
 * device (SCE_MESH.md §9.6.5 wire-17 `ParentEvent`). The event name is the
 * only identity both ends share, so it travels as a string while the
 * `_event` fields ride alongside it.
 *
 * Both boundaries carry the same struct so the local and remote autoforward
 * paths cannot drift in what they preserve. `target` is deliberately absent:
 * it is a routing decision belonging to the `<send>` that produced the
 * original event, and copying it would re-route the forwarded copy back onto
 * the mesh or HTTP path instead of delivering it to the child.
 *
 * Typed payloads (`EventWithMetadata::typedPayload`) are likewise absent —
 * they are generated per owning `Event` enum and cannot cross to a machine
 * with a different one. The receiver re-hydrates from `data`, the same way
 * the mesh wire-16 inbound path does.
 */
struct ForwardedEvent {
    // §scxml-5.10.1 `_event` fields, in the spec's own order.
    std::string name;
    std::string data;  // serialized payload
    std::string origin;
    std::string sendId;
    std::string type;
    std::string originType;
    std::string invokeId;
};

}  // namespace SCE::Common
