// SCE-MAP: codec_present_if_length_ref:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H
#define SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecPresentIfLengthRef {

/// The elements a value of `CodecPresentIfLengthRef` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_flags {
void flags() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.flags())) -> decltype(s.flags()) {
    return s.flags();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(flags(s))) -> decltype(flags(s)) {
    return flags(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_flags
/// The `flags` element of a source.
inline constexpr detail_flags::Read flags{};
namespace detail_flags_has_payload {
void flags_has_payload() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.flags_has_payload())) -> decltype(s.flags_has_payload()) {
    return s.flags_has_payload();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(flags_has_payload(s))) -> decltype(flags_has_payload(s)) {
    return flags_has_payload(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_flags_has_payload
/// The `flags_has_payload` element of a source.
inline constexpr detail_flags_has_payload::Read flags_has_payload{};
namespace detail_payload_size {
void payload_size() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.payload_size())) -> decltype(s.payload_size()) {
    return s.payload_size();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(payload_size(s))) -> decltype(payload_size(s)) {
    return payload_size(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_payload_size
/// The `payload_size` element of a source.
inline constexpr detail_payload_size::Read payload_size{};
namespace detail_payload {
void payload() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.payload())) -> decltype(s.payload()) {
    return s.payload();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(payload(s))) -> decltype(payload(s)) {
    return payload(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_payload
/// The `payload` element of a source.
inline constexpr detail_payload::Read payload{};
}  // namespace source

/// The fields `flags.has_payload` decides (docs/adr/0065 §6 R4b, R4c) present while `flags.has_payload` holds.
class CodecPresentIfLengthRefFlagsHasPayloadHasPayload {
public:
    CodecPresentIfLengthRefFlagsHasPayloadHasPayload() = default;
    /// The value of each field, in declaration order.
    CodecPresentIfLengthRefFlagsHasPayloadHasPayload(::SCE::Forge::Bytes<32> payload)
        : payload_(payload) {}
    /// The `payload` field.
    [[nodiscard]] ::SCE::Forge::Bytes<32> payload() const noexcept { return payload_; }

private:
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 32> payload_{};
};

/// The fields `flags.has_payload` decides (docs/adr/0065 §6 R4b, R4c) present while `flags.has_payload` does not hold.
class CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload {
public:
    CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload() = default;
    /// The value of each field, in declaration order.
    constexpr CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload(uint8_t payload_size) noexcept
        : payload_size_(payload_size) {}
    /// The `payload_size` field.
    [[nodiscard]] constexpr uint8_t payload_size() const noexcept { return payload_size_; }

private:
    uint8_t payload_size_{};
};

/// The fields `flags.has_payload` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm, and its encode derives the gates from which.
using CodecPresentIfLengthRefFlagsHasPayload = std::variant<CodecPresentIfLengthRefFlagsHasPayloadHasPayload, CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload>;

/// Whether `G` presents every field of `CodecPresentIfLengthRefFlagsHasPayloadHasPayload`, each convertible to its type.
template <class G, class = void>
struct is_flags_has_payload_has_payload_members : std::false_type {};
template <class G>
struct is_flags_has_payload_has_payload_members<G, std::void_t<decltype(source::payload(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::payload(std::declval<const G&>())), ::SCE::Forge::Bytes<32>>, ::SCE::Forge::Detail::Lasting<decltype(source::payload(std::declval<const G&>()))>> {};
/// Whether `G` presents every field of `CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload`, each convertible to its type.
template <class G, class = void>
struct is_flags_has_payload_not_has_payload_members : std::false_type {};
template <class G>
struct is_flags_has_payload_not_has_payload_members<G, std::void_t<decltype(source::payload_size(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::payload_size(std::declval<const G&>())), uint8_t>> {};
/// Whether `R` is what a source may present for `flags_has_payload`, the fields `flags.has_payload`
/// decides: a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_flags_has_payload_group : std::false_type {};
template <class G0, class G1>
struct is_flags_has_payload_group<std::variant<G0, G1>>
    : std::conjunction<is_flags_has_payload_has_payload_members<G0>, is_flags_has_payload_not_has_payload_members<G1>> {};

/// Whether `S` presents every element of `CodecPresentIfLengthRef`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::flags(std::declval<const S&>())), decltype(source::flags_has_payload(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::flags(std::declval<const S&>())), uint8_t>, is_flags_has_payload_group<std::decay_t<decltype(source::flags_has_payload(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::flags(std::declval<const S&>()))) && noexcept(source::flags_has_payload(std::declval<const S&>())) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<32>>(source::payload(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::flags_has_payload(std::declval<const S&>()))>>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::payload_size(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::flags_has_payload(std::declval<const S&>()))>>&>())))> {};

/// The encode of `CodecPresentIfLengthRef` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecPresentIfLengthRef: flags, flags_has_payload");
        const auto& _r_flags_has_payload = source::flags_has_payload(src);
        std::optional<uint8_t> _e_payload_size;
        std::optional<::SCE::Forge::Bytes<32>> _e_payload;
        if (const auto* _a = std::get_if<0>(&_r_flags_has_payload)) {
            _e_payload = static_cast<::SCE::Forge::Bytes<32>>(source::payload(*_a));
        }
        if (const auto* _a = std::get_if<1>(&_r_flags_has_payload)) {
            _e_payload_size = static_cast<uint8_t>(source::payload_size(*_a));
        }
        const uint8_t _d_payload_size = _e_payload.has_value() ? static_cast<uint8_t>(_e_payload->size()) : _e_payload_size.value_or(uint8_t{0});
        std::uint64_t _f_flags = static_cast<std::uint64_t>(source::flags(src));
        _f_flags = (_f_flags & ~0x1ull) | static_cast<std::uint64_t>(_e_payload.has_value());
        const uint8_t _c_flags = static_cast<uint8_t>(_f_flags);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_flags)); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_d_payload_size)); _e) return _e;
        if (_e_payload) {
            if (auto _e = w.write_bytes(_e_payload->data(), _e_payload->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecPresentIfLengthRef {
    CodecPresentIfLengthRef() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecPresentIfLengthRef(uint8_t flags, CodecPresentIfLengthRefFlagsHasPayload flags_has_payload)
        : flags_(flags), flags_has_payload_(std::move(flags_has_payload)) {}
    /// The `flags` element.
    [[nodiscard]] constexpr uint8_t flags() const noexcept { return flags_; }
    /// The fields `flags.has_payload` decides, as the arm the value holds.
    [[nodiscard]] const CodecPresentIfLengthRefFlagsHasPayload& flags_has_payload() const noexcept { return flags_has_payload_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecPresentIfLengthRef> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t flags;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            flags = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        uint8_t payload_size;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            payload_size = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<uint8_t>> payload;
        if ((flags & 0x01) != 0) {
            const auto _len = payload_size;
            if (_len > 32) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            payload.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<::SCE::Forge::Bytes<32>> _b_payload;
        if (payload) {
            _b_payload = ::SCE::Forge::Bytes<32>::make(*payload);
            if (!_b_payload) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)payload_size;
        CodecPresentIfLengthRefFlagsHasPayload _g_flags_has_payload;
        if ((((flags >> 0) & 0x1) != 0)) {
            _g_flags_has_payload.emplace<0>(*_b_payload);
        } else if ((((flags >> 0) & 0x1) == 0)) {
            _g_flags_has_payload.emplace<1>(payload_size);
        }
        CodecPresentIfLengthRef _decoded{
            /*flags=*/flags,
            /*flags_has_payload=*/std::move(_g_flags_has_payload),
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool has_payload() const noexcept {
        return (this->flags_ & 0x01) != 0;
    }

    void set_has_payload(bool v) noexcept {
        if (v) {
            this->flags_ = static_cast<uint8_t>(this->flags_ | 0x01);
        } else {
            this->flags_ = static_cast<uint8_t>(this->flags_ & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 34;

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
    uint8_t flags_{};
    CodecPresentIfLengthRefFlagsHasPayload flags_has_payload_{std::in_place_index_t<1>{}};
};

}  // namespace SCE::Generated::CodecPresentIfLengthRef

#endif  // SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H
