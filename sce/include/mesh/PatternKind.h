// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh PatternKind — wire-stable communication pattern discriminator.
//
// Values are IMMUTABLE once shipped. Range 1-9 is in use; 10-13 is reserved
// for future Stream patterns — wire-layer optimizations on EventSubscribe /
// EventNotification that pair an initial state snapshot with delta-encoded
// change events (SCE_MESH.md §mesh-8.1). 14-20 cover the full remote invoke
// lifecycle (§mesh-9.6.2, Session F) — all seven wires are active and carry
// the parent/child session lifecycle: wire 14 `InvokeStart` (P→C) starts
// a child session, wire 15 `InvokeStarted` (C→P) stashes the child's
// session endpoint, wire 16 `ChildEvent` (C→P) carries child→parent
// events, wire 17 `ParentEvent` (P→C) carries parent→child autoforwarded
// events, wire 18 `InvokeDone` (C→P) signals child final-state completion
// with donedata, wire 19 `InvokeCancel` (P→C) terminates the child
// session, and wire 20 `InvokeError` (bidirectional) reports instantiation
// or transport-unavailable failures (SCE_MESH.md §mesh-9.6). 21 is
// `ParallelRegionDone` — the distributed parallel-final barrier envelope
// (§mesh-16.5) whose consumer is the `ParallelCompletionTracker` on the root
// partition. Adding a variant requires a new wire value — never reuse.
// Serialized into MeshEnvelope key 3 as CBOR uint16.

#pragma once

#include <cstdint>

namespace SCE::Mesh {

enum class PatternKind : uint16_t {
    FireForget = 1,
    RpcRequest = 2,
    RpcReply = 3,  // success or error — see envelope.rpc_status
    EventSubscribe = 4,
    EventUnsubscribe = 5,
    EventNotify = 6,
    FieldRead = 7,
    FieldWrite = 8,
    FieldNotify = 9,
    // 10-13 RESERVED for Stream* variants (snapshot + delta wire-layer optimization, SCE_MESH.md §mesh-8.1). Do not
    // assign. SCE_MESH.md §mesh-9.6.2 — full remote invoke lifecycle (Session F). All seven wires (14-20) are active;
    // each carries one edge of the §scxml-6.4 parent/child session lifecycle over same-device shm.
    InvokeStart = 14,
    InvokeStarted = 15,
    ChildEvent = 16,
    ParentEvent = 17,
    InvokeDone = 18,
    InvokeCancel = 19,
    InvokeError = 20,
    ParallelRegionDone = 21,  // SCE_MESH.md §mesh-16.5 — distributed parallel-final barrier envelope.
};

/// Wire-value guards. Any reorder or renumber breaks cross-machine
/// envelopes and must fail to compile. Update ONLY when intentionally
/// introducing a new variant at an unused value.
static_assert(static_cast<uint16_t>(PatternKind::FireForget) == 1, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::RpcRequest) == 2, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::RpcReply) == 3, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventSubscribe) == 4, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventUnsubscribe) == 5, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::EventNotify) == 6, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldRead) == 7, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldWrite) == 8, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::FieldNotify) == 9, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::InvokeStart) == 14, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::InvokeStarted) == 15, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::ChildEvent) == 16, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::ParentEvent) == 17, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::InvokeDone) == 18, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::InvokeCancel) == 19, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::InvokeError) == 20, "PatternKind wire value changed");
static_assert(static_cast<uint16_t>(PatternKind::ParallelRegionDone) == 21, "PatternKind wire value changed");

}  // namespace SCE::Mesh
