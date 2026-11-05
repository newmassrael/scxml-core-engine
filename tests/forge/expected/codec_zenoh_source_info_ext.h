// SCE-MAP: codec_zenoh_source_info_ext:49 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_EXT_H
#define SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_EXT_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_source_info.h"

namespace SCE::Generated::CodecZenohSourceInfoExt {

/// The elements a value of `CodecZenohSourceInfoExt` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_info {
void info() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.info())) -> decltype(s.info()) {
    return s.info();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(info(s))) -> decltype(info(s)) {
    return info(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_info
/// The `info` element of a source.
inline constexpr detail_info::Read info{};
}  // namespace source

/// Whether `S` presents every element of `CodecZenohSourceInfoExt`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::info(std::declval<const S&>()))>>
    : std::conjunction<::SCE::Generated::CodecZenohSourceInfo::is_source<std::decay_t<decltype(source::info(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(source::info(std::declval<const S&>())) && ::SCE::Generated::CodecZenohSourceInfo::is_nothrow_source<std::decay_t<decltype(source::info(std::declval<const S&>()))>>::value> {};

/// The encode of `CodecZenohSourceInfoExt` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohSourceInfoExt: info");
        const auto& _r_info = source::info(src);
        ::SCE::Forge::CountingSink _k_info;
        (void)::SCE::Generated::CodecZenohSourceInfo::Wire::encode_source(_r_info, _k_info);
        const uint64_t _d_ext_size = static_cast<uint64_t>(_k_info.position());
        if (auto _e = w.write_vle_u64(static_cast<std::uint64_t>(_d_ext_size)); _e) return _e;
        if (auto _e = ::SCE::Generated::CodecZenohSourceInfo::Wire::encode_source(_r_info, w); _e) return _e;
        return std::nullopt;
    }
};

struct CodecZenohSourceInfoExt {
    CodecZenohSourceInfoExt() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohSourceInfoExt(::SCE::Generated::CodecZenohSourceInfo::CodecZenohSourceInfo info)
        : info_(std::move(info)) {}
    /// The `info` element, the body it holds.
    [[nodiscard]] const ::SCE::Generated::CodecZenohSourceInfo::CodecZenohSourceInfo& info() const noexcept { return info_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohSourceInfoExt> decode(::SCE::Forge::SceCursor& cursor) {
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
        auto ext_size_opt = cursor.read_vle_u64();
        if (!ext_size_opt.has_value()) return std::nullopt;
        auto ext_size = static_cast<std::uint64_t>(*ext_size_opt);
        ::SCE::Generated::CodecZenohSourceInfo::CodecZenohSourceInfo info;
        {
            std::size_t _len = static_cast<std::size_t>(ext_size);
            const std::uint8_t* _raw = cursor.peek_slice(_len);
            if (_raw == nullptr) return std::nullopt;
            ::SCE::Forge::SceCursor _inner(_raw, _len);
            auto _emb = ::SCE::Generated::CodecZenohSourceInfo::CodecZenohSourceInfo::decode(_inner);
            if (!_emb.has_value()) return std::nullopt;
            if (!cursor.advance(_len)) return std::nullopt;
            info = std::move(*_emb);
        }
        (void)ext_size;
        CodecZenohSourceInfoExt _decoded{
            /*info=*/std::move(info),
        };
        return _decoded;
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 36;

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
    ::SCE::Generated::CodecZenohSourceInfo::CodecZenohSourceInfo info_{};
};

}  // namespace SCE::Generated::CodecZenohSourceInfoExt

#endif  // SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_EXT_H
