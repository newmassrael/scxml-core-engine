// SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H
#define SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_value_slice.h"

namespace SCE::Generated::CodecZenohValueZbuf {

/// The elements a value of `CodecZenohValueZbuf` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_after_shm {
void after_shm() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.after_shm())) -> decltype(s.after_shm()) {
    return s.after_shm();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(after_shm(s))) -> decltype(after_shm(s)) {
    return after_shm(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_after_shm
/// The `after_shm` element of a source.
inline constexpr detail_after_shm::Read after_shm{};
namespace detail_value_len {
void value_len() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.value_len())) -> decltype(s.value_len()) {
    return s.value_len();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(value_len(s))) -> decltype(value_len(s)) {
    return value_len(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_value_len
/// The `value_len` element of a source.
inline constexpr detail_value_len::Read value_len{};
namespace detail_value {
void value() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.value())) -> decltype(s.value()) {
    return s.value();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(value(s))) -> decltype(value(s)) {
    return value(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_value
/// The `value` element of a source.
inline constexpr detail_value::Read value{};
namespace detail_encoding {
void encoding() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.encoding())) -> decltype(s.encoding()) {
    return s.encoding();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(encoding(s))) -> decltype(encoding(s)) {
    return encoding(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_encoding
/// The `encoding` element of a source.
inline constexpr detail_encoding::Read encoding{};
namespace detail_slices {
void slices() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.slices())) -> decltype(s.slices()) {
    return s.slices();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(slices(s))) -> decltype(slices(s)) {
    return slices(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_slices
/// The `slices` element of a source.
inline constexpr detail_slices::Read slices{};
}  // namespace source

/// The fields `after_shm` decides (docs/adr/0065 §6 R4b, R4c) present while `after_shm` holds.
class CodecZenohValueZbufAfterShmAfterShm {
public:
    CodecZenohValueZbufAfterShmAfterShm() = default;
    /// The value of each field, in declaration order.
    CodecZenohValueZbufAfterShmAfterShm(uint64_t encoding, ::SCE::Forge::List<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice, ::SCE::Forge::kUnbounded> slices, uint64_t value_len)
        : encoding_(encoding), slices_(slices), value_len_(value_len) {}
    /// The `encoding` field.
    [[nodiscard]] constexpr uint64_t encoding() const noexcept { return encoding_; }
    /// The `slices` field.
    [[nodiscard]] ::SCE::Forge::List<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice, ::SCE::Forge::kUnbounded> slices() const noexcept { return slices_; }
    /// The `value_len` field.
    [[nodiscard]] constexpr uint64_t value_len() const noexcept { return value_len_; }

private:
    uint64_t encoding_{};
    ::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice>, ::SCE::Forge::kUnbounded> slices_{};
    uint64_t value_len_{};
};

/// The fields `after_shm` decides (docs/adr/0065 §6 R4b, R4c) present while `after_shm` does not hold.
class CodecZenohValueZbufAfterShmNotAfterShm {
public:
    CodecZenohValueZbufAfterShmNotAfterShm() = default;
    /// The value of each field, in declaration order.
    CodecZenohValueZbufAfterShmNotAfterShm(::SCE::Forge::Bytes<4294967295> value)
        : value_(value) {}
    /// The `value` field.
    [[nodiscard]] ::SCE::Forge::Bytes<4294967295> value() const noexcept { return value_; }

private:
    ::SCE::Forge::Bounded<std::vector<std::uint8_t>, 4294967295> value_{};
};

/// The fields `after_shm` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm, and its encode derives the gates from which.
using CodecZenohValueZbufAfterShm = std::variant<CodecZenohValueZbufAfterShmAfterShm, CodecZenohValueZbufAfterShmNotAfterShm>;

/// Whether `R` is a list a source may present for `slices`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_slices_list : std::false_type {};
template <class R>
struct is_slices_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, ::SCE::Forge::kUnbounded>::value>>
    : ::SCE::Generated::CodecZenohValueSlice::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `G` presents every field of `CodecZenohValueZbufAfterShmAfterShm`, each convertible to its type.
template <class G, class = void>
struct is_after_shm_after_shm_members : std::false_type {};
template <class G>
struct is_after_shm_after_shm_members<G, std::void_t<decltype(source::encoding(std::declval<const G&>())), decltype(source::slices(std::declval<const G&>())), decltype(source::value_len(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::encoding(std::declval<const G&>())), uint64_t>, is_slices_list<std::decay_t<decltype(source::slices(std::declval<const G&>()))>>, std::is_convertible<decltype(source::value_len(std::declval<const G&>())), uint64_t>> {};
/// Whether `G` presents every field of `CodecZenohValueZbufAfterShmNotAfterShm`, each convertible to its type.
template <class G, class = void>
struct is_after_shm_not_after_shm_members : std::false_type {};
template <class G>
struct is_after_shm_not_after_shm_members<G, std::void_t<decltype(source::value(std::declval<const G&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::value(std::declval<const G&>())), ::SCE::Forge::Bytes<4294967295>>, ::SCE::Forge::Detail::Lasting<decltype(source::value(std::declval<const G&>()))>> {};
/// Whether `R` is what a source may present for `after_shm`, the fields `after_shm`
/// decides: a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_after_shm_group : std::false_type {};
template <class G0, class G1>
struct is_after_shm_group<std::variant<G0, G1>>
    : std::conjunction<is_after_shm_after_shm_members<G0>, is_after_shm_not_after_shm_members<G1>> {};

/// Whether `S` presents every element of `CodecZenohValueZbuf`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::after_shm(std::declval<const S&>()))>>
    : std::conjunction<is_after_shm_group<std::decay_t<decltype(source::after_shm(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(source::after_shm(std::declval<const S&>())) && noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::encoding(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::after_shm(std::declval<const S&>()))>>&>()))) && noexcept(std::declval<::SCE::Forge::Detail::Kept<decltype(source::slices(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::after_shm(std::declval<const S&>()))>>&>()))>&>().keep(source::slices(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::after_shm(std::declval<const S&>()))>>&>()))) && ::SCE::Generated::CodecZenohValueSlice::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::slices(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::after_shm(std::declval<const S&>()))>>&>()))>>>::value && noexcept(::SCE::Forge::Detail::converts_to<uint64_t>(source::value_len(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::after_shm(std::declval<const S&>()))>>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<::SCE::Forge::Bytes<4294967295>>(source::value(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::after_shm(std::declval<const S&>()))>>&>())))> {};

/// The encode of `CodecZenohValueZbuf` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohValueZbuf: after_shm");
        const auto& _r_after_shm = source::after_shm(src);
        std::optional<uint64_t> _e_value_len;
        std::optional<::SCE::Forge::Bytes<4294967295>> _e_value;
        std::optional<uint64_t> _e_encoding;
        ::SCE::Forge::Detail::Kept<decltype(source::slices(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(_r_after_shm)>>&>()))> _r_slices;
        if (const auto* _a = std::get_if<0>(&_r_after_shm)) {
            _e_encoding = static_cast<uint64_t>(source::encoding(*_a));
            _r_slices.keep(source::slices(*_a));
            _e_value_len = static_cast<uint64_t>(source::value_len(*_a));
        }
        if (const auto* _a = std::get_if<1>(&_r_after_shm)) {
            _e_value = static_cast<::SCE::Forge::Bytes<4294967295>>(source::value(*_a));
        }
        const std::uint8_t after_shm = static_cast<std::uint8_t>((_r_after_shm.index() == 0u) ? 1 : 0);
        (void)after_shm;
        const uint64_t _d_value_len = _e_value.has_value() ? static_cast<uint64_t>(_e_value->size()) : _e_value_len.value_or(uint64_t{0});
        const uint64_t _d_slice_count = _r_slices.has_value() ? static_cast<uint64_t>((*_r_slices).size()) : uint64_t{0};
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_value_len)); _e) return _e;
        if (_e_value) {
            if (auto _e = w.write_bytes(_e_value->data(), _e_value->size()); _e) return _e;
        }
        if (_e_encoding) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>((*_e_encoding))); _e) return _e;
        }
        if (((static_cast<std::uint64_t>(after_shm) & 0x1ull) != 0)) {
            if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_slice_count)); _e) return _e;
        }
        if (_r_slices) {
            for (const auto& _x : (*_r_slices)) {
                if (auto _e = ::SCE::Generated::CodecZenohValueSlice::Wire::encode_source(_x, w); _e) return _e;
            }
        }
        return std::nullopt;
    }
};

/// The flag input `after_shm` a source of `CodecZenohValueZbuf` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_after_shm(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecZenohValueZbuf");
    return static_cast<std::uint8_t>((source::after_shm(src).index() == 0u) ? 1 : 0);
}

struct CodecZenohValueZbuf {
    CodecZenohValueZbuf() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohValueZbuf(CodecZenohValueZbufAfterShm after_shm)
        : after_shm_(std::move(after_shm)) {}
    /// The fields `after_shm` decides, as the arm the value holds.
    [[nodiscard]] const CodecZenohValueZbufAfterShm& after_shm() const noexcept { return after_shm_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohValueZbuf> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t after_shm) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)after_shm;
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
        auto value_len_opt = cursor.read_vle_u64();
        if (!value_len_opt.has_value()) return std::nullopt;
        auto value_len = static_cast<std::uint64_t>(*value_len_opt);
        std::optional<std::vector<uint8_t>> value;
        if ((after_shm & 0x01) == 0) {
            const auto _len = value_len;
            if (_len > 4294967295ULL) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange); return std::nullopt;
            }
            if (static_cast<std::uint64_t>(_len) > static_cast<std::uint64_t>(SIZE_MAX) - 0u) return std::nullopt;
            std::size_t _n = static_cast<std::size_t>(_len);
            const std::uint8_t* raw = cursor.peek_slice(_n);
            if (raw == nullptr) return std::nullopt;
            value.emplace(raw, raw + _n);
            if (!cursor.advance(_n)) return std::nullopt;
        }
        std::optional<uint64_t> encoding;
        if ((after_shm & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            encoding = _v;
        }
        std::optional<uint64_t> slice_count;
        if ((after_shm & 0x01) != 0) {
            auto _v_opt = cursor.read_vle_u64();
        if (!_v_opt.has_value()) return std::nullopt;
        auto _v = static_cast<std::uint64_t>(*_v_opt);
            slice_count = _v;
        }
        std::optional<std::vector<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice>> slices;
        if ((after_shm & 0x01) != 0) {
            auto _n = slice_count.value();
            std::vector<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice> _list;
            _list.reserve(cursor.entries_backed(_n));
            for (auto _i = decltype(_n){0}; _i < _n; ++_i) {
                auto _elem = ::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice::decode(cursor);
                if (!_elem.has_value()) return std::nullopt;
                _list.push_back(*_elem);
            }
            slices = std::move(_list);
        }
        std::optional<::SCE::Forge::List<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice, ::SCE::Forge::kUnbounded>> _b_slices;
        if (slices) {
            _b_slices = ::SCE::Forge::List<::SCE::Generated::CodecZenohValueSlice::CodecZenohValueSlice, ::SCE::Forge::kUnbounded>::make(*slices);
            if (!_b_slices) {
                cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
                return std::nullopt;
            }
        }
        std::optional<::SCE::Forge::Bytes<4294967295>> _b_value;
        if (value) {
            _b_value = ::SCE::Forge::Bytes<4294967295>::make(*value);
            if (!_b_value) {
                cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);
                return std::nullopt;
            }
        }
        (void)value_len;
        (void)slice_count;
        CodecZenohValueZbufAfterShm _g_after_shm;
        if (((after_shm & 0x01) != 0)) {
            _g_after_shm.emplace<0>(*encoding, *_b_slices, value_len);
        } else if (((after_shm & 0x01) == 0)) {
            _g_after_shm.emplace<1>(*_b_value);
        }
        CodecZenohValueZbuf _decoded{
            /*after_shm=*/std::move(_g_after_shm),
        };
        return _decoded;
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
    CodecZenohValueZbufAfterShm after_shm_{std::in_place_index_t<1>{}};
};

}  // namespace SCE::Generated::CodecZenohValueZbuf

#endif  // SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H
