// SCE-MAP: codec_zenoh_encoding:69 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_ENCODING_H
#define SCE_FORGE_CODEC_ZENOH_ENCODING_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <string>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohEncoding {

/// The elements a value of `CodecZenohEncoding` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_packed_id {
void packed_id() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.packed_id())) -> decltype(s.packed_id()) {
    return s.packed_id();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(packed_id(s))) -> decltype(packed_id(s)) {
    return packed_id(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_packed_id
/// The `packed_id` element of a source.
inline constexpr detail_packed_id::Read packed_id{};
namespace detail_schema {
void schema() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.schema())) -> decltype(s.schema()) {
    return s.schema();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(schema(s))) -> decltype(schema(s)) {
    return schema(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_schema
/// The `schema` element of a source.
inline constexpr detail_schema::Read schema{};
}  // namespace source

/// Whether `S` presents every element of `CodecZenohEncoding`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::packed_id(std::declval<const S&>())), decltype(source::schema(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::packed_id(std::declval<const S&>())), uint32_t>, std::is_convertible<decltype(source::schema(std::declval<const S&>())), std::optional<::SCE::Forge::Text<255>>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint32_t>(source::packed_id(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Text<255>>>(source::schema(std::declval<const S&>())))> {};

/// The encode of `CodecZenohEncoding` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohEncoding: packed_id, schema");
        const auto& _r_schema = source::schema(src);
        const std::optional<::SCE::Forge::Text<255>> _e_schema = _r_schema;
        const uint64_t _d_schema_len = _e_schema.has_value() ? static_cast<uint64_t>(_e_schema->size()) : uint64_t{0};
        std::uint64_t _f_packed_id = static_cast<std::uint64_t>(source::packed_id(src));
        _f_packed_id = (_f_packed_id & ~0x1ull) | static_cast<std::uint64_t>(_e_schema.has_value());
        const uint32_t _c_packed_id = static_cast<uint32_t>(_f_packed_id);
        if (auto _e = w.write_vle_u32(static_cast<std::uint32_t>(_c_packed_id)); _e) return _e;
        if (((static_cast<std::uint64_t>(_c_packed_id) & 0x1ull) != 0)) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_schema_len)); _e) return _e;
        }
        if (_e_schema) {
            if (auto _e = w.write_bytes(reinterpret_cast<const std::uint8_t*>(_e_schema->data()), _e_schema->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecZenohEncoding {
    CodecZenohEncoding() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohEncoding(uint32_t packed_id, std::optional<::SCE::Forge::Text<255>> schema)
        : packed_id_(packed_id), schema_(schema) {}
    /// The `packed_id` element.
    [[nodiscard]] constexpr uint32_t packed_id() const noexcept { return packed_id_; }
    /// The `schema` element, where a gate leaves it in.
    [[nodiscard]] std::optional<::SCE::Forge::Text<255>> schema() const noexcept { if (!schema_) { return std::nullopt; } return ::SCE::Forge::Text<255>(*schema_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohEncoding> decode(::SCE::Forge::SceCursor& cursor) {
        // Streaming cursor decode (SSOT selection: `needs_streaming`).
        // The positional `raw[byte_off]` path is valid only when every
        // field's absolute offset is fixed at codegen time; this branch
        // handles every codec where it is not — present-if-gated fields
        // (runtime presence), VLE / repeat / TLV-chain / embed fields
        // (runtime width), string fields (UTF-8 decode), and a fixed field
        // after a variable-length payload (offset depends on the payload
        // length). Each field reads its own bytes from the cursor and
        // advances past exactly what it consumed. Per-field `is_repeat` /
        // `is_tlv_chain` / `is_embed` route to their dedicated helpers;
        // every other field flows through `present_if_decode_stmt`.
        auto packed_id_opt = cursor.read_vle_u32();
        if (!packed_id_opt.has_value()) return std::nullopt;
        auto packed_id = static_cast<std::uint32_t>(*packed_id_opt);
        std::optional<uint64_t> schema_len;
        if ((packed_id & 0x00000001) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            schema_len = _v;
        }
        std::optional<std::string> schema;
        if ((packed_id & 0x00000001) != 0) {
            const auto _len = schema_len.value();
            if (_len > 255) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            if (!::SCE::Forge::is_valid_utf8(raw, _n)) return std::nullopt;
            schema.emplace(reinterpret_cast<const char*>(raw), _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<::SCE::Forge::Text<255>> _b_schema;
        if (schema) {
            _b_schema = ::SCE::Forge::Text<255>::make(*schema);
            if (!_b_schema) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)schema_len;
        CodecZenohEncoding _decoded{
            /*packed_id=*/packed_id,
            /*schema=*/_b_schema,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_schema() const noexcept {
        return (this->packed_id_ & 0x00000001) != 0;
    }

    void set_has_schema(bool v) noexcept {
        if (v) {
            this->packed_id_ = static_cast<uint32_t>(this->packed_id_ | 0x00000001);
        } else {
            this->packed_id_ = static_cast<uint32_t>(this->packed_id_ & static_cast<uint32_t>(~0x00000001));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 269;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // The encode over any source of the codec, this value one of them,
        // handed the inputs the caller hands this encode.
        return Wire::encode_source(*this, w);
    }

    /// Heap-backed convenience facade. Pre-reserves `MAX_ENCODED_BYTES`
    /// so the worst-case write path performs at most one allocation,
    /// then delegates to `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        _sce_v.reserve(MAX_ENCODED_BYTES);
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }

private:
    uint32_t packed_id_{};
    std::optional<::SCE::Forge::Bounded<std::string, 255>> schema_{};
};

}  // namespace SCE::Generated::CodecZenohEncoding

#endif  // SCE_FORGE_CODEC_ZENOH_ENCODING_H
