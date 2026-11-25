// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Rust codec reads each `<sce:tlv-chain>` by one routine of the runtime,
//! shared by every chain of one entry type (docs/adr/0136).
//!
//! The eager decode hands `read_chain_into` the list it fills and what the
//! chain is (how an entry is read with its inputs, where the chain ends, what
//! it carries, which kinds the gates ask for), and prints no loop of its own; a
//! view hands `walk_chain` its walk and its hook. What the routines do, frame by
//! frame, is held by their unit tests in `backends/rust/forge-runtime/src/chain.rs`
//! and by the round trips the conformance crate runs over the goldens.

use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// The Rust the codec `name` generates under `options`, every file joined.
fn rust_with(name: &str, options: &ForgeCompileOptions) -> String {
    let path = resource_dir().join(format!("{name}.scxml"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    sce_build::compile_forge_with_imports(
        &source,
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        options,
    )
    .unwrap_or_else(|e| panic!("{name} generates for Rust: {e}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// The Rust the codec `name` generates with its views.
fn rust_of(name: &str) -> String {
    rust_with(
        name,
        &ForgeCompileOptions {
            views: true,
            ..ForgeCompileOptions::default()
        },
    )
}

/// The bodies of every method `name` the code declares, from its signature to
/// its closing brace.
fn bodies<'c>(code: &'c str, name: &str) -> Vec<&'c str> {
    let head = format!("    pub fn {name}");
    code.match_indices(&head)
        .filter(|(at, _)| {
            let after = &code[at + head.len()..];
            after.starts_with('(') || after.starts_with('<')
        })
        .map(|(at, _)| {
            let body = &code[at..];
            let end = body
                .find("\n    }\n")
                .unwrap_or_else(|| panic!("`{name}` closes"));
            &body[..end]
        })
        .collect()
}

/// The eager decode of `code`: the one `read_into` that reads the frame, not a
/// view's, which forwards to its `read`.
fn eager_read<'c>(name: &str, code: &'c str) -> &'c str {
    bodies(code, "read_into")
        .into_iter()
        .find(|body| !body.contains("*_out = Self::read("))
        .unwrap_or_else(|| panic!("{name} has an eager read_into"))
}

/// The view's read of `code`, the one that reads with a hook.
fn view_read<'c>(name: &str, code: &'c str) -> &'c str {
    bodies(code, "read_with")
        .into_iter()
        .find(|body| body.contains("_hook: &mut H) -> Self {"))
        .unwrap_or_else(|| panic!("{name} has a view read_with"))
}

/// `body` with its whitespace read as one space, so a statement is found however
/// it is laid out.
fn flat(body: &str) -> String {
    body.split_whitespace().collect::<Vec<_>>().join(" ")
}

const RUNTIME: &str = "::sce_forge_runtime::codec::";

/// Codecs with one chain read in its own field: one to the end of its frame at
/// a depth, one closed by an entry flag that carries a value to the next entry
/// and hands it its parent's flag, the same chain gated, and a frame's chain of
/// extensions closed by an entry flag.
const ONE_CHAIN: [&str; 4] = [
    "codec_tlv_chain_basic",
    "codec_chain_prev_carrier",
    "codec_chain_prev_gated",
    "codec_zenoh_ext_envelope",
];

/// The eager decode of a chain is one call of the runtime's `read_chain_into`
/// inside its list's fill: no loop, no read of an entry, no `push_keeping` and
/// no guard of its own.
#[test]
fn an_eager_chain_is_read_by_one_call_of_the_runtime_and_no_loop_of_its_own() {
    for name in ONE_CHAIN {
        let code = rust_of(name);
        let body = eager_read(name, &code);
        let call = format!("{RUNTIME}read_chain_into::<");
        assert_eq!(
            body.matches(&call).count(),
            1,
            "{name} does not read its chain by one call of the runtime:\n{body}"
        );
        for printed in [
            "loop {",
            "for _ in 0..",
            "push_keeping(",
            "_more",
            "let _entry",
            "if cursor.failed() { break; }",
            "CodecError::TooManyElements",
        ] {
            assert!(
                !body.contains(printed),
                "{name} prints the chain's loop (`{printed}`):\n{body}"
            );
        }
    }
}

/// What a chain is reaches the routine as data: the entry's read handed its
/// inputs by position, the inputs the first entry starts with, the depth, the
/// flag that ends it and what it carries to the next entry, rewritten by the
/// test the loop made.
#[test]
fn a_chain_hands_the_routine_what_it_is() {
    let code = rust_of("codec_chain_prev_carrier");
    let body = flat(eager_read("codec_chain_prev_carrier", &code));
    for said in [
        format!("{RUNTIME}read_chain_into::<CodecChainPrevEntry, _, 2>(cursor, _vec, {RUNTIME}ChainRead {{"),
        "read: |_r, _a| CodecChainPrevEntry::read(_r, _a[0], _a[1]),".to_string(),
        "args: [0, head & 0x1],".to_string(),
        "depth: Some(4),".to_string(),
        format!("end: {RUNTIME}ChainEnd::Flag(|_e| _e.z()),"),
        "carry: Some(|_e, _a| { _a[0] = u8::from(u64::from(_e.ext_id()) == 4u64); }),".to_string(),
        "kinds: &[], }, &mut [])".to_string(),
    ] {
        assert!(
            body.contains(&said),
            "codec_chain_prev_carrier does not hand the routine `{said}`:\n{body}"
        );
    }
    let code = rust_of("codec_tlv_chain_basic");
    let body = flat(eager_read("codec_tlv_chain_basic", &code));
    for said in [
        format!("{RUNTIME}read_chain_into::<CodecTlvEntryOwned<S>, _, 0>(cursor, _vec, "),
        "read: |_r, _a| CodecTlvEntryOwned::<S>::read(_r),".to_string(),
        "args: [],".to_string(),
        "depth: Some(8),".to_string(),
        format!("end: {RUNTIME}ChainEnd::Frame,"),
        "carry: None,".to_string(),
    ] {
        assert!(
            body.contains(&said),
            "codec_tlv_chain_basic does not hand the routine `{said}`:\n{body}"
        );
    }
}

/// The kinds of entry the gates after a chain ask for are noted by the routine
/// in one list the decode declares, and each gate's local is taken from it,
/// where the chain is read for a group of its arms as well (docs/adr/0065 §6
/// R6a).
#[test]
fn the_kinds_a_gate_asks_of_a_chain_are_the_list_the_routine_notes_them_in() {
    let code = rust_of("codec_chain_has_tagged");
    let body = flat(eager_read("codec_chain_has_tagged", &code));
    for said in [
        "let mut _found_entries = [false; 2];",
        "kinds: &[|_e| u64::from(_e.entry_type) == 7u64, |_e| u64::from(_e.entry_type) == 9u64], }, &mut _found_entries)",
        "_has_entries_7 = _found_entries[0];",
        "_has_entries_9 = _found_entries[1];",
    ] {
        assert!(
            body.contains(said),
            "codec_chain_has_tagged does not note its kinds as `{said}`:\n{body}"
        );
    }
    assert!(
        !body.contains("push_keeping("),
        "codec_chain_has_tagged prints the chain's loop:\n{body}"
    );
}

/// A view scans a chain by one call of the runtime's `walk_chain` with its walk
/// and its hook, and the site the hook is told: no loop, no show of an entry and
/// no count of its own.
#[test]
fn a_view_scans_a_chain_by_one_call_of_the_runtime_with_its_walk() {
    for name in ONE_CHAIN {
        let code = rust_of(name);
        let body = view_read(name, &code);
        let call = format!("{RUNTIME}walk_chain::<");
        assert_eq!(
            body.matches(&call).count(),
            1,
            "{name}'s view does not scan its chain by one call of the runtime:\n{body}"
        );
        for printed in [
            "loop {",
            "for _ in 0..",
            "show_entry(",
            "_count +=",
            "_more",
        ] {
            assert!(
                !body.contains(printed),
                "{name}'s view prints the chain's scan (`{printed}`):\n{body}"
            );
        }
        assert!(
            body.contains("ListView::new(_rest, _count, "),
            "{name}'s view does not hold the list the scan counted:\n{body}"
        );
    }
    let code = rust_of("codec_chain_prev_carrier");
    let body = flat(view_read("codec_chain_prev_carrier", &code));
    for said in [
        format!("{RUNTIME}walk_chain::<CodecChainPrevCarrierEntriesWalk, _, 2>(cursor, _hook, {RUNTIME}ChainScan {{"),
        format!("site: {RUNTIME}ChainSite {{ codec: \"codec_chain_prev_carrier\", field: \"entries\" }},"),
        "args: [0, head & 0x1],".to_string(),
        "depth: Some(4),".to_string(),
        format!("end: {RUNTIME}ChainEnd::Flag(|_e| _e.z()),"),
    ] {
        assert!(
            body.contains(&said),
            "codec_chain_prev_carrier's view does not hand the scan `{said}`:\n{body}"
        );
    }
}
