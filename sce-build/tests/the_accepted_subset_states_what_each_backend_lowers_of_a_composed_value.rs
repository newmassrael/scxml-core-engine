// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `docs/SCE_ACCEPTED_SUBSET.md` section 2.15 lists what each backend lowers
//! of a composed value (docs/adr/0086, section 7.3), a row per backend, and
//! the harness that runs the composed fixtures on each backend that reads
//! one. The rows are the one table of what is lowered
//! (`ComposedLowering::of`, `sce-build/src/forge/event_schema_check.rs`)
//! rendered, so the document cannot state a row the build does not hold, and
//! a backend whose row comes to read a composed payload names its harness
//! there. The fixtures name no backend, so this list is the one place a
//! backend's step writes down that it lowers them.
//!
//! # What is held
//!
//! * The rows between the markers are, line for line, each backend's row of
//!   the table in `Language::ALL` order and then the Interpreter's lowering.
//! * Each backend whose row reads a composed payload has a line in the list of
//!   harnesses, and no other backend has one.

use sce_build::forge::event_schema_check::ComposedLowering;
use sce_build::generator::Language;

const SUBSET_DOC: &str = include_str!("../../docs/SCE_ACCEPTED_SUBSET.md");

const ROWS_BEGIN: &str = "<!-- composed-lowering-rows: begin -->";
const ROWS_END: &str = "<!-- composed-lowering-rows: end -->";
const HARNESSES_BEGIN: &str = "<!-- composed-lowering-harnesses: begin -->";
const HARNESSES_END: &str = "<!-- composed-lowering-harnesses: end -->";

/// The text of the document between `begin` and `end`, each present once.
fn between(begin: &str, end: &str) -> &'static str {
    assert_eq!(
        SUBSET_DOC.matches(begin).count(),
        1,
        "docs/SCE_ACCEPTED_SUBSET.md holds `{begin}` once"
    );
    let (_, after) = SUBSET_DOC
        .split_once(begin)
        .unwrap_or_else(|| panic!("`{begin}` in docs/SCE_ACCEPTED_SUBSET.md"));
    let (inside, _) = after
        .split_once(end)
        .unwrap_or_else(|| panic!("`{end}` after `{begin}` in docs/SCE_ACCEPTED_SUBSET.md"));
    inside
}

/// Every lowering the table answers for: each backend's, in `Language::ALL`
/// order, and the Interpreter's.
fn lowerings() -> Vec<ComposedLowering> {
    Language::ALL
        .iter()
        .map(|language| ComposedLowering::of(*language))
        .chain(std::iter::once(ComposedLowering::ECMASCRIPT))
        .collect()
}

/// The line the document states for `lowering`: the positions it lowers and
/// those it refuses by name, each in the table's order, `none` for an empty
/// side.
fn row(lowering: ComposedLowering) -> String {
    let positions = [
        ("reads", lowering.reads),
        ("copies", lowering.copies),
        ("unions", lowering.unions),
        ("records", lowering.records),
        ("sends", lowering.sends),
    ];
    let side = |lowered: bool| {
        let named: Vec<&str> = positions
            .iter()
            .filter(|(_, held)| *held == lowered)
            .map(|(name, _)| *name)
            .collect();
        if named.is_empty() {
            "none".to_string()
        } else {
            named.join(", ")
        }
    };
    format!(
        "- `{}` lowers {}; refuses {}",
        lowering.name,
        side(true),
        side(false)
    )
}

/// The lines of a list between two markers that open an item (`- `), each
/// trimmed of the space around it; the blank lines that keep two items apart
/// and an item's continuation lines are left out.
fn items(list: &str) -> Vec<String> {
    list.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- "))
        .map(str::to_string)
        .collect()
}

/// Pinned by `ComposedLowering::of`: the document's rows are the table's.
#[test]
fn the_rows_the_document_states_are_the_table() {
    let stated = items(between(ROWS_BEGIN, ROWS_END));
    let derived: Vec<String> = lowerings().into_iter().map(row).collect();
    assert_eq!(
        stated, derived,
        "docs/SCE_ACCEPTED_SUBSET.md (\"Which backend lowers a composed value\") \
         states rows the table does not hold: write each row as derived, one item \
         per backend, a blank line between two items"
    );
}

/// Pinned by `ComposedLowering::of`'s `reads`: a backend that reads a composed
/// payload runs the composed fixtures in a harness the document names, and a
/// backend that reads none names no harness.
#[test]
fn each_backend_that_reads_a_composed_payload_names_its_harness() {
    let named: Vec<String> = items(between(HARNESSES_BEGIN, HARNESSES_END))
        .iter()
        .map(|item| {
            item.strip_prefix("- `")
                .and_then(|rest| rest.split_once("`:"))
                .map(|(name, _)| name.to_string())
                .unwrap_or_else(|| panic!("a harness item opens `- `<backend>`:`: {item}"))
        })
        .collect();
    let reading: Vec<String> = lowerings()
        .into_iter()
        .filter(|lowering| lowering.reads)
        .map(|lowering| lowering.name.to_string())
        .collect();
    assert!(!reading.is_empty(), "some backend reads a composed payload");
    assert_eq!(
        named, reading,
        "docs/SCE_ACCEPTED_SUBSET.md names a harness for exactly the backends whose \
         row reads a composed payload, in the table's order"
    );
}
