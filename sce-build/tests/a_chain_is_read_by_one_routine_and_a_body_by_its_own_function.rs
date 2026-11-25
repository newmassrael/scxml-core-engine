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
//!
//! The function a parent reads a codec's body through (an arm of a dispatching
//! variant, an embedded body, a chain's entry) is out of line, so it is one
//! function every reader of the body calls; `inline_decode` asks it inlined.

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
    rust_of_source(name, &source, options)
}

/// The Rust the codec `name` written as `source` generates under `options`, its
/// imports read from the resources, every file joined.
fn rust_of_source(name: &str, source: &str, options: &ForgeCompileOptions) -> String {
    sce_build::compile_forge_with_imports(
        source,
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

/// A codec that embeds `codec_tlv_entry` and reads a chain of it after: the
/// embedded body is held as its view, and the chain reads by the walk of the
/// entry's module.
const EMBEDDED_AND_CHAINED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_tlv_entry_held">
  <sce:import src="codec_tlv_entry.scxml" kind="codec" as="codec_tlv_entry"/>
  <datamodel>
    <sce:field id="header_flags" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="first" type="codec_tlv_entry" sce:byte="1"/>
    <sce:tlv-chain id="rest" type="codec_tlv_entry" sce:byte="2" max-depth="8"/>
  </datamodel>
</scxml>"#;

/// How many times `code` imports the view of the codec `module` as `view`.
fn view_imports(code: &str, module: &str, view: &str) -> usize {
    let import = format!("use super::{module}::{view};");
    code.lines().filter(|line| line.trim() == import).count()
}

/// A view imports exactly the views its code names: a chain that reads by the
/// walk of its entry's module names that walk by its path and its entry's view
/// nowhere, so a codec whose only use of an imported codec is such a chain
/// imports no view of it, and one that embeds the codec imports its view once.
#[test]
fn a_view_imports_the_view_of_a_codec_it_embeds_and_not_of_one_it_only_chains() {
    let code = rust_of("codec_tlv_chain_basic");
    assert!(
        code.contains("pub type CodecTlvChainBasicExtensionsWalk = super::codec_tlv_entry::CodecTlvEntryWalk;"),
        "codec_tlv_chain_basic's chain does not read by its entry's walk:\n{code}"
    );
    assert_eq!(
        view_imports(&code, "codec_tlv_entry", "CodecTlvEntryView"),
        0,
        "codec_tlv_chain_basic imports the view of an entry it only chains:\n{code}"
    );

    let code = rust_of_source(
        "codec_tlv_entry_held",
        EMBEDDED_AND_CHAINED,
        &ForgeCompileOptions {
            views: true,
            ..ForgeCompileOptions::default()
        },
    );
    assert_eq!(
        view_imports(&code, "codec_tlv_entry", "CodecTlvEntryView"),
        1,
        "a codec that embeds codec_tlv_entry does not import its view once:\n{code}"
    );

    // The frame's message embeds its timestamp and chains its extensions.
    let code = rust_of("codec_zenoh_msg_put");
    assert_eq!(
        view_imports(&code, "codec_zenoh_timestamp", "CodecZenohTimestampView"),
        1,
        "codec_zenoh_msg_put does not import the view of the body it embeds:\n{code}"
    );
    assert_eq!(
        view_imports(&code, "codec_zenoh_ext_entry", "CodecZenohExtEntryView"),
        0,
        "codec_zenoh_msg_put imports the view of an entry it only chains:\n{code}"
    );
}

/// A chain that carries a value from one entry to the next reads by a walk of
/// its own, which reads each entry by its view, so its codec imports that view.
#[test]
fn a_chain_with_a_walk_of_its_own_imports_its_entrys_view() {
    let code = rust_of("codec_chain_prev_carrier");
    assert!(
        code.contains("pub struct CodecChainPrevCarrierEntriesWalk;"),
        "codec_chain_prev_carrier's chain has no walk of its own:\n{code}"
    );
    assert_eq!(
        view_imports(&code, "codec_chain_prev_entry", "CodecChainPrevEntryView"),
        1,
        "codec_chain_prev_carrier does not import the view its walk reads by:\n{code}"
    );
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

/// Each `read`, `read_into` and `read_with` the code declares as a method of
/// its own: the line before its signature, trimmed, and its body.
fn reads(code: &str) -> Vec<(&str, &str)> {
    ["read", "read_into", "read_with"]
        .into_iter()
        .flat_map(|name| {
            let head = format!("    pub fn {name}");
            code.match_indices(&head)
                .filter(|(at, _)| {
                    let after = &code[at + head.len()..];
                    after.starts_with('(') || after.starts_with('<')
                })
                .map(|(at, _)| {
                    let before = code[..at].lines().last().unwrap_or_default().trim();
                    let body = &code[at..];
                    let end = body.find("\n    }\n").unwrap_or(body.len());
                    (before, &body[..end])
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether `body` reads the frame and is no call of another of the codec's
/// reads: the eager read that fills its destination, the eager read of a codec
/// built as a value, and the view's hooked read.
fn reads_the_body(body: &str) -> bool {
    ![
        "*_out = Self::read(",
        "Self::read_into(cursor, &mut _out",
        "Self::read_with(cursor",
        "        Self::read(cursor",
    ]
    .iter()
    .any(|forward| body.contains(forward))
}

/// Codecs of each shape a body read takes: a dispatching variant, a codec with
/// a chain and an embedded body, a chain alone, and a frame built as a value.
const BODIES: [&str; 4] = [
    "codec_zenoh_network_envelope",
    "codec_zenoh_oam",
    "codec_tlv_chain_basic",
    "codec_simple_frame",
];

/// The function that reads a codec's body is `#[inline(never)]` by default and
/// `#[inline(always)]` under `inline_decode`; a read that only calls it keeps no
/// attribute of the option's.
#[test]
fn a_body_read_is_out_of_line_by_default_and_inlined_when_the_build_asks() {
    for (inline_decode, attribute) in [(false, "#[inline(never)]"), (true, "#[inline(always)]")] {
        let options = ForgeCompileOptions {
            views: true,
            inline_decode,
            ..ForgeCompileOptions::default()
        };
        for name in BODIES {
            let code = rust_with(name, &options);
            let all = reads(&code);
            let body_reads: Vec<&(&str, &str)> = all
                .iter()
                .filter(|(_, body)| reads_the_body(body))
                .collect();
            assert!(!body_reads.is_empty(), "{name} has no body read:\n{code}");
            for (before, body) in &body_reads {
                assert_eq!(
                    *before, attribute,
                    "{name}'s body read is not marked `{attribute}`:\n{body}"
                );
            }
            for (before, body) in all.iter().filter(|(_, body)| !reads_the_body(body)) {
                assert_ne!(
                    *before, "#[inline(never)]",
                    "{name} keeps a read that only forwards out of line:\n{body}"
                );
            }
        }
    }
}

/// `inline_decode` changes the attribute of each body read and nothing else: the
/// two outputs are the same lines but for `#[inline(never)]` made
/// `#[inline(always)]`, and at least one is.
#[test]
fn inline_decode_changes_nothing_but_the_body_reads_attribute() {
    for name in BODIES {
        let off = rust_of(name);
        let on = rust_with(
            name,
            &ForgeCompileOptions {
                views: true,
                inline_decode: true,
                ..ForgeCompileOptions::default()
            },
        );
        let (off_lines, on_lines): (Vec<&str>, Vec<&str>) =
            (off.lines().collect(), on.lines().collect());
        assert_eq!(
            off_lines.len(),
            on_lines.len(),
            "{name}: inline_decode adds or removes lines"
        );
        let mut marked = 0usize;
        for (plain, inlined) in off_lines.iter().zip(&on_lines) {
            if plain == inlined {
                continue;
            }
            assert_eq!(
                (plain.trim(), inlined.trim()),
                ("#[inline(never)]", "#[inline(always)]"),
                "{name}: inline_decode changes more than a body read's attribute"
            );
            marked += 1;
        }
        assert!(marked > 0, "{name}: inline_decode marks no body read");
    }
}
