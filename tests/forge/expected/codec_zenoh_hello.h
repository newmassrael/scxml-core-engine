// SCE-MAP: codec_zenoh_hello:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_HELLO_H
#define SCE_FORGE_CODEC_ZENOH_HELLO_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_locator.h"

namespace SCE::Generated::CodecZenohHello {

/// The elements a value of `CodecZenohHello` presents, each read by name: the
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
namespace detail_locators {
void locators() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.locators())) -> decltype(s.locators()) {
    return s.locators();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(locators(s))) -> decltype(locators(s)) {
    return locators(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_locators
/// The `locators` element of a source.
inline constexpr detail_locators::Read locators{};
}  // namespace source

/// Whether `R` is a list a source may present for `locators`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_locators_list : std::false_type {};
template <class R>
struct is_locators_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, ::SCE::Forge::kUnbounded>::value>>
    : ::SCE::Generated::CodecZenohLocator::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is what a source may present for `locators`, a list a gate
/// decides: a `std::optional` of a list `is_locators_list` accepts.
template <class R>
struct is_locators_present_list : std::false_type {};
template <class L>
struct is_locators_present_list<std::optional<L>> : is_locators_list<L> {};

/// Whether `S` presents every element of `CodecZenohHello`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::version(std::declval<const S&>())), decltype(source::cbyte(std::declval<const S&>())), decltype(source::zid(std::declval<const S&>())), decltype(source::locators(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::version(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::cbyte(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::zid(std::declval<const S&>())), ::SCE::Forge::Bytes<16, 1>>, is_locators_present_list<std::decay_t<decltype(source::locators(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::version(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::cbyte(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<16, 1>>(source::zid(std::declval<const S&>()))) && noexcept(source::locators(std::declval<const S&>())) && ::SCE::Generated::CodecZenohLocator::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename std::decay_t<decltype(source::locators(std::declval<const S&>()))>::value_type>>::value> {};

/// The encode of `CodecZenohHello` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohHello: version, cbyte, zid, locators");
        const auto& _r_zid = source::zid(src);
        const ::SCE::Forge::Bytes<16, 1> _e_zid = _r_zid;
        const auto& _r_locators = source::locators(src);
        const std::uint8_t l = static_cast<std::uint8_t>(_r_locators.has_value() ? 1 : 0);
        (void)l;
        const uint64_t _d_num_locators = _r_locators.has_value() ? static_cast<uint64_t>((*_r_locators).size()) : uint64_t{0};
        std::uint64_t _f_cbyte = static_cast<std::uint64_t>(source::cbyte(src));
        _f_cbyte = (_f_cbyte & ~0xF0ull) | (static_cast<std::uint64_t>(_e_zid.size() - 1u) << 4);
        const uint8_t _c_cbyte = static_cast<uint8_t>(_f_cbyte);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::version(src))); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_cbyte)); _e) return _e;
        if (auto _e = w.write_bytes(_e_zid.data(), _e_zid.size()); _e) return _e;
        if (((static_cast<std::uint64_t>(l) & 0x1ull) != 0)) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_num_locators)); _e) return _e;
        }
        if (_r_locators) {
            for (const auto& _x : (*_r_locators)) {
                if (auto _e = ::SCE::Generated::CodecZenohLocator::Wire::encode_source(_x, w); _e) return _e;
            }
        }
        return std::nullopt;
    }
};

/// The flag input `L` a source of `CodecZenohHello` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_l(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohHello");
    return static_cast<std::uint8_t>(static_cast<std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohLocator::CodecZenohLocator, ::SCE::Forge::kUnbounded>>>(source::locators(src)).has_value() ? 1 : 0);
}

struct CodecZenohHello {
    CodecZenohHello() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohHello(uint8_t version, uint8_t cbyte, ::SCE::Forge::Bytes<16, 1> zid, std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohLocator::CodecZenohLocator, ::SCE::Forge::kUnbounded>> locators)
        : version_(version), cbyte_(cbyte), zid_(zid), locators_(locators) {}
    /// The `version` element.
    [[nodiscard]] constexpr uint8_t version() const noexcept { return version_; }
    /// The `cbyte` element.
    [[nodiscard]] constexpr uint8_t cbyte() const noexcept { return cbyte_; }
    /// The `zid` element.
    [[nodiscard]] ::SCE::Forge::Bytes<16, 1> zid() const noexcept { return zid_; }
    /// The `locators` element, the entries it holds where a gate leaves them in.
    [[nodiscard]] std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohLocator::CodecZenohLocator, ::SCE::Forge::kUnbounded>> locators() const noexcept { if (!locators_) { return std::nullopt; } return ::SCE::Forge::List<::SCE::Generated::CodecZenohLocator::CodecZenohLocator, ::SCE::Forge::kUnbounded>(*locators_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohHello> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t l) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)l;
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
        std::vector<uint8_t> zid;
        {
            const auto _len = ((cbyte >> 4) & 0xF);
            std::size_t _n = static_cast<std::size_t>(_len) + 1;
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            zid.assign(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<uint64_t> num_locators;
        if ((l & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            num_locators = _v;
        }
        std::optional<std::vector<::SCE::Generated::CodecZenohLocator::CodecZenohLocator>> locators;
        if ((l & 0x01) != 0) {
            auto _n = num_locators.value();
            std::vector<::SCE::Generated::CodecZenohLocator::CodecZenohLocator> _list;
            _list.reserve(cursor.entries_backed(_n));
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecZenohLocator::CodecZenohLocator::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            locators = std::move(_list);
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohLocator::CodecZenohLocator, ::SCE::Forge::kUnbounded>> _b_locators;
        if (locators) {
            _b_locators = ::SCE::Forge::List<::SCE::Generated::CodecZenohLocator::CodecZenohLocator, ::SCE::Forge::kUnbounded>::make(*locators);
            if (!_b_locators) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        const auto _b_zid = ::SCE::Forge::Bytes<16, 1>::make(zid);
        if (!_b_zid) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        (void)num_locators;
        CodecZenohHello _decoded{
            /*version=*/version,
            /*cbyte=*/cbyte,
            /*zid=*/*_b_zid,
            /*locators=*/_b_locators,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    uint8_t whatami() const noexcept {
        return static_cast<uint8_t>(
            (this->cbyte_ >> 0) & static_cast<uint8_t>(0x03)
        );
    }

    void set_whatami(uint8_t v) noexcept {
        const uint8_t _shifted_mask =
            static_cast<uint8_t>(
                static_cast<uint8_t>(0x03) << 0
            );
        const uint8_t _val =
            static_cast<uint8_t>(
                (static_cast<uint8_t>(v) & static_cast<uint8_t>(0x03)) << 0
            );
        this->cbyte_ = static_cast<uint8_t>(
            (this->cbyte_ & static_cast<uint8_t>(~_shifted_mask)) | _val
        );
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


    /// Encode `self` into the caller-owned sink. Returns
    /// `CodecError::BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable sinks
    /// (e.g. `VectorSink`) are effectively infallible.
    [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {
        // The encode over any source of the codec, this value one of them,
        // handed the inputs the caller hands this encode.
        return Wire::encode_source(*this, w);
    }

    /// Heap-backed convenience facade. It reserves nothing, since the
    /// largest frame passes `u32` (docs/adr/0077 §7), and delegates to
    /// `encode` over a `VectorSink`. Returns the
    /// freshly-encoded byte vector, or `std::nullopt` when `encode`
    /// refuses the value before its first byte: a gated field and the
    /// `sce:present-if` that gates it contradict each other
    /// (`CodecError::PresentIfMismatch`), or a byte string, text or list
    /// lies outside its bound (`CodecError::LengthOutOfRange`,
    /// `CodecError::TooManyElements`). Callers that need the reason, and
    /// zero-alloc hot paths, call `encode` directly against a caller-owned
    /// sink.
    [[nodiscard]] std::optional<std::vector<std::uint8_t>> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        if (encode(_sce_sink)) {
            return std::nullopt;
        }
        return _sce_v;
    }

private:
    uint8_t version_{};
    uint8_t cbyte_{};
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 16, 1> zid_{};
    std::optional<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecZenohLocator::CodecZenohLocator>, ::SCE::Forge::kUnbounded>> locators_{};
};

}  // namespace SCE::Generated::CodecZenohHello

#endif  // SCE_FORGE_CODEC_ZENOH_HELLO_H
