// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Byte-level mapping from an expanded SCXML document back to its
// source origins, so diagnostics that fire *after* preprocessor
// expansion (XInclude, and later sce:template) can report author
// file/row/col instead of in-memory expanded coordinates.
//
// # Problem
//
// The AOT pipeline runs
//   source bytes → xinclude::expand → [template::expand] → parse_impl
// Every post-expansion diagnostic (XSD line numbers, roxmltree
// parse row/col, semantic validation on the expanded tree) names
// positions in the *expanded* in-memory string — not in any file the
// operator can open. Without a side-channel that records which
// expanded byte came from which source, a `NotFound` XSD error at
// "expanded line 127" is unactionable: line 127 of the expanded
// document is an artefact of the preprocessor and does not exist on
// disk.
//
// # Shape
//
// A `PositionMap` is a sorted list of half-open byte ranges over the
// expanded document. Each range carries an [`Origin`] describing
// where those bytes came from. Two origin kinds exist:
//
//   * `Origin::File`       — bytes copied 1:1 from a source file
//                            (the outer document, or an XInclude
//                            fragment). The mapping is affine within
//                            the entry: `source_offset +
//                            (expanded_offset - expanded_start)`.
//
//   * `Origin::CallSite`   — bytes synthesised by template parameter
//                            substitution. Every byte in the range
//                            shares the same (row, col) — column
//                            precision *inside* the substituted
//                            value is deliberately collapsed per
//                            RFC §6.3 Q3 "depth-1" rule. Consumed by
//                            the sce:template expander; not produced
//                            by the XInclude expander.
//
// Entries are contiguous, non-overlapping, and cover `[0, len)` of
// the expanded text. The identity case is a single `Origin::File`
// entry covering the whole document — used by `parse_string`
// (no filesystem) and by documents with no `<xi:include>` /
// `<sce:use>`.
//
// # Lookup
//
// `lookup(expanded_offset)` returns a [`SourcePos`] naming the
// author file and 1-based (row, col) in that file. Callers that
// have an expanded (row, col) instead of a byte offset use
// [`rowcol_to_offset`] first — (row, col) ↔ byte conversion over
// the expanded text is kept as a free function rather than stashed
// on the map so the map stays cheap to construct even when the
// expanded text is never handed over.
//
// # Memory shape
//
// The map owns the *text* of every source file it references, held
// as `Arc<str>` so composition across preprocessor stages can share
// buffers. This is intentional: a diagnostic that maps back to an
// included file needs the included file's text to translate an
// offset into (row, col), and re-reading the file on every lookup
// is both slow and racy (the file can be modified between read and
// lookup). Total memory is O(Σ source file sizes) — bounded and
// dominated by the outer document in practice.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// An origin-specific source location: the author file, 1-based
/// row, 1-based column — matching roxmltree's [`roxmltree::TextPos`]
/// convention used everywhere else in the forge error pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePos {
    pub file: PathBuf,
    pub row: u32,
    pub col: u32,
}

/// Where an expanded-document byte range came from.
///
/// Marked `#[non_exhaustive]` so a future third variant (e.g.
/// fully-synthesised bytes with no source pointer, should such a
/// case ever be needed) can be added without breaking external
/// consumers — per `feedback_built_but_unconsumed.md` we do not
/// add the variant speculatively, but we leave the enum extensible.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Origin {
    /// Bytes copied 1:1 from `path` starting at `source_offset`.
    /// Expanded offset `k` in this entry's range maps to
    /// `source_offset + (k - expanded_start)` in `path`.
    File { path: PathBuf, source_offset: usize },

    /// Bytes synthesised by template parameter substitution. All
    /// bytes in the entry's range map to the single (row, col) of
    /// the `<sce:param>` element that supplied the value —
    /// depth-1 per RFC §6.3 Q3. Consumed by the sce:template
    /// expander; the XInclude expander does not produce this
    /// variant.
    CallSite { path: PathBuf, row: u32, col: u32 },
}

#[derive(Debug, Clone)]
struct Entry {
    expanded_start: usize,
    expanded_end: usize,
    origin: Origin,
}

/// Line-start byte offsets for a source file, keyed alongside its
/// text so offset → (row, col) translation stays O(log N) per
/// lookup (binary search into `line_starts`, then a linear
/// char-count inside the matched line).
#[derive(Debug, Clone)]
struct FileText {
    text: Arc<str>,
    line_starts: Arc<[usize]>,
}

impl FileText {
    fn new(text: &str) -> Self {
        let arc: Arc<str> = Arc::from(text);
        let line_starts = build_line_starts(&arc);
        Self {
            text: arc,
            line_starts: Arc::from(line_starts),
        }
    }
}

/// A byte-range mapping from the expanded document back to source
/// origins. See the module comment for semantics.
#[derive(Debug, Clone, Default)]
pub struct PositionMap {
    entries: Vec<Entry>,
    files: HashMap<PathBuf, FileText>,
}

impl PositionMap {
    /// Identity map: the whole expanded document is a 1:1 copy of
    /// `text` from the file labelled `file`. Used by the
    /// short-circuit path in [`crate::xinclude::expand`] when no
    /// `<xi:include>` is present, and by
    /// [`crate::parser::SCXMLParser::parse_string`] when there is
    /// no filesystem path.
    pub fn identity(file: impl Into<PathBuf>, text: &str) -> Self {
        let path = file.into();
        let len = text.len();
        let mut files = HashMap::new();
        files.insert(path.clone(), FileText::new(text));
        Self {
            entries: vec![Entry {
                expanded_start: 0,
                expanded_end: len,
                origin: Origin::File {
                    path,
                    source_offset: 0,
                },
            }],
            files,
        }
    }

    /// Translate an expanded byte offset to its source (file, row,
    /// col). Panics on an empty map (must be constructed with at
    /// least one entry — all public constructors ensure this) but
    /// is tolerant of `offset` that lies at or past the last
    /// entry's `expanded_end`: it clamps to the last entry so a
    /// diagnostic at EOF still resolves rather than panicking.
    ///
    /// The clamp choice matches roxmltree's behaviour for
    /// `text_pos_at` at end-of-document — both return the
    /// last valid position rather than an error.
    pub fn lookup(&self, expanded_offset: usize) -> SourcePos {
        assert!(
            !self.entries.is_empty(),
            "PositionMap must contain at least one entry"
        );
        let entry = self.entry_for(expanded_offset);
        match &entry.origin {
            Origin::File {
                path,
                source_offset,
            } => {
                let delta = expanded_offset.saturating_sub(entry.expanded_start);
                let src_offset = source_offset + delta;
                let ft = self.files.get(path).expect(
                    "Origin::File referenced a path with no stored text — PositionMap \
                     construction must register the file before adding the entry",
                );
                let (row, col) = offset_to_rowcol(&ft.text, &ft.line_starts, src_offset);
                SourcePos {
                    file: path.clone(),
                    row,
                    col,
                }
            }
            Origin::CallSite { path, row, col } => SourcePos {
                file: path.clone(),
                row: *row,
                col: *col,
            },
        }
    }

    /// Return the entry covering `offset`, clamped to the last
    /// entry if `offset` is at or past the expanded document's
    /// end.
    fn entry_for(&self, offset: usize) -> &Entry {
        // partition_point returns the first index `i` where the
        // predicate is false. `expanded_end <= offset` means "this
        // entry is entirely before offset"; the first entry for
        // which that is false is the one containing offset.
        let idx = self
            .entries
            .partition_point(|e| e.expanded_end <= offset)
            .min(self.entries.len() - 1);
        &self.entries[idx]
    }

    /// True when the map has exactly one `Origin::File` entry
    /// whose `source_offset == 0` and whose range is the full
    /// expanded length — i.e. no preprocessor expansion happened.
    /// Callers can short-circuit remap work in this case (common
    /// hot path: documents without XInclude).
    pub fn is_identity(&self) -> bool {
        self.entries.len() == 1
            && matches!(
                self.entries[0].origin,
                Origin::File {
                    source_offset: 0,
                    ..
                }
            )
    }

    // ── builder-style API (consumed by xinclude::expand) ──

    /// Register `text` as the content of `path` so later entries
    /// referencing `path` can resolve offsets to (row, col).
    /// Idempotent: calling twice with the same path is a no-op on
    /// the second call (first call's text wins, which is correct
    /// because every file is read exactly once per expansion).
    pub(crate) fn register_file(&mut self, path: PathBuf, text: &str) {
        self.files
            .entry(path)
            .or_insert_with(|| FileText::new(text));
    }

    /// Append an entry covering `[expanded_start, expanded_end)`.
    /// Callers must push entries in ascending `expanded_start`
    /// order and must ensure they form a contiguous cover of the
    /// expanded document before calling [`lookup`].
    pub(crate) fn push_entry(
        &mut self,
        expanded_start: usize,
        expanded_end: usize,
        origin: Origin,
    ) {
        debug_assert!(
            expanded_end >= expanded_start,
            "entry ranges must be non-negative"
        );
        if let Some(last) = self.entries.last() {
            debug_assert_eq!(
                last.expanded_end, expanded_start,
                "entries must be contiguous"
            );
        } else {
            debug_assert_eq!(
                expanded_start, 0,
                "first entry must start at expanded offset 0"
            );
        }
        self.entries.push(Entry {
            expanded_start,
            expanded_end,
            origin,
        });
    }

    /// Append entries from `inner` that overlap `[inner_start,
    /// inner_end)`, remapping their expanded offsets to begin at
    /// `outer_start` in `self`. Copies any file texts `inner`
    /// registered that are not already in `self`.
    ///
    /// Used by xinclude recursion: the included file's expansion
    /// produced its own `PositionMap`; when the outer splice
    /// extracts a substring of that expansion (the root element's
    /// children), the outer map must inherit the inner map's
    /// byte-level origin knowledge for exactly that substring.
    ///
    /// # Invariants
    ///
    /// * Callers must invoke this in ascending `outer_start` order
    ///   and keep the resulting entries contiguous with prior
    ///   entries — same rules as [`push_entry`].
    /// * `inner_end` must not exceed `inner`'s expanded length; a
    ///   range spilling past the inner document silently produces
    ///   fewer entries than expected (no-overlap entries are
    ///   skipped).
    pub(crate) fn append_mapped_substring(
        &mut self,
        inner: &PositionMap,
        inner_start: usize,
        inner_end: usize,
        outer_start: usize,
    ) {
        debug_assert!(inner_end >= inner_start, "inner range must be non-negative");
        // Copy inner's file texts — idempotent.
        for (path, ft) in &inner.files {
            self.files.entry(path.clone()).or_insert_with(|| ft.clone());
        }
        // Walk inner's entries, clip to [inner_start, inner_end),
        // and remap to outer coordinates.
        for entry in &inner.entries {
            if entry.expanded_end <= inner_start || entry.expanded_start >= inner_end {
                continue;
            }
            let clip_start = entry.expanded_start.max(inner_start);
            let clip_end = entry.expanded_end.min(inner_end);
            if clip_end <= clip_start {
                continue;
            }
            let new_start = outer_start + (clip_start - inner_start);
            let new_end = outer_start + (clip_end - inner_start);
            let new_origin = match &entry.origin {
                Origin::File {
                    path,
                    source_offset,
                } => {
                    // The inner entry covers [entry.expanded_start,
                    // entry.expanded_end) → source offsets
                    // [source_offset, source_offset + entry_len).
                    // Clipping off the left advances the source
                    // offset by the same amount.
                    let delta = clip_start - entry.expanded_start;
                    Origin::File {
                        path: path.clone(),
                        source_offset: source_offset + delta,
                    }
                }
                Origin::CallSite { path, row, col } => Origin::CallSite {
                    path: path.clone(),
                    row: *row,
                    col: *col,
                },
            };
            self.push_entry(new_start, new_end, new_origin);
        }
    }
}

/// Build the line-start byte-offset table for `text`.
///
/// Returns a vector whose length equals `1 + newline_count`.
/// Element 0 is always 0 (start of line 1). Each subsequent
/// element is the byte offset *immediately after* a newline.
fn build_line_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::with_capacity(1 + text.bytes().filter(|&b| b == b'\n').count());
    starts.push(0);
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Translate a byte offset into 1-based (row, col) within `text`,
/// using the precomputed `line_starts` table.
///
/// Column counts Unicode *scalar values* rather than bytes, to
/// match the convention roxmltree's [`roxmltree::TextPos`] uses and
/// to match the way every existing `forge::error::SourceLocation`
/// is rendered. Byte-based counting would under-report column on
/// multibyte characters and create an invisible skew between
/// remapped and non-remapped diagnostics.
fn offset_to_rowcol(text: &str, line_starts: &[usize], byte_offset: usize) -> (u32, u32) {
    debug_assert!(
        !line_starts.is_empty(),
        "line_starts always has at least element 0"
    );
    let clamped = byte_offset.min(text.len());
    // Largest line-start ≤ clamped. partition_point returns the
    // first index where `start > clamped`, so the row index is
    // one less.
    let row_idx = line_starts
        .partition_point(|&s| s <= clamped)
        .saturating_sub(1);
    let row = (row_idx as u32).saturating_add(1);
    let line_start = line_starts[row_idx];
    let line_prefix = &text[line_start..clamped];
    let col_chars = line_prefix.chars().count();
    let col = (col_chars as u32).saturating_add(1);
    (row, col)
}

/// Translate 1-based (row, col) within `expanded_text` into a byte
/// offset, for callers that receive row/col from roxmltree /
/// libxml2 but need a byte offset to pass to
/// [`PositionMap::lookup`].
///
/// Exposed as a free function (not a method on `PositionMap`)
/// because the map does not own the expanded text — callers that
/// already hold the string in `parse_impl` pass it explicitly.
/// Clamps `row` and `col` so diagnostics at or past EOF still
/// translate to a valid offset rather than panicking.
pub fn rowcol_to_offset(expanded_text: &str, row: u32, col: u32) -> usize {
    // 1-based → 0-based; saturate on underflow so row=0 / col=0
    // (which roxmltree never emits but defensive callers might)
    // map to the document start.
    let row0 = row.saturating_sub(1) as usize;
    let col0 = col.saturating_sub(1) as usize;
    let line_starts = build_line_starts(expanded_text);
    let line_idx = row0.min(line_starts.len().saturating_sub(1));
    let line_start = line_starts[line_idx];
    // Walk `col0` chars into the matched line, clamping at the
    // line's end (next newline or EOF).
    let line_end = line_starts
        .get(line_idx + 1)
        .map_or(expanded_text.len(), |&s| s - 1);
    let line_slice = &expanded_text[line_start..line_end];
    let mut bytes_into_line = line_slice.len();
    for (i, (b, _)) in line_slice.char_indices().enumerate() {
        if i == col0 {
            bytes_into_line = b;
            break;
        }
    }
    line_start + bytes_into_line
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drift test pinning the C++ `Origin` variant shape against
    /// the Rust `Origin` enum declared above. Follows the
    /// `cpp_template_subtypes_match_rust_diagnostic_codes` precedent
    /// in `sce-build/src/template.rs::tests` (Phase B M4): read the
    /// authoritative C++ header via `include_str!` at test compile
    /// time, regex-scan the struct declarations, assert set
    /// equality of struct names and field-name-and-type-family
    /// allowlists.
    ///
    /// What this test pins (Phase C RFC §1 Q4):
    ///
    ///   * Both `FileOrigin` and `CallSiteOrigin` struct declarations
    ///     exist in the C++ header.
    ///   * `FileOrigin` declares exactly two fields: one path-typed,
    ///     one size/integer-typed named `source_offset`.
    ///   * `CallSiteOrigin` declares exactly three fields: one
    ///     path-typed, two integer-typed named `row` and `col`.
    ///   * Path-family types: `std::filesystem::path` (C++) /
    ///     `PathBuf` / `std::string_view`.
    ///   * Integer-family types: `size_t` / `uint32_t` / `uint64_t` /
    ///     `u32` / `usize`.
    ///
    /// What this test does NOT pin: exact C++ types (so
    /// portability-driven swaps like `size_t` → `uint64_t` stay
    /// allowed), method signatures, `std::variant` discriminant
    /// order (the Rust `#[non_exhaustive]` enum's discriminants
    /// evolve independently), implementation shape of `lookup` or
    /// `append_mapped_substring`. Those are covered by the P1
    /// C++ unit tests in `tests/parsing/PositionMap_test.cpp`
    /// (implementation-level correctness) rather than drift.
    ///
    /// Load-bearing: rename `FileOrigin` to `FileOriginRenamed` in
    /// the C++ header → this test reds with a pointed missing-struct
    /// assertion instead of passing silently.
    #[test]
    fn cpp_origin_shape_matches_rust() {
        let hdr = include_str!("../../sce/include/parsing/PositionMap.h");

        // Path-family allowlist — any of these types is acceptable
        // for a path-typed field on either side.
        let path_family = r"(?:std::filesystem::path|PathBuf|std::string_view)";
        // Integer-family allowlist — any of these types is acceptable
        // for a size/offset/row/col field. Portability-driven swaps
        // within this set do not red the test.
        let int_family = r"(?:size_t|uint32_t|uint64_t|u32|usize)";

        // FileOrigin: `struct FileOrigin { <path-family> path; <int-family> source_offset{...}; };`
        // Match liberally on field initialisers (brace-or-paren or
        // nothing) and whitespace to survive benign header reflows.
        let file_origin_re = regex::Regex::new(&format!(
            r"struct\s+FileOrigin\s*\{{\s*{path}\s+path\s*(?:\{{[^}}]*\}})?\s*;\s*{int}\s+source_offset\s*(?:\{{[^}}]*\}})?\s*;\s*\}}\s*;",
            path = path_family,
            int = int_family,
        ))
        .expect("FileOrigin regex must compile");

        // CallSiteOrigin: path + two int-family fields named row, col.
        let call_site_origin_re = regex::Regex::new(&format!(
            r"struct\s+CallSiteOrigin\s*\{{\s*{path}\s+path\s*(?:\{{[^}}]*\}})?\s*;\s*{int}\s+row\s*(?:\{{[^}}]*\}})?\s*;\s*{int}\s+col\s*(?:\{{[^}}]*\}})?\s*;\s*\}}\s*;",
            path = path_family,
            int = int_family,
        ))
        .expect("CallSiteOrigin regex must compile");

        assert!(
            file_origin_re.is_match(hdr),
            "sce/include/parsing/PositionMap.h must declare `struct \
             FileOrigin {{ <path-family> path; <int-family> \
             source_offset; }};` matching the Rust `Origin::File` \
             variant. If the field order or type family changed, \
             update this drift test in the same commit — see RFC \
             §1 Q4 (claudedocs/rfc-sce-template-phase-c.md) for the \
             invariant. Path family allowlist: {path}. Integer \
             family allowlist: {int}.",
            path = path_family,
            int = int_family,
        );

        assert!(
            call_site_origin_re.is_match(hdr),
            "sce/include/parsing/PositionMap.h must declare `struct \
             CallSiteOrigin {{ <path-family> path; <int-family> row; \
             <int-family> col; }};` matching the Rust \
             `Origin::CallSite` variant. If the field order or type \
             family changed, update this drift test in the same \
             commit — see RFC §1 Q4 \
             (claudedocs/rfc-sce-template-phase-c.md) for the \
             invariant. Path family allowlist: {path}. Integer \
             family allowlist: {int}.",
            path = path_family,
            int = int_family,
        );

        // Cross-check: the C++ header must also expose the
        // `SCE::parsing::Origin` alias as a `std::variant` over
        // both structs. A future rewrite that drops `std::variant`
        // (e.g. switching to a tagged class hierarchy) loses the
        // 1:1 shape agreement with Rust's enum and the P2 wiring
        // contract would silently diverge.
        let origin_variant_re = regex::Regex::new(
            r"using\s+Origin\s*=\s*std::variant\s*<\s*FileOrigin\s*,\s*CallSiteOrigin\s*>\s*;",
        )
        .expect("Origin variant regex must compile");
        assert!(
            origin_variant_re.is_match(hdr),
            "sce/include/parsing/PositionMap.h must alias \
             `SCE::parsing::Origin` as `std::variant<FileOrigin, \
             CallSiteOrigin>` — the 1:1 shape agreement with the \
             Rust `Origin` enum depends on this exact pair. \
             Adding a third variant requires updating the Rust \
             enum and this drift test in the same commit."
        );
    }

    #[test]
    fn identity_single_line_ascii() {
        let text = "<root/>";
        let map = PositionMap::identity("main.scxml", text);
        assert!(map.is_identity());

        let pos = map.lookup(0);
        assert_eq!(pos.file, PathBuf::from("main.scxml"));
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 1);

        let pos = map.lookup(3);
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 4);
    }

    #[test]
    fn identity_multi_line_ascii() {
        let text = "<root>\n  <state id=\"s1\"/>\n</root>";
        let map = PositionMap::identity("main.scxml", text);

        // Offset of '<state' — after "<root>\n  "
        let offset = text.find("<state").unwrap();
        let pos = map.lookup(offset);
        assert_eq!(pos.row, 2);
        assert_eq!(pos.col, 3);

        // Offset of '</root>'
        let offset = text.find("</root>").unwrap();
        let pos = map.lookup(offset);
        assert_eq!(pos.row, 3);
        assert_eq!(pos.col, 1);
    }

    #[test]
    fn identity_eof_is_clamped_not_panicking() {
        let text = "<root/>";
        let map = PositionMap::identity("main.scxml", text);
        // One past end is tolerated by both entry_for (via min
        // clamp) and offset_to_rowcol (via saturating/clamp). The
        // result names EOF as (row 1, col len+1) — consistent with
        // roxmltree's text_pos_at behaviour.
        let pos = map.lookup(text.len() + 1);
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, (text.len() as u32) + 1);
    }

    #[test]
    fn column_counts_unicode_scalars_not_bytes() {
        // "한" is three UTF-8 bytes but one Unicode scalar — column
        // 2 must point at the character after it, not at byte 3.
        let text = "한abc";
        let map = PositionMap::identity("main.scxml", text);

        let byte_offset = "한".len();
        let pos = map.lookup(byte_offset);
        // One scalar ("한") has been traversed → column 2.
        assert_eq!(pos.row, 1);
        assert_eq!(pos.col, 2);
    }

    #[test]
    fn hand_crafted_xinclude_shaped_map_resolves_to_included_file() {
        // Simulate: outer.scxml contains "<pfx>{INC}</pfx>" where
        // {INC} is a 5-byte fragment spliced in from frag.xml byte
        // offset 7 (matching a real included file's 5 bytes).
        //
        // Expanded layout:
        //   expanded bytes [0, 5)   → outer.scxml[0..5]   ("<pfx>")
        //   expanded bytes [5, 10)  → frag.xml[7..12]     ("<x/>\n")
        //   expanded bytes [10, 16) → outer.scxml[5..11]  ("</pfx>")
        let outer_text = "<pfx></pfx>"; // 11 bytes
        let frag_text = "header:<x/>\nfoot"; // "<x/>\n" starts at byte 7
        let mut map = PositionMap::default();
        map.register_file(PathBuf::from("outer.scxml"), outer_text);
        map.register_file(PathBuf::from("frag.xml"), frag_text);
        map.push_entry(
            0,
            5,
            Origin::File {
                path: PathBuf::from("outer.scxml"),
                source_offset: 0,
            },
        );
        map.push_entry(
            5,
            10,
            Origin::File {
                path: PathBuf::from("frag.xml"),
                source_offset: 7,
            },
        );
        map.push_entry(
            10,
            16,
            Origin::File {
                path: PathBuf::from("outer.scxml"),
                source_offset: 5,
            },
        );

        // Expanded offset 0 → outer.scxml (1,1)
        let p = map.lookup(0);
        assert_eq!(p.file, PathBuf::from("outer.scxml"));
        assert_eq!((p.row, p.col), (1, 1));

        // Expanded offset 6 → frag.xml[8] = 'x' (row 1, col after "header:<")
        let p = map.lookup(6);
        assert_eq!(p.file, PathBuf::from("frag.xml"));
        assert_eq!(p.row, 1);
        // 7 + (6 - 5) = 8 → char index 8 on line 1 = col 9
        assert_eq!(p.col, 9);

        // Expanded offset 12 → outer.scxml[7] = 'f' of "fx>"
        let p = map.lookup(12);
        assert_eq!(p.file, PathBuf::from("outer.scxml"));
        assert_eq!(p.row, 1);
        assert_eq!(p.col, 8);
    }

    #[test]
    fn callsite_origin_returns_stored_rowcol_regardless_of_offset() {
        // A substitution region's every byte maps to the same
        // (row, col) — depth-1 collapse.
        let outer_text = "<outer/>";
        let mut map = PositionMap::default();
        map.register_file(PathBuf::from("caller.scxml"), outer_text);
        map.push_entry(
            0,
            10,
            Origin::CallSite {
                path: PathBuf::from("caller.scxml"),
                row: 42,
                col: 17,
            },
        );

        for offset in [0usize, 3, 9] {
            let p = map.lookup(offset);
            assert_eq!(p.file, PathBuf::from("caller.scxml"));
            assert_eq!(p.row, 42);
            assert_eq!(p.col, 17);
        }
    }

    #[test]
    fn is_identity_rejects_multi_entry_maps() {
        let outer_text = "<a/>";
        let mut map = PositionMap::default();
        map.register_file(PathBuf::from("x.scxml"), outer_text);
        map.push_entry(
            0,
            2,
            Origin::File {
                path: PathBuf::from("x.scxml"),
                source_offset: 0,
            },
        );
        map.push_entry(
            2,
            4,
            Origin::File {
                path: PathBuf::from("x.scxml"),
                source_offset: 2,
            },
        );
        assert!(!map.is_identity());
    }

    #[test]
    fn is_identity_rejects_offset_file_entry() {
        // Single entry but source_offset != 0 → not identity.
        let mut map = PositionMap::default();
        map.register_file(PathBuf::from("x.scxml"), "zzz<a/>");
        map.push_entry(
            0,
            4,
            Origin::File {
                path: PathBuf::from("x.scxml"),
                source_offset: 3,
            },
        );
        assert!(!map.is_identity());
    }

    #[test]
    fn rowcol_to_offset_ascii() {
        let text = "<root>\n  <state/>\n</root>";
        // (1, 1) → 0
        assert_eq!(rowcol_to_offset(text, 1, 1), 0);
        // (2, 1) → after "<root>\n" = 7
        assert_eq!(rowcol_to_offset(text, 2, 1), 7);
        // (2, 3) → '<state/>' = 9
        assert_eq!(rowcol_to_offset(text, 2, 3), 9);
        // (3, 1) → after "<root>\n  <state/>\n" = 18
        assert_eq!(rowcol_to_offset(text, 3, 1), 18);
    }

    #[test]
    fn rowcol_to_offset_past_eof_clamps() {
        let text = "<a/>";
        // Defensive: row past EOF clamps to last line; col past
        // EOL clamps to line end.
        let offset = rowcol_to_offset(text, 99, 99);
        assert!(offset <= text.len());
    }

    #[test]
    fn rowcol_to_offset_unicode_column() {
        // "한" is 3 bytes but 1 char. (1, 2) must point at the
        // byte *after* "한" = byte offset 3.
        let text = "한abc";
        assert_eq!(rowcol_to_offset(text, 1, 2), "한".len());
        assert_eq!(rowcol_to_offset(text, 1, 3), "한".len() + 1);
    }

    #[test]
    fn rowcol_to_offset_zero_saturates_to_start() {
        // Defensive: row=0 or col=0 (roxmltree never emits these,
        // but defensive callers might) saturates to row 1 / col 1
        // rather than underflowing. A 0-valued coordinate is
        // treated as if it were 1 on that axis — so (0, 0) maps
        // to the document start, (0, 5) maps to col 5 of line 1
        // (clamped to line end if shorter), and (1, 0) maps to
        // the start of line 1.
        let text = "abc";
        assert_eq!(rowcol_to_offset(text, 0, 0), 0);
        // (0, 5) → (1, 5) → clamp to line end = 3 (|"abc"|).
        assert_eq!(rowcol_to_offset(text, 0, 5), 3);
        assert_eq!(rowcol_to_offset(text, 1, 0), 0);
    }

    #[test]
    fn build_line_starts_no_newlines() {
        let starts = build_line_starts("abc");
        assert_eq!(starts, vec![0]);
    }

    #[test]
    fn build_line_starts_trailing_newline() {
        let starts = build_line_starts("a\nb\n");
        // Lines start at 0, 2, 4 — the offset *after* the final
        // newline is a valid (empty) line start.
        assert_eq!(starts, vec![0, 2, 4]);
    }

    #[test]
    fn append_mapped_substring_identity_sub_composes_into_outer() {
        // Inner map: identity over "abcdef" (6 bytes) from inner.xml.
        let inner_text = "abcdef";
        let inner = PositionMap::identity(PathBuf::from("inner.xml"), inner_text);

        // Outer: [0, 3) is from outer.xml, [3, 6) is a splice from
        // inner.xml bytes [2, 5) — i.e. "cde".
        let outer_text = "out";
        let mut outer = PositionMap::default();
        outer.register_file(PathBuf::from("outer.xml"), outer_text);
        outer.push_entry(
            0,
            3,
            Origin::File {
                path: PathBuf::from("outer.xml"),
                source_offset: 0,
            },
        );
        outer.append_mapped_substring(&inner, 2, 5, 3);

        // Expanded byte 0 (outer "o") → outer.xml (1, 1)
        let p = outer.lookup(0);
        assert_eq!(p.file, PathBuf::from("outer.xml"));
        assert_eq!((p.row, p.col), (1, 1));

        // Expanded byte 3 → inner.xml[2] = 'c'
        let p = outer.lookup(3);
        assert_eq!(p.file, PathBuf::from("inner.xml"));
        assert_eq!((p.row, p.col), (1, 3));

        // Expanded byte 5 → inner.xml[4] = 'e'
        let p = outer.lookup(5);
        assert_eq!(p.file, PathBuf::from("inner.xml"));
        assert_eq!((p.row, p.col), (1, 5));
    }

    #[test]
    fn append_mapped_substring_skips_non_overlapping_entries() {
        // Inner has two entries; we only splice the right half.
        let inner_text_a = "xxxxx"; // 5 bytes — file "a.xml"
        let inner_text_b = "yyyyy"; // 5 bytes — file "b.xml"
        let mut inner = PositionMap::default();
        inner.register_file(PathBuf::from("a.xml"), inner_text_a);
        inner.register_file(PathBuf::from("b.xml"), inner_text_b);
        inner.push_entry(
            0,
            5,
            Origin::File {
                path: PathBuf::from("a.xml"),
                source_offset: 0,
            },
        );
        inner.push_entry(
            5,
            10,
            Origin::File {
                path: PathBuf::from("b.xml"),
                source_offset: 0,
            },
        );

        // Splice only the right half (inner bytes [5, 10)) into outer.
        let mut outer = PositionMap::default();
        outer.register_file(PathBuf::from("outer.xml"), "o");
        outer.push_entry(
            0,
            1,
            Origin::File {
                path: PathBuf::from("outer.xml"),
                source_offset: 0,
            },
        );
        outer.append_mapped_substring(&inner, 5, 10, 1);

        // Expanded byte 3 should now point at b.xml, not a.xml —
        // the a.xml entry was out of the clip range.
        let p = outer.lookup(3);
        assert_eq!(p.file, PathBuf::from("b.xml"));
        assert_eq!((p.row, p.col), (1, 3));
    }
}
