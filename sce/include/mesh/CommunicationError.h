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
// envelope_id), the §16.7 row-8 `PEER_PARTITIONED` extras
// (target / last_seen_ms_ago), the §16.7 row-12 `ORDERING_GAP`
// extras (lost_seq_lo / lost_seq_hi), the §16.7 row-6
// `PARALLEL_BARRIER_TIMEOUT` extras (parallel_id / missing_regions /
// timeout_ms), the §16.7 row-13 `REGION_PARTITIONED` extras
// (machine / partition, reusing last_seen_ms_ago), and the §16.7
// row-4 `ENVELOPE_CORRUPT` extras (transport / codec / position).
// Other §16.7 rows add their own extras (invoke_id, etc.) — those
// grow here when a raise site needs them. Each raise site populates
// only the extras named in its row of the catalog; the remainder
// stay empty and are skipped on render.
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

    /// §16.7 row 8 (PEER_PARTITIONED): deploy.yaml name of the peer
    /// whose liveliness token transitioned to DELETE (e.g. "motor").
    /// Authors branch on this inside
    /// `<transition event="error.communication"
    ///  cond="_event.data.reason == 'PEER_PARTITIONED' &&
    ///        _event.data.target == 'motor'">` to react to a specific
    /// peer's drop.
    std::optional<std::string> target;

    /// §16.7 row 8 (PEER_PARTITIONED): milliseconds since the last
    /// PUT sample was observed for this peer, measured against
    /// `std::chrono::steady_clock`. Signed so a reasonable sentinel
    /// exists should a future raise site need one; current raise
    /// sites populate it only when a PUT has been observed.
    std::optional<std::int64_t> last_seen_ms_ago;

    /// §16.7 row 9 (BACKPRESSURE_DROP) and other transport-keyed rows
    /// (1 TRANSPORT_UNAVAILABLE, 2 SEND_FAILED, 3 DELIVERY_EXHAUSTED,
    /// 4 ENVELOPE_CORRUPT): the transport kind ("someip", "zenoh",
    /// etc.) whose plumbing observed the condition. The target-keyed
    /// extra reuses the `target` member above.
    std::optional<std::string> transport;

    /// §16.7 row 4 (ENVELOPE_CORRUPT): payload codec name of the
    /// inbound envelope whose deserialization failed. Spec restricts
    /// the value to `"cbor" | "json" | "typed" | "raw"`. SCE's
    /// MeshEnvelope wire is canonical CBOR (§7.5) so every current
    /// raise site stamps `"cbor"`; the field exists in string form
    /// so future per-binding-codec transports can mark their slot
    /// without an enum-coupling refactor through the raise sites.
    std::optional<std::string> codec;

    /// §16.7 row 4 (ENVELOPE_CORRUPT): byte offset within the
    /// envelope at which deserialization failed, when the codec
    /// reports one. CBOR/tinycbor's parser does not expose a
    /// post-failure cursor through `decodeEnvelope`'s bool return,
    /// so current raise sites leave this absent — preserving the
    /// optional contract documented in §16.7. A future codec
    /// upgrade that surfaces fault position populates this without
    /// touching the catalog row.
    std::optional<std::int64_t> position;

    /// §16.7 row 9 (BACKPRESSURE_DROP): outbound buffer depth at the
    /// moment the overflow was observed. Signed for symmetry with
    /// `last_seen_ms_ago`; depth is never negative in practice.
    std::optional<std::int64_t> queue_depth;

    /// §16.7 row 7 (DEDUP_WINDOW_OVERFLOW): per-sender dedup ring
    /// capacity (`DedupWindow::kCapacity`, 256 at HEAD) at the moment
    /// an eviction was observed. The "sustained rate exceeds capacity"
    /// condition the spec names is detected operationally as
    /// "novel-id insert evicted an existing entry" — the runtime
    /// cannot retain unbounded history to confirm a leaked duplicate,
    /// so eviction is the closest in-bounds signal of the underlying
    /// fault.
    std::optional<std::int64_t> window_size;

    /// §16.7 row 12 (ORDERING_GAP): inclusive low end of the fast-
    /// forwarded sequence range.
    std::optional<std::uint64_t> lost_seq_lo;

    /// §16.7 row 12 (ORDERING_GAP): inclusive high end of the fast-
    /// forwarded sequence range.
    std::optional<std::uint64_t> lost_seq_hi;

    /// §16.7 row 6 (PARALLEL_BARRIER_TIMEOUT): author-declared id of
    /// the `<parallel>` whose §16.5 completion barrier fired. Always
    /// populated by the barrier-timeout raise site; absent for every
    /// other row. Authors branch on the combination of
    /// `reason == 'PARALLEL_BARRIER_TIMEOUT'` and
    /// `_event.data.parallel_id == '<id>'` to react to a specific
    /// barrier.
    std::optional<std::string> parallel_id;

    /// §16.7 row 6 (PARALLEL_BARRIER_TIMEOUT): sorted list of region
    /// ids that had not reported completion when the barrier fired.
    /// Sender contract: non-empty — at least one region must be
    /// missing when the timer wins the race against the completion
    /// threshold. An empty vector indicates a raise-site contract
    /// violation. Rendered as a JSON array of strings.
    std::optional<std::vector<std::string>> missing_regions;

    /// §16.7 row 6 (PARALLEL_BARRIER_TIMEOUT): the deploy-declared
    /// `barrier_timeout_ms:` value in force when the timer armed.
    /// Serialized as an unsigned milliseconds count to match the
    /// shape §14 accepts at parse time.
    std::optional<std::uint64_t> timeout_ms;

    /// §16.7 row 13 (REGION_PARTITIONED): deploy.yaml machine name of
    /// the peer whose **partition** liveliness token transitioned to
    /// DELETE. Orthogonal to `target` (row 8) — row 13 is a machine +
    /// partition pair, row 8 is a machine identity. Authors branch on
    /// `_event.data.reason == 'REGION_PARTITIONED' &&
    ///  _event.data.machine == 'motor' &&
    ///  _event.data.partition == 'motor_right'` to react to a specific
    /// region going down within a sibling machine.
    std::optional<std::string> machine;

    /// §16.7 row 13 (REGION_PARTITIONED): deploy.yaml partition name
    /// under the machine named in `machine`. Populated together with
    /// `machine` by the per-partition liveliness subscriber when it
    /// observes a DELETE on `sce/live/<machine>/<partition>`.
    std::optional<std::string> partition;

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
        if (codec) {
            j["codec"] = *codec;
        }
        if (position) {
            j["position"] = *position;
        }
        if (queue_depth) {
            j["queue_depth"] = *queue_depth;
        }
        if (window_size) {
            j["window_size"] = *window_size;
        }
        if (lost_seq_lo) {
            j["lost_seq_lo"] = *lost_seq_lo;
        }
        if (lost_seq_hi) {
            j["lost_seq_hi"] = *lost_seq_hi;
        }
        if (parallel_id) {
            j["parallel_id"] = *parallel_id;
        }
        if (missing_regions) {
            j["missing_regions"] = *missing_regions;
        }
        if (timeout_ms) {
            j["timeout_ms"] = *timeout_ms;
        }
        if (machine) {
            j["machine"] = *machine;
        }
        if (partition) {
            j["partition"] = *partition;
        }
        const std::string out = j.dump();
        return std::vector<std::uint8_t>(out.begin(), out.end());
    }
};

}  // namespace SCE::Mesh
