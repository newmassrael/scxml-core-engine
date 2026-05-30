// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "mesh/MeshEnvelopeCodec.h"

#include <cbor.h>

#include <cstring>

namespace SCE::Mesh {

namespace {

// ── Encoder helpers ─────────────────────────────────────────────────────
//
// Each helper returns the CborError; callers OR them into a running error
// and bail at the end. The common case is CborNoError + CborErrorOutOfMemory
// (latter triggers the two-pass resize).

CborError encodeIntKey(CborEncoder *m, int key) {
    return cbor_encode_int(m, key);
}

CborError encodeBytes16Key(CborEncoder *m, int key, const std::array<uint8_t, 16> &v) {
    CborError e = encodeIntKey(m, key);
    if (e == CborNoError) e = cbor_encode_byte_string(m, v.data(), v.size());
    return e;
}

CborError encodeStringKey(CborEncoder *m, int key, const std::string &v) {
    CborError e = encodeIntKey(m, key);
    if (e == CborNoError) e = cbor_encode_text_string(m, v.data(), v.size());
    return e;
}

CborError encodeUintKey(CborEncoder *m, int key, uint64_t v) {
    CborError e = encodeIntKey(m, key);
    if (e == CborNoError) e = cbor_encode_uint(m, v);
    return e;
}

CborError encodeByteStringKey(CborEncoder *m, int key, const std::vector<uint8_t> &v) {
    CborError e = encodeIntKey(m, key);
    if (e == CborNoError) e = cbor_encode_byte_string(m, v.data(), v.size());
    return e;
}

// Emit map key+value for every field present, returning the first error.
CborError encodeBody(CborEncoder *m, const MeshEnvelope &env) {
    CborError e = CborNoError;

    // Required (always emitted, in key order).
    if (e == CborNoError) e = encodeBytes16Key(m, kEnvelopeKeyId, env.id);
    if (e == CborNoError) e = encodeStringKey(m, kEnvelopeKeySource, env.source);
    if (e == CborNoError) e = encodeStringKey(m, kEnvelopeKeyType, env.type);
    if (e == CborNoError) e = encodeUintKey(m, kEnvelopeKeyPattern, static_cast<uint64_t>(env.pattern));
    if (e == CborNoError) e = encodeUintKey(m, kEnvelopeKeyDatacontenttype, static_cast<uint64_t>(env.datacontenttype));
    if (e == CborNoError) e = encodeByteStringKey(m, kEnvelopeKeyData, env.data);

    // Optional (omit when absent).
    if (e == CborNoError && env.subject)
        e = encodeStringKey(m, kEnvelopeKeySubject, *env.subject);
    if (e == CborNoError && env.correlation_id)
        e = encodeBytes16Key(m, kEnvelopeKeyCorrelationId, *env.correlation_id);
    if (e == CborNoError && env.reply_to)
        e = encodeStringKey(m, kEnvelopeKeyReplyTo, *env.reply_to);
    if (e == CborNoError && env.invoke_id)
        e = encodeBytes16Key(m, kEnvelopeKeyInvokeId, *env.invoke_id);
    if (e == CborNoError && env.rpc_status)
        e = encodeUintKey(m, kEnvelopeKeyRpcStatus, static_cast<uint64_t>(*env.rpc_status));
    if (e == CborNoError && env.rpc_error_message)
        e = encodeStringKey(m, kEnvelopeKeyRpcErrorMessage, *env.rpc_error_message);
    if (e == CborNoError && env.deadline_unix_ms)
        e = encodeUintKey(m, kEnvelopeKeyDeadlineUnixMs, *env.deadline_unix_ms);

    // qos: not yet on the wire (no transport consumes it). Field is
    // optional; serialization (key 13, nested map) is added together with
    // the first consuming transport in Sessions C/D.

    if (e == CborNoError && env.sequence_no)
        e = encodeUintKey(m, kEnvelopeKeySequenceNo, *env.sequence_no);
    if (e == CborNoError && env.routing_id)
        e = encodeBytes16Key(m, kEnvelopeKeyRoutingId, *env.routing_id);
    if (e == CborNoError && env.parallel_id)
        e = encodeStringKey(m, kEnvelopeKeyParallelId, *env.parallel_id);
    if (e == CborNoError && env.region_id)
        e = encodeStringKey(m, kEnvelopeKeyRegionId, *env.region_id);
    if (e == CborNoError && env.child_session_id)
        e = encodeStringKey(m, kEnvelopeKeyChildSessionId, *env.child_session_id);

    return e;
}

// Count the number of map entries the body will emit.
size_t countEntries(const MeshEnvelope &env) {
    size_t n = 6;  // required fields
    if (env.subject)           ++n;
    if (env.correlation_id)    ++n;
    if (env.reply_to)          ++n;
    if (env.invoke_id)         ++n;
    if (env.rpc_status)        ++n;
    if (env.rpc_error_message) ++n;
    if (env.deadline_unix_ms)  ++n;
    if (env.sequence_no)       ++n;
    if (env.routing_id)        ++n;
    if (env.parallel_id)       ++n;
    if (env.region_id)         ++n;
    if (env.child_session_id)  ++n;
    return n;
}

// Encode envelope into `buf` of size `cap`. Sets `out_used` to bytes written.
// Returns the tinycbor error so the caller can detect OOM and resize.
CborError tryEncode(const MeshEnvelope &env, uint8_t *buf, size_t cap, size_t &out_used,
                    size_t &out_extra) {
    CborEncoder root;
    cbor_encoder_init(&root, buf, cap, 0);

    CborEncoder map;
    CborError e = cbor_encoder_create_map(&root, &map, countEntries(env));
    if (e == CborNoError || e == CborErrorOutOfMemory) {
        // Continue body encoding even on partial OOM — tinycbor tracks
        // extra-bytes-needed across the whole encode and we want the final
        // size estimate to cover the entire envelope, not just the prefix.
        CborError be = encodeBody(&map, env);
        if (e == CborNoError) e = be;
    }
    if (e == CborNoError || e == CborErrorOutOfMemory) {
        CborError ce = cbor_encoder_close_container(&root, &map);
        if (e == CborNoError) e = ce;
    }

    out_used = (e == CborNoError) ? cbor_encoder_get_buffer_size(&root, buf) : 0;
    out_extra = cbor_encoder_get_extra_bytes_needed(&root);
    return e;
}

// ── Decoder helpers ─────────────────────────────────────────────────────

// Max lengths for variable-size fields. Rejects CBOR headers that claim
// unreasonable sizes before any allocation — prevents a crafted envelope
// from triggering multi-GB allocations on the receiver (DoS mitigation).
// kMaxDataLen is the single source `SCE::Mesh::kMaxEnvelopeBytes`
// re-exported from MeshEnvelopeCodec.h so downstream transports
// (custom_tcp frame reader, future TCP-like wires) consume the same
// value without rebinding a literal.
constexpr size_t kMaxDataLen   = ::SCE::Mesh::kMaxEnvelopeBytes;
constexpr size_t kMaxStringLen = 256 * 1024;         // 256 KiB per text field

bool readBytes16(CborValue *it, std::array<uint8_t, 16> &out) {
    if (!cbor_value_is_byte_string(it)) return false;
    size_t len = 0;
    if (cbor_value_get_string_length(it, &len) != CborNoError) return false;
    if (len != 16) return false;
    size_t buflen = 16;
    if (cbor_value_copy_byte_string(it, out.data(), &buflen, it) != CborNoError) return false;
    return buflen == 16;
}

bool readByteString(CborValue *it, std::vector<uint8_t> &out) {
    if (!cbor_value_is_byte_string(it)) return false;
    size_t len = 0;
    if (cbor_value_get_string_length(it, &len) != CborNoError) return false;
    if (len > kMaxDataLen) return false;
    out.resize(len);
    size_t buflen = len;
    if (cbor_value_copy_byte_string(it, out.data(), &buflen, it) != CborNoError) return false;
    return buflen == len;
}

bool readText(CborValue *it, std::string &out) {
    if (!cbor_value_is_text_string(it)) return false;
    size_t len = 0;
    if (cbor_value_get_string_length(it, &len) != CborNoError) return false;
    if (len > kMaxStringLen) return false;
    out.resize(len);
    size_t buflen = len;
    char *dst = out.empty() ? nullptr : out.data();
    if (cbor_value_copy_text_string(it, dst, &buflen, it) != CborNoError) return false;
    return buflen == len;
}

bool readU64(CborValue *it, uint64_t &out) {
    if (!cbor_value_is_unsigned_integer(it)) return false;
    if (cbor_value_get_uint64(it, &out) != CborNoError) return false;
    return cbor_value_advance(it) == CborNoError;
}

// ── Enum range validators ───────────────────────────────────────────────

bool isValidPatternKind(uint64_t v) {
    // 1-9 in-use; 14-20 are the full §mesh-9.6 remote-invoke lifecycle
    // (SCE_MESH.md §mesh-9.6.2): InvokeStart, InvokeStarted, ChildEvent,
    // ParentEvent, InvokeDone, InvokeCancel, InvokeError. 21 is
    // ParallelRegionDone (SCE_MESH.md §mesh-16.5). Values 10-13 remain
    // reserved for Stream wire-layer optimizations.
    return (v >= 1 && v <= 9) || (v >= 14 && v <= 21);
}

bool isValidPayloadCodec(uint64_t v) {
    return v <= 4;
}

bool isValidRpcStatus(uint64_t v) {
    switch (v) {
    case 0: case 1: case 3: case 4: case 5: case 12: case 13: case 14:
        return true;
    default:
        return false;
    }
}

}  // namespace

std::vector<uint8_t> encodeEnvelope(const MeshEnvelope &env) {
    // First pass on a stack buffer covers ~95% of envelopes (small JSON
    // payloads, no large strings). If it overflows, tinycbor reports the
    // exact extra bytes needed and we re-encode in a sized heap buffer.
    constexpr size_t kStackCap = 256;
    uint8_t stack_buf[kStackCap];
    size_t used = 0;
    size_t extra = 0;
    CborError e = tryEncode(env, stack_buf, kStackCap, used, extra);

    if (e == CborNoError) {
        return std::vector<uint8_t>(stack_buf, stack_buf + used);
    }
    if (e != CborErrorOutOfMemory) {
        return {};  // structural failure (e.g. bad string length)
    }

    // Two-pass: heap buffer sized for the exact need. tinycbor reports
    // `extra_bytes_needed` based on whatever the encoder observed before
    // giving up, which is not always the full tail — when a container
    // header and its payload each overflow in turn, the reported delta
    // can under-count. Grow in a bounded loop until the encoder either
    // succeeds or returns a structural error. Cap by `kMaxEncodeSize`
    // so a pathological envelope cannot loop forever.
    constexpr size_t kMaxEncodeSize = 32 * 1024 * 1024;  // 32 MiB safety cap
    size_t heap_cap = kStackCap + extra + kStackCap;
    while (true) {
        if (heap_cap > kMaxEncodeSize) return {};
        std::vector<uint8_t> heap(heap_cap);
        e = tryEncode(env, heap.data(), heap.size(), used, extra);
        if (e == CborNoError) {
            heap.resize(used);
            return heap;
        }
        if (e != CborErrorOutOfMemory) return {};
        // Still OOM — grow by the reported shortfall plus a fresh margin,
        // or double if the shortfall is zero (rare but seen when the
        // encoder bails mid-string).
        size_t growth = extra > 0 ? (extra + kStackCap) : heap_cap;
        heap_cap += growth;
    }
}

bool decodeEnvelope(const uint8_t *raw, std::size_t len, MeshEnvelope &out) {
    if (raw == nullptr || len == 0) return false;

    CborParser parser;
    CborValue root;
    if (cbor_parser_init(raw, len, 0, &parser, &root) != CborNoError) return false;
    if (!cbor_value_is_map(&root)) return false;

    CborValue it;
    if (cbor_value_enter_container(&root, &it) != CborNoError) return false;

    bool seen_id = false, seen_source = false, seen_type = false,
         seen_pattern = false, seen_codec = false, seen_data = false;

    out = MeshEnvelope{};  // reset to defaults so missing optionals stay nullopt

    while (!cbor_value_at_end(&it)) {
        if (!cbor_value_is_integer(&it)) return false;
        int key = 0;
        if (cbor_value_get_int_checked(&it, &key) != CborNoError) return false;
        if (cbor_value_advance(&it) != CborNoError) return false;

        switch (key) {
        case kEnvelopeKeyId:
            if (!readBytes16(&it, out.id)) return false;
            seen_id = true;
            break;
        case kEnvelopeKeySource:
            if (!readText(&it, out.source)) return false;
            seen_source = true;
            break;
        case kEnvelopeKeyType:
            if (!readText(&it, out.type)) return false;
            seen_type = true;
            break;
        case kEnvelopeKeyPattern: {
            uint64_t v = 0;
            if (!readU64(&it, v)) return false;
            if (!isValidPatternKind(v)) return false;
            out.pattern = static_cast<PatternKind>(v);
            seen_pattern = true;
            break;
        }
        case kEnvelopeKeyDatacontenttype: {
            uint64_t v = 0;
            if (!readU64(&it, v)) return false;
            if (!isValidPayloadCodec(v)) return false;
            out.datacontenttype = static_cast<PayloadCodec>(v);
            seen_codec = true;
            break;
        }
        case kEnvelopeKeyData:
            if (!readByteString(&it, out.data)) return false;
            seen_data = true;
            break;
        case kEnvelopeKeySubject: {
            std::string s;
            if (!readText(&it, s)) return false;
            out.subject = std::move(s);
            break;
        }
        case kEnvelopeKeyCorrelationId: {
            std::array<uint8_t, 16> v{};
            if (!readBytes16(&it, v)) return false;
            out.correlation_id = v;
            break;
        }
        case kEnvelopeKeyReplyTo: {
            std::string s;
            if (!readText(&it, s)) return false;
            out.reply_to = std::move(s);
            break;
        }
        case kEnvelopeKeyInvokeId: {
            std::array<uint8_t, 16> v{};
            if (!readBytes16(&it, v)) return false;
            out.invoke_id = v;
            break;
        }
        case kEnvelopeKeyRpcStatus: {
            uint64_t v = 0;
            if (!readU64(&it, v)) return false;
            if (!isValidRpcStatus(v)) return false;
            out.rpc_status = static_cast<RpcStatus>(v);
            break;
        }
        case kEnvelopeKeyRpcErrorMessage: {
            std::string s;
            if (!readText(&it, s)) return false;
            out.rpc_error_message = std::move(s);
            break;
        }
        case kEnvelopeKeyDeadlineUnixMs: {
            uint64_t v = 0;
            if (!readU64(&it, v)) return false;
            out.deadline_unix_ms = v;
            break;
        }
        case kEnvelopeKeySequenceNo: {
            uint64_t v = 0;
            if (!readU64(&it, v)) return false;
            out.sequence_no = v;
            break;
        }
        case kEnvelopeKeyRoutingId: {
            std::array<uint8_t, 16> v{};
            if (!readBytes16(&it, v)) return false;
            out.routing_id = v;
            break;
        }
        case kEnvelopeKeyParallelId: {
            std::string s;
            if (!readText(&it, s)) return false;
            out.parallel_id = std::move(s);
            break;
        }
        case kEnvelopeKeyRegionId: {
            std::string s;
            if (!readText(&it, s)) return false;
            out.region_id = std::move(s);
            break;
        }
        case kEnvelopeKeyChildSessionId: {
            std::string s;
            if (!readText(&it, s)) return false;
            out.child_session_id = std::move(s);
            break;
        }
        default:
            // Unknown integer key — forward-compat: skip the value entirely.
            // cbor_value_advance walks past any well-formed value, including
            // nested maps/arrays, in a single call.
            if (cbor_value_advance(&it) != CborNoError) return false;
            break;
        }
    }

    if (cbor_value_leave_container(&root, &it) != CborNoError) return false;

    return seen_id && seen_source && seen_type && seen_pattern && seen_codec && seen_data;
}

}  // namespace SCE::Mesh
