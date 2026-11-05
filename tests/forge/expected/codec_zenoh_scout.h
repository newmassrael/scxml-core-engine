// SCE-MAP: codec_zenoh_scout:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_SCOUT_H
#define SCE_FORGE_CODEC_ZENOH_SCOUT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohScout {

/// The elements a value of `CodecZenohScout` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_version {
void version() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.version())) -> decltype(s.version()) {
    return s.version();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(version(s))) -> decltype(version(s)) {
    return version(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_version
/// The `version` element of a source.
inline constexpr detail_version::Read version{};
namespace detail_cbyte {
void cbyte() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.cbyte())) -> decltype(s.cbyte()) {
    return s.cbyte();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(cbyte(s))) -> decltype(cbyte(s)) {
    return cbyte(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_cbyte
/// The `cbyte` element of a source.
inline constexpr detail_cbyte::Read cbyte{};
namespace detail_zid {
void zid() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.zid())) -> decltype(s.zid()) {
    return s.zid();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(zid(s))) -> decltype(zid(s)) {
    return zid(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_zid
/// The `zid` element of a source.
inline constexpr detail_zid::Read zid{};
}  // namespace source

/// Whether `S` presents every element of `CodecZenohScout`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::version(std::declval<const S&>())), decltype(source::cbyte(std::declval<const S&>())), decltype(source::zid(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::version(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::cbyte(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::zid(std::declval<const S&>())), std::optional<::SCE::Forge::Bytes<16, 1>>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::version(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::cbyte(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Bytes<16, 1>>>(source::zid(std::declval<const S&>())))> {};

/// The encode of `CodecZenohScout` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohScout: version, cbyte, zid");
        const auto& _r_zid = source::zid(src);
        const std::optional<::SCE::Forge::Bytes<16, 1>> _e_zid = _r_zid;
        std::uint64_t _f_cbyte = static_cast<std::uint64_t>(source::cbyte(src));
        _f_cbyte = (_f_cbyte & ~0x8ull) | (static_cast<std::uint64_t>(_e_zid.has_value()) << 3);
        if (_e_zid.has_value()) {
            _f_cbyte = (_f_cbyte & ~0xF0ull) | (static_cast<std::uint64_t>(_e_zid->size() - 1u) << 4);
        }
        const uint8_t _c_cbyte = static_cast<uint8_t>(_f_cbyte);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::version(src))); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_cbyte)); _e) return _e;
        if (_e_zid) {
            if (auto _e = w.write_bytes(_e_zid->data(), _e_zid->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecZenohScout {
    CodecZenohScout() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohScout(uint8_t version, uint8_t cbyte, std::optional<::SCE::Forge::Bytes<16, 1>> zid)
        : version_(version), cbyte_(cbyte), zid_(zid) {}
    /// The `version` element.
    [[nodiscard]] constexpr uint8_t version() const noexcept { return version_; }
    /// The `cbyte` element.
    [[nodiscard]] constexpr uint8_t cbyte() const noexcept { return cbyte_; }
    /// The `zid` element, where a gate leaves it in.
    [[nodiscard]] std::optional<::SCE::Forge::Bytes<16, 1>> zid() const noexcept { if (!zid_) { return std::nullopt; } return ::SCE::Forge::Bytes<16, 1>(*zid_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohScout> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t version;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            version = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        uint8_t cbyte;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            cbyte = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<std::vector<uint8_t>> zid;
        if ((cbyte & 0x08) != 0) {
            const auto _len = ((cbyte >> 4) & 0xF);
            std::size_t _n = static_cast<std::size_t>(_len) + 1;
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            zid.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<::SCE::Forge::Bytes<16, 1>> _b_zid;
        if (zid) {
            _b_zid = ::SCE::Forge::Bytes<16, 1>::make(*zid);
            if (!_b_zid) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        CodecZenohScout _decoded{
            /*version=*/version,
            /*cbyte=*/cbyte,
            /*zid=*/_b_zid,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t what() const noexcept {
        return static_cast<uint8_t>(
            (this->cbyte_ >> 0) & static_cast<uint8_t>(0x07)
        );
    }

    void set_what(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x07) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x07)) << 0
            );
        this->cbyte_ = static_cast<uint8_t>(
            (this->cbyte_ & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    bool i() const noexcept {
        return (this->cbyte_ & 0x08) != 0;
    }

    void set_i(bool v) noexcept {
        if (v) {
            this->cbyte_ = static_cast<uint8_t>(this->cbyte_ | 0x08);
        } else {
            this->cbyte_ = static_cast<uint8_t>(this->cbyte_ & static_cast<uint8_t>(~0x08));
        }
    }

    uint8_t zid_len_m1() const noexcept {
        return static_cast<uint8_t>(
            (this->cbyte_ >> 4) & static_cast<uint8_t>(0x0F)
        );
    }

    void set_zid_len_m1(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x0F) << 4
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x0F)) << 4
            );
        this->cbyte_ = static_cast<uint8_t>(
            (this->cbyte_ & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 18;

    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // The encode over any source of the codec, this value one of them.
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
    uint8_t version_{};
    uint8_t cbyte_{};
    std::optional<::SCE::Forge::Bounded<std::vector<std::uint8_t>, 16, 1>> zid_{};
};

}  // namespace SCE::Generated::CodecZenohScout

#endif  // SCE_FORGE_CODEC_ZENOH_SCOUT_H
