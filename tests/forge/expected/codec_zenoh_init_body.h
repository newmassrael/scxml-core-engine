// SCE-MAP: codec_zenoh_init_body:43 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_INIT_BODY_H
#define SCE_FORGE_CODEC_ZENOH_INIT_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecZenohInitBody {

/// The elements a value of `CodecZenohInitBody` presents, each read by name: the
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
namespace detail_s {
void s() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.s())) -> decltype(s.s()) {
    return s.s();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(s(s))) -> decltype(s(s)) {
    return s(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_s
/// The `s` element of a source.
inline constexpr detail_s::Read s{};
namespace detail_cookie {
void cookie() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.cookie())) -> decltype(s.cookie()) {
    return s.cookie();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(cookie(s))) -> decltype(cookie(s)) {
    return cookie(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_cookie
/// The `cookie` element of a source.
inline constexpr detail_cookie::Read cookie{};
namespace detail_sn_res {
void sn_res() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.sn_res())) -> decltype(s.sn_res()) {
    return s.sn_res();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(sn_res(s))) -> decltype(sn_res(s)) {
    return sn_res(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_sn_res
/// The `sn_res` element of a source.
inline constexpr detail_sn_res::Read sn_res{};
namespace detail_batch_size {
void batch_size() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.batch_size())) -> decltype(s.batch_size()) {
    return s.batch_size();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(batch_size(s))) -> decltype(batch_size(s)) {
    return batch_size(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_batch_size
/// The `batch_size` element of a source.
inline constexpr detail_batch_size::Read batch_size{};
}  // namespace source

/// The fields `S` decides (docs/adr/0065 §6 R4b), present together while
/// `S` holds: a value holds all of them or none, and its encode derives the
/// gate from which.
class CodecZenohInitBodyS {
public:
    CodecZenohInitBodyS() = default;
    /// The value of each field, in declaration order.
    constexpr CodecZenohInitBodyS(uint8_t sn_res, uint16_t batch_size) noexcept
        : sn_res_(sn_res), batch_size_(batch_size) {}
    /// The `sn_res` field.
    [[nodiscard]] constexpr uint8_t sn_res() const noexcept { return sn_res_; }
    /// The `batch_size` field.
    [[nodiscard]] constexpr uint16_t batch_size() const noexcept { return batch_size_; }

private:
    uint8_t sn_res_{};
    uint16_t batch_size_{};
};

/// Whether `G` presents every field of `CodecZenohInitBodyS`, each convertible to its type.
template <class G, class = void>
struct is_s_members : std::false_type {};
template <class G>
struct is_s_members<G, std::void_t<decltype(source::sn_res(std::declval<const G&>())), decltype(source::batch_size(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::sn_res(std::declval<const G&>())), uint8_t>, std::is_convertible<decltype(source::batch_size(std::declval<const G&>())), uint16_t>> {};
/// Whether `R` is what a source may present for `s`, the fields `S`
/// decides: a `std::optional` of a type `is_s_members` accepts.
template <class R>
struct is_s_group : std::false_type {};
template <class G>
struct is_s_group<std::optional<G>> : is_s_members<G> {};

/// Whether `S` presents every element of `CodecZenohInitBody`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::version(std::declval<const S&>())), decltype(source::cbyte(std::declval<const S&>())), decltype(source::zid(std::declval<const S&>())), decltype(source::s(std::declval<const S&>())), decltype(source::cookie(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::version(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::cbyte(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::zid(std::declval<const S&>())), ::SCE::Forge::Bytes<16, 1>>, is_s_group<std::decay_t<decltype(source::s(std::declval<const S&>()))>>, std::is_convertible<decltype(source::cookie(std::declval<const S&>())), std::optional<::SCE::Forge::Bytes<65535>>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::version(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::cbyte(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<16, 1>>(source::zid(std::declval<const S&>()))) && noexcept(source::s(std::declval<const S&>())) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::sn_res(std::declval<const typename std::decay_t<decltype(source::s(std::declval<const S&>()))>::value_type&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::batch_size(std::declval<const typename std::decay_t<decltype(source::s(std::declval<const S&>()))>::value_type&>()))) && noexcept(::SCE::Forge::Detail::converts_to<std::optional<::SCE::Forge::Bytes<65535>>>(source::cookie(std::declval<const S&>())))> {};

/// The encode of `CodecZenohInitBody` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohInitBody: version, cbyte, zid, s, cookie");
        const auto& _r_zid = source::zid(src);
        const ::SCE::Forge::Bytes<16, 1> _e_zid = _r_zid;
        const auto& _r_cookie = source::cookie(src);
        const std::optional<::SCE::Forge::Bytes<65535>> _e_cookie = _r_cookie;
        const auto& _r_s = source::s(src);
        std::optional<uint8_t> _e_sn_res;
        std::optional<uint16_t> _e_batch_size;
        if (_r_s) {
            _e_sn_res = static_cast<uint8_t>(source::sn_res(*_r_s));
            _e_batch_size = static_cast<uint16_t>(source::batch_size(*_r_s));
        }
        const std::uint8_t a = static_cast<std::uint8_t>(_e_cookie.has_value() ? 1 : 0);
        (void)a;
        const uint64_t _d_cookie_len = _e_cookie.has_value() ? static_cast<uint64_t>(_e_cookie->size()) : uint64_t{0};
        std::uint64_t _f_cbyte = static_cast<std::uint64_t>(source::cbyte(src));
        _f_cbyte = (_f_cbyte & ~0xF0ull) | (static_cast<std::uint64_t>(_e_zid.size() - 1u) << 4);
        const uint8_t _c_cbyte = static_cast<uint8_t>(_f_cbyte);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::version(src))); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_cbyte)); _e) return _e;
        if (auto _e = w.write_bytes(_e_zid.data(), _e_zid.size()); _e) return _e;
        if (_e_sn_res) {
            if (auto _e = w.write_u8(static_cast<std::uint8_t>((*_e_sn_res))); _e) return _e;
        }
        if (_e_batch_size) {
            if (auto _e = w.write_u16_le(static_cast<std::uint16_t>((*_e_batch_size))); _e) return _e;
        }
        if (((static_cast<std::uint64_t>(a) & 0x1ull) != 0)) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_cookie_len)); _e) return _e;
        }
        if (_e_cookie) {
            if (auto _e = w.write_bytes(_e_cookie->data(), _e_cookie->size()); _e) return _e;
        }
        return std::nullopt;
    }
};

/// The flag input `S` a source of `CodecZenohInitBody` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_s(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohInitBody");
    return static_cast<std::uint8_t>(source::s(src).has_value() ? 1 : 0);
}

/// The flag input `A` a source of `CodecZenohInitBody` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_a(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohInitBody");
    return static_cast<std::uint8_t>(static_cast<std::optional<::SCE::Forge::Bytes<65535>>>(source::cookie(src)).has_value() ? 1 : 0);
}

struct CodecZenohInitBody {
    CodecZenohInitBody() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohInitBody(uint8_t version, uint8_t cbyte, ::SCE::Forge::Bytes<16, 1> zid, std::optional<CodecZenohInitBodyS> s, std::optional<::SCE::Forge::Bytes<65535>> cookie)
        : version_(version), cbyte_(cbyte), zid_(zid), s_(s), cookie_(cookie) {}
    /// The `version` element.
    [[nodiscard]] constexpr uint8_t version() const noexcept { return version_; }
    /// The `cbyte` element.
    [[nodiscard]] constexpr uint8_t cbyte() const noexcept { return cbyte_; }
    /// The `zid` element.
    [[nodiscard]] ::SCE::Forge::Bytes<16, 1> zid() const noexcept { return zid_; }
    /// The fields `S` decides, where its gate leaves them in.
    [[nodiscard]] constexpr const std::optional<CodecZenohInitBodyS>& s() const noexcept { return s_; }
    /// The `cookie` element, where a gate leaves it in.
    [[nodiscard]] std::optional<::SCE::Forge::Bytes<65535>> cookie() const noexcept { if (!cookie_) { return std::nullopt; } return ::SCE::Forge::Bytes<65535>(*cookie_); }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohInitBody> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t s, std::uint8_t a) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)s;
        (void)a;
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
        std::optional<uint8_t> sn_res;
        if ((s & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            sn_res = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> batch_size;
        if ((s & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            batch_size = static_cast<uint16_t>(static_cast<uint16_t>(raw[0] | (static_cast<uint16_t>(raw[1]) << 8)));
            if (!cursor.advance(2)) return std::nullopt;
        }
        std::optional<uint64_t> cookie_len;
        if ((a & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            cookie_len = _v;
        }
        std::optional<std::vector<uint8_t>> cookie;
        if ((a & 0x01) != 0) {
            const auto _len = cookie_len.value();
            if (_len > 65535) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            cookie.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        const auto _b_zid = ::SCE::Forge::Bytes<16, 1>::make(zid);
        if (!_b_zid) {
            cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
            return std::nullopt;
        }
        std::optional<::SCE::Forge::Bytes<65535>> _b_cookie;
        if (cookie) {
            _b_cookie = ::SCE::Forge::Bytes<65535>::make(*cookie);
            if (!_b_cookie) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)cookie_len;
        std::optional<CodecZenohInitBodyS> _g_s;
        if (sn_res.has_value() && batch_size.has_value()) {
            _g_s.emplace(*sn_res, *batch_size);
        }
        CodecZenohInitBody _decoded{
            /*version=*/version,
            /*cbyte=*/cbyte,
            /*zid=*/*_b_zid,
            /*s=*/_g_s,
            /*cookie=*/_b_cookie,
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

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 65566;

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
        _sce_v.reserve(MAX_ENCODED_BYTES);
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
    std::optional<CodecZenohInitBodyS> s_{};
    std::optional<::SCE::Forge::Bounded<std::vector<std::uint8_t>, 65535>> cookie_{};
};

}  // namespace SCE::Generated::CodecZenohInitBody

#endif  // SCE_FORGE_CODEC_ZENOH_INIT_BODY_H
