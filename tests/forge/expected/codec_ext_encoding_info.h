// SCE-MAP: codec_ext_encoding_info:44 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_EXT_ENCODING_INFO_H
#define SCE_FORGE_CODEC_EXT_ENCODING_INFO_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecExtEncodingInfo {

/// The elements a value of `CodecExtEncodingInfo` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_combined_id {
void combined_id() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.combined_id())) -> decltype(s.combined_id()) {
    return s.combined_id();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(combined_id(s))) -> decltype(combined_id(s)) {
    return combined_id(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_combined_id
/// The `combined_id` element of a source.
inline constexpr detail_combined_id::Read combined_id{};
namespace detail_combined_id_has_schema {
void combined_id_has_schema() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.combined_id_has_schema())) -> decltype(s.combined_id_has_schema()) {
    return s.combined_id_has_schema();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(combined_id_has_schema(s))) -> decltype(combined_id_has_schema(s)) {
    return combined_id_has_schema(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_combined_id_has_schema
/// The `combined_id_has_schema` element of a source.
inline constexpr detail_combined_id_has_schema::Read combined_id_has_schema{};
namespace detail_schema_size {
void schema_size() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.schema_size())) -> decltype(s.schema_size()) {
    return s.schema_size();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(schema_size(s))) -> decltype(schema_size(s)) {
    return schema_size(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_schema_size
/// The `schema_size` element of a source.
inline constexpr detail_schema_size::Read schema_size{};
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

/// The fields `combined_id.has_schema` decides (docs/adr/0065 §6 R4b, R4c) present while `combined_id.has_schema` holds.
class CodecExtEncodingInfoCombinedIdHasSchemaHasSchema {
public:
    CodecExtEncodingInfoCombinedIdHasSchemaHasSchema() = default;
    /// The value of each field, in declaration order.
    CodecExtEncodingInfoCombinedIdHasSchemaHasSchema(::SCE::Forge::Bytes<64> schema)
        : schema_(schema) {}
    /// The `schema` field.
    [[nodiscard]] ::SCE::Forge::Bytes<64> schema() const noexcept { return schema_; }

private:
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 64> schema_{};
};

/// The fields `combined_id.has_schema` decides (docs/adr/0065 §6 R4b, R4c) present while `combined_id.has_schema` does not hold.
class CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema {
public:
    CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema() = default;
    /// The value of each field, in declaration order.
    constexpr CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema(uint8_t schema_size) noexcept
        : schema_size_(schema_size) {}
    /// The `schema_size` field.
    [[nodiscard]] constexpr uint8_t schema_size() const noexcept { return schema_size_; }

private:
    uint8_t schema_size_{};
};

/// The fields `combined_id.has_schema` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm, and its encode derives the gates from which.
using CodecExtEncodingInfoCombinedIdHasSchema = std::variant<CodecExtEncodingInfoCombinedIdHasSchemaHasSchema, CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema>;

/// Whether `G` presents every field of `CodecExtEncodingInfoCombinedIdHasSchemaHasSchema`, each convertible to its type.
template <class G, class = void>
struct is_combined_id_has_schema_has_schema_members : std::false_type {};
template <class G>
struct is_combined_id_has_schema_has_schema_members<G, std::void_t<decltype(source::schema(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::schema(std::declval<const G&>())), ::SCE::Forge::Bytes<64>>, ::SCE::Forge::Detail::Lasting<decltype(source::schema(std::declval<const G&>()))>> {};
/// Whether `G` presents every field of `CodecExtEncodingInfoCombinedIdHasSchemaNotHasSchema`, each convertible to its type.
template <class G, class = void>
struct is_combined_id_has_schema_not_has_schema_members : std::false_type {};
template <class G>
struct is_combined_id_has_schema_not_has_schema_members<G, std::void_t<decltype(source::schema_size(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::schema_size(std::declval<const G&>())), uint8_t>> {};
/// Whether `R` is what a source may present for `combined_id_has_schema`, the fields `combined_id.has_schema`
/// decides: a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_combined_id_has_schema_group : std::false_type {};
template <class G0, class G1>
struct is_combined_id_has_schema_group<std::variant<G0, G1>>
    : std::conjunction<is_combined_id_has_schema_has_schema_members<G0>, is_combined_id_has_schema_not_has_schema_members<G1>> {};

/// Whether `S` presents every element of `CodecExtEncodingInfo`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::combined_id(std::declval<const S&>())), decltype(source::combined_id_has_schema(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::combined_id(std::declval<const S&>())), uint32_t>, is_combined_id_has_schema_group<std::decay_t<decltype(source::combined_id_has_schema(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint32_t>(source::combined_id(std::declval<const S&>()))) && noexcept(source::combined_id_has_schema(std::declval<const S&>())) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<64>>(source::schema(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::combined_id_has_schema(std::declval<const S&>()))>>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::schema_size(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::combined_id_has_schema(std::declval<const S&>()))>>&>())))> {};

/// The encode of `CodecExtEncodingInfo` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecExtEncodingInfo: combined_id, combined_id_has_schema");
        const auto& _r_combined_id_has_schema = source::combined_id_has_schema(src);
        std::optional<uint8_t> _e_schema_size;
        std::optional<::SCE::Forge::Bytes<64>> _e_schema;
        if (const auto* _a = std::get_if<0>(&_r_combined_id_has_schema)) {
            _e_schema = static_cast<::SCE::Forge::Bytes<64>>(source::schema(*_a));
        }
        if (const auto* _a = std::get_if<1>(&_r_combined_id_has_schema)) {
            _e_schema_size = static_cast<uint8_t>(source::schema_size(*_a));
        }
        const uint8_t _d_schema_size = _e_schema.has_value() ? static_cast<uint8_t>(_e_schema->size()) : _e_schema_size.value_or(uint8_t{0});
        std::uint64_t _f_combined_id = static_cast<std::uint64_t>(source::combined_id(src));
        _f_combined_id = (_f_combined_id & ~0x1ull) | static_cast<std::uint64_t>(_e_schema.has_value());
        const uint32_t _c_combined_id = static_cast<uint32_t>(_f_combined_id);
        if (auto _e = w.write_vle_u32(static_cast<std::uint32_t>(_c_combined_id)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_schema_size)); _e) return _e;
        if (_e_schema) {
            if (auto _e = w.write_bytes(_e_schema->data(), _e_schema->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecExtEncodingInfo {
    CodecExtEncodingInfo() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecExtEncodingInfo(uint32_t combined_id, CodecExtEncodingInfoCombinedIdHasSchema combined_id_has_schema)
        : combined_id_(combined_id), combined_id_has_schema_(std::move(combined_id_has_schema)) {}
    /// The `combined_id` element.
    [[nodiscard]] constexpr uint32_t combined_id() const noexcept { return combined_id_; }
    /// The fields `combined_id.has_schema` decides, as the arm the value holds.
    [[nodiscard]] const CodecExtEncodingInfoCombinedIdHasSchema& combined_id_has_schema() const noexcept { return combined_id_has_schema_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecExtEncodingInfo> decode(::SCE::Forge::SceCursor& cursor) {
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
        auto combined_id_opt = cursor.read_vle_u32();
        if (!combined_id_opt.has_value()) return std::nullopt;
        auto combined_id = static_cast<std::uint32_t>(*combined_id_opt);
        uint8_t schema_size;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            schema_size = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<uint8_t>> schema;
        if ((combined_id & 0x00000001) != 0) {
            const auto _len = schema_size;
            if (_len > 64) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            schema.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<::SCE::Forge::Bytes<64>> _b_schema;
        if (schema) {
            _b_schema = ::SCE::Forge::Bytes<64>::make(*schema);
            if (!_b_schema) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)schema_size;
        CodecExtEncodingInfoCombinedIdHasSchema _g_combined_id_has_schema;
        if ((((combined_id >> 0) & 0x1) != 0)) {
            _g_combined_id_has_schema.emplace<0>(*_b_schema);
        } else if ((((combined_id >> 0) & 0x1) == 0)) {
            _g_combined_id_has_schema.emplace<1>(schema_size);
        }
        CodecExtEncodingInfo _decoded{
            /*combined_id=*/combined_id,
            /*combined_id_has_schema=*/std::move(_g_combined_id_has_schema),
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_schema() const noexcept {
        return (this->combined_id_ & 0x00000001) != 0;
    }

    void set_has_schema(bool v) noexcept {
        if (v) {
            this->combined_id_ = static_cast<uint32_t>(this->combined_id_ | 0x00000001);
        } else {
            this->combined_id_ = static_cast<uint32_t>(this->combined_id_ & static_cast<uint32_t>(~0x00000001));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 71;

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
    uint32_t combined_id_{};
    CodecExtEncodingInfoCombinedIdHasSchema combined_id_has_schema_{std::in_place_index_t<1>{}};
};

}  // namespace SCE::Generated::CodecExtEncodingInfo

#endif  // SCE_FORGE_CODEC_EXT_ENCODING_INFO_H
