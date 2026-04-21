// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh PatternKind — wire-stable communication pattern discriminator.
//
// Values are IMMUTABLE once shipped. Range 1-9 is in use; 10-13 is reserved
// for future Stream patterns — wire-layer optimizations on EventSubscribe /
// EventNotification that pair an initial state snapshot with delta-encoded
// change events (SCE_MESH.md §8.1). 14-20 cover the full remote invoke
// lifecycle (§9.6.2, Session F); wire 14 (`InvokeStart`) and wire 20
// (`InvokeError`) are active and carry the first Session F round's
// `SESSION_F_NOT_IMPLEMENTED` round-trip (§9.6 line 1396), while 15-19
// stay deferred so their envelopes parse as "unknown pattern" and drop
// until each wire's consumer lands. 21 is `ParallelRegionDone` — the
// distributed parallel-final barrier envelope (§16.5) whose consumer is
// the `ParallelCompletionTracker` on the root partition. Adding a variant
// requires a new wire value — never reuse. Serialized into MeshEnvelope
// key 3 as CBOR uint16.

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
    // SCE_MESH.md §9.6.2 — full remote invoke lifecycle (Session F). Wire 14
    // (InvokeStart, Parent→Child) and wire 20 (InvokeError, bidirectional)
    // are the first Session F pair; 15-19 stay deferred (envelopes parse as
    // "unknown pattern" and drop) until each intermediate wire's consumer
    // lands in a subsequent Session F round.
    InvokeStart        = 14,
    InvokeError        = 20,
    ParallelRegionDone = 21,  // SCE_MESH.md §16.5 — distributed parallel-final barrier envelope.
};

/// Wire-value guards. Any reorder or renumber breaks cross-machine
/// envelopes and must fail to compile. Update ONLY when intentionally
/// introducing a new variant at an unused value.
static_assert(static_cast<uint16_t>(PatternKind::FireForget)         == 1,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::RpcRequest)         == 2,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::RpcReply)           == 3,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventSubscribe)     == 4,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventUnsubscribe)   == 5,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventNotify)        == 6,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldRead)          == 7,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldWrite)         == 8,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldNotify)        == 9,  "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::InvokeStart)        == 14, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::InvokeError)        == 20, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::ParallelRegionDone) == 21, "PatternKind wire value changed");

}  // namespace SCE::Mesh
