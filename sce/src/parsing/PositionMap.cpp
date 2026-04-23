// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/PositionMap.h"

#include <algorithm>
#include <cassert>

// Byte-level source-origin map for expanded SCXML documents.
// Mirrors `sce-build/src/position_map.rs` line-for-line where
// practical — the Rust side is the authoritative reference and
// the Origin enum shape is pinned by the drift test
// `cpp_origin_shape_matches_rust` in
// `sce-build/src/position_map.rs::tests`. Any semantic change here
// must land in the same commit as the Rust-side equivalent.

namespace SCE::parsing {

namespace {

// Build the line-start byte-offset table for `text`. Mirrors Rust
// `build_line_starts`. Returns a vector whose length equals
// `1 + newline_count`. Element 0 is always 0 (start of line 1);
// each subsequent element is the byte offset *immediately after* a
// newline. A trailing newline therefore produces a valid (empty)
// final line-start entry.
std::vector<size_t> build_line_starts(std::string_view text) {
    std::vector<size_t> starts;
    starts.reserve(1 + static_cast<size_t>(std::count(text.begin(), text.end(), '\n')));
    starts.push_back(0);
    for (size_t i = 0; i < text.size(); ++i) {
        if (text[i] == '\n') {
            starts.push_back(i + 1);
        }
    }
    return starts;
}

// Return the index of the largest element in `line_starts` that is
// `<= value`. Equivalent to Rust's `partition_point(|&s| s <= value) - 1`.
// `line_starts` always contains at least element 0 (invariant
// preserved by `build_line_starts`), so the saturating_sub branch
// in the Rust side is redundant here — we assert non-empty and
// compute directly.
size_t line_index_for(const std::vector<size_t> &line_starts, size_t value) {
    assert(!line_starts.empty() && "line_starts always has at least element 0");
    // std::upper_bound gives the first element strictly greater
    // than `value`; subtracting one yields the index of the largest
    // element `<= value`.
    auto it = std::upper_bound(line_starts.begin(), line_starts.end(), value);
    if (it == line_starts.begin()) {
        // value < line_starts[0] == 0 — impossible for size_t, but
        // defensive: report row 0 (caller maps to row 1 via +1).
        return 0;
    }
    return static_cast<size_t>((it - 1) - line_starts.begin());
}

// Translate a byte offset into 1-based (row, col) within `text`
// using the precomputed `line_starts` table. Column counts Unicode
// scalar values rather than bytes — mirrors Rust `offset_to_rowcol`.
//
// UTF-8 continuation bytes are the 10xxxxxx range (0x80..0xBF);
// counting bytes that are NOT continuation bytes yields the
// scalar-value count for the line prefix. For ASCII input this
// collapses to byte counting, matching the Rust `chars().count()`
// behaviour on the same content.
std::pair<uint32_t, uint32_t> offset_to_rowcol(std::string_view text,
                                                const std::vector<size_t> &line_starts,
                                                size_t byte_offset) {
    const size_t clamped = std::min(byte_offset, text.size());
    const size_t row_idx = line_index_for(line_starts, clamped);
    const uint32_t row = static_cast<uint32_t>(row_idx) + 1;
    const size_t line_start = line_starts[row_idx];
    // Count scalar values (non-continuation bytes) in [line_start, clamped).
    uint32_t col_chars = 0;
    for (size_t i = line_start; i < clamped; ++i) {
        const unsigned char b = static_cast<unsigned char>(text[i]);
        // Continuation bytes (10xxxxxx) do not start a new scalar.
        if ((b & 0xC0) != 0x80) {
            ++col_chars;
        }
    }
    return {row, col_chars + 1};
}

}  // namespace

PositionMap PositionMap::identity(std::filesystem::path file, std::string_view text) {
    PositionMap m;
    const size_t len = text.size();
    m.register_file(file, text);
    m.push_entry(0, len,
                 FileOrigin{std::move(file), 0});
    return m;
}

SourcePos PositionMap::lookup(size_t expanded_offset) const {
    assert(!entries_.empty() && "PositionMap must contain at least one entry");
    const Entry &entry = entry_for(expanded_offset);
    if (std::holds_alternative<FileOrigin>(entry.origin)) {
        const auto &origin = std::get<FileOrigin>(entry.origin);
        const size_t delta =
            expanded_offset > entry.expanded_start ? (expanded_offset - entry.expanded_start) : 0;
        const size_t src_offset = origin.source_offset + delta;
        auto it = files_.find(origin.path.string());
        assert(it != files_.end() &&
               "FileOrigin referenced a path with no stored text — PositionMap "
               "construction must register the file before adding the entry");
        const auto &ft = it->second;
        auto [row, col] = offset_to_rowcol(ft.text, ft.line_starts, src_offset);
        return SourcePos{origin.path, row, col};
    }
    const auto &origin = std::get<CallSiteOrigin>(entry.origin);
    return SourcePos{origin.path, origin.row, origin.col};
}

bool PositionMap::is_identity() const noexcept {
    if (entries_.size() != 1) {
        return false;
    }
    const auto *file = std::get_if<FileOrigin>(&entries_.front().origin);
    return file != nullptr && file->source_offset == 0;
}

void PositionMap::register_file(std::filesystem::path path, std::string_view text) {
    // Idempotent: first registration wins. Mirrors Rust
    // `entry(path).or_insert_with`.
    const std::string key = path.string();
    if (files_.find(key) != files_.end()) {
        return;
    }
    FileText ft;
    ft.text.assign(text);
    ft.line_starts = build_line_starts(ft.text);
    files_.emplace(std::move(key), std::move(ft));
}

void PositionMap::push_entry(size_t expanded_start, size_t expanded_end, Origin origin) {
    assert(expanded_end >= expanded_start && "entry ranges must be non-negative");
    if (!entries_.empty()) {
        assert(entries_.back().expanded_end == expanded_start &&
               "entries must be contiguous");
    } else {
        assert(expanded_start == 0 && "first entry must start at expanded offset 0");
    }
    entries_.push_back(Entry{expanded_start, expanded_end, std::move(origin)});
}

void PositionMap::append_mapped_substring(const PositionMap &inner, size_t inner_start,
                                          size_t inner_end, size_t outer_start) {
    assert(inner_end >= inner_start && "inner range must be non-negative");
    // Copy inner's file texts — idempotent.
    for (const auto &[path_key, ft] : inner.files_) {
        if (files_.find(path_key) == files_.end()) {
            files_.emplace(path_key, ft);
        }
    }
    // Walk inner's entries, clip to [inner_start, inner_end), and
    // remap to outer coordinates. Mirrors Rust
    // `append_mapped_substring` line-for-line.
    for (const auto &entry : inner.entries_) {
        if (entry.expanded_end <= inner_start || entry.expanded_start >= inner_end) {
            continue;
        }
        const size_t clip_start = std::max(entry.expanded_start, inner_start);
        const size_t clip_end = std::min(entry.expanded_end, inner_end);
        if (clip_end <= clip_start) {
            continue;
        }
        const size_t new_start = outer_start + (clip_start - inner_start);
        const size_t new_end = outer_start + (clip_end - inner_start);
        Origin new_origin = std::visit(
            [&](const auto &o) -> Origin {
                using T = std::decay_t<decltype(o)>;
                if constexpr (std::is_same_v<T, FileOrigin>) {
                    const size_t delta = clip_start - entry.expanded_start;
                    return FileOrigin{o.path, o.source_offset + delta};
                } else {
                    return CallSiteOrigin{o.path, o.row, o.col};
                }
            },
            entry.origin);
        push_entry(new_start, new_end, std::move(new_origin));
    }
}

const PositionMap::Entry &PositionMap::entry_for(size_t offset) const {
    // partition_point: first index `i` where `entries_[i].expanded_end > offset`.
    // An equivalent std::upper_bound on a derived projection is
    // awkward; a direct lower_bound on `expanded_end > offset`
    // matches the Rust `partition_point(|e| e.expanded_end <= offset)`
    // semantics. Clamp to last entry for offsets at or past EOF
    // (mirrors Rust `.min(entries.len() - 1)` + roxmltree
    // text_pos_at EOF behaviour).
    auto it = std::lower_bound(entries_.begin(), entries_.end(), offset,
                               [](const Entry &e, size_t off) {
                                   return e.expanded_end <= off;
                               });
    if (it == entries_.end()) {
        return entries_.back();
    }
    return *it;
}

size_t rowcol_to_offset(std::string_view expanded_text, uint32_t row, uint32_t col) {
    // 1-based → 0-based; saturating on underflow so row=0 / col=0
    // (which roxmltree never emits but defensive callers might)
    // map to the document start. Mirrors Rust `rowcol_to_offset`.
    const size_t row0 = row > 0 ? static_cast<size_t>(row - 1) : 0;
    const size_t col0 = col > 0 ? static_cast<size_t>(col - 1) : 0;
    const std::vector<size_t> line_starts = build_line_starts(expanded_text);
    const size_t line_idx = std::min(row0, line_starts.empty() ? 0 : line_starts.size() - 1);
    const size_t line_start = line_starts[line_idx];
    // Line end excludes the trailing '\n' byte when the line is
    // not the last; the last line runs to EOF. Mirrors Rust's
    // `line_starts.get(line_idx + 1).map(|&s| s - 1)` fallback to
    // `expanded_text.len()`.
    const size_t line_end = (line_idx + 1 < line_starts.size())
                                ? line_starts[line_idx + 1] - 1
                                : expanded_text.size();
    const std::string_view line_slice = expanded_text.substr(line_start, line_end - line_start);
    // Walk col0 scalar values into line_slice, clamping at line end.
    size_t bytes_into_line = line_slice.size();
    size_t scalar_idx = 0;
    for (size_t i = 0; i < line_slice.size(); ++i) {
        const unsigned char b = static_cast<unsigned char>(line_slice[i]);
        if ((b & 0xC0) == 0x80) {
            // UTF-8 continuation byte — part of the current scalar.
            continue;
        }
        if (scalar_idx == col0) {
            bytes_into_line = i;
            break;
        }
        ++scalar_idx;
    }
    return line_start + bytes_into_line;
}

}  // namespace SCE::parsing
