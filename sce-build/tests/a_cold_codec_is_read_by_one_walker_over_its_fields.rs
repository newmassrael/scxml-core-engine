// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A codec a build reads once per event, outside the hot closure of the roots
//! it names, is read by the runtime's one walker over the table of its fields
//! where every field is of a kind a row states (docs/adr/0155): its read is one
//! call of `read_table`, and what it declares is what it declares with no codec
//! named. No hot body calls the walker, and a build that names no hot root
//! prints no table, so no golden moves.

use std::path::PathBuf;

use sce_build::forge::body_reads::{BodyReadSites, InlineDecode};
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// What the set of `documents`, each in the resources, reads of each codec's
/// body, every codec generated with its view or none.
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

/// The lines of `code` that declare a public item or member.
fn declarations(code: &str) -> Vec<&str> {
    code.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub ") || line.starts_with("pub("))
        .collect()
}

/// The calls of the runtime's walkers, none of which a hot body makes.
const WALKERS: [&str; 3] = ["read_table(", "write_table(", "read_chain_table("];

/// Every codec of the request's set.
const SET: [&str; 11] = [
    "codec_zenoh_request",
    "codec_zenoh_msg_put",
    "codec_zenoh_msg_del",
    "codec_zenoh_query",
    "codec_zenoh_wireexpr",
    "codec_zenoh_timestamp",
    "codec_zenoh_encoding",
    "codec_zenoh_ext_entry",
    "codec_zenoh_ext_unit",
    "codec_zenoh_ext_zint",
    "codec_zenoh_ext_zbuf",
];

/// With the put named hot, its closure is printed as before and calls no
/// walker; the cold codecs whose fields are rows read by one call of the
/// walker, and the request, which dispatches a variant, is printed as it is.
/// Every codec declares what it declares with no codec named, and with no
/// codec named none calls a walker.
#[test]
fn no_hot_body_calls_the_walker_and_a_cold_codec_of_rows_is_one_call_of_it() {
    for views in [false, true] {
        let sites = sites_of(&["codec_zenoh_request.scxml"], views);
        let hot = sites
            .hot_codecs(["codec_zenoh_msg_put"])
            .unwrap_or_else(|e| panic!("codec_zenoh_msg_put is a codec of the set: {e}"));
        let named = ForgeCompileOptions {
            views,
            inline_decode: InlineDecode::Codecs(hot.clone()),
            body_reads: sites.clone(),
            ..ForgeCompileOptions::default()
        };
        let none = ForgeCompileOptions {
            views,
            body_reads: sites.clone(),
            ..ForgeCompileOptions::default()
        };
        for name in SET {
            assert!(sites.holds(name), "{name} is no codec of the set");
            let code = rust_with(name, &named);
            let plain = rust_with(name, &none);
            for walker in WALKERS {
                assert!(
                    !plain.contains(walker),
                    "{name} calls `{walker}` with no codec named (views: {views}):\n{plain}"
                );
                if hot.is_hot(name) {
                    assert!(
                        !code.contains(walker),
                        "the hot {name} calls `{walker}` (views: {views}):\n{code}"
                    );
                }
            }
            assert_eq!(
                declarations(&code),
                declarations(&plain),
                "{name} declares otherwise with the put named hot (views: {views})"
            );
        }
        for (name, call) in [
            (
                "codec_zenoh_wireexpr",
                "::sce_forge_runtime::codec::read_table(cursor, &CODEC_ZENOH_WIREEXPR_FIELDS, \
                 &[u64::from(n)], _out);",
            ),
            (
                "codec_zenoh_msg_del",
                "::sce_forge_runtime::codec::read_table(cursor, &CODEC_ZENOH_MSG_DEL_FIELDS, \
                 &[], _out);",
            ),
            (
                "codec_zenoh_query",
                "::sce_forge_runtime::codec::read_table(cursor, &CODEC_ZENOH_QUERY_FIELDS, \
                 &[], _out);",
            ),
        ] {
            let code = rust_with(name, &named);
            assert!(
                code.contains(call),
                "the cold {name} is not read by one call of the walker (views: {views}):\n{code}"
            );
            assert_eq!(
                code.matches("read_table(").count(),
                1,
                "the cold {name} calls the walker other than once (views: {views}):\n{code}"
            );
        }
        assert_eq!(
            rust_with("codec_zenoh_request", &named),
            rust_with("codec_zenoh_request", &none),
            "codec_zenoh_request dispatches a variant and is printed otherwise than with no \
             codec named (views: {views})"
        );
    }
}
