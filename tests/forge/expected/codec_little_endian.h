// SCE-MAP: codec_little_endian:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_LITTLE_ENDIAN_H
#define SCE_FORGE_CODEC_LITTLE_ENDIAN_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"

namespace SCE::Generated::CodecLittleEndian {

/// The elements a value of `CodecLittleEndian` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_sensorId {
void sensorId() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.sensorId())) -> decltype(s.sensorId()) {
    return s.sensorId();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(sensorId(s))) -> decltype(sensorId(s)) {
    return sensorId(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_sensorId
/// The `sensorId` element of a source.
inline constexpr detail_sensorId::Read sensorId{};
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
namespace detail_status {
void status() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.status())) -> decltype(s.status()) {
    return s.status();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(status(s))) -> decltype(status(s)) {
    return status(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_status
/// The `status` element of a source.
inline constexpr detail_status::Read status{};
}  // namespace source

/// Whether `S` presents every element of `CodecLittleEndian`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::sensorId(std::declval<const S&>())), decltype(source::value(std::declval<const S&>())), decltype(source::status(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::sensorId(std::declval<const S&>())), uint8_t>, std::is_convertible<decltype(source::value(std::declval<const S&>())), uint16_t>, std::is_convertible<decltype(source::status(std::declval<const S&>())), uint8_t>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::sensorId(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::value(std::declval<const S&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::status(std::declval<const S&>())))> {};

/// The encode of `CodecLittleEndian` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecLittleEndian: sensorId, value, status");
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::sensorId(src))); _e) return _e;
        if (auto _e = w.write_u16_le(static_cast<std::uint16_t>(source::value(src))); _e) return _e;
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(source::status(src))); _e) return _e;
        return std::nullopt;
    }
};

struct CodecLittleEndian {
    CodecLittleEndian() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    constexpr CodecLittleEndian(uint8_t sensorId, uint16_t value, uint8_t status) noexcept
        : sensorId_(sensorId), value_(value), status_(status) {}
    /// The `sensorId` element.
    [[nodiscard]] constexpr uint8_t sensorId() const noexcept { return sensorId_; }
    /// The `value` element.
    [[nodiscard]] constexpr uint16_t value() const noexcept { return value_; }
    /// The `status` element.
    [[nodiscard]] constexpr uint8_t status() const noexcept { return status_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecLittleEndian> decode(::SCE::Forge::SceCursor& cursor) {
        const std::uint8_t* raw = cursor.peek_slice(4);
        if (raw == nullptr) return std::nullopt;
        uint8_t sensorId = raw[0];
        uint16_t value = static_cast<uint16_t>(raw[1] | (static_cast<uint16_t>(raw[2]) << 8));
        uint8_t status = raw[3];
        CodecLittleEndian _decoded{
            /*sensorId=*/sensorId,
            /*value=*/value,
            /*status=*/status,
        };
        if (!cursor.advance(4)) return std::nullopt;
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
    uint8_t sensorId_{};
    uint16_t value_{};
    uint8_t status_{};
};

}  // namespace SCE::Generated::CodecLittleEndian

#endif  // SCE_FORGE_CODEC_LITTLE_ENDIAN_H
