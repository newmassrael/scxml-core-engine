// SCE-MAP: codec_chain_prev_carrier:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_PREV_CARRIER_H
#define SCE_FORGE_CODEC_CHAIN_PREV_CARRIER_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_chain_prev_entry.h"

namespace SCE::Generated::CodecChainPrevCarrier {

/// The elements a value of `CodecChainPrevCarrier` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_head {
void head() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.head())) -> decltype(s.head()) {
    return s.head();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(head(s))) -> decltype(head(s)) {
    return head(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_head
/// The `head` element of a source.
inline constexpr detail_head::Read head{};
namespace detail_entries {
void entries() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.entries())) -> decltype(s.entries()) {
    return s.entries();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(entries(s))) -> decltype(entries(s)) {
    return entries(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_entries
/// The `entries` element of a source.
inline constexpr detail_entries::Read entries{};
}  // namespace source

/// The input `tagged` each entry of `entries` says, which the list hands every
/// entry as `head.T` and holds to its law (docs/adr/0065 §6 R8).
struct CodecChainPrevCarrierEntriesTagged {
    template <class E>
    [[nodiscard]] static std::optional<bool> says(const E& entry) {
        return ::SCE::Generated::CodecChainPrevEntry::input_tagged(entry) != 0;
    }
};

/// The input `after_marker` each entry of `entries` says, which the list hands every
/// entry as whether the entry before has the identifier 0x4 and holds to its law (docs/adr/0065 §6 R8).
struct CodecChainPrevCarrierEntriesAfterMarker {
    template <class E>
    [[nodiscard]] static std::optional<bool> says(const E& entry) {
        return ::SCE::Generated::CodecChainPrevEntry::input_after_marker(entry) != 0;
    }
};

/// The kind of entry after which `entries` supplies its input set: an entry
/// whose identifier is 0x4 (docs/adr/0065 §6 R8b).
struct CodecChainPrevCarrierEntriesHas0x4Key {
    template <class E>
    [[nodiscard]] static bool matches(const E& entry) {
        return ((static_cast<std::uint64_t>(::SCE::Generated::CodecChainPrevEntry::source::header(entry)) >> 0) & 0xFu) == 0x4u;
    }
};

/// The law the entries of `entries` follow in the inputs they say, which the
/// list holds them to (docs/adr/0065 §6 R8).
using CodecChainPrevCarrierEntriesLaw = ::SCE::Forge::Laws<::SCE::Forge::Uniform<CodecChainPrevCarrierEntriesTagged>, ::SCE::Forge::AfterKey<CodecChainPrevCarrierEntriesAfterMarker, CodecChainPrevCarrierEntriesHas0x4Key>>;

/// Whether `R` is a list a source may present for `entries`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_entries_bounded : std::false_type {};
template <class R>
struct is_entries_bounded<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 4, 1>::value>>
    : ::SCE::Generated::CodecChainPrevEntry::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is a list a source may present for `entries`: a list
/// `is_entries_bounded` accepts, held to `CodecChainPrevCarrierEntriesLaw` (docs/adr/0065 §6 R8).
template <class R>
struct is_entries_list : std::false_type {};
template <class L>
struct is_entries_list<::SCE::Forge::Lawful<L, CodecChainPrevCarrierEntriesLaw>> : is_entries_bounded<L> {};

/// Whether `S` presents every element of `CodecChainPrevCarrier`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::head(std::declval<const S&>())), decltype(source::entries(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::head(std::declval<const S&>())), uint8_t>, is_entries_list<std::decay_t<decltype(source::entries(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::head(std::declval<const S&>()))) && noexcept(source::entries(std::declval<const S&>())) && ::SCE::Generated::CodecChainPrevEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<std::decay_t<decltype(source::entries(std::declval<const S&>()))>>>::value> {};

/// The encode of `CodecChainPrevCarrier` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecChainPrevCarrier: head, entries");
        const auto& _r_entries = source::entries(src);
        std::uint64_t _f_head = static_cast<std::uint64_t>(source::head(src));
        _f_head = (_f_head & ~0x1ull) | static_cast<std::uint64_t>(static_cast<std::size_t>(std::get<0>(_r_entries.start())));
        const uint8_t _c_head = static_cast<uint8_t>(_f_head);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_head)); _e) return _e;
        {
            std::size_t _left_entries = _r_entries.size();
            for (const auto& _x : _r_entries) {
                --_left_entries;
                ::SCE::Generated::CodecChainPrevEntry::Overrides _o_entries{};
                _o_entries.header_z = _left_entries != 0;
                if (auto _e = ::SCE::Generated::CodecChainPrevEntry::Wire::encode_source(_x, w, _o_entries); _e) return _e;
            }
        }
        return std::nullopt;
    }
};

struct CodecChainPrevCarrier {
    CodecChainPrevCarrier() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecChainPrevCarrier(uint8_t head, ::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry, 4, 1>, CodecChainPrevCarrierEntriesLaw> entries)
        : head_(head), entries_(entries) {}
    /// The `head` element.
    [[nodiscard]] constexpr uint8_t head() const noexcept { return head_; }
    /// The `entries` element, the entries it holds.
    [[nodiscard]] ::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry, 4, 1>, CodecChainPrevCarrierEntriesLaw> entries() const noexcept { return entries_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainPrevCarrier> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t head;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            head = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry> entries;
        entries.reserve(4);
        std::uint8_t _prev_entries_after_marker = 0;
        bool _more = false;
        for (std::size_t _i = 0; _i < 4; ++_i) {
            auto _elem = ::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry::decode(cursor, _prev_entries_after_marker, static_cast<std::uint8_t>((head >> 0) & 0x1));
            if (!_elem.has_value()) return std::nullopt;
            _more = _elem->z();
            _prev_entries_after_marker = static_cast<std::uint8_t>(static_cast<std::uint64_t>((*_elem).ext_id()) == 4ULL);
            entries.push_back(*_elem);
            if (!_more) break;
        }
        if (_more && cursor.remaining() == 0) { return std::nullopt; }
        if (_more) { cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt; }
        const auto _l_entries = ::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry, 4, 1>, CodecChainPrevCarrierEntriesLaw>::list_type::make(entries);
        if (!_l_entries) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
            return std::nullopt;
        }
        const auto _b_entries = ::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry, 4, 1>, CodecChainPrevCarrierEntriesLaw>::make(::SCE::Forge::Lawful<::SCE::Forge::List<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry, 4, 1>, CodecChainPrevCarrierEntriesLaw>::start_type{(((head >> 0) & 0x1) != 0), ::SCE::Forge::FixedStart{}}, *_l_entries);
        if (!_b_entries) {
            cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
            return std::nullopt;
        }
        CodecChainPrevCarrier _decoded{
            /*head=*/head,
            /*entries=*/*_b_entries,
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool t() const noexcept {
        return (this->head_ & 0x01) != 0;
    }

    void set_t(bool v) noexcept {
        if (v) {
            this->head_ = static_cast<uint8_t>(this->head_ | 0x01);
        } else {
            this->head_ = static_cast<uint8_t>(this->head_ & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 21;

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
    uint8_t head_{};
    ::SCE::Forge::Lawful<::SCE::Forge::Bounded<std::vector<::SCE::Generated::CodecChainPrevEntry::CodecChainPrevEntry>, 4, 1>, CodecChainPrevCarrierEntriesLaw> entries_{};
};

}  // namespace SCE::Generated::CodecChainPrevCarrier

#endif  // SCE_FORGE_CODEC_CHAIN_PREV_CARRIER_H
