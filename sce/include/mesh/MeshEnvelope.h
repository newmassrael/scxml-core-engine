// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Mesh MeshEnvelope — cross-transport message envelope.
//
// CloudEvents v1.0 field naming where applicable (id, source, type, subject,
// datacontenttype, data) with SCE-specific extensions (pattern, rpc_status,
// invoke_id, qos). Wire form is canonical CBOR (RFC 8949 §4.2.1) with
// integer keys; see MeshEnvelopeCodec for encode/decode.
//
// Authoritative schema and key map: SCE_MESH.md Section 13 Phase 3.5.

#pragma once

#include "mesh/PatternKind.h"
#include "mesh/PayloadCodec.h"
#include "mesh/QosHints.h"
#include "mesh/RpcStatus.h"

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Mesh {

struct MeshEnvelope {
    // ── Required (CloudEvents core + SCE pattern discriminator) ──
    std::array<uint8_t, 16> id{};       // CE 'id'              — UUID v7 (v4 fallback)
    std::string             source;     // CE 'source'          — sender machine name
    std::string             type;       // CE 'type'            — SCXML event name
    PatternKind             pattern     = PatternKind::FireForget;  // SCE extension — pattern discriminator
    PayloadCodec            datacontenttype = PayloadCodec::None;   // CE 'datacontenttype' — payload codec
    std::vector<uint8_t>    data;       // CE 'data'            — payload bytes (codec-encoded; may be empty)

    // ── Optional ──
    std::optional<std::string>             subject;             // CE 'subject' — interaction key (someip method_id, zenoh key)
    std::optional<std::array<uint8_t, 16>> correlation_id;      // RPC req↔resp matching (UUID v7)
    std::optional<std::string>             reply_to;            // CE-extension — response routing endpoint
    std::optional<std::array<uint8_t, 16>> invoke_id;           // RESERVED for <invoke type="sce:mesh-rpc"> lifecycle
    /// SCE-extension — per-session routing identifier (UUID v7, generated
    /// once per TransportRouter construction). `source` carries the stable
    /// document identity (machine_name); `routing_id` discriminates sessions
    /// of the same document at echo-filter and self-observation sites.
    /// Absent on inbound envelopes from peers that have not yet been
    /// migrated to stamp this field — self-filter callers MUST require both
    /// `has_value()` AND equality against the local `routing_id_`, so an
    /// unstamped envelope never collides with the local session. Document
    /// identity invariant lives in SCE_MESH.md §10.9.
    std::optional<std::array<uint8_t, 16>> routing_id;
    std::optional<RpcStatus>               rpc_status;          // RpcReply only (absent ⇒ Ok)
    std::optional<std::string>             rpc_error_message;   // RpcReply non-Ok detail
    std::optional<uint64_t>                deadline_unix_ms;    // RPC absolute deadline
    /// NOT YET SERIALIZED: present in-memory only. encode/decode silently
    /// drops this field. Do not rely on round-trip equality when qos is set.
    /// Wire serialization (CBOR key 13) is added in Session C/D together
    /// with the first transport that consumes QoS hints.
    std::optional<QosHints>                qos;
    /// Per-(source, target) monotonic sequence number stamped by the
    /// sender's mesh-send-callback when the route requires ordering. Wire
    /// serialization is CBOR integer key 14 (SCE_MESH.md §10.6.3). Absent
    /// on envelopes whose sending route has `ordering: none` or runs on a
    /// transport that `supplies_ordering=true`; receivers with an active
    /// OrderingBuffer drop envelopes missing this field and raise
    /// error.communication.missing_sequence (pre-release wire format; no
    /// backward-compat shim).
    std::optional<std::uint64_t>           sequence_no;

    bool operator==(const MeshEnvelope &other) const noexcept {
        return id                == other.id
            && source            == other.source
            && type              == other.type
            && pattern           == other.pattern
            && datacontenttype   == other.datacontenttype
            && data              == other.data
            && subject           == other.subject
            && correlation_id    == other.correlation_id
            && reply_to          == other.reply_to
            && invoke_id         == other.invoke_id
            && rpc_status        == other.rpc_status
            && rpc_error_message == other.rpc_error_message
            && deadline_unix_ms  == other.deadline_unix_ms
            && qos               == other.qos
            && sequence_no       == other.sequence_no
            && routing_id        == other.routing_id;
    }
};

}  // namespace SCE::Mesh
