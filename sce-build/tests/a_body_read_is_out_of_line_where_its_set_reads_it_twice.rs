// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The function a parent reads a codec's body through is out of line where the
//! generated set reads the body from two sites or more, and carries no attribute
//! where the set reads it from one, which leaves it to the compiler
//! (docs/adr/0137). The sites are counted once, from the set's import graph
//! (`BodyReadSites::count`), and the eager read and the view's are two functions,
//! each read from its own sites; `inline_decode` asks every body read inlined
//! (docs/adr/0136), or names the codecs on a build's hot path, each checked to be
//! a codec of the set by its stem or an import alias (docs/adr/0139).
//!
//! Hotness is a property of a site (docs/adr/0140): the body of a codec named
//! is hot, and so is every read a hot body makes, transitively. A hot body is
//! printed once in forms held in place (`read_into_inline`, `read_inline`,
//! `read_with_inline`), which every hot site calls and which read their ZInts
//! held in place; its entry points are calls of them, which the cold sites call
//! and which keep the rule of those sites alone.
//!
//! A decode is a thin call: a view's decode answers the view its read builds,
//! with no default of it made first, and an eager codec built as a value does
//! the same; the verdict and the move of the cursor are one function of the
//! runtime for every codec, which the runtime's tests hold.

use std::path::{Path, PathBuf};

use sce_build::forge::body_reads::{BodyReadSites, BodyReads, InlineDecode};
use sce_build::forge::error::{ForgeError, GenerateError};
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
/// built as a value, and the view's hooked read, whether an entry point or a
/// form held in place.
fn reads_the_body(body: &str) -> bool {
    ![
        "*_out = Self::read(",
        "Self::read_into(cursor, &mut _out",
        "Self::read_with(cursor",
        "        Self::read(cursor",
        "Self::read_into_inline(cursor",
        "Self::read_inline(cursor",
        "Self::read_with_inline(cursor",
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

/// Each form held in place the code declares (docs/adr/0140).
fn held_reads(code: &str) -> Vec<(&str, &str)> {
    ["read_inline", "read_into_inline", "read_with_inline"]
        .into_iter()
        .flat_map(|name| methods(code, name))
        .collect()
}

/// The line before the eager body read held in place of the hot codec `name`
/// whose code is `code`, and its body.
fn held_eager_body_read<'c>(name: &str, code: &'c str) -> (&'c str, &'c str) {
    held_reads(code)
        .into_iter()
        .find(|(_, body)| reads_the_body(body) && !is_view_read(body))
        .unwrap_or_else(|| panic!("{name} has an eager body read held in place:\n{code}"))
}

/// The line before the view's body read held in place of the hot codec `name`,
/// and its body.
fn held_view_body_read<'c>(name: &str, code: &'c str) -> (&'c str, &'c str) {
    held_reads(code)
        .into_iter()
        .find(|(_, body)| reads_the_body(body) && is_view_read(body))
        .unwrap_or_else(|| panic!("{name} has a view body read held in place:\n{code}"))
}

/// The line before the eager entry point of the hot codec `name`, the read
/// whose one statement calls the eager form held in place, and its body.
fn eager_entry_read<'c>(name: &str, code: &'c str) -> (&'c str, &'c str) {
    reads(code)
        .into_iter()
        .find(|(_, body)| {
            !is_view_read(body)
                && statements(body).len() == 1
                && (body.contains("Self::read_into_inline(cursor, _out")
                    || body.contains("Self::read_inline(cursor"))
        })
        .unwrap_or_else(|| panic!("{name} has an eager entry point:\n{code}"))
}

/// The line before the view's entry point of the hot codec `name`, the read
/// whose one statement calls the view's form held in place, and its body.
fn view_entry_read<'c>(name: &str, code: &'c str) -> (&'c str, &'c str) {
    reads(code)
        .into_iter()
        .find(|(_, body)| {
            is_view_read(body)
                && statements(body).len() == 1
                && body.contains("Self::read_with_inline(cursor")
        })
        .unwrap_or_else(|| panic!("{name} has a view entry point:\n{code}"))
}

/// The lines of `code` that call a body read of another codec: a path, not
/// `Self`, before `::read`, `::read_into` or `::read_with`, the entry point
/// or the form held in place (`_inline`).
fn child_reads(code: &str) -> Vec<&str> {
    code.lines()
        .map(str::trim)
        .filter(|line| {
            line.match_indices("::read").any(|(at, head)| {
                let rest = &line[at + head.len()..];
                let rest = rest
                    .strip_prefix("_into")
                    .or_else(|| rest.strip_prefix("_with"))
                    .unwrap_or(rest);
                let rest = rest.strip_prefix("_inline").unwrap_or(rest);
                rest.starts_with('(') && !line[..at].ends_with("Self")
            })
        })
        .collect()
}

/// The options that generate a codec of the set of `sites` with the codecs
/// `names` named hot.
fn hot_options(sites: &BodyReadSites, names: &[&str], views: bool) -> ForgeCompileOptions {
    let hot = sites
        .hot_codecs(names)
        .unwrap_or_else(|e| panic!("{names:?} are codecs of the set: {e}"));
    ForgeCompileOptions {
        views,
        inline_decode: InlineDecode::Codecs(hot),
        body_reads: sites.clone(),
        ..ForgeCompileOptions::default()
    }
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
            inline_decode: InlineDecode::Every,
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

/// A hot body is printed once, in the forms held in place, `#[inline(always)]`,
/// the eager read and the view's; its entry points are calls of them, which keep
/// ADR 0137's rule over the cold sites alone: the sites of hot bodies call the
/// forms held in place and are not counted (docs/adr/0140).
#[test]
fn a_hot_body_is_held_in_place_and_its_entry_points_keep_the_rule_of_its_cold_sites() {
    let sites = sites_of(&["codec_zenoh_oam.scxml"], true);
    // The entry is read from one site of the eager decode and two of the view,
    // the arm codec from eight, all eager: it is its own view.
    assert_eq!(
        sites.of("codec_zenoh_ext_entry"),
        BodyReads { eager: 1, view: 2 }
    );
    assert_eq!(
        sites.of("codec_zenoh_ext_unit"),
        BodyReads { eager: 8, view: 0 }
    );
    let options = hot_options(
        &sites,
        &["codec_zenoh_ext_entry", "codec_zenoh_ext_unit"],
        true,
    );

    // The frame reads the entry and is cold: every site of the entry is cold.
    let code = rust_with("codec_zenoh_ext_entry", &options);
    let (before, body) = held_eager_body_read("codec_zenoh_ext_entry", &code);
    assert_eq!(
        before, "#[inline(always)]",
        "codec_zenoh_ext_entry is hot and its eager body is not held in place:\n{body}"
    );
    let (before, body) = held_view_body_read("codec_zenoh_ext_entry", &code);
    assert_eq!(
        before, "#[inline(always)]",
        "codec_zenoh_ext_entry is hot and its view's body is not held in place:\n{body}"
    );
    let (before, body) = eager_entry_read("codec_zenoh_ext_entry", &code);
    assert!(
        !before.starts_with("#[inline"),
        "codec_zenoh_ext_entry's eager entry point, called from one cold site, carries \
         `{before}`:\n{body}"
    );
    let (before, body) = view_entry_read("codec_zenoh_ext_entry", &code);
    assert_eq!(
        before, "#[inline(never)]",
        "codec_zenoh_ext_entry's view entry point, called from two cold sites, is not out \
         of line:\n{body}"
    );

    // The arm codec is read from four sites of the cold frame and four of the
    // hot entry: its entry point is out of line for the four cold ones.
    let code = rust_with("codec_zenoh_ext_unit", &options);
    let (before, body) = held_eager_body_read("codec_zenoh_ext_unit", &code);
    assert_eq!(
        before, "#[inline(always)]",
        "codec_zenoh_ext_unit is hot and its body is not held in place:\n{body}"
    );
    let (before, body) = eager_entry_read("codec_zenoh_ext_unit", &code);
    assert_eq!(
        before, "#[inline(never)]",
        "codec_zenoh_ext_unit's entry point, called from four cold sites, is not out of \
         line:\n{body}"
    );
}

/// A codec the build does not name and no hot body reads keeps the rule of its
/// sites, and prints what it prints with no codec named: a hot codec makes the
/// codecs it reads hot, never the codecs that read it (docs/adr/0137,
/// docs/adr/0140).
#[test]
fn a_codec_no_hot_body_reads_keeps_the_rule_of_its_sites() {
    let sites = sites_of(&["codec_zenoh_oam.scxml"], true);
    let options = hot_options(&sites, &["codec_zenoh_ext_unit"], true);
    let cold = ForgeCompileOptions {
        views: true,
        body_reads: sites,
        ..ForgeCompileOptions::default()
    };

    let code = rust_with("codec_zenoh_ext_entry", &options);
    let (before, body) = eager_body_read("codec_zenoh_ext_entry", &code);
    assert!(
        !before.starts_with("#[inline"),
        "codec_zenoh_ext_entry is not hot and its eager read of one site carries \
         `{before}`:\n{body}"
    );
    let (before, body) = view_body_read("codec_zenoh_ext_entry", &code);
    assert_eq!(
        before, "#[inline(never)]",
        "codec_zenoh_ext_entry is not hot and its view, read from two sites, is not \
         out of line:\n{body}"
    );
    assert_eq!(
        code,
        rust_with("codec_zenoh_ext_entry", &cold),
        "codec_zenoh_ext_entry, which no hot body reads, prints otherwise than with no \
         codec named"
    );

    let code = rust_with("codec_zenoh_ext_zint", &options);
    let (before, body) = eager_body_read("codec_zenoh_ext_zint", &code);
    assert_eq!(
        before, "#[inline(never)]",
        "codec_zenoh_ext_zint is not hot and its read of two sites is not out of line:\n{body}"
    );
}

/// A codec read from a hot body and from a cold one is printed in both forms
/// from its one body, and each parent calls its own: the hot parent the form
/// held in place, through an embedded body and a chain's rule alike, and the
/// cold parent the entry point, so the cold parent holds no line of the hot
/// child's body (docs/adr/0140).
#[test]
fn a_child_of_a_hot_and_a_cold_parent_prints_both_forms_and_each_parent_calls_its_own() {
    let sites = sites_of(&["codec_zenoh_request.scxml"], false);
    let options = hot_options(&sites, &["codec_zenoh_msg_put"], false);

    // The timestamp is embedded by the hot put and the cold delete.
    let code = rust_with("codec_zenoh_timestamp", &options);
    for form in ["read_inline", "read_into_inline"] {
        assert!(
            !methods(&code, form).is_empty(),
            "codec_zenoh_timestamp is read by a hot body and has no `{form}`:\n{code}"
        );
    }
    let (_, held) = held_eager_body_read("codec_zenoh_timestamp", &code);
    let (_, entry) = eager_entry_read("codec_zenoh_timestamp", &code);
    assert!(
        !entry.contains("read_vle"),
        "codec_zenoh_timestamp's entry point holds a read of its own, not one call of its \
         body:\n{entry}"
    );
    assert_eq!(
        code.matches("read_vle_u64").count(),
        held.matches("read_vle_u64").count(),
        "codec_zenoh_timestamp prints its body more than once:\n{code}"
    );

    let put = rust_with("codec_zenoh_msg_put", &options);
    let put_reads = child_reads(&put);
    assert!(
        put_reads.iter().any(|line| line.contains("timestamp"))
            && put_reads.iter().any(|line| line.contains("(_r")),
        "codec_zenoh_msg_put reads no timestamp or no chain entry: {put_reads:#?}"
    );
    for line in &put_reads {
        assert!(
            line.contains("_inline("),
            "the hot codec_zenoh_msg_put calls an entry point: {line}"
        );
    }

    let del = rust_with("codec_zenoh_msg_del", &options);
    let del_reads = child_reads(&del);
    assert!(
        del_reads.iter().any(|line| line.contains("timestamp"))
            && del_reads.iter().any(|line| line.contains("(_r")),
        "codec_zenoh_msg_del reads no timestamp or no chain entry: {del_reads:#?}"
    );
    assert!(
        !del.contains("_inline"),
        "the cold codec_zenoh_msg_del calls or holds a form held in place:\n{del}"
    );
}

/// The hot closure is transitive: the body of each codec a hot body reads is
/// hot, through every level of the set, and no codec that reads a hot one is
/// made hot by it. A site of a hot body is no site of an entry point
/// (docs/adr/0140).
#[test]
fn the_hot_closure_is_transitive_and_a_hot_site_calls_no_entry_point() {
    let sites = sites_of(&["codec_zenoh_request.scxml"], false);
    let hot = sites
        .hot_codecs(["codec_zenoh_msg_put"])
        .unwrap_or_else(|e| panic!("codec_zenoh_msg_put is a codec of the set: {e}"));
    assert!(hot.named("codec_zenoh_msg_put"));
    // The put reads the timestamp, the encoding and the chain's entry, and the
    // entry reads its arms: two levels below the codec named.
    for codec in [
        "codec_zenoh_msg_put",
        "codec_zenoh_timestamp",
        "codec_zenoh_encoding",
        "codec_zenoh_ext_entry",
        "codec_zenoh_ext_unit",
        "codec_zenoh_ext_zint",
        "codec_zenoh_ext_zbuf",
    ] {
        assert!(hot.is_hot(codec), "{codec} is not in the hot closure");
    }
    for codec in [
        "codec_zenoh_request",
        "codec_zenoh_msg_del",
        "codec_zenoh_query",
        "codec_zenoh_wireexpr",
    ] {
        assert!(!hot.is_hot(codec), "{codec} reads no hot body and is hot");
    }
    // The entry's arm and default arm read the unit body, and the entry is
    // hot; no cold body reads the unit body.
    assert_eq!(
        hot.sites_of("codec_zenoh_ext_unit"),
        BodyReads { eager: 2, view: 0 }
    );
    assert_eq!(
        hot.sites_of("codec_zenoh_ext_unit"),
        sites.of("codec_zenoh_ext_unit")
    );

    // Two levels below the codec named, the entry reads its arms by their
    // forms held in place.
    let options = hot_options(&sites, &["codec_zenoh_msg_put"], false);
    let entry = rust_with("codec_zenoh_ext_entry", &options);
    let arms = child_reads(&entry);
    assert!(
        !arms.is_empty()
            && arms
                .iter()
                .all(|line| line.contains("::read_into_inline(cursor, _b")),
        "the hot codec_zenoh_ext_entry does not read each arm held in place: {arms:#?}"
    );
    // The unit body is read from its two hot sites alone, so its entry point,
    // out of line with no codec named, carries no attribute.
    let unit = rust_with("codec_zenoh_ext_unit", &options);
    let (before, body) = eager_entry_read("codec_zenoh_ext_unit", &unit);
    assert!(
        !before.starts_with("#[inline"),
        "codec_zenoh_ext_unit is called from no cold site and its entry point carries \
         `{before}`:\n{body}"
    );
    let cold = ForgeCompileOptions {
        body_reads: sites,
        ..ForgeCompileOptions::default()
    };
    let unit = rust_with("codec_zenoh_ext_unit", &cold);
    let (before, body) = eager_body_read("codec_zenoh_ext_unit", &unit);
    assert_eq!(
        before, "#[inline(never)]",
        "codec_zenoh_ext_unit, read from two sites with no codec named, is not out of \
         line:\n{body}"
    );
}

/// A hot body reads its ZInts by the frame reader's reads held in place, and
/// any other body by the reader's one read out of line (docs/adr/0140).
#[test]
fn a_hot_body_reads_its_zints_held_in_place() {
    let sites = sites_of(&["codec_zenoh_oam.scxml"], false);
    let hot = hot_options(&sites, &["codec_zenoh_ext_zint"], false);
    let cold = ForgeCompileOptions {
        body_reads: sites,
        ..ForgeCompileOptions::default()
    };

    let code = rust_with("codec_zenoh_ext_zint", &hot);
    let (_, held) = held_eager_body_read("codec_zenoh_ext_zint", &code);
    assert!(
        held.contains("cursor.read_vle_u64_inline()"),
        "the hot codec_zenoh_ext_zint does not read its ZInt held in place:\n{held}"
    );
    assert!(
        !code.contains("cursor.read_vle_u64()"),
        "the hot codec_zenoh_ext_zint calls the reader's ZInt read:\n{code}"
    );

    let code = rust_with("codec_zenoh_ext_zint", &cold);
    assert!(
        code.contains("cursor.read_vle_u64()") && !code.contains("_inline"),
        "codec_zenoh_ext_zint with no codec named does not call the reader's one ZInt \
         read:\n{code}"
    );
    // The frame, not hot, reads its own ZInt by the call.
    let code = rust_with("codec_zenoh_oam", &hot);
    assert!(
        code.contains("cursor.read_vle_u16()") && !code.contains("read_vle_u16_inline"),
        "the cold codec_zenoh_oam does not call the reader's ZInt read:\n{code}"
    );
}

/// A body a length says the size of is read held in place where a hot body
/// reads it, the eager read and the view's alike, and the view the length
/// hands its bytes is read by its form held in place (docs/adr/0140).
#[test]
fn a_body_within_a_length_is_read_held_in_place_at_a_hot_site() {
    let sites = sites_of(&["codec_zenoh_decl_ext_keyexpr.scxml"], true);
    let options = hot_options(&sites, &["codec_zenoh_decl_ext_keyexpr"], true);
    let code = rust_with("codec_zenoh_decl_ext_keyexpr", &options);
    let within: Vec<&str> = code
        .lines()
        .map(str::trim)
        .filter(|line| line.contains("cursor.within("))
        .collect();
    assert_eq!(
        within.len(),
        2,
        "codec_zenoh_decl_ext_keyexpr reads its inner body within a length once eager and \
         once as a view: {within:#?}"
    );
    for line in &within {
        assert!(
            line.contains("_inline"),
            "the hot codec_zenoh_decl_ext_keyexpr reads its inner body by an entry point: {line}"
        );
    }
    let inner = rust_with("codec_zenoh_decl_ext_keyexpr_inner", &options);
    assert!(
        !methods(&inner, "read_inline").is_empty(),
        "codec_zenoh_decl_ext_keyexpr_inner is read by a hot body and has no `read_inline`:\n{inner}"
    );
}

/// A codec outside the hot closure prints exactly what it prints with no codec
/// named: the forms held in place are printed for hot bodies alone, and no
/// golden of a build that names none moves (docs/adr/0140).
#[test]
fn a_codec_outside_the_hot_closure_prints_what_it_printed() {
    let sites = sites_of(&["codec_zenoh_request.scxml"], false);
    let hot = hot_options(&sites, &["codec_zenoh_msg_put"], false);
    let cold = ForgeCompileOptions {
        body_reads: sites,
        ..ForgeCompileOptions::default()
    };
    for name in [
        "codec_zenoh_request",
        "codec_zenoh_msg_del",
        "codec_zenoh_query",
        "codec_zenoh_wireexpr",
    ] {
        assert_eq!(
            rust_with(name, &hot),
            rust_with(name, &cold),
            "{name}, outside the hot closure, prints otherwise than with no codec named"
        );
    }
}

/// A codec is named by its document's stem or by an alias an import of the set
/// gives it, both the one codec; a name that is a stem of one codec and an
/// alias of another names two and is refused (docs/adr/0140).
#[test]
fn a_codec_is_named_by_its_stem_or_an_import_alias_and_a_name_of_two_is_refused() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let frame = aliased_set(tmp.path());
    let sites = BodyReadSites::of_documents(&[frame.display().to_string()], &[], |_| false);

    // The alias names the one codec it is given to.
    let by_alias = sites
        .hot_codecs(["codec_x_body"])
        .unwrap_or_else(|e| panic!("codec_x_body names a codec of the set: {e}"));
    assert!(
        by_alias.named("body") && !by_alias.named("other"),
        "codec_x_body does not name the codec body alone"
    );
    let by_stem = sites
        .hot_codecs(["other"])
        .unwrap_or_else(|e| panic!("other is a codec of the set: {e}"));
    assert!(by_stem.named("other") && !by_stem.named("body"));
    // A codec given by its stem and by an alias is one codec.
    let both = sites
        .hot_codecs(["other", "codec_x_body"])
        .unwrap_or_else(|e| panic!("both name codecs of the set: {e}"));
    assert!(both.named("other") && both.named("body"));

    let refusal = sites
        .hot_codecs(["codec_x_body", "body"])
        .expect_err("a name of two codecs is refused");
    let ForgeError::Generate(boxed) = &refusal else {
        panic!("the refusal is not a generation error: {refusal:?}");
    };
    let GenerateError::CodegenInlineDecodeCodecAmbiguous { name, codecs } = boxed.as_ref() else {
        panic!("the refusal is not codegen/inline-decode-codec-ambiguous: {refusal:?}");
    };
    assert_eq!(name, "body");
    assert_eq!(codecs, &["body".to_string(), "other".to_string()]);
}

/// A frame that embeds the codec `body` under the alias `codec_x_body` and the
/// codec `other` under the alias `body`, written into `dir` with the two
/// codecs, and its path.
fn aliased_set(dir: &Path) -> PathBuf {
    let codec = |name: &str, datamodel: &str| {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\" \
             sce:kind=\"codec\" sce:default-endian=\"big\" name=\"{name}\">\n{datamodel}</scxml>\n"
        )
    };
    let write = |name: &str, text: String| {
        let path = dir.join(format!("{name}.scxml"));
        std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {name}: {e}"));
        path
    };
    write(
        "body",
        codec(
            "body",
            "  <datamodel>\n    <sce:field id=\"value\" sce:type=\"uint64\" sce:byte=\"0\" sce:bit-size=\"vle\"/>\n  </datamodel>\n",
        ),
    );
    write(
        "other",
        codec(
            "other",
            "  <datamodel>\n    <data id=\"n\" sce:type=\"uint8\" sce:byte=\"0\" sce:bit-size=\"8\"/>\n  </datamodel>\n",
        ),
    );
    write(
        "frame",
        codec(
            "frame",
            "  <sce:import src=\"body.scxml\" kind=\"codec\" as=\"codec_x_body\"/>\n  \
             <sce:import src=\"other.scxml\" kind=\"codec\" as=\"body\"/>\n  <datamodel>\n    \
             <sce:field id=\"tag\" sce:type=\"uint8\" sce:byte=\"0\" sce:bit-size=\"8\"/>\n    \
             <sce:embed id=\"first\" type=\"codec_x_body\" sce:byte=\"1\"/>\n    \
             <sce:embed id=\"second\" type=\"body\" sce:byte=\"2\"/>\n  </datamodel>\n",
        ),
    )
}

/// A name that is no codec of the set is refused, naming the set's codecs, so a
/// name misspelt cannot leave a hot read out of line unnoticed; a codec of the
/// set that no site reads is one a build can name (docs/adr/0139).
#[test]
fn a_name_that_is_no_codec_of_the_set_is_refused() {
    let sites = sites_of(&["codec_zenoh_oam.scxml"], false);
    let refusal = sites
        .hot_codecs(["codec_zenoh_ext_unit", "codec_zenoh_ext_untit"])
        .expect_err("a name of no codec of the set is refused");
    let ForgeError::Generate(boxed) = &refusal else {
        panic!("the refusal is not a generation error: {refusal:?}");
    };
    let GenerateError::CodegenInlineDecodeCodecNotInSet { codec, set } = boxed.as_ref() else {
        panic!("the refusal is not codegen/inline-decode-codec-not-in-set: {refusal:?}");
    };
    assert_eq!(codec, "codec_zenoh_ext_untit");
    for held in [
        "codec_zenoh_oam",
        "codec_zenoh_ext_entry",
        "codec_zenoh_ext_unit",
    ] {
        assert!(
            set.iter().any(|name| name == held),
            "the refusal does not name {held} among the set's codecs: {set:?}"
        );
    }

    // The frame is a codec of the set although no site of it reads its body.
    assert_eq!(sites.of("codec_zenoh_oam"), BodyReads::default());
    assert!(sites.holds("codec_zenoh_oam"));
    assert!(sites.hot_codecs(["codec_zenoh_oam"]).is_ok());

    // A document generated alone is a set of one that was not counted: it
    // holds no codec a build could name, and the refusal says so.
    let refusal = BodyReadSites::default()
        .hot_codecs(["codec_zenoh_oam"])
        .expect_err("a set that was not counted holds no codec");
    assert!(
        refusal.to_string().ends_with("it holds no codec"),
        "the refusal does not say the set holds no codec: {refusal}"
    );
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
            ["cursor.read_frame(Self::read)"],
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
        said[0].starts_with("cursor.read_frame(Self::read")
            || said[0].starts_with("cursor.read_frame(|cursor| Self::read(cursor"),
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
