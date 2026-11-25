// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Rust view shows each entry of every chain it reads to a hook, in its one
//! read (docs/adr/0130).
//!
//! Every view reads with `read_with(cursor, .., hook)`, `read` being that read
//! with `()`, and hands the hook to every body it reads: an arm, an embedded
//! body, a list's entry. A `<sce:tlv-chain>` scan shows the hook each entry
//! right after it is read, named by the chain's codec and field, and the
//! frame's reader keeps what the hook refuses. What a hook sees, frame by
//! frame, is held by `forge_source_round_trip`'s `records` module.

use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// The Rust the codec `name` generates, with its views, every file joined.
fn rust_of(name: &str) -> String {
    let path = resource_dir().join(format!("{name}.scxml"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    sce_build::compile_forge_with_imports(
        &source,
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        &ForgeCompileOptions {
            views: true,
            ..ForgeCompileOptions::default()
        },
    )
    .unwrap_or_else(|e| panic!("{name} generates for Rust: {e}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
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

/// A view's `read` is its `read_with` with the hook that admits every entry,
/// so the two are one read.
#[test]
fn a_views_read_is_its_hooked_read_with_unit() {
    for name in [
        "codec_zenoh_oam",
        "codec_tlv_chain_basic",
        "codec_zenoh_network_envelope",
    ] {
        let code = rust_of(name);
        let hooked = bodies(&code, "read_with");
        assert!(!hooked.is_empty(), "{name} has no read_with");
        let view_reads: Vec<&str> = bodies(&code, "read")
            .into_iter()
            .filter(|b| b.contains("Self::read_with("))
            .collect();
        assert!(
            !view_reads.is_empty(),
            "{name}'s view read does not forward to read_with"
        );
        for body in view_reads {
            assert!(
                body.contains(", &mut ())"),
                "{name}'s read hands read_with another hook than ():\n{body}"
            );
        }
    }
}

/// Every body a view reads is read with the view's hook: no `View::read(` is
/// left in a `read_with`, so a chain nested in an arm, an embedded body or a
/// list's entry is shown to the same hook.
#[test]
fn a_view_hands_its_hook_to_every_body_it_reads() {
    for name in [
        "codec_zenoh_oam",
        "codec_zenoh_network_envelope",
        "codec_zenoh_push",
    ] {
        let code = rust_of(name);
        for body in bodies(&code, "read_with") {
            assert!(
                !body.contains("View::read("),
                "{name} reads a body without its hook:\n{body}"
            );
        }
    }
    let envelope = rust_of("codec_zenoh_network_envelope");
    assert!(
        envelope.contains("CodecZenohOamView::read_with(cursor, _hook)"),
        "the envelope's arm reads its body without the hook"
    );
}

/// A chain's scan shows each entry to the hook right after it is read, named by
/// the codec that declares the chain and the chain's field.
#[test]
fn a_chain_scan_shows_each_entry_to_the_hook_by_its_site() {
    let code = rust_of("codec_zenoh_oam");
    let shown = "::sce_forge_runtime::codec::show_entry(cursor, _hook, \
                 ::sce_forge_runtime::codec::ChainSite { codec: \"codec_zenoh_oam\", \
                 field: \"extensions\" }, _at);";
    let at = code
        .find(shown)
        .unwrap_or_else(|| panic!("the OAM chain shows no entry to its hook:\n{code}"));
    let before = &code[..at];
    let mark = before
        .rfind("let _at = cursor.mark();")
        .expect("the entry's start is marked before it is read");
    let read = before
        .rfind("CodecZenohExtEntryView::read_with(cursor, _hook)")
        .expect("the entry is read with the hook");
    assert!(mark < read, "the mark is taken after the entry is read");
    // The walk reads entries the scan has shown already, and shows none again.
    for walk in code.match_indices("fn read<'b>(_r: &mut SceReader<'b>") {
        let body = &code[walk.0..];
        let body = &body[..body.find("\n    }\n").expect("the walk's read closes")];
        assert!(
            !body.contains("_hook"),
            "a walk shows an entry again:\n{body}"
        );
    }
}
