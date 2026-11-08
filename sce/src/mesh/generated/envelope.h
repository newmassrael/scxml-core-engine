// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: envelope.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_ENVELOPE_H
#define SCE_FORGE_ENVELOPE_H

#include <cstdint>
#include <limits>
#include <optional>
#include <string>
#include <utility>
#include <vector>

#include "causal_entry.h"
#include "pattern_kind.h"
#include "payload_codec.h"
#include "rpc_status.h"
#include "sce/forge/cbor.h"
#include "sce/forge/codec.h"

namespace SCE::Generated::Envelope {

/// One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and
/// skips one this codec does not declare; encode writes the entries present in
/// ascending key order, every head in its shortest form. A required entry
/// starts at its type's default and an optional one absent, so
/// `Envelope{}` is the infallible constructor every codec offers.
struct Envelope {
    /// Map key 0, required.
    std::vector<uint8_t> id{};
    /// Map key 1, required.
    std::string source{};
    /// Map key 2, required.
    std::string event_type{};
    /// Map key 3, required.
    SCE::Generated::PatternKind::PatternKind pattern{SCE::Generated::PatternKind::PatternKind::FireForget};
    /// Map key 4, required.
    SCE::Generated::PayloadCodec::PayloadCodec datacontenttype{SCE::Generated::PayloadCodec::PayloadCodec::None};
    /// Map key 5, required.
    std::vector<uint8_t> data{};
    /// Map key 6.
    std::optional<std::string> subject{};
    /// Map key 7.
    std::optional<std::vector<uint8_t>> correlation_id{};
    /// Map key 8.
    std::optional<std::string> reply_to{};
    /// Map key 9.
    std::optional<std::vector<uint8_t>> invoke_id{};
    /// Map key 10.
    std::optional<SCE::Generated::RpcStatus::RpcStatus> rpc_status{};
    /// Map key 11.
    std::optional<std::string> rpc_error_message{};
    /// Map key 12.
    std::optional<uint64_t> deadline_unix_ms{};
    /// Map key 14.
    std::optional<uint64_t> sequence_no{};
    /// Map key 15.
    std::optional<std::vector<uint8_t>> routing_id{};
    /// Map key 16.
    std::optional<std::string> parallel_id{};
    /// Map key 17.
    std::optional<std::string> region_id{};
    /// Map key 18.
    std::optional<std::string> child_session_id{};
    /// Map key 19.
    std::optional<uint64_t> causal_epoch{};
    /// Map key 20.
    std::optional<std::vector<SCE::Generated::CausalEntry::CausalEntryPayload>> causal_vector{};

    /// Decode one map from `cursor`. On success the cursor advances past it;
    /// `std::nullopt` on any refusal, with the cursor left where it was.
    static std::optional<Envelope> decode(::SCE::Forge::SceCursor &cursor) {
        ::SCE::Forge::SceCursor c = cursor;
        auto value = decode_at(c);
        if (value) {
            cursor = c;
        }
        return value;
    }

    /// Encode this map into `w`; `std::nullopt` on success. An entry whose
    /// value breaks its declared bound — an exact length, a maximum size — is
    /// refused, not written.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink &w) const noexcept {
        namespace cbor = ::SCE::Forge::Cbor;
        std::uint64_t count = 6;
        if (subject.has_value()) {
            ++count;
        }
        if (correlation_id.has_value()) {
            ++count;
        }
        if (reply_to.has_value()) {
            ++count;
        }
        if (invoke_id.has_value()) {
            ++count;
        }
        if (rpc_status.has_value()) {
            ++count;
        }
        if (rpc_error_message.has_value()) {
            ++count;
        }
        if (deadline_unix_ms.has_value()) {
            ++count;
        }
        if (sequence_no.has_value()) {
            ++count;
        }
        if (routing_id.has_value()) {
            ++count;
        }
        if (parallel_id.has_value()) {
            ++count;
        }
        if (region_id.has_value()) {
            ++count;
        }
        if (child_session_id.has_value()) {
            ++count;
        }
        if (causal_epoch.has_value()) {
            ++count;
        }
        if (causal_vector.has_value()) {
            ++count;
        }
        if (auto e = cbor::write_map_head(w, count)) {
            return e;
        }
        {
            const auto &v = id;
            if (auto e = cbor::write_uint(w, 0)) {
                return e;
            }
            if (v.size() != 16) {
                return ::SCE::Forge::CodecError::CborWrongLength;
            }
            if (auto e = cbor::write_bytes(w, v)) {
                return e;
            }
        }
        {
            const auto &v = source;
            if (auto e = cbor::write_uint(w, 1)) {
                return e;
            }
            if (v.size() > 262144) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_text(w, v)) {
                return e;
            }
        }
        {
            const auto &v = event_type;
            if (auto e = cbor::write_uint(w, 2)) {
                return e;
            }
            if (v.size() > 262144) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_text(w, v)) {
                return e;
            }
        }
        {
            const auto &v = pattern;
            if (auto e = cbor::write_uint(w, 3)) {
                return e;
            }
            const auto u = SCE::Generated::PatternKind::to_underlying(v);
            if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(u))) {
                return e;
            }
        }
        {
            const auto &v = datacontenttype;
            if (auto e = cbor::write_uint(w, 4)) {
                return e;
            }
            const auto u = SCE::Generated::PayloadCodec::to_underlying(v);
            if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(u))) {
                return e;
            }
        }
        {
            const auto &v = data;
            if (auto e = cbor::write_uint(w, 5)) {
                return e;
            }
            if (v.size() > 16777216) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_bytes(w, v)) {
                return e;
            }
        }
        if (subject.has_value()) {
            const auto &v = *subject;
            if (auto e = cbor::write_uint(w, 6)) {
                return e;
            }
            if (v.size() > 262144) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_text(w, v)) {
                return e;
            }
        }
        if (correlation_id.has_value()) {
            const auto &v = *correlation_id;
            if (auto e = cbor::write_uint(w, 7)) {
                return e;
            }
            if (v.size() != 16) {
                return ::SCE::Forge::CodecError::CborWrongLength;
            }
            if (auto e = cbor::write_bytes(w, v)) {
                return e;
            }
        }
        if (reply_to.has_value()) {
            const auto &v = *reply_to;
            if (auto e = cbor::write_uint(w, 8)) {
                return e;
            }
            if (v.size() > 262144) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_text(w, v)) {
                return e;
            }
        }
        if (invoke_id.has_value()) {
            const auto &v = *invoke_id;
            if (auto e = cbor::write_uint(w, 9)) {
                return e;
            }
            if (v.size() != 16) {
                return ::SCE::Forge::CodecError::CborWrongLength;
            }
            if (auto e = cbor::write_bytes(w, v)) {
                return e;
            }
        }
        if (rpc_status.has_value()) {
            const auto &v = *rpc_status;
            if (auto e = cbor::write_uint(w, 10)) {
                return e;
            }
            const auto u = SCE::Generated::RpcStatus::to_underlying(v);
            if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(u))) {
                return e;
            }
        }
        if (rpc_error_message.has_value()) {
            const auto &v = *rpc_error_message;
            if (auto e = cbor::write_uint(w, 11)) {
                return e;
            }
            if (v.size() > 262144) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_text(w, v)) {
                return e;
            }
        }
        if (deadline_unix_ms.has_value()) {
            const auto &v = *deadline_unix_ms;
            if (auto e = cbor::write_uint(w, 12)) {
                return e;
            }
            if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(v))) {
                return e;
            }
        }
        if (sequence_no.has_value()) {
            const auto &v = *sequence_no;
            if (auto e = cbor::write_uint(w, 14)) {
                return e;
            }
            if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(v))) {
                return e;
            }
        }
        if (routing_id.has_value()) {
            const auto &v = *routing_id;
            if (auto e = cbor::write_uint(w, 15)) {
                return e;
            }
            if (v.size() != 16) {
                return ::SCE::Forge::CodecError::CborWrongLength;
            }
            if (auto e = cbor::write_bytes(w, v)) {
                return e;
            }
        }
        if (parallel_id.has_value()) {
            const auto &v = *parallel_id;
            if (auto e = cbor::write_uint(w, 16)) {
                return e;
            }
            if (v.size() > 262144) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_text(w, v)) {
                return e;
            }
        }
        if (region_id.has_value()) {
            const auto &v = *region_id;
            if (auto e = cbor::write_uint(w, 17)) {
                return e;
            }
            if (v.size() > 262144) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_text(w, v)) {
                return e;
            }
        }
        if (child_session_id.has_value()) {
            const auto &v = *child_session_id;
            if (auto e = cbor::write_uint(w, 18)) {
                return e;
            }
            if (v.size() > 262144) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_text(w, v)) {
                return e;
            }
        }
        if (causal_epoch.has_value()) {
            const auto &v = *causal_epoch;
            if (auto e = cbor::write_uint(w, 19)) {
                return e;
            }
            if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(v))) {
                return e;
            }
        }
        if (causal_vector.has_value()) {
            const auto &v = *causal_vector;
            if (auto e = cbor::write_uint(w, 20)) {
                return e;
            }
            if (v.size() > 64) {
                return ::SCE::Forge::CodecError::CborOutOfRange;
            }
            if (auto e = cbor::write_array_head(w, v.size())) {
                return e;
            }
            for (const auto &record : v) {
                if (auto e = encode_record_causal_entry(w, record)) {
                    return e;
                }
            }
        }
        return std::nullopt;
    }

    /// `encode` into a new vector; `std::nullopt` when encode refuses the value.
    [[nodiscard]] std::optional<std::vector<std::uint8_t>> encode_to_vec() const {
        std::vector<std::uint8_t> out;
        ::SCE::Forge::VectorSink sink(out);
        if (encode(sink)) {
            return std::nullopt;
        }
        return out;
    }

private:
    static std::optional<Envelope> decode_at(::SCE::Forge::SceCursor &c) {
        namespace cbor = ::SCE::Forge::Cbor;
        const auto count = cbor::read_map_len(c);
        if (!count) {
            return std::nullopt;
        }
        std::optional<std::vector<uint8_t>> id;
        std::optional<std::string> source;
        std::optional<std::string> event_type;
        std::optional<SCE::Generated::PatternKind::PatternKind> pattern;
        std::optional<SCE::Generated::PayloadCodec::PayloadCodec> datacontenttype;
        std::optional<std::vector<uint8_t>> data;
        std::optional<std::string> subject;
        std::optional<std::vector<uint8_t>> correlation_id;
        std::optional<std::string> reply_to;
        std::optional<std::vector<uint8_t>> invoke_id;
        std::optional<SCE::Generated::RpcStatus::RpcStatus> rpc_status;
        std::optional<std::string> rpc_error_message;
        std::optional<uint64_t> deadline_unix_ms;
        std::optional<uint64_t> sequence_no;
        std::optional<std::vector<uint8_t>> routing_id;
        std::optional<std::string> parallel_id;
        std::optional<std::string> region_id;
        std::optional<std::string> child_session_id;
        std::optional<uint64_t> causal_epoch;
        std::optional<std::vector<SCE::Generated::CausalEntry::CausalEntryPayload>> causal_vector;
        for (std::uint64_t i = 0; i < *count; ++i) {
            const auto key = cbor::read_uint(c);
            if (!key) {
                return std::nullopt;
            }
            switch (*key) {
            case 0: {
                // A key given twice is not one map entry.
                if (id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes_exact(c, 16);
                if (!v) {
                    return std::nullopt;
                }
                id = std::move(*v);
                break;
            }
            case 1: {
                // A key given twice is not one map entry.
                if (source.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                source = std::move(*v);
                break;
            }
            case 2: {
                // A key given twice is not one map entry.
                if (event_type.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                event_type = std::move(*v);
                break;
            }
            case 3: {
                // A key given twice is not one map entry.
                if (pattern.has_value()) {
                    return std::nullopt;
                }
                const auto raw =
                    cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<uint16_t>::max()));
                if (!raw) {
                    return std::nullopt;
                }
                auto v = SCE::Generated::PatternKind::from_underlying(static_cast<uint16_t>(*raw));
                if (!v) {
                    return std::nullopt;
                }
                pattern = std::move(*v);
                break;
            }
            case 4: {
                // A key given twice is not one map entry.
                if (datacontenttype.has_value()) {
                    return std::nullopt;
                }
                const auto raw =
                    cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<uint8_t>::max()));
                if (!raw) {
                    return std::nullopt;
                }
                auto v = SCE::Generated::PayloadCodec::from_underlying(static_cast<uint8_t>(*raw));
                if (!v) {
                    return std::nullopt;
                }
                datacontenttype = std::move(*v);
                break;
            }
            case 5: {
                // A key given twice is not one map entry.
                if (data.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes(c, 16777216);
                if (!v) {
                    return std::nullopt;
                }
                data = std::move(*v);
                break;
            }
            case 6: {
                // A key given twice is not one map entry.
                if (subject.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                subject = std::move(*v);
                break;
            }
            case 7: {
                // A key given twice is not one map entry.
                if (correlation_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes_exact(c, 16);
                if (!v) {
                    return std::nullopt;
                }
                correlation_id = std::move(*v);
                break;
            }
            case 8: {
                // A key given twice is not one map entry.
                if (reply_to.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                reply_to = std::move(*v);
                break;
            }
            case 9: {
                // A key given twice is not one map entry.
                if (invoke_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes_exact(c, 16);
                if (!v) {
                    return std::nullopt;
                }
                invoke_id = std::move(*v);
                break;
            }
            case 10: {
                // A key given twice is not one map entry.
                if (rpc_status.has_value()) {
                    return std::nullopt;
                }
                const auto raw =
                    cbor::read_uint_upto(c, static_cast<std::uint64_t>(std::numeric_limits<uint8_t>::max()));
                if (!raw) {
                    return std::nullopt;
                }
                auto v = SCE::Generated::RpcStatus::from_underlying(static_cast<uint8_t>(*raw));
                if (!v) {
                    return std::nullopt;
                }
                rpc_status = std::move(*v);
                break;
            }
            case 11: {
                // A key given twice is not one map entry.
                if (rpc_error_message.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                rpc_error_message = std::move(*v);
                break;
            }
            case 12: {
                // A key given twice is not one map entry.
                if (deadline_unix_ms.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_uint(c);
                if (!v) {
                    return std::nullopt;
                }
                deadline_unix_ms = std::move(*v);
                break;
            }
            case 14: {
                // A key given twice is not one map entry.
                if (sequence_no.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_uint(c);
                if (!v) {
                    return std::nullopt;
                }
                sequence_no = std::move(*v);
                break;
            }
            case 15: {
                // A key given twice is not one map entry.
                if (routing_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_bytes_exact(c, 16);
                if (!v) {
                    return std::nullopt;
                }
                routing_id = std::move(*v);
                break;
            }
            case 16: {
                // A key given twice is not one map entry.
                if (parallel_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                parallel_id = std::move(*v);
                break;
            }
            case 17: {
                // A key given twice is not one map entry.
                if (region_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                region_id = std::move(*v);
                break;
            }
            case 18: {
                // A key given twice is not one map entry.
                if (child_session_id.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_text(c, 262144);
                if (!v) {
                    return std::nullopt;
                }
                child_session_id = std::move(*v);
                break;
            }
            case 19: {
                // A key given twice is not one map entry.
                if (causal_epoch.has_value()) {
                    return std::nullopt;
                }
                auto v = cbor::read_uint(c);
                if (!v) {
                    return std::nullopt;
                }
                causal_epoch = std::move(*v);
                break;
            }
            case 20: {
                // A key given twice is not one map entry.
                if (causal_vector.has_value()) {
                    return std::nullopt;
                }
                // A definite-length array within the list's counts, each item
                // one record's map.
                const auto len = cbor::read_array_len(c);
                if (!len || *len > 64) {
                    return std::nullopt;
                }
                std::optional<std::vector<SCE::Generated::CausalEntry::CausalEntryPayload>> v{std::in_place};
                for (std::uint64_t n = 0; n < *len; ++n) {
                    auto record = decode_record_causal_entry(c);
                    if (!record) {
                        return std::nullopt;
                    }
                    v->push_back(std::move(*record));
                }
                if (!v) {
                    return std::nullopt;
                }
                causal_vector = std::move(*v);
                break;
            }
            default:
                if (!cbor::skip(c)) {
                    return std::nullopt;
                }
                break;
            }
        }
        if (!id.has_value()) {
            return std::nullopt;
        }
        if (!source.has_value()) {
            return std::nullopt;
        }
        if (!event_type.has_value()) {
            return std::nullopt;
        }
        if (!pattern.has_value()) {
            return std::nullopt;
        }
        if (!datacontenttype.has_value()) {
            return std::nullopt;
        }
        if (!data.has_value()) {
            return std::nullopt;
        }
        Envelope out;
        out.id = std::move(*id);
        out.source = std::move(*source);
        out.event_type = std::move(*event_type);
        out.pattern = std::move(*pattern);
        out.datacontenttype = std::move(*datacontenttype);
        out.data = std::move(*data);
        out.subject = std::move(subject);
        out.correlation_id = std::move(correlation_id);
        out.reply_to = std::move(reply_to);
        out.invoke_id = std::move(invoke_id);
        out.rpc_status = std::move(rpc_status);
        out.rpc_error_message = std::move(rpc_error_message);
        out.deadline_unix_ms = std::move(deadline_unix_ms);
        out.sequence_no = std::move(sequence_no);
        out.routing_id = std::move(routing_id);
        out.parallel_id = std::move(parallel_id);
        out.region_id = std::move(region_id);
        out.child_session_id = std::move(child_session_id);
        out.causal_epoch = std::move(causal_epoch);
        out.causal_vector = std::move(causal_vector);
        return out;
    }

    /// Read one record of the event schema `causal_entry` (`record:CausalEntry`)
    /// from `c`, as a list of this map holds it (docs/adr/0085, Landing order
    /// step 3a), into the payload type the schema's own unit declares: one CBOR
    /// map of every field, each under its id as text (docs/adr/0086, section
    /// 9.2). Its keys in any order, a key the schema does not declare skipped
    /// whole; a key given twice, a field missing and a text or byte string past
    /// its bound are `std::nullopt`.
    static std::optional<SCE::Generated::CausalEntry::CausalEntryPayload>
    decode_record_causal_entry(::SCE::Forge::SceCursor &c) {
        namespace cbor = ::SCE::Forge::Cbor;
        const auto count = cbor::read_map_len(c);
        if (!count) {
            return std::nullopt;
        }
        std::optional<uint64_t> held_hi;
        std::optional<uint64_t> held_lo;
        std::optional<uint64_t> held_seq;
        for (std::uint64_t i = 0; i < *count; ++i) {
            const auto key = cbor::read_text(c, std::nullopt);
            if (!key) {
                return std::nullopt;
            }
            if (*key == "hi") {
                // A key given twice is not one map entry.
                if (held_hi.has_value()) {
                    return std::nullopt;
                }
                held_hi = cbor::read_uint(c);
                if (!held_hi) {
                    return std::nullopt;
                }
            } else if (*key == "lo") {
                // A key given twice is not one map entry.
                if (held_lo.has_value()) {
                    return std::nullopt;
                }
                held_lo = cbor::read_uint(c);
                if (!held_lo) {
                    return std::nullopt;
                }
            } else if (*key == "seq") {
                // A key given twice is not one map entry.
                if (held_seq.has_value()) {
                    return std::nullopt;
                }
                held_seq = cbor::read_uint(c);
                if (!held_seq) {
                    return std::nullopt;
                }
            } else if (!cbor::skip(c)) {
                return std::nullopt;
            }
        }
        if (!held_hi.has_value()) {
            return std::nullopt;
        }
        if (!held_lo.has_value()) {
            return std::nullopt;
        }
        if (!held_seq.has_value()) {
            return std::nullopt;
        }
        SCE::Generated::CausalEntry::CausalEntryPayload record;
        record.hi = std::move(*held_hi);
        record.lo = std::move(*held_lo);
        record.seq = std::move(*held_seq);
        return record;
    }

    /// Write one record of `causal_entry` as one map of every field, the keys
    /// in the order RFC 8949 §4.2.1 sorts their encodings: a shorter key first.
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError>
    encode_record_causal_entry(::SCE::Forge::SceSink &w,
                               const SCE::Generated::CausalEntry::CausalEntryPayload &record) noexcept {
        namespace cbor = ::SCE::Forge::Cbor;
        if (auto e = cbor::write_map_head(w, 3)) {
            return e;
        }
        if (auto e = cbor::write_text(w, "hi")) {
            return e;
        }
        if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(record.hi))) {
            return e;
        }
        if (auto e = cbor::write_text(w, "lo")) {
            return e;
        }
        if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(record.lo))) {
            return e;
        }
        if (auto e = cbor::write_text(w, "seq")) {
            return e;
        }
        if (auto e = cbor::write_uint(w, static_cast<std::uint64_t>(record.seq))) {
            return e;
        }
        return std::nullopt;
    }
};

}  // namespace SCE::Generated::Envelope

#endif  // SCE_FORGE_ENVELOPE_H
