// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The function a parent reads a codec's body through is out of line where the
//! generated set reads the body from two sites or more, and carries no attribute
//! where the set reads it from one, which leaves it to the compiler
//! (docs/adr/0137). The sites are counted once, from the set's import graph
//! (`BodyReadSites::count`), and the eager read and the view's are two functions,
//! each read from its own sites; `inline_decode` asks every body read inlined.
//!
//! A decode is a thin call: a view's decode answers the view its read builds,
//! with no default of it made first, and an eager codec built as a value does
//! the same; the verdict and the move of the cursor are one function of the
//! runtime for every codec, which the runtime's tests hold.

use std::path::PathBuf;

use sce_build::forge::body_reads::{BodyReadSites, BodyReads};
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// What the set of `documents`, each in the resources, and every codec they
/// import reads of each codec's body, every codec generated with its view or
/// none.
fn sites_of(documents: &[&str], views: bool) -> BodyReadSites {
    let documents: Vec<String> = documents
        .iter()
        .map(|document| resource_dir().join(document).display().to_string())
        .collect();
    BodyReadSites::of_documents(&documents, &[], |_| views)
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

/// Each method `name` the code declares: the line before its signature,
/// trimmed, and its body.
fn methods<'c>(code: &'c str, name: &str) -> Vec<(&'c str, &'c str)> {
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
        .collect()
}

/// Each `read`, `read_into` and `read_with` the code declares.
fn reads(code: &str) -> Vec<(&str, &str)> {
    ["read", "read_into", "read_with"]
        .into_iter()
        .flat_map(|name| methods(code, name))
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

/// Whether `body` is a view's read, the one that reads with a hook.
fn is_view_read(body: &str) -> bool {
    body.contains("_hook: &mut H) -> Self {")
}

/// The line before the eager body read of `code`, and its body.
fn eager_body_read<'c>(name: &str, code: &'c str) -> (&'c str, &'c str) {
    reads(code)
        .into_iter()
        .find(|(_, body)| reads_the_body(body) && !is_view_read(body))
        .unwrap_or_else(|| panic!("{name} has an eager body read:\n{code}"))
}

/// The line before the view's body read of `code`, and its body.
fn view_body_read<'c>(name: &str, code: &'c str) -> (&'c str, &'c str) {
    reads(code)
        .into_iter()
        .find(|(_, body)| reads_the_body(body) && is_view_read(body))
        .unwrap_or_else(|| panic!("{name} has a view body read:\n{code}"))
}

/// The statements of `body` after its signature.
fn statements(body: &str) -> Vec<&str> {
    body.lines()
        .skip(1)
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

/// Each arm and default arm of a variant, and each embedded body, is one site;
/// a list's entry is one site of the eager decode and two of a view, its scan
/// and its walk; a codec that is its own view is read by the views of the set
/// through its eager read. A document reached twice is counted once.
#[test]
fn a_set_counts_each_site_that_reads_a_body() {
    let sites = sites_of(&["codec_zenoh_oam.scxml"], false);
    // An arm and the default arm, of the frame and of each of its extensions.
    assert_eq!(
        sites.of("codec_zenoh_ext_unit"),
        BodyReads { eager: 4, view: 0 }
    );
    assert_eq!(
        sites.of("codec_zenoh_ext_zint"),
        BodyReads { eager: 2, view: 0 }
    );
    // The entry of the frame's chain, read by the chain's rule.
    assert_eq!(
        sites.of("codec_zenoh_ext_entry"),
        BodyReads { eager: 1, view: 0 }
    );
    // No codec of the set reads the frame itself.
    assert_eq!(sites.of("codec_zenoh_oam"), BodyReads::default());

    let viewed = sites_of(&["codec_zenoh_oam.scxml"], true);
    assert_eq!(
        viewed.of("codec_zenoh_ext_entry"),
        BodyReads { eager: 1, view: 2 }
    );
    assert_eq!(
        viewed.of("codec_zenoh_ext_unit"),
        BodyReads { eager: 8, view: 0 }
    );

    assert_eq!(
        sites_of(
            &[
                "codec_zenoh_oam.scxml",
                "codec_zenoh_ext_entry.scxml",
                "codec_zenoh_oam.scxml",
            ],
            false
        ),
        sites
    );

    // A body one arm of one envelope dispatches to is read from one site.
    assert_eq!(
        sites_of(&["codec_transport_envelope.scxml"], false).of("codec_zenoh_init_body"),
        BodyReads { eager: 1, view: 0 }
    );
}

/// A body the set reads from one site carries no attribute, which leaves it to
/// the compiler; so does every body of a document generated alone, a set of one.
#[test]
fn a_body_read_from_one_site_carries_no_attribute() {
    for options in [
        ForgeCompileOptions {
            body_reads: sites_of(&["codec_transport_envelope.scxml"], false),
            ..ForgeCompileOptions::default()
        },
        ForgeCompileOptions::default(),
    ] {
        let code = rust_with("codec_zenoh_init_body", &options);
        let (before, body) = eager_body_read("codec_zenoh_init_body", &code);
        assert!(
            !before.starts_with("#[inline"),
            "codec_zenoh_init_body's one-site read carries `{before}`:\n{body}"
        );
    }
}

/// A body the set reads from two sites or more is one function, out of line.
#[test]
fn a_body_read_from_two_sites_is_out_of_line() {
    let options = ForgeCompileOptions {
        body_reads: sites_of(&["codec_zenoh_oam.scxml"], false),
        ..ForgeCompileOptions::default()
    };
    for name in ["codec_zenoh_ext_unit", "codec_zenoh_ext_zint"] {
        let code = rust_with(name, &options);
        let (before, body) = eager_body_read(name, &code);
        assert_eq!(
            before, "#[inline(never)]",
            "{name}'s shared read is not kept out of line:\n{body}"
        );
    }
}

/// The eager read and the view's are two functions, each kept out of line by
/// its own sites: a chain's entry is read once by the eager chain and twice by a
/// view, which scans the chain and walks it.
#[test]
fn the_eager_read_and_the_views_are_judged_by_their_own_sites() {
    let options = ForgeCompileOptions {
        views: true,
        body_reads: sites_of(&["codec_zenoh_oam.scxml"], true),
        ..ForgeCompileOptions::default()
    };
    let code = rust_with("codec_zenoh_ext_entry", &options);
    let (before, body) = eager_body_read("codec_zenoh_ext_entry", &code);
    assert!(
        !before.starts_with("#[inline"),
        "codec_zenoh_ext_entry's eager read of one site carries `{before}`:\n{body}"
    );
    let (before, body) = view_body_read("codec_zenoh_ext_entry", &code);
    assert_eq!(
        before, "#[inline(never)]",
        "codec_zenoh_ext_entry's view, scanned and walked, is not out of line:\n{body}"
    );
}

/// `inline_decode` marks every body read `#[inline(always)]`, whatever its set
/// reads of it, and a read that only calls another is never kept out of line.
#[test]
fn inline_decode_inlines_every_body_read_whatever_its_sites() {
    for (name, set) in [
        ("codec_zenoh_init_body", "codec_transport_envelope.scxml"),
        ("codec_zenoh_ext_unit", "codec_zenoh_oam.scxml"),
        ("codec_zenoh_ext_entry", "codec_zenoh_oam.scxml"),
    ] {
        let options = ForgeCompileOptions {
            views: true,
            inline_decode: true,
            body_reads: sites_of(&[set], true),
            ..ForgeCompileOptions::default()
        };
        let code = rust_with(name, &options);
        let all = reads(&code);
        let body_reads: Vec<&(&str, &str)> = all
            .iter()
            .filter(|(_, body)| reads_the_body(body))
            .collect();
        assert!(!body_reads.is_empty(), "{name} has no body read:\n{code}");
        for (before, body) in body_reads {
            assert_eq!(
                *before, "#[inline(always)]",
                "{name}'s body read is not inlined under inline_decode:\n{body}"
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

/// A view's decode answers the view its read builds, as one read of the frame:
/// no default of the view is made first and copied over.
#[test]
fn a_views_decode_answers_the_view_its_read_builds() {
    let options = ForgeCompileOptions {
        views: true,
        ..ForgeCompileOptions::default()
    };
    for name in ["codec_zenoh_ext_entry", "codec_zenoh_oam"] {
        let code = rust_with(name, &options);
        let decode = methods(&code, "decode")
            .into_iter()
            .map(|(_, body)| body)
            .find(|body| body.starts_with("    pub fn decode(cursor: &mut SceCursor<"))
            .unwrap_or_else(|| panic!("{name} has a view decode:\n{code}"));
        assert_eq!(
            statements(decode),
            ["cursor.read_frame(|cursor| Self::read(cursor))"],
            "{name}'s view decode is more than the read of its frame:\n{decode}"
        );
    }
}

/// An eager codec built as a value decodes as its view does, answering the
/// value its read builds; one that reads into its destination decodes into a
/// default of itself, which its lists are built in.
#[test]
fn an_eager_decode_of_a_value_answers_the_value_its_read_builds() {
    let code = rust_with("codec_simple_frame", &ForgeCompileOptions::default());
    let decode = methods(&code, "decode")
        .into_iter()
        .map(|(_, body)| body)
        .next()
        .unwrap_or_else(|| panic!("codec_simple_frame has a decode:\n{code}"));
    let said = statements(decode);
    assert_eq!(
        said.len(),
        1,
        "codec_simple_frame's decode is more than the read of its frame:\n{decode}"
    );
    assert!(
        said[0].starts_with("cursor.read_frame(|cursor| Self::read(cursor"),
        "codec_simple_frame's decode does not answer the value its read builds:\n{decode}"
    );

    let code = rust_with("codec_zenoh_ext_entry", &ForgeCompileOptions::default());
    let decode = methods(&code, "decode")
        .into_iter()
        .map(|(_, body)| body)
        .next()
        .unwrap_or_else(|| panic!("codec_zenoh_ext_entry has a decode:\n{code}"));
    assert!(
        decode.contains("let mut _out = Self::default();"),
        "codec_zenoh_ext_entry's decode does not read into a destination:\n{decode}"
    );
}
