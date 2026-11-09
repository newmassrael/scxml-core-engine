// SCE-MAP: codec_init_syn_body:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_INIT_SYN_BODY_H
#define SCE_FORGE_CODEC_INIT_SYN_BODY_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecInitSynBody {

/// The elements a value of `CodecInitSynBody` presents, each read by name: the
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
class CodecInitSynBodyS {
public:
    CodecInitSynBodyS() = default;
    /// The value of each field, in declaration order.
    constexpr CodecInitSynBodyS(uint8_t sn_res, uint16_t batch_size) noexcept
        : sn_res_(sn_res), batch_size_(batch_size) {}
    /// The `sn_res` field.
    [[nodiscard]] constexpr uint8_t sn_res() const noexcept { return sn_res_; }
    /// The `batch_size` field.
    [[nodiscard]] constexpr uint16_t batch_size() const noexcept { return batch_size_; }

private:
    uint8_t sn_res_{};
    uint16_t batch_size_{};
};

/// Whether `G` presents every field of `CodecInitSynBodyS`, each convertible to its type.
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

/// Whether `S` presents every element of `CodecInitSynBody`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::version(std::declval<const S&>())), decltype(source::s(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::version(std::declval<const S&>())), uint8_t>, is_s_group<std::decay_t<decltype(source::s(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::version(std::declval<const S&>()))) && noexcept(source::s(std::declval<const S&>())) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::sn_res(std::declval<const typename std::decay_t<decltype(source::s(std::declval<const S&>()))>::value_type&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::batch_size(std::declval<const typename std::decay_t<decltype(source::s(std::declval<const S&>()))>::value_type&>())))> {};

/// The encode of `CodecInitSynBody` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecInitSynBody: version, s");
        const auto& _r_s = source::s(src);
        std::optional<uint8_t> _e_sn_res;
        std::optional<uint16_t> _e_batch_size;
        if (_r_s) {
            _e_sn_res = static_cast<uint8_t>(source::sn_res(*_r_s));
            _e_batch_size = static_cast<uint16_t>(source::batch_size(*_r_s));
        }
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::version(src))); _e) return _e;
        if (_e_sn_res) {
            if (auto _e = w.write_u8(static_cast<std::uint8_t>((*_e_sn_res))); _e) return _e;
        }
        if (_e_batch_size) {
            if (auto _e = w.write_u16_be(static_cast<std::uint16_t>((*_e_batch_size))); _e) return _e;
        }
        return std::nullopt;
    }
};

/// The flag input `S` a source of `CodecInitSynBody` says, which its encode is not
/// handed (docs/adr/0065 §6 R4a).
template <class S>
[[nodiscard]] std::uint8_t input_s(const S& src) noexcept(is_nothrow_source<S>::value) {
    static_assert(is_source<S>::value, "S presents every element of CodecInitSynBody");
    return static_cast<std::uint8_t>(source::s(src).has_value() ? 1 : 0);
}

struct CodecInitSynBody {
    CodecInitSynBody() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    constexpr CodecInitSynBody(uint8_t version, std::optional<CodecInitSynBodyS> s) noexcept
        : version_(version), s_(s) {}
    /// The `version` element.
    [[nodiscard]] constexpr uint8_t version() const noexcept { return version_; }
    /// The fields `S` decides, where its gate leaves them in.
    [[nodiscard]] constexpr const std::optional<CodecInitSynBodyS>& s() const noexcept { return s_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecInitSynBody> decode(::SCE::Forge::SceCursor& cursor, std::uint8_t s) {
        // Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
        // `<sce:flag-input>` so codecs that haven't (yet) consumed an
        // input via `present-if` compile cleanly under -Wunused.
        (void)s;
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
            batch_size = static_cast<uint16_t>(static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]));
            if (!cursor.advance(2)) return std::nullopt;
        }
        std::optional<CodecInitSynBodyS> _g_s;
        if (sn_res.has_value() && batch_size.has_value()) {
            _g_s.emplace(*sn_res, *batch_size);
        }
        CodecInitSynBody _decoded{
            /*version=*/version,
            /*s=*/_g_s,
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 4;

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
    uint8_t version_{};
    std::optional<CodecInitSynBodyS> s_{};
};

}  // namespace SCE::Generated::CodecInitSynBody

#endif  // SCE_FORGE_CODEC_INIT_SYN_BODY_H
