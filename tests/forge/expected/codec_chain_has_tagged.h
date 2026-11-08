// SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#pragma once
#ifndef SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H
#define SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H

#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>
#include <type_traits>
#include <utility>

#include "sce/forge/codec.h"
#include "codec_chain_has_tagged_entry.h"

namespace SCE::Generated::CodecChainHasTagged {

/// The elements a value of `CodecChainHasTagged` presents, each read by name: the
/// member function where the type has one, otherwise a function the type's own
/// namespace declares (docs/adr/0081, step 4).
namespace source {
namespace detail_header {
void header() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.header())) -> decltype(s.header()) {
    return s.header();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(header(s))) -> decltype(header(s)) {
    return header(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_header
/// The `header` element of a source.
inline constexpr detail_header::Read header{};
namespace detail_entries_has_0x7_has_0x9_header_wide {
void entries_has_0x7_has_0x9_header_wide() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.entries_has_0x7_has_0x9_header_wide())) -> decltype(s.entries_has_0x7_has_0x9_header_wide()) {
    return s.entries_has_0x7_has_0x9_header_wide();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(entries_has_0x7_has_0x9_header_wide(s))) -> decltype(entries_has_0x7_has_0x9_header_wide(s)) {
    return entries_has_0x7_has_0x9_header_wide(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_entries_has_0x7_has_0x9_header_wide
/// The `entries_has_0x7_has_0x9_header_wide` element of a source.
inline constexpr detail_entries_has_0x7_has_0x9_header_wide::Read entries_has_0x7_has_0x9_header_wide{};
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
namespace detail_priority {
void priority() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.priority())) -> decltype(s.priority()) {
    return s.priority();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(priority(s))) -> decltype(priority(s)) {
    return priority(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_priority
/// The `priority` element of a source.
inline constexpr detail_priority::Read priority{};
namespace detail_checksum {
void checksum() = delete;
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.checksum())) -> decltype(s.checksum()) {
    return s.checksum();
}
template <class S>
constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept(checksum(s))) -> decltype(checksum(s)) {
    return checksum(s);
}
struct Read {
    template <class S>
    constexpr auto operator()(const S& s) const
        noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{})))
        -> decltype(read(s, ::SCE::Forge::PreferMember{})) {
        return read(s, ::SCE::Forge::PreferMember{});
    }
};
}  // namespace detail_checksum
/// The `checksum` element of a source.
inline constexpr detail_checksum::Read checksum{};
}  // namespace source

/// The kind of entry `entries.has(0x7)` names: an entry whose identifier
/// is 0x7, which a value holds where it stands (docs/adr/0065 §6 R6b).
struct CodecChainHasTaggedEntriesHas0x7Key {
    template <class E>
    [[nodiscard]] static bool matches(const E& entry) {
        return static_cast<std::uint64_t>(::SCE::Generated::CodecChainHasTaggedEntry::source::entry_type(entry)) == 0x7u;
    }
};

/// The kind of entry `entries.has(0x9)` names: an entry whose identifier
/// is 0x9, which a value holds where it stands (docs/adr/0065 §6 R6b).
struct CodecChainHasTaggedEntriesHas0x9Key {
    template <class E>
    [[nodiscard]] static bool matches(const E& entry) {
        return static_cast<std::uint64_t>(::SCE::Generated::CodecChainHasTaggedEntry::source::entry_type(entry)) == 0x9u;
    }
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c) present while `entries.has(0x7)` holds and `entries.has(0x9)` holds and `header.wide` holds.
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide {
public:
    /// The value of each field, in declaration order.
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide(::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, 3, 1> entries, uint8_t priority)
        : entries_(entries), priority_(priority) {}
    /// The `entries` field.
    [[nodiscard]] ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, 3, 1> entries() const noexcept { return entries_; }
    /// The `priority` field.
    [[nodiscard]] constexpr uint8_t priority() const noexcept { return priority_; }

private:
    ::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, 3, 1> entries_;
    uint8_t priority_{};
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c) present while `entries.has(0x7)` holds and `entries.has(0x9)` holds and `header.wide` does not hold.
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9 {
public:
    /// The value of each field, in declaration order.
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9(::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, 3, 1> entries, uint8_t priority)
        : entries_(entries), priority_(priority) {}
    /// The `entries` field.
    [[nodiscard]] ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, 3, 1> entries() const noexcept { return entries_; }
    /// The `priority` field.
    [[nodiscard]] constexpr uint8_t priority() const noexcept { return priority_; }

private:
    ::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, 3, 1> entries_;
    uint8_t priority_{};
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c) present while `entries.has(0x7)` holds and `entries.has(0x9)` does not hold and `header.wide` holds.
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide {
public:
    /// The value of each field, in declaration order.
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide(::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries, uint8_t priority, uint16_t checksum)
        : entries_(entries), priority_(priority), checksum_(checksum) {}
    /// The `entries` field.
    [[nodiscard]] ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries() const noexcept { return entries_; }
    /// The `priority` field.
    [[nodiscard]] constexpr uint8_t priority() const noexcept { return priority_; }
    /// The `checksum` field.
    [[nodiscard]] constexpr uint16_t checksum() const noexcept { return checksum_; }

private:
    ::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries_;
    uint8_t priority_{};
    uint16_t checksum_{};
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c) present while `entries.has(0x7)` does not hold and `entries.has(0x9)` holds and `header.wide` holds.
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide {
public:
    /// The value of each field, in declaration order.
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide(::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, 3, 1> entries, uint8_t priority)
        : entries_(entries), priority_(priority) {}
    /// The `entries` field.
    [[nodiscard]] ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, 3, 1> entries() const noexcept { return entries_; }
    /// The `priority` field.
    [[nodiscard]] constexpr uint8_t priority() const noexcept { return priority_; }

private:
    ::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, 3, 1> entries_;
    uint8_t priority_{};
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c) present while `entries.has(0x7)` holds and `entries.has(0x9)` does not hold and `header.wide` does not hold.
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7 {
public:
    /// The value of each field, in declaration order.
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7(::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries, uint8_t priority, uint16_t checksum)
        : entries_(entries), priority_(priority), checksum_(checksum) {}
    /// The `entries` field.
    [[nodiscard]] ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries() const noexcept { return entries_; }
    /// The `priority` field.
    [[nodiscard]] constexpr uint8_t priority() const noexcept { return priority_; }
    /// The `checksum` field.
    [[nodiscard]] constexpr uint16_t checksum() const noexcept { return checksum_; }

private:
    ::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries_;
    uint8_t priority_{};
    uint16_t checksum_{};
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c) present while `entries.has(0x7)` does not hold and `entries.has(0x9)` holds and `header.wide` does not hold.
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9 {
public:
    /// The value of each field, in declaration order.
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9(::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, 3, 1> entries)
        : entries_(entries) {}
    /// The `entries` field.
    [[nodiscard]] ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, 3, 1> entries() const noexcept { return entries_; }

private:
    ::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, 3, 1> entries_;
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c) present while `entries.has(0x7)` does not hold and `entries.has(0x9)` does not hold and `header.wide` holds.
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide {
public:
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide() = default;
    /// The value of each field, in declaration order.
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide(::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries, uint8_t priority, uint16_t checksum)
        : entries_(entries), priority_(priority), checksum_(checksum) {}
    /// The `entries` field.
    [[nodiscard]] ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries() const noexcept { return entries_; }
    /// The `priority` field.
    [[nodiscard]] constexpr uint8_t priority() const noexcept { return priority_; }
    /// The `checksum` field.
    [[nodiscard]] constexpr uint16_t checksum() const noexcept { return checksum_; }

private:
    ::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries_{};
    uint8_t priority_{};
    uint16_t checksum_{};
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c) present while `entries.has(0x7)` does not hold and `entries.has(0x9)` does not hold and `header.wide` does not hold.
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither {
public:
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither() = default;
    /// The value of each field, in declaration order.
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither(::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries, uint16_t checksum)
        : entries_(entries), checksum_(checksum) {}
    /// The `entries` field.
    [[nodiscard]] ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries() const noexcept { return entries_; }
    /// The `checksum` field.
    [[nodiscard]] constexpr uint16_t checksum() const noexcept { return checksum_; }

private:
    ::SCE::Forge::Chain<std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, 3, 1> entries_{};
    uint16_t checksum_{};
};

/// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides (docs/adr/0065 §6 R4b, R4c): a value holds one
/// arm, and its encode derives the gates from which.
using CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide = std::variant<CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither>;

/// Whether `R` is a list a source may present for `entries`: held to bounds
/// within its own, of any type, each entry a source of the entry codec.
template <class R, class = void>
struct is_entries_list : std::false_type {};
template <class R>
struct is_entries_list<R, std::enable_if_t<::SCE::Forge::Detail::IsBoundedWithin<R, 3, 1>::value>>
    : ::SCE::Generated::CodecChainHasTaggedEntry::is_source<::SCE::Forge::Detail::ElementOf<R>> {};

/// Whether `R` is what a source may present for `entries` where the chain holds
/// an entry of each kind of `::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>` and of none of `::SCE::Forge::Kinds<>`
/// (docs/adr/0065 §6 R6b): a `Chain` of those kinds, held to bounds within its
/// own, of any list, each entry a source of the entry codec.
template <class R>
struct is_entries_chain_has_0x7_has_0x9 : std::false_type {};
template <class L, std::size_t Max, std::size_t Min>
struct is_entries_chain_has_0x7_has_0x9<::SCE::Forge::Chain<L, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, Max, Min>>
    : std::conjunction<::SCE::Forge::Detail::WithinBounds<3, 1, Max, Min>,
                       ::SCE::Generated::CodecChainHasTaggedEntry::is_source<::SCE::Forge::Detail::ElementOf<L>>> {};

/// Whether `R` is what a source may present for `entries` where the chain holds
/// an entry of each kind of `::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>` and of none of `::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>`
/// (docs/adr/0065 §6 R6b): a `Chain` of those kinds, held to bounds within its
/// own, of any list, each entry a source of the entry codec.
template <class R>
struct is_entries_chain_has_0x7_not_0x9 : std::false_type {};
template <class L, std::size_t Max, std::size_t Min>
struct is_entries_chain_has_0x7_not_0x9<::SCE::Forge::Chain<L, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, Max, Min>>
    : std::conjunction<::SCE::Forge::Detail::WithinBounds<3, 1, Max, Min>,
                       ::SCE::Generated::CodecChainHasTaggedEntry::is_source<::SCE::Forge::Detail::ElementOf<L>>> {};

/// Whether `R` is what a source may present for `entries` where the chain holds
/// an entry of each kind of `::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>` and of none of `::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>`
/// (docs/adr/0065 §6 R6b): a `Chain` of those kinds, held to bounds within its
/// own, of any list, each entry a source of the entry codec.
template <class R>
struct is_entries_chain_has_0x9_not_0x7 : std::false_type {};
template <class L, std::size_t Max, std::size_t Min>
struct is_entries_chain_has_0x9_not_0x7<::SCE::Forge::Chain<L, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, Max, Min>>
    : std::conjunction<::SCE::Forge::Detail::WithinBounds<3, 1, Max, Min>,
                       ::SCE::Generated::CodecChainHasTaggedEntry::is_source<::SCE::Forge::Detail::ElementOf<L>>> {};

/// Whether `R` is what a source may present for `entries` where the chain holds
/// an entry of each kind of `::SCE::Forge::Kinds<>` and of none of `::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>`
/// (docs/adr/0065 §6 R6b): a `Chain` of those kinds, held to bounds within its
/// own, of any list, each entry a source of the entry codec.
template <class R>
struct is_entries_chain_not_0x7_not_0x9 : std::false_type {};
template <class L, std::size_t Max, std::size_t Min>
struct is_entries_chain_not_0x7_not_0x9<::SCE::Forge::Chain<L, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, Max, Min>>
    : std::conjunction<::SCE::Forge::Detail::WithinBounds<3, 1, Max, Min>,
                       ::SCE::Generated::CodecChainHasTaggedEntry::is_source<::SCE::Forge::Detail::ElementOf<L>>> {};

/// Whether `G` presents every field of `CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide`, each convertible to its type.
template <class G, class = void>
struct is_entries_has_0x7_has_0x9_header_wide_has0x7_has0x9_wide_members : std::false_type {};
template <class G>
struct is_entries_has_0x7_has_0x9_header_wide_has0x7_has0x9_wide_members<G, std::void_t<decltype(source::entries(std::declval<const G&>())), decltype(source::priority(std::declval<const G&>()))>>
    : std::conjunction<is_entries_chain_has_0x7_has_0x9<std::decay_t<decltype(source::entries(std::declval<const G&>()))>>, std::is_convertible<decltype(source::priority(std::declval<const G&>())), uint8_t>> {};
/// Whether `G` presents every field of `CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9`, each convertible to its type.
template <class G, class = void>
struct is_entries_has_0x7_has_0x9_header_wide_has0x7_has0x9_members : std::false_type {};
template <class G>
struct is_entries_has_0x7_has_0x9_header_wide_has0x7_has0x9_members<G, std::void_t<decltype(source::entries(std::declval<const G&>())), decltype(source::priority(std::declval<const G&>()))>>
    : std::conjunction<is_entries_chain_has_0x7_has_0x9<std::decay_t<decltype(source::entries(std::declval<const G&>()))>>, std::is_convertible<decltype(source::priority(std::declval<const G&>())), uint8_t>> {};
/// Whether `G` presents every field of `CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide`, each convertible to its type.
template <class G, class = void>
struct is_entries_has_0x7_has_0x9_header_wide_has0x7_wide_members : std::false_type {};
template <class G>
struct is_entries_has_0x7_has_0x9_header_wide_has0x7_wide_members<G, std::void_t<decltype(source::entries(std::declval<const G&>())), decltype(source::priority(std::declval<const G&>())), decltype(source::checksum(std::declval<const G&>()))>>
    : std::conjunction<is_entries_chain_has_0x7_not_0x9<std::decay_t<decltype(source::entries(std::declval<const G&>()))>>, std::is_convertible<decltype(source::priority(std::declval<const G&>())), uint8_t>, std::is_convertible<decltype(source::checksum(std::declval<const G&>())), uint16_t>> {};
/// Whether `G` presents every field of `CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide`, each convertible to its type.
template <class G, class = void>
struct is_entries_has_0x7_has_0x9_header_wide_has0x9_wide_members : std::false_type {};
template <class G>
struct is_entries_has_0x7_has_0x9_header_wide_has0x9_wide_members<G, std::void_t<decltype(source::entries(std::declval<const G&>())), decltype(source::priority(std::declval<const G&>()))>>
    : std::conjunction<is_entries_chain_has_0x9_not_0x7<std::decay_t<decltype(source::entries(std::declval<const G&>()))>>, std::is_convertible<decltype(source::priority(std::declval<const G&>())), uint8_t>> {};
/// Whether `G` presents every field of `CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7`, each convertible to its type.
template <class G, class = void>
struct is_entries_has_0x7_has_0x9_header_wide_has0x7_members : std::false_type {};
template <class G>
struct is_entries_has_0x7_has_0x9_header_wide_has0x7_members<G, std::void_t<decltype(source::entries(std::declval<const G&>())), decltype(source::priority(std::declval<const G&>())), decltype(source::checksum(std::declval<const G&>()))>>
    : std::conjunction<is_entries_chain_has_0x7_not_0x9<std::decay_t<decltype(source::entries(std::declval<const G&>()))>>, std::is_convertible<decltype(source::priority(std::declval<const G&>())), uint8_t>, std::is_convertible<decltype(source::checksum(std::declval<const G&>())), uint16_t>> {};
/// Whether `G` presents every field of `CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9`, each convertible to its type.
template <class G, class = void>
struct is_entries_has_0x7_has_0x9_header_wide_has0x9_members : std::false_type {};
template <class G>
struct is_entries_has_0x7_has_0x9_header_wide_has0x9_members<G, std::void_t<decltype(source::entries(std::declval<const G&>()))>>
    : std::conjunction<is_entries_chain_has_0x9_not_0x7<std::decay_t<decltype(source::entries(std::declval<const G&>()))>>> {};
/// Whether `G` presents every field of `CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide`, each convertible to its type.
template <class G, class = void>
struct is_entries_has_0x7_has_0x9_header_wide_wide_members : std::false_type {};
template <class G>
struct is_entries_has_0x7_has_0x9_header_wide_wide_members<G, std::void_t<decltype(source::entries(std::declval<const G&>())), decltype(source::priority(std::declval<const G&>())), decltype(source::checksum(std::declval<const G&>()))>>
    : std::conjunction<is_entries_chain_not_0x7_not_0x9<std::decay_t<decltype(source::entries(std::declval<const G&>()))>>, std::is_convertible<decltype(source::priority(std::declval<const G&>())), uint8_t>, std::is_convertible<decltype(source::checksum(std::declval<const G&>())), uint16_t>> {};
/// Whether `G` presents every field of `CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither`, each convertible to its type.
template <class G, class = void>
struct is_entries_has_0x7_has_0x9_header_wide_neither_members : std::false_type {};
template <class G>
struct is_entries_has_0x7_has_0x9_header_wide_neither_members<G, std::void_t<decltype(source::entries(std::declval<const G&>())), decltype(source::checksum(std::declval<const G&>()))>>
    : std::conjunction<is_entries_chain_not_0x7_not_0x9<std::decay_t<decltype(source::entries(std::declval<const G&>()))>>, std::is_convertible<decltype(source::checksum(std::declval<const G&>())), uint16_t>> {};
/// Whether `A` and `B` both present `entries` as a chain of one list, kept
/// alike.
template <class A, class B, class = void>
struct is_entries_read_alike : std::false_type {};
template <class A, class B>
struct is_entries_read_alike<A, B, std::void_t<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const A&>()))>, ::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const B&>()))>>>
    : std::is_same<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const A&>()))>, ::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const B&>()))>> {};

/// Whether `R` is what a source may present for `entries_has_0x7_has_0x9_header_wide`, the fields `entries.has(0x7) || entries.has(0x9) || header.wide`
/// decides: a `std::variant` of a type for each arm, in order, that presents its fields.
template <class R>
struct is_entries_has_0x7_has_0x9_header_wide_group : std::false_type {};
template <class G0, class G1, class G2, class G3, class G4, class G5, class G6, class G7>
struct is_entries_has_0x7_has_0x9_header_wide_group<std::variant<G0, G1, G2, G3, G4, G5, G6, G7>>
    : std::conjunction<is_entries_has_0x7_has_0x9_header_wide_has0x7_has0x9_wide_members<G0>, is_entries_has_0x7_has_0x9_header_wide_has0x7_has0x9_members<G1>, is_entries_has_0x7_has_0x9_header_wide_has0x7_wide_members<G2>, is_entries_has_0x7_has_0x9_header_wide_has0x9_wide_members<G3>, is_entries_has_0x7_has_0x9_header_wide_has0x7_members<G4>, is_entries_has_0x7_has_0x9_header_wide_has0x9_members<G5>, is_entries_has_0x7_has_0x9_header_wide_wide_members<G6>, is_entries_has_0x7_has_0x9_header_wide_neither_members<G7>, is_entries_read_alike<G0, G1>, is_entries_read_alike<G0, G2>, is_entries_read_alike<G0, G3>, is_entries_read_alike<G0, G4>, is_entries_read_alike<G0, G5>, is_entries_read_alike<G0, G6>, is_entries_read_alike<G0, G7>> {};

/// Whether `S` presents every element of `CodecChainHasTagged`, each convertible to
/// its type (a C++17 trait, ADR 0059 §3).
template <class S, class = void>
struct is_source : std::false_type {};
template <class S>
struct is_source<S, std::void_t<decltype(source::header(std::declval<const S&>())), decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>
    : std::conjunction<std::is_convertible<decltype(source::header(std::declval<const S&>())), uint8_t>, is_entries_has_0x7_has_0x9_header_wide_group<std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>> {};

/// Whether reading every element of a source `S` throws nothing.
template <class S, class = void>
struct is_nothrow_source : std::false_type {};
template <class S>
struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>
    : std::bool_constant<noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::header(std::declval<const S&>()))) && noexcept(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>())) && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>&>().keep(source::entries(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()).inner())) && ::SCE::Generated::CodecChainHasTaggedEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::entries(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>::type>>::value && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::priority(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>&>().keep(source::entries(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()).inner())) && ::SCE::Generated::CodecChainHasTaggedEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::entries(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>::type>>::value && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::priority(std::declval<const std::variant_alternative_t<1, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<2, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>&>().keep(source::entries(std::declval<const std::variant_alternative_t<2, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()).inner())) && ::SCE::Generated::CodecChainHasTaggedEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::entries(std::declval<const std::variant_alternative_t<2, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>::type>>::value && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::priority(std::declval<const std::variant_alternative_t<2, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::checksum(std::declval<const std::variant_alternative_t<2, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<3, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>&>().keep(source::entries(std::declval<const std::variant_alternative_t<3, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()).inner())) && ::SCE::Generated::CodecChainHasTaggedEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::entries(std::declval<const std::variant_alternative_t<3, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>::type>>::value && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::priority(std::declval<const std::variant_alternative_t<3, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<4, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>&>().keep(source::entries(std::declval<const std::variant_alternative_t<4, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()).inner())) && ::SCE::Generated::CodecChainHasTaggedEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::entries(std::declval<const std::variant_alternative_t<4, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>::type>>::value && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::priority(std::declval<const std::variant_alternative_t<4, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::checksum(std::declval<const std::variant_alternative_t<4, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<5, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>&>().keep(source::entries(std::declval<const std::variant_alternative_t<5, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()).inner())) && ::SCE::Generated::CodecChainHasTaggedEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::entries(std::declval<const std::variant_alternative_t<5, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>::type>>::value && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<6, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>&>().keep(source::entries(std::declval<const std::variant_alternative_t<6, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()).inner())) && ::SCE::Generated::CodecChainHasTaggedEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::entries(std::declval<const std::variant_alternative_t<6, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>::type>>::value && noexcept(::SCE::Forge::Detail::converts_to<uint8_t>(source::priority(std::declval<const std::variant_alternative_t<6, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::checksum(std::declval<const std::variant_alternative_t<6, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))) && noexcept(std::declval<::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<7, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>&>().keep(source::entries(std::declval<const std::variant_alternative_t<7, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()).inner())) && ::SCE::Generated::CodecChainHasTaggedEntry::is_nothrow_source<::SCE::Forge::Detail::ElementOf<typename ::SCE::Forge::Detail::ChainOf<std::decay_t<decltype(source::entries(std::declval<const std::variant_alternative_t<7, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>()))>>::type>>::value && noexcept(::SCE::Forge::Detail::converts_to<uint16_t>(source::checksum(std::declval<const std::variant_alternative_t<7, std::decay_t<decltype(source::entries_has_0x7_has_0x9_header_wide(std::declval<const S&>()))>>&>())))> {};

/// The encode of `CodecChainHasTagged` over any source of it: each element read
/// through `source`, written as the plan lays it out (docs/adr/0081). It
/// refuses nothing; what it returns is the sink's own failure, and it
/// throws only what a read of `src` throws.
struct Wire {
    template <class S>
    [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {
        static_assert(is_source<S>::value, "S presents every element of CodecChainHasTagged: header, entries_has_0x7_has_0x9_header_wide");
        const auto& _r_entries_has_0x7_has_0x9_header_wide = source::entries_has_0x7_has_0x9_header_wide(src);
        ::SCE::Forge::Detail::Kept<::SCE::Forge::Detail::InnerRead<decltype(source::entries(std::declval<const std::variant_alternative_t<0, std::decay_t<decltype(_r_entries_has_0x7_has_0x9_header_wide)>>&>()))>> _r_entries;
        std::optional<uint8_t> _e_priority;
        std::optional<uint16_t> _e_checksum;
        if (const auto* _a = std::get_if<0>(&_r_entries_has_0x7_has_0x9_header_wide)) {
            _r_entries.keep(source::entries(*_a).inner());
            _e_priority = static_cast<uint8_t>(source::priority(*_a));
        }
        if (const auto* _a = std::get_if<1>(&_r_entries_has_0x7_has_0x9_header_wide)) {
            _r_entries.keep(source::entries(*_a).inner());
            _e_priority = static_cast<uint8_t>(source::priority(*_a));
        }
        if (const auto* _a = std::get_if<2>(&_r_entries_has_0x7_has_0x9_header_wide)) {
            _r_entries.keep(source::entries(*_a).inner());
            _e_priority = static_cast<uint8_t>(source::priority(*_a));
            _e_checksum = static_cast<uint16_t>(source::checksum(*_a));
        }
        if (const auto* _a = std::get_if<3>(&_r_entries_has_0x7_has_0x9_header_wide)) {
            _r_entries.keep(source::entries(*_a).inner());
            _e_priority = static_cast<uint8_t>(source::priority(*_a));
        }
        if (const auto* _a = std::get_if<4>(&_r_entries_has_0x7_has_0x9_header_wide)) {
            _r_entries.keep(source::entries(*_a).inner());
            _e_priority = static_cast<uint8_t>(source::priority(*_a));
            _e_checksum = static_cast<uint16_t>(source::checksum(*_a));
        }
        if (const auto* _a = std::get_if<5>(&_r_entries_has_0x7_has_0x9_header_wide)) {
            _r_entries.keep(source::entries(*_a).inner());
        }
        if (const auto* _a = std::get_if<6>(&_r_entries_has_0x7_has_0x9_header_wide)) {
            _r_entries.keep(source::entries(*_a).inner());
            _e_priority = static_cast<uint8_t>(source::priority(*_a));
            _e_checksum = static_cast<uint16_t>(source::checksum(*_a));
        }
        if (const auto* _a = std::get_if<7>(&_r_entries_has_0x7_has_0x9_header_wide)) {
            _r_entries.keep(source::entries(*_a).inner());
            _e_checksum = static_cast<uint16_t>(source::checksum(*_a));
        }
        std::uint64_t _f_header = static_cast<std::uint64_t>(source::header(src));
        _f_header = (_f_header & ~0x1ull) | static_cast<std::uint64_t>(static_cast<std::uint8_t>((_r_entries_has_0x7_has_0x9_header_wide.index() == 0u || _r_entries_has_0x7_has_0x9_header_wide.index() == 2u || _r_entries_has_0x7_has_0x9_header_wide.index() == 3u || _r_entries_has_0x7_has_0x9_header_wide.index() == 6u) ? 1 : 0));
        const uint8_t _c_header = static_cast<uint8_t>(_f_header);
        if (auto _e = w.write_u8(static_cast<std::uint8_t>(_c_header)); _e) return _e;
        if (_r_entries) {
            {
                std::size_t _left_entries = (*_r_entries).size();
                for (const auto& _x : (*_r_entries)) {
                    --_left_entries;
                    ::SCE::Generated::CodecChainHasTaggedEntry::Overrides _o_entries{};
                    _o_entries.ctl_more = _left_entries != 0;
                    if (auto _e = ::SCE::Generated::CodecChainHasTaggedEntry::Wire::encode_source(_x, w, _o_entries); _e) return _e;
                }
            }
        }
        if (_e_priority) {
            if (auto _e = w.write_u8(static_cast<std::uint8_t>((*_e_priority))); _e) return _e;
        }
        if (_e_checksum) {
            if (auto _e = w.write_u16_be(static_cast<std::uint16_t>((*_e_checksum))); _e) return _e;
        }
        return std::nullopt;
    }
};

struct CodecChainHasTagged {
    CodecChainHasTagged() = default;
    /// The value of each element it presents, in declaration order (ADR 0059 §5).
    CodecChainHasTagged(uint8_t header, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide entries_has_0x7_has_0x9_header_wide)
        : header_(header), entries_has_0x7_has_0x9_header_wide_(std::move(entries_has_0x7_has_0x9_header_wide)) {}
    /// The `header` element.
    [[nodiscard]] constexpr uint8_t header() const noexcept { return header_; }
    /// The fields `entries.has(0x7) || entries.has(0x9) || header.wide` decides, as the arm the value holds.
    [[nodiscard]] const CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide& entries_has_0x7_has_0x9_header_wide() const noexcept { return entries_has_0x7_has_0x9_header_wide_; }

    /// Decode the next frame from `cursor`. On success the cursor
    /// advances past the consumed bytes; on `NeedMoreBytes` the cursor
    /// is left untouched so the caller can resume after appending more
    /// bytes (RFC §synth-5-B L494-519). Returns `std::nullopt` on the
    /// `NeedMoreBytes` boundary; later phases attach a typed error via
    /// `cursor.last_error()`.
    static std::optional<CodecChainHasTagged> decode(::SCE::Forge::SceCursor& cursor) {
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
        uint8_t header;
        {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            header = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::vector<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry> entries;
        entries.reserve(3);
        bool _more = false;
        for (std::size_t _i = 0; _i < 3; ++_i) {
            auto _elem = ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry::decode(cursor);
            if (!_elem.has_value()) return std::nullopt;
            _more = _elem->more();
            entries.push_back(*_elem);
            if (!_more) break;
        }
        if (_more && cursor.remaining() == 0) { return std::nullopt; }
        if (_more) { cursor.refuse(::SCE::Forge::CodecError::TooManyElements); return std::nullopt; }
        bool _has_entries_7 = false;
        for (const auto& _e : entries) {
            if (static_cast<std::uint64_t>(_e.entry_type()) == 7ULL) {
                _has_entries_7 = true;
                break;
            }
        }
        bool _has_entries_9 = false;
        for (const auto& _e : entries) {
            if (static_cast<std::uint64_t>(_e.entry_type()) == 9ULL) {
                _has_entries_9 = true;
                break;
            }
        }
        std::optional<uint8_t> priority;
        if (_has_entries_7 || (header & 0x01) != 0) {
            const std::uint8_t* raw = cursor.peek_slice(1);
            if (raw == nullptr) return std::nullopt;
            priority = static_cast<uint8_t>(raw[0]);
            if (!cursor.advance(1)) return std::nullopt;
        }
        std::optional<uint16_t> checksum;
        if (!_has_entries_9) {
            const std::uint8_t* raw = cursor.peek_slice(2);
            if (raw == nullptr) return std::nullopt;
            checksum = static_cast<uint16_t>(static_cast<uint16_t>((static_cast<uint16_t>(raw[0]) << 8) | raw[1]));
            if (!cursor.advance(2)) return std::nullopt;
        }
        const auto _b_entries = ::SCE::Forge::List<::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry, 3, 1>::make(entries);
        if (!_b_entries) {
            cursor.refuse(::SCE::Forge::CodecError::TooManyElements);
            return std::nullopt;
        }
        CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide _g_entries_has_0x7_has_0x9_header_wide{std::in_place_index_t<7>{}};
        if (::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x7Key>(entries).has_value() && ::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x9Key>(entries).has_value() && (((header >> 0) & 0x1) != 0)) {
            const auto _ch_entries = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, 3, 1>::make(_b_entries->inner());
            if (!_ch_entries) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _g_entries_has_0x7_has_0x9_header_wide.emplace<0>(*_ch_entries, *priority);
        } else if (::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x7Key>(entries).has_value() && ::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x9Key>(entries).has_value() && (((header >> 0) & 0x1) == 0)) {
            const auto _ch_entries = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<>, 3, 1>::make(_b_entries->inner());
            if (!_ch_entries) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _g_entries_has_0x7_has_0x9_header_wide.emplace<1>(*_ch_entries, *priority);
        } else if (::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x7Key>(entries).has_value() && !::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x9Key>(entries).has_value() && (((header >> 0) & 0x1) != 0)) {
            const auto _ch_entries = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, 3, 1>::make(_b_entries->inner());
            if (!_ch_entries) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _g_entries_has_0x7_has_0x9_header_wide.emplace<2>(*_ch_entries, *priority, *checksum);
        } else if (!::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x7Key>(entries).has_value() && ::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x9Key>(entries).has_value() && (((header >> 0) & 0x1) != 0)) {
            const auto _ch_entries = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, 3, 1>::make(_b_entries->inner());
            if (!_ch_entries) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _g_entries_has_0x7_has_0x9_header_wide.emplace<3>(*_ch_entries, *priority);
        } else if (::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x7Key>(entries).has_value() && !::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x9Key>(entries).has_value() && (((header >> 0) & 0x1) == 0)) {
            const auto _ch_entries = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, 3, 1>::make(_b_entries->inner());
            if (!_ch_entries) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _g_entries_has_0x7_has_0x9_header_wide.emplace<4>(*_ch_entries, *priority, *checksum);
        } else if (!::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x7Key>(entries).has_value() && ::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x9Key>(entries).has_value() && (((header >> 0) & 0x1) == 0)) {
            const auto _ch_entries = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x9Key>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key>, 3, 1>::make(_b_entries->inner());
            if (!_ch_entries) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _g_entries_has_0x7_has_0x9_header_wide.emplace<5>(*_ch_entries);
        } else if (!::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x7Key>(entries).has_value() && !::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x9Key>(entries).has_value() && (((header >> 0) & 0x1) != 0)) {
            const auto _ch_entries = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, 3, 1>::make(_b_entries->inner());
            if (!_ch_entries) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _g_entries_has_0x7_has_0x9_header_wide.emplace<6>(*_ch_entries, *priority, *checksum);
        } else if (!::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x7Key>(entries).has_value() && !::SCE::Forge::Detail::first_of_kind<CodecChainHasTaggedEntriesHas0x9Key>(entries).has_value() && (((header >> 0) & 0x1) == 0)) {
            const auto _ch_entries = ::SCE::Forge::Chain<::SCE::Forge::Span<const ::SCE::Generated::CodecChainHasTaggedEntry::CodecChainHasTaggedEntry>, ::SCE::Forge::Kinds<>, ::SCE::Forge::Kinds<CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key>, 3, 1>::make(_b_entries->inner());
            if (!_ch_entries) {
                cursor.refuse(::SCE::Forge::CodecError::PresentIfMismatch);
                return std::nullopt;
            }
            _g_entries_has_0x7_has_0x9_header_wide.emplace<7>(*_ch_entries, *checksum);
        }
        CodecChainHasTagged _decoded{
            /*header=*/header,
            /*entries_has_0x7_has_0x9_header_wide=*/std::move(_g_entries_has_0x7_has_0x9_header_wide),
        };
        return _decoded;
    }

    // RFC §synth-5-B flags primitive: per-bit-range accessors.
    // Single-bit (width=1) reads as bool; multi-bit (width>=2) reads as
    // the smallest unsigned integer type that fits the range. Setters
    // mask + shift on the way in so out-of-range callers can't corrupt
    // sibling bits. Wire layout is unchanged.
    bool wide() const noexcept {
        return (this->header_ & 0x01) != 0;
    }

    void set_wide(bool v) noexcept {
        if (v) {
            this->header_ = static_cast<uint8_t>(this->header_ | 0x01);
        } else {
            this->header_ = static_cast<uint8_t>(this->header_ & static_cast<uint8_t>(~0x01));
        }
    }

    /// Worst-case encoded byte count for this codec — the upper bound
    /// against which `VectorSink::new` reserves capacity in the
    /// `encode_to_vec` facade, and the natural reserve hint for
    /// caller-owned `SpanSink` allocations.
    static constexpr std::size_t MAX_ENCODED_BYTES = 14;

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
    uint8_t header_{};
    CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide entries_has_0x7_has_0x9_header_wide_{std::in_place_index_t<7>{}};
};

}  // namespace SCE::Generated::CodecChainHasTagged

#endif  // SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H
