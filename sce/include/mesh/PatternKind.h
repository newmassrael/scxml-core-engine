// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh PatternKind — wire-stable communication pattern discriminator.
//
// Values are IMMUTABLE once shipped. Range 1-9 is in use; 10-13 is reserved
// for future Stream patterns — wire-layer optimizations on EventSubscribe /
// EventNotification that pair an initial state snapshot with delta-encoded
// change events (SCE_MESH.md §8.1). Adding a variant requires a new wire
// value — never reuse. Serialized into MeshEnvelope key 3 as CBOR uint16.

#pragma once

#include <cstdint>

namespace SCE::Mesh {

enum class PatternKind : uint16_t {
    FireForget         = 1,
    RpcRequest         = 2,
    RpcReply           = 3,  // success or error — see envelope.rpc_status
    EventSubscribe     = 4,
    EventUnsubscribe   = 5,
    EventNotify        = 6,
    FieldRead          = 7,
    FieldWrite         = 8,
    FieldNotify        = 9,
    // 10-13 RESERVED for Stream* variants (snapshot + delta wire-layer optimization, SCE_MESH.md §8.1). Do not assign.
};

/// Wire-value guards. Any reorder or renumber breaks cross-machine
/// envelopes and must fail to compile. Update ONLY when intentionally
/// introducing a new variant at an unused value.
static_assert(static_cast<uint16_t>(PatternKind::FireForget)       == 1, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::RpcRequest)       == 2, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::RpcReply)         == 3, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventSubscribe)   == 4, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventUnsubscribe) == 5, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventNotify)      == 6, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldRead)        == 7, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldWrite)       == 8, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldNotify)      == 9, "PatternKind wire value changed");

}  // namespace SCE::Mesh
