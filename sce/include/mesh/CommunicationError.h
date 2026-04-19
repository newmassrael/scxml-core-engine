// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh CommunicationError — `_event.data` builder for the
// `error.communication` event class.
//
// SCE_MESH.md §10.7.1 pins the JSON-shaped `_event.data` convention for
// `error.*` events; §16.7 catalogues the reason codes and their per-
// condition extra fields. This header is the runtime single-source-of-
// truth for that shape: every call site that needs to raise
// `error.communication` populates a `CommunicationError` value and asks
// `toJsonBytes()` to render it. The resulting bytes are suitable for
// `MeshEnvelope::data` when `datacontenttype == PayloadCodec::Json`.
//
// Field set covers the baseline (errorName, reason, detail, source,
// envelope_id), the §16.7 row-9 `PEER_PARTITIONED` extras
// (target / last_seen_ms_ago), and the §16.7 row-13 `ORDERING_GAP`
// extras (lost_seq_lo / lost_seq_hi). Other §16.7 rows add their own
// extras (transport, invoke_id, etc.) — those grow here when a raise
// site needs them. Each raise site populates only the extras named in
// its row of the catalog; the remainder stay empty and are skipped on
// render.
//
// Design notes:
//   * Pure header; the canonical JSON render uses
//     `nlohmann::ordered_json` so insertion order is preserved — the
//     fixed field order (errorName → reason → optional extras) is
//     therefore the source order of the assignments inside
//     `toJsonBytes`.
//   * RFC 8259 escapes (`"`, `\\`, control chars) and the omit-when-
//     absent semantics for optionals are delegated to nlohmann; the
//     hand-rolled encoder this header used to carry is gone now that
//     `nlohmann/json.hpp` is already published through `sce_core` for
//     every consumer.
//   * `envelope_id` renders as the RFC 4122 §3 canonical 36-char form
//     produced by `SCE::uuid::to_string`, so authors comparing against
//     engine-side `_event.data.envelope_id` get the familiar hex-dash
//     shape rather than raw bytes.

#pragma once

#include "common/Uuid.h"

#include <nlohmann/json.hpp>

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Mesh {

struct CommunicationError {
    /// §16.7 machine-readable reason code (e.g. "MISSING_SEQUENCE",
    /// "ORDERING_GAP"). Required — raise sites must populate it.
    std::string reason;

    /// §10.7.1 baseline: human-readable detail string. Optional.
    std::optional<std::string> detail;

    /// §10.7.1 baseline: envelope `source` (sender machine name) for
    /// inbound-triggered conditions. Absent for outbound-triggered
    /// conditions where `source` is not yet known.
    std::optional<std::string> source;

    /// §10.7.1 baseline: envelope id (UUID v7 bytes). Rendered as the
    /// RFC 4122 canonical 36-char string via SCE::uuid::to_string.
    std::optional<std::array<std::uint8_t, 16>> envelope_id;

    /// §16.7 row 9 (PEER_PARTITIONED): deploy.yaml name of the peer
    /// whose liveliness token transitioned to DELETE (e.g. "motor").
    /// Authors branch on this inside
    /// `<transition event="error.communication"
    ///  cond="_event.data.reason == 'PEER_PARTITIONED' &&
    ///        _event.data.target == 'motor'">` to react to a specific
    /// peer's drop.
    std::optional<std::string> target;

    /// §16.7 row 9 (PEER_PARTITIONED): milliseconds since the last
    /// PUT sample was observed for this peer, measured against
    /// `std::chrono::steady_clock`. Signed so a reasonable sentinel
    /// exists should a future raise site need one; current raise
    /// sites populate it only when a PUT has been observed.
    std::optional<std::int64_t> last_seen_ms_ago;

    /// §16.7 row 10 (BACKPRESSURE_DROP) and other transport-keyed rows
    /// (1 TRANSPORT_UNAVAILABLE, 2 SEND_FAILED, 3 DELIVERY_EXHAUSTED,
    /// 4 ENVELOPE_CORRUPT): the transport kind ("someip", "zenoh",
    /// etc.) whose plumbing observed the condition. The target-keyed
    /// extra reuses the `target` member above.
    std::optional<std::string> transport;

    /// §16.7 row 10 (BACKPRESSURE_DROP): outbound buffer depth at the
    /// moment the overflow was observed. Signed for symmetry with
    /// `last_seen_ms_ago`; depth is never negative in practice.
    std::optional<std::int64_t> queue_depth;

    /// §16.7 row 13 (ORDERING_GAP): inclusive low end of the fast-
    /// forwarded sequence range.
    std::optional<std::uint64_t> lost_seq_lo;

    /// §16.7 row 13 (ORDERING_GAP): inclusive high end of the fast-
    /// forwarded sequence range.
    std::optional<std::uint64_t> lost_seq_hi;

    /// Render to canonical JSON bytes for `MeshEnvelope::data`.
    ///
    /// Uses `nlohmann::ordered_json` so the wire field order matches
    /// the source order of the assignments below — errorName, reason,
    /// then any populated optional in declaration order. Optional
    /// fields whose `std::optional` is empty are skipped entirely;
    /// they are not emitted as JSON `null`, matching §10.7.1's
    /// "absent or null" wording and keeping the payload small on the
    /// happy path.
    [[nodiscard]] std::vector<std::uint8_t> toJsonBytes() const {
        nlohmann::ordered_json j;
        j["errorName"] = "communication";
        j["reason"] = reason;
        if (detail) {
            j["detail"] = *detail;
        }
        if (source) {
            j["source"] = *source;
        }
        if (envelope_id) {
            j["envelope_id"] = SCE::uuid::to_string(*envelope_id);
        }
        if (target) {
            j["target"] = *target;
        }
        if (last_seen_ms_ago) {
            j["last_seen_ms_ago"] = *last_seen_ms_ago;
        }
        if (transport) {
            j["transport"] = *transport;
        }
        if (queue_depth) {
            j["queue_depth"] = *queue_depth;
        }
        if (lost_seq_lo) {
            j["lost_seq_lo"] = *lost_seq_lo;
        }
        if (lost_seq_hi) {
            j["lost_seq_hi"] = *lost_seq_hi;
        }
        const std::string out = j.dump();
        return std::vector<std::uint8_t>(out.begin(), out.end());
    }
};

}  // namespace SCE::Mesh
