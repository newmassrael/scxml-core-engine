// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A codec a build reads once per event, outside the hot closure of the roots
//! it names, is read by the runtime's one walker over the table of its fields
//! where every field is of a kind a row states (docs/adr/0155): its read is one
//! call of `read_table`, its write, where its encode refuses no value, one
//! call of `write_table`, and what it declares is what it declares with no
//! codec named. No hot body calls the walker, and a build that names no hot root
//! prints no table, so no golden moves.

use std::path::PathBuf;

use sce_build::forge::body_reads::{BodyReadSites, ColdCodecs, InlineDecode};
use sce_build::forge::shared_tables::{CodecShape, ColdCodec, NotATable};
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

/// What the codec `name` generates for Rust under `options`.
fn output_with(name: &str, options: &ForgeCompileOptions) -> sce_build::generator::GeneratedOutput {
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
}

/// The Rust the codec `name` generates under `options`, every file joined.
fn rust_with(name: &str, options: &ForgeCompileOptions) -> String {
    output_with(name, options)
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
/// Every codec declares what it declares with the same codec named and its cold
/// codecs printed, so the tables alone are measured, and neither that build nor
/// one that names no codec calls a walker.
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
        let printed = ForgeCompileOptions {
            cold_codecs: ColdCodecs::Printed,
            ..named.clone()
        };
        let none = ForgeCompileOptions {
            views,
            body_reads: sites.clone(),
            ..ForgeCompileOptions::default()
        };
        for name in SET {
            assert!(sites.holds(name), "{name} is no codec of the set");
            let code = rust_with(name, &named);
            let plain = rust_with(name, &printed);
            let unnamed = rust_with(name, &none);
            for walker in WALKERS {
                assert!(
                    !plain.contains(walker),
                    "{name} calls `{walker}` with its cold codecs printed (views: {views}):\n{plain}"
                );
                assert!(
                    !unnamed.contains(walker),
                    "{name} calls `{walker}` with no codec named (views: {views}):\n{unnamed}"
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
                "{name} declares otherwise by its table than printed (views: {views})"
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
        // docs/adr/0155 section 3: a cold table codec whose encode refuses no
        // value is written by one call of the runtime's writer over the same
        // table, made with the numbers its encode derives; the key expression,
        // whose suffix is gated by an input its encode holds the value to, can
        // refuse a value and keeps its write printed.
        for (name, call, derives) in [
            (
                "codec_zenoh_msg_del",
                "::sce_forge_runtime::codec::write_table(w, &CODEC_ZENOH_MSG_DEL_FIELDS, &[], \
                 &CodecZenohMsgDelRows(src, over))",
                &[
                    "Derive { into: Bits::range(Source::Row(0), 5, 1), from: Derived::Present(1) },",
                    "Derive { into: Bits::range(Source::Row(0), 7, 1), from: Derived::Present(2) },",
                ][..],
            ),
            (
                "codec_zenoh_query",
                "::sce_forge_runtime::codec::write_table(w, &CODEC_ZENOH_QUERY_FIELDS, &[], \
                 &CodecZenohQueryRows(src))",
                &[
                    "Derive { into: Bits::range(Source::Row(0), 5, 1), from: Derived::Present(1) },",
                    "Derive { into: Bits::range(Source::Row(0), 6, 1), from: Derived::Present(3) },",
                    "Derive { into: Bits::range(Source::Row(0), 7, 1), from: Derived::Present(4) },",
                    "Derive { into: Bits::whole(Source::Row(2)), from: Derived::Count(3) },",
                ][..],
            ),
        ] {
            let code = rust_with(name, &named);
            assert!(
                code.contains(call),
                "the cold {name} is not written by one call of the writer (views: {views}):\n{code}"
            );
            assert_eq!(
                code.matches("write_table(").count(),
                1,
                "the cold {name} calls the writer other than once (views: {views}):\n{code}"
            );
            assert!(
                code.contains("FieldTable::with_derived(ROWS, DERIVED)"),
                "the table of the cold {name} is made with no derive (views: {views}):\n{code}"
            );
            for derive in derives {
                assert!(
                    code.contains(derive),
                    "the table of the cold {name} has no `{derive}` (views: {views}):\n{code}"
                );
            }
        }
        // The delete's `X` flag is a parent's to set (the end flag of a chain
        // of deletes): the adapter sets it over the header the value says, in a
        // bit no derive writes, as the printed write does.
        let delete = rust_with("codec_zenoh_msg_del", &named);
        for printed in [
            "struct CodecZenohMsgDelRows<'s, S: ?Sized>(&'s S, &'s CodecZenohMsgDelOverrides);",
            "let mut _n: u64 = u64::from(self.0.header());",
            "if let Some(_o) = self.1.header_x {",
            "_n = (_n & !0x40) | (u64::from(_o) << 6);",
        ] {
            assert!(
                delete.contains(printed),
                "the delete's adapter has no `{printed}` (views: {views}):\n{delete}"
            );
        }
        // The adapter is printed by the codec template from the arms the
        // generator lowers, as the table and the stores are: a presence for
        // each flag a gated field is announced by, the bytes of the counted
        // field, and the write of the chain by its printed statement.
        let query = rust_with("codec_zenoh_query", &named);
        for printed in [
            "struct CodecZenohQueryRows<'s, S: ?Sized>(&'s S);",
            "::sce_forge_runtime::codec::TableSource<E> for CodecZenohQueryRows<'_, S> {",
            "fn present(&self, row: usize) -> bool {",
            "fn bytes(&self, row: usize) -> &[u8] {",
            "let w = &mut ::sce_forge_runtime::codec::DynSink(sink);",
        ] {
            assert!(
                query.contains(printed),
                "the query's adapter has no `{printed}` (views: {views}):\n{query}"
            );
        }
        let wireexpr = rust_with("codec_zenoh_wireexpr", &named);
        // The template prints the rows, the gates and the stores of the
        // table from the context the generator hands it (docs/adr/0155).
        for printed in [
            "static CODEC_ZENOH_WIREEXPR_FIELDS: ::sce_forge_runtime::codec::FieldTable = {",
            "Row { read: Read::Zint { bits: 64 }, gate: Gate::Always },",
            "Row { read: Read::Zint { bits: 64 }, gate: \
             Gate::Set(Bits::range(Source::Input(0), 0, 1)) },",
            "Row { read: Read::Text { len: Bits::whole(Source::Row(1)), max: 65535 }, gate: \
             Gate::Set(Bits::range(Source::Input(0), 0, 1)) },",
            "::sce_forge_runtime::codec::TableOut<'f> for CodecZenohWireexprIn<S> {",
            "self.id = value;",
            "self.suffix_len = Some(value);",
            "self.suffix = None;",
            ".store::<S, CodecZenohWireexprSuffixField>() {",
        ] {
            assert!(
                wireexpr.contains(printed),
                "the table of codec_zenoh_wireexpr has no `{printed}` (views: {views}):\n{wireexpr}"
            );
        }
        assert!(
            !wireexpr.contains("write_table(") && wireexpr.contains("FieldTable::new(ROWS)"),
            "codec_zenoh_wireexpr can refuse a value and is written by the walker \
             (views: {views}):\n{wireexpr}"
        );
        assert_eq!(
            rust_with("codec_zenoh_request", &named),
            rust_with("codec_zenoh_request", &printed),
            "codec_zenoh_request dispatches a variant and is printed otherwise than with its \
             cold codecs printed (views: {views})"
        );
    }
}

/// The generation of each cold codec of the set answers the table it prints or
/// why it prints none (docs/adr/0155 section 2), and a hot codec answers
/// nothing: with the put named hot, the delete, the query and the key
/// expression are tables, and the request, which dispatches a variant, is not.
#[test]
fn every_cold_codec_says_its_table_or_why_it_has_none() {
    let sites = sites_of(&["codec_zenoh_request.scxml"], false);
    let hot = sites
        .hot_codecs(["codec_zenoh_msg_put"])
        .unwrap_or_else(|e| panic!("codec_zenoh_msg_put is a codec of the set: {e}"));
    let named = ForgeCompileOptions {
        inline_decode: InlineDecode::Codecs(hot.clone()),
        body_reads: sites.clone(),
        ..ForgeCompileOptions::default()
    };
    let mut said = std::collections::BTreeMap::new();
    for name in SET {
        let cold = output_with(name, &named).cold_codecs;
        if hot.is_hot(name) {
            assert!(cold.is_empty(), "the hot {name} says {cold:?}");
            continue;
        }
        let [one] = cold.as_slice() else {
            panic!("the cold {name} says other than one thing of its table: {cold:?}");
        };
        said.insert(name, one.clone());
        // With its cold codecs printed, the build says nothing of a table.
        let printed = ForgeCompileOptions {
            cold_codecs: ColdCodecs::Printed,
            ..named.clone()
        };
        assert!(
            output_with(name, &printed).cold_codecs.is_empty(),
            "the cold {name} says something of a table with its cold codecs printed"
        );
    }
    // The delete and the query are written by the one writer over their
    // tables; the key expression's encode can refuse a value, and it says why
    // its write stays printed.
    for (name, written) in [
        ("codec_zenoh_msg_del", true),
        ("codec_zenoh_query", true),
        ("codec_zenoh_wireexpr", false),
    ] {
        assert!(
            matches!(
                said.get(name),
                Some(ColdCodec::Table(table))
                    if table.codec == name && table.not_written.is_none() == written
            ),
            "{name} is no table, or is written otherwise: {said:?}"
        );
    }
    assert_eq!(
        said.get("codec_zenoh_request"),
        Some(&ColdCodec::Printed {
            codec: "codec_zenoh_request".to_string(),
            why: NotATable::Codec(CodecShape::Variant),
        }),
        "{said:?}"
    );
    assert_eq!(
        said.len(),
        4,
        "the cold codecs of the set are other than the four: {said:?}"
    );
}
