// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <variant>
#include <vector>

// Byte-level mapping from an expanded SCXML document back to its
// source origins, so diagnostics that fire *after* preprocessor
// expansion (XInclude, and later `sce:template`) can report author
// file/row/col instead of in-memory expanded coordinates.
//
// Mirrors `sce-build/src/position_map.rs` line-for-line where
// practical; the Rust side is the authoritative reference and the
// Origin enum shape is pinned across the two sides by the drift
// test `cpp_origin_shape_matches_rust` in
// `sce-build/src/position_map.rs::tests`. Consumers:
// `PugiXMLDocument::processXInclude` / `processSceTemplate`
// compose these maps across the two preprocessor stages.

namespace SCE::parsing {

// An origin-specific source location: the author file plus 1-based
// (row, col). Column counts Unicode scalar values rather than bytes
// to match the convention used by every existing forge error-pipeline
// SourceLocation and by roxmltree's TextPos — byte-based counting
// would under-report column on multibyte characters and create an
// invisible skew between remapped and non-remapped diagnostics.
// Mirrors Rust `SourcePos`.
struct SourcePos {
    std::filesystem::path file;
    uint32_t row{1};  // 1-based
    uint32_t col{1};  // 1-based
};

// Bytes copied 1:1 from `path` starting at `source_offset`. Mirrors
// Rust `Origin::File`. Expanded offset `k` in an entry with this
// origin maps to `source_offset + (k - expanded_start)` in `path`.
struct FileOrigin {
    std::filesystem::path path;
    size_t source_offset{0};
};

// Bytes synthesised by template parameter substitution. Mirrors
// Rust `Origin::CallSite`. All bytes in the entry's range map to
// the single (row, col) of the `<sce:param>` element that supplied
// the value — depth-1 per RFC §6.3 Q3. Consumed by the sce:template
// expander; the XInclude expander does not produce this variant.
struct CallSiteOrigin {
    std::filesystem::path path;
    uint32_t row{1};
    uint32_t col{1};
};

// Where an expanded-document byte range came from. Mirrors Rust
// `Origin` (marked `#[non_exhaustive]` on the Rust side so a future
// third variant can be added without breaking external consumers;
// std::variant does not have an equivalent annotation, but per
// feedback_built_but_unconsumed.md we do not add the third variant
// speculatively).
using Origin = std::variant<FileOrigin, CallSiteOrigin>;

// A byte-range mapping from the expanded document back to source
// origins. Mirrors Rust `PositionMap`. See the module comment for
// semantics.
class PositionMap {
public:
    // Identity map: the whole expanded document is a 1:1 copy of
    // `text` from the file labelled `file`. Mirrors Rust
    // `PositionMap::identity`.
    static PositionMap identity(std::filesystem::path file, std::string_view text);

    // Translate an expanded byte offset to its source (file, row,
    // col). Mirrors Rust `PositionMap::lookup`. Tolerant of `offset`
    // past the last entry's `expanded_end`: clamps to the last
    // entry so a diagnostic at EOF still resolves rather than
    // aborting.
    SourcePos lookup(size_t expanded_offset) const;

    // True when the map has exactly one FileOrigin entry whose
    // source_offset == 0 and whose range is the full expanded
    // length — i.e. no preprocessor expansion happened. Mirrors
    // Rust `PositionMap::is_identity`.
    bool is_identity() const noexcept;

    // Register `text` as the content of `path` so later entries
    // referencing `path` can resolve offsets to (row, col).
    // Idempotent: repeated calls with the same path are a no-op on
    // subsequent calls. Mirrors Rust `PositionMap::register_file`.
    void register_file(std::filesystem::path path, std::string_view text);

    // Append an entry covering `[expanded_start, expanded_end)`.
    // Callers must push entries in ascending `expanded_start` order
    // and must ensure they form a contiguous cover of the expanded
    // document before calling `lookup`. Mirrors Rust
    // `PositionMap::push_entry`.
    void push_entry(size_t expanded_start, size_t expanded_end, Origin origin);

    // Append entries from `inner` that overlap
    // `[inner_start, inner_end)`, remapping their expanded offsets
    // to begin at `outer_start` in `self`. Copies any file texts
    // `inner` registered that are not already in `self`. Mirrors
    // Rust `PositionMap::append_mapped_substring`.
    void append_mapped_substring(const PositionMap &inner, size_t inner_start, size_t inner_end,
                                 size_t outer_start);

private:
    struct Entry {
        size_t expanded_start;
        size_t expanded_end;
        Origin origin;
    };

    // Line-start byte offsets for a source file, keyed alongside
    // its text so offset → (row, col) translation stays O(log N)
    // per lookup (binary search into `line_starts`, then a linear
    // char-count inside the matched line). Mirrors Rust `FileText`.
    struct FileText {
        std::string text;
        std::vector<size_t> line_starts;
    };

    const Entry &entry_for(size_t offset) const;

    std::vector<Entry> entries_;
    std::unordered_map<std::string, FileText> files_;

    friend size_t rowcol_to_offset(std::string_view expanded_text, uint32_t row, uint32_t col);
};

// Translate 1-based (row, col) within `expanded_text` into a byte
// offset, for callers that receive row/col from roxmltree / libxml2
// but need a byte offset to pass to `PositionMap::lookup`. Exposed
// as a free function (not a method on PositionMap) because the map
// does not own the expanded text. Clamps `row` and `col` so
// diagnostics at or past EOF still translate to a valid offset
// rather than asserting. Mirrors Rust `rowcol_to_offset`.
size_t rowcol_to_offset(std::string_view expanded_text, uint32_t row, uint32_t col);

}  // namespace SCE::parsing
