// SCE-MAP: codec_zenoh_query_value:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H
#define SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_zenoh_value_entry.h"

namespace SCE::Generated::CodecZenohQueryValue {

/// The elements a value of `CodecZenohQueryValue` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_exts {
void exts() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.exts())) -> decltype(s.exts()) {
    return s.exts();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(exts(s))) -> decltype(exts(s)) {
    return exts(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_exts
/// The `exts` element of a source.
inline constexpr detail_exts::Read exts{};
}  // namespace source

/// The input `after_shm` each entry of `exts` says, which the list hands every
/// entry as whether the entry before has the identifier 0x4 and holds to its law (docs/adr/0065 §6 R8).
struct CodecZenohQueryValueExtsAfterShm {
    template <class E>
    [[nodiscard]] static std::optional<bool> says(const E& entry) {
        const std::optional<std::uint8_t> said = ::SCE::Generated::CodecZenohValueEntry::input_after_shm(entry);
        return said.has_value() ? std::optional<bool>(*said != 0) : std::nullopt;
    }
};

/// The kind of entry after which `exts` supplies its input set: an entry
/// whose identifier is 0x4 (docs/adr/0065 §6 R8b).
struct CodecZenohQueryValueExtsHas0x4Key {
    template <class E>
    [[nodiscard]] static bool matches(const E& entry) {
        return ((static_cast<std::uint64_t>(::SCE::Generated::CodecZenohValueEntry::source::header(entry)) >> 0) & 0xFu) == 0x4u;
    }
};

/// The law the entries of `exts` follow in the inputs they say, which the
/// list holds them to (docs/adr/0065 §6 R8).
using CodecZenohQueryValueExtsLaw = ::SCE::Forge::AfterKey<CodecZenohQueryValueExtsAfterShm, CodecZenohQueryValueExtsHas0x4Key>;

/// Whether `R` is a list a source may present for `exts`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_exts_bounded : std::false_type {};
template <class R>
struct is_exts_bounded<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, ::SCE::Forge::kUnbounded, 1>::value>>
    : ::SCE::Generated::CodecZenohValueEntry::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is a list a source may present for `exts`: a list
/// `is_exts_bounded` accepts, held to `CodecZenohQueryValueExtsLaw` (docs/adr/0065 §6 R8).
template <class R>
struct is_exts_list : std::false_type {};
template <class L>
struct is_exts_list<::SCE::Forge::Lawful<L, CodecZenohQueryValueExtsLaw>> : is_exts_bounded<L> {};

/// Whether `S` presents every element of `CodecZenohQueryValue`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::exts(std::declval<const S&>()))>>
    : std::conjunction<is_exts_list<std::decay_t<decltype(source::exts(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(source::exts(std::declval<const S&>())) && ::SCE::Generated::CodecZenohValueEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::exts(std::declval<const S&>()))>>>::value> {};

/// The encode of `CodecZenohQueryValue` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecZenohQueryValue: exts");
        const auto& _r_exts = source::exts(src);
        {
            std::size_t _left_exts = _r_exts.size();
            for (const auto& _x : _r_exts) {
                --_left_exts;
                ::SCE::Generated::CodecZenohValueEntry::Overrides _o_exts{};
                _o_exts.header_z = _left_exts != 0;
                if (auto _e = ::SCE::Generated::CodecZenohValueEntry::Wire::encode_source(_x, w, _o_exts); _e) return _e;
            }
        }
        return std::nullopt;
    }
};

struct CodecZenohQueryValue {
    CodecZenohQueryValue() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecZenohQueryValue(::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry, ::SCE::Forge::kUnbounded, 1>, CodecZenohQueryValueExtsLaw> exts)
        : exts_(exts) {}
    /// The `exts` element, the entries it holds.
    [[nodiscard]] ::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry, ::SCE::Forge::kUnbounded, 1>, CodecZenohQueryValueExtsLaw> exts() const noexcept { return exts_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecZenohQueryValue> decode(::SCE::Forge::SceCursor& cursor) {
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
        std::vector<::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry> exts;
        std::uint8_t _prev_exts_after_shm = 0;
        bool _more = false;
        for (;;) {
            auto _elem = ::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry::decode(cursor, _prev_exts_after_shm);
            if (!_elem.has_value()) return std::nullopt;
            _more = _elem->z();
            _prev_exts_after_shm = static_cast<std::uint8_t>(static_cast<std::uint64_t>((*_elem).ext_id()) == 4ULL);
            exts.push_back(*_elem);
            if (!_more) break;
        }
        if (_more && cursor.remaining() == 0) { return std::nullopt; }
        const auto _l_exts = ::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry, ::SCE::Forge::kUnbounded, 1>, CodecZenohQueryValueExtsLaw>::list_type::make(exts);
        if (!_l_exts) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
            return std::nullopt;
        }
        const auto _b_exts = ::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry, ::SCE::Forge::kUnbounded, 1>, CodecZenohQueryValueExtsLaw>::make(::SCE::Forge::FixedStart{}, *_l_exts);
        if (!_b_exts) {
            cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
            return std::nullopt;
        }
        CodecZenohQueryValue _decoded{
            /*exts=*/*_b_exts,
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
    /// freshly-encoded byte vector. Callers targeting zero-alloc hot
    /// paths should call `encode` directly against a caller-owned sink.
    [[nodiscard]] std::vector<std::uint8_t> encode_to_vec() const {
        std::vector<std::uint8_t> _sce_v;
        ::SCE::Forge::VectorSink _sce_sink(_sce_v);
        (void)encode(_sce_sink);
        return _sce_v;
    }

private:
    ::SCE::Forge::Lawful<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecZenohValueEntry::CodecZenohValueEntry>, ::SCE::Forge::kUnbounded, 1>, CodecZenohQueryValueExtsLaw> exts_{};
};

}  // namespace SCE::Generated::CodecZenohQueryValue

#endif  // SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H
