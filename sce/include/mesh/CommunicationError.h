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
// (machine / partition, reusing last_seen_ms_ago), the §16.7
// row-4 `ENVELOPE_CORRUPT` extras (transport / codec / position),
// the §16.7 row-5 `INVOKE_CHILD_LOST` extras (invoke_id as a wire
// string covering both §9.5 UUID-form and §9.6 W3C-string-form,
// reusing target), the §16.7 row-2 `SEND_FAILED` extras
// (transport_error, reusing transport / target), and the §16.7
// row-3 `DELIVERY_EXHAUSTED` extras (attempts, reusing
// transport / target / transport_error to carry the last observed
// API decline). Each raise site populates only the extras named in
// its row of the catalog; the remainder stay empty and are skipped
// on render.
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
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::Mesh {

/// §16.7 reason-code vocabulary. One variant per row of the catalog
/// (`SCE_MESH.md` §16.7 rows 1-13). Raise sites populate
/// `CommunicationError::reason` with one of these values; the JSON
/// wire emission converts to the canonical UPPER_SNAKE string via
/// `reasonCodeString()`.
///
/// Variant naming follows the existing `PayloadCodec` enum convention
/// (UpperCamel C++ identifiers, UPPER_SNAKE wire strings). The
/// bidirectional binding between variant and wire string is enforced
/// by `kReasonCodeTable` (the single source of truth) and verified at
/// build time by `tests/mesh/ReasonCodeCatalogTest` against
/// `SCE_MESH.md` §16.7.
enum class ReasonCode : std::uint8_t {
    /// §16.7 row 1 — wire `"TRANSPORT_UNAVAILABLE"`. Transport connect
    /// or reconnect failure.
    TransportUnavailable,
    /// §16.7 row 2 — wire `"SEND_FAILED"`. Envelope `send()` returns
    /// error from transport API.
    SendFailed,
    /// §16.7 row 3 — wire `"DELIVERY_EXHAUSTED"`. Reliable transport
    /// unable to deliver after configured retries.
    DeliveryExhausted,
    /// §16.7 row 4 — wire `"ENVELOPE_CORRUPT"`. Inbound envelope
    /// deserialization or schema validation fails.
    EnvelopeCorrupt,
    /// §16.7 row 5 — wire `"INVOKE_CHILD_LOST"`. Invoke child device
    /// unreachable (transport-level).
    InvokeChildLost,
    /// §16.7 row 6 — wire `"PARALLEL_BARRIER_TIMEOUT"`. Parallel
    /// barrier timeout (§16.5).
    ParallelBarrierTimeout,
    /// §16.7 row 7 — wire `"DEDUP_WINDOW_OVERFLOW"`. Envelope dedup
    /// window overflow (sustained rate exceeds window capacity).
    DedupWindowOverflow,
    /// §16.7 row 8 — wire `"PEER_PARTITIONED"`. Peer machine's
    /// liveness signal observed lost.
    PeerPartitioned,
    /// §16.7 row 9 — wire `"BACKPRESSURE_DROP"`. Transport backpressure
    /// queue full, outbound envelope dropped (§10.10
    /// `OutboundBuffer` at `max_pending_per_target`).
    BackpressureDrop,
    /// §16.7 row 10 — wire `"UNAUTHORIZED"`. Peer rejected envelope
    /// due to authorization failure.
    Unauthorized,
    /// §16.7 row 11 — wire `"MISSING_SEQUENCE"`. Inbound envelope
    /// reached an active `OrderingBuffer` without `sequence_no`
    /// (§10.6.3).
    MissingSequence,
    /// §16.7 row 12 — wire `"ORDERING_GAP"`. `OrderingBuffer`
    /// fast-forwarded past a missing sequence range after
    /// `gap_timeout` expired (§10.6.4).
    OrderingGap,
    /// §16.7 row 13 — wire `"REGION_PARTITIONED"`. Peer
    /// region-partition's liveness signal observed lost (§16.4
    /// per-partition liveness).
    RegionPartitioned,
};

/// Canonical mapping table — single source of truth for both JSON
/// emission and the build-time cross-doc test against
/// `SCE_MESH.md` §16.7. Adding a §16.7 row requires extending
/// this table; the cross-doc test fails the build if the table and
/// the markdown catalog diverge.
inline constexpr std::array<std::pair<ReasonCode, std::string_view>, 13>
    kReasonCodeTable = {{
        {ReasonCode::TransportUnavailable, "TRANSPORT_UNAVAILABLE"},
        {ReasonCode::SendFailed, "SEND_FAILED"},
        {ReasonCode::DeliveryExhausted, "DELIVERY_EXHAUSTED"},
        {ReasonCode::EnvelopeCorrupt, "ENVELOPE_CORRUPT"},
        {ReasonCode::InvokeChildLost, "INVOKE_CHILD_LOST"},
        {ReasonCode::ParallelBarrierTimeout, "PARALLEL_BARRIER_TIMEOUT"},
        {ReasonCode::DedupWindowOverflow, "DEDUP_WINDOW_OVERFLOW"},
        {ReasonCode::PeerPartitioned, "PEER_PARTITIONED"},
        {ReasonCode::BackpressureDrop, "BACKPRESSURE_DROP"},
        {ReasonCode::Unauthorized, "UNAUTHORIZED"},
        {ReasonCode::MissingSequence, "MISSING_SEQUENCE"},
        {ReasonCode::OrderingGap, "ORDERING_GAP"},
        {ReasonCode::RegionPartitioned, "REGION_PARTITIONED"},
    }};

/// Map a `ReasonCode` to its canonical wire string. Linear lookup
/// over the 13-entry table; the compiler collapses the loop under
/// `-O2`. Crashes (logic-error path) if a future variant is added
/// to the enum without extending `kReasonCodeTable` — the missing-
/// entry case is caught earlier by the cross-doc test, but the
/// runtime guard exists so a partial-edit during development surfaces
/// as a deterministic failure rather than reading off the end.
[[nodiscard]] inline constexpr std::string_view reasonCodeString(ReasonCode code) {
    for (const auto &entry : kReasonCodeTable) {
        if (entry.first == code) {
            return entry.second;
        }
    }
    // Unreachable in a well-formed build (kReasonCodeTable covers
    // every enum variant; the cross-doc test enforces this). The
    // runtime fallback below avoids UB and surfaces the gap.
    return "UNKNOWN_REASON";
}

struct CommunicationError {
    /// §16.7 machine-readable reason code. Required — raise sites
    /// must populate it via the `ReasonCode` enum; the JSON emit
    /// converts to the canonical UPPER_SNAKE wire string.
    ReasonCode reason{ReasonCode::TransportUnavailable};

    /// §10.7.1 baseline: human-readable detail string. Optional.
    std::optional<std::string> detail;

    /// §10.7.1 baseline: envelope `source` (sender machine name) for
    /// inbound-triggered conditions. Absent for outbound-triggered
    /// conditions where `source` is not yet known.
    std::optional<std::string> source;

    /// §10.7.1 baseline: envelope id (UUID v7 bytes). Rendered as the
    /// RFC 4122 canonical 36-char string via SCE::uuid::to_string.
    std::optional<std::array<std::uint8_t, 16>> envelope_id;

    /// §16.7 row 5 (INVOKE_CHILD_LOST): wire-level invoke id of the
    /// invoke whose reply will never arrive. Two source-equivalent
    /// shapes share this single wire field:
    ///   * §9.5 `<invoke type="sce:mesh-rpc">` — caller stringifies
    ///     the UUID v7 correlation key (`SCE::uuid::to_string`) so
    ///     the wire shape is the canonical 36-char RFC 4122 form,
    ///     matching `envelope_id`.
    ///   * §9.6 `<invoke type="scxml">` — caller stashes the W3C
    ///     SCXML invokeId string directly (an author-declared
    ///     `<invoke id="myInvoke">` literal, or the codegen
    ///     auto-generated `_invoke_0` form). W3C 6.4.1 specifies
    ///     invokeid as a free-form string with no UUID requirement.
    ///
    /// The C++ field is `optional<string>` (not `array<uint8_t,16>`)
    /// to accommodate the §9.6 path natively without a parallel
    /// `invoke_string_id` field. Distinct from `envelope_id` (which
    /// keys the envelope that triggered an inbound condition) —
    /// `invoke_id` keys the invoke whose reply will never arrive.
    std::optional<std::string> invoke_id;

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

    /// §16.7 row 2 (SEND_FAILED): underlying transport-API error
    /// message captured by the dispatcher at the moment of decline.
    /// SOME/IP populates a sentinel string when `app.send()` returns
    /// `false` (vsomeip exposes no errno equivalent); Zenoh populates
    /// `ZException::what()` when `Publisher::put` throws; both keep
    /// the field absent on the happy path. Distinct from the
    /// human-readable `detail` field (§10.7.1 baseline): `detail` is
    /// SCE-authored prose, `transport_error` is the raw API surface
    /// the SCXML author may want to log or correlate to transport
    /// telemetry. Pairs with `transport` (which names the binding)
    /// and `target` (which names the peer that declined).
    ///
    /// Row 3 DELIVERY_EXHAUSTED reuses this field to carry the LAST
    /// observed transport_error before the retry layer gave up — so
    /// the author can correlate the exhaustion event with the
    /// underlying API decline that drove the final attempt's failure.
    std::optional<std::string> transport_error;

    /// §16.7 row 10 (UNAUTHORIZED): underlying transport-API status
    /// string captured at the moment the peer rejected on
    /// authorization. Distinct from `transport_error` (row 2's
    /// SEND_FAILED dispatcher-decline): row 10 fires at the trust-
    /// boundary handshake (Zenoh TLS / SOMEIP SD denial), so the
    /// status string is the raw rejection text from the underlying
    /// API — `ZException::what()` for zenoh (carrying the TLS error
    /// chain), the SD response code label for SOMEIP. Lets the author
    /// log / correlate the rejection without re-deriving it from
    /// transport telemetry, while keeping the SCE-authored reason
    /// (`reason: "UNAUTHORIZED"`) on the wire as the SCE-level signal.
    /// Absent on every other row's raise site — row 10 is the only
    /// producer.
    ///
    /// Production-deferred under current zenoh-cpp (axis-6 limitation,
    /// docs/SCE_AXIS_6_PATTERNS.md A6-001). zenoh-cpp generic-wraps
    /// every connection error into `Z_ENETWORK` so the `ZException::what()`
    /// substring scan in `mesh/third_party/AuthClassifier.h` does not
    /// fire — the zenoh-side row-10 raise path is dead code until
    /// upstream exposes a typed auth-failure discriminator. The SOMEIP
    /// SD-denial arm remains live (binding-declared classification, no
    /// text inspection needed).
    std::optional<std::string> transport_status;

    /// §16.7 row 3 (DELIVERY_EXHAUSTED): total number of dispatch
    /// attempts the retry layer made before giving up. Equals
    /// `max_retries + 1` for the common "exhausted after configured
    /// retries" path (first attempt + N retries); equals `1` when
    /// the dispatcher classified its first failure as TERMINAL
    /// (`SendResult.retryable == false`) and the retry layer
    /// fast-failed without consuming attempts. Absent on every other
    /// row's raise site — row 3 is the only producer.
    std::optional<std::int64_t> attempts;

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
        j["reason"] = reasonCodeString(reason);
        if (detail) {
            j["detail"] = *detail;
        }
        if (source) {
            j["source"] = *source;
        }
        if (envelope_id) {
            j["envelope_id"] = SCE::uuid::to_string(*envelope_id);
        }
        if (invoke_id) {
            j["invoke_id"] = *invoke_id;
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
        if (transport_error) {
            j["transport_error"] = *transport_error;
        }
        if (transport_status) {
            j["transport_status"] = *transport_status;
        }
        if (attempts) {
            j["attempts"] = *attempts;
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
