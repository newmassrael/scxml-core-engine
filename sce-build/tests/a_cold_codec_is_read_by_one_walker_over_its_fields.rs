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
use sce_build::forge::shared_tables::{CodecShape, ColdCodec, NotATable, NotWrittenByTable};
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
            "Some(value) => value.store::<S, CodecZenohWireexprSuffixField>(),",
            "None => Err(CodecError::LengthOutOfRange),",
        ] {
            assert!(
                wireexpr.contains(printed),
                "the table of codec_zenoh_wireexpr has no `{printed}` (views: {views}):\n{wireexpr}"
            );
        }
        // A text past its type's bound and one its storage refuses leave the
        // field in the one state: absent, as it is gated, with the refusal
        // the store's answer and no early return past the reset.
        assert!(
            !wireexpr.contains(".ok_or(CodecError::LengthOutOfRange)?"),
            "a refusal of the bound of codec_zenoh_wireexpr's suffix returns before the field is \
             reset (views: {views}):\n{wireexpr}"
        );
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

/// A document of a codec whose field `seq` is gated by a disjunction of two
/// flags (`a || b`), one of which gates another field: no gate group holds
/// it, so the codec is a table whose row of `seq` the disjunction opens. A
/// value can set a flag of the disjunction and hold `seq` absent, which its
/// encode refuses, so the codec declares that its encode may fail.
const DISJUNCTION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encode-may-fail="true" sce:default-endian="big"
       name="codec_cold_disjunction">
  <datamodel>
    <sce:flags id="flags" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="wants_a" bit="0"/>
      <sce:flag name="wants_b" bit="1"/>
    </sce:flags>
    <sce:field id="extra" sce:type="uint8" sce:byte="1" sce:bit-size="8"
               sce:present-if="flags.wants_a"/>
    <sce:field id="seq" sce:type="uint16" sce:byte="2" sce:bit-size="16"
               sce:present-if="flags.wants_a || flags.wants_b"/>
  </datamodel>
</scxml>
"#;

/// A hot root of the set the disjunction is generated in, which reads no
/// other codec, so the disjunction is cold.
const ROOT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="codec_cold_root">
  <datamodel>
    <sce:field id="value" sce:type="uint64" sce:byte="0" sce:bit-size="vle"/>
  </datamodel>
</scxml>
"#;

/// A cold codec with a row a disjunction gates is read by its table, the row
/// opened where any clause of the gate is, and keeps its write printed for
/// that reason, said by name (docs/adr/0155 section 2): the flags of a
/// disjunction do not say alone whether its field is present, and the writer
/// opens a row by them.
#[test]
fn a_cold_codec_gated_by_a_disjunction_keeps_its_write_printed_and_says_why() {
    let dir = std::env::temp_dir().join(format!("sce_cold_disjunction_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory for the set");
    let names = ["codec_cold_root.scxml", "codec_cold_disjunction.scxml"];
    for (name, text) in names.iter().zip([ROOT, DISJUNCTION]) {
        std::fs::write(dir.join(name), text).expect("a document of the set");
    }
    let documents: Vec<String> = names
        .iter()
        .map(|name| dir.join(name).display().to_string())
        .collect();
    let sites = BodyReadSites::of_documents(&documents, &[], |_| false);
    let hot = sites
        .hot_codecs(["codec_cold_root"])
        .unwrap_or_else(|e| panic!("codec_cold_root is a codec of the set: {e}"));
    assert!(
        !hot.is_hot("codec_cold_disjunction"),
        "the root reads the disjunction"
    );
    let named = ForgeCompileOptions {
        inline_decode: InlineDecode::Codecs(hot),
        body_reads: sites,
        ..ForgeCompileOptions::default()
    };
    let output = sce_build::compile_forge_with_imports(
        DISJUNCTION,
        DocumentLabel::symmetric("codec_cold_disjunction"),
        Language::Rust,
        &dir,
        &named,
    );
    let _ = std::fs::remove_dir_all(&dir);
    let output =
        output.unwrap_or_else(|e| panic!("codec_cold_disjunction generates for Rust: {e}"));
    assert!(
        matches!(
            output.cold_codecs.as_slice(),
            [ColdCodec::Table(table)]
                if table.codec == "codec_cold_disjunction"
                    && table.not_written == Some(NotWrittenByTable::Disjunction)
        ),
        "the cold codec_cold_disjunction says other than a table whose write a \
         disjunction keeps printed: {:?}",
        output.cold_codecs
    );
    let code = output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n");
    for printed in [
        "::sce_forge_runtime::codec::read_table(cursor, &CODEC_COLD_DISJUNCTION_FIELDS, \
         &[], _out);",
        "gate: Gate::Any(&[Gate::Set(Bits::range(Source::Row(0), 0, 1)), \
         Gate::Set(Bits::range(Source::Row(0), 1, 1))]) },",
    ] {
        assert!(
            code.contains(printed),
            "the table of codec_cold_disjunction has no `{printed}`:\n{code}"
        );
    }
    assert!(
        !code.contains("write_table(") && code.contains("FieldTable::new(ROWS)"),
        "codec_cold_disjunction is written by the walker over a row a disjunction gates:\n{code}"
    );
}

/// A view of a type of its own is built once as a value from the locals its
/// fields are read into, with no default of it made first (docs/adr/0137),
/// and the walker stores into a value it is handed: a cold codec whose view is
/// its own type keeps that view printed and says why (docs/adr/0155 section
/// 2), its value alone answering the table's stores. A codec whose view is its
/// value says nothing of a view, and neither does a build with no views.
#[test]
fn a_cold_view_of_its_own_type_is_printed_and_says_why() {
    for views in [false, true] {
        let sites = sites_of(&["codec_zenoh_request.scxml"], views);
        let hot = sites
            .hot_codecs(["codec_zenoh_msg_put"])
            .unwrap_or_else(|e| panic!("codec_zenoh_msg_put is a codec of the set: {e}"));
        let named = ForgeCompileOptions {
            views,
            inline_decode: InlineDecode::Codecs(hot),
            body_reads: sites,
            ..ForgeCompileOptions::default()
        };
        for (name, struct_name, own_view) in [
            ("codec_zenoh_msg_del", "CodecZenohMsgDel", true),
            ("codec_zenoh_query", "CodecZenohQuery", true),
            ("codec_zenoh_wireexpr", "CodecZenohWireexpr", false),
        ] {
            let output = output_with(name, &named);
            let printed_view = (views && own_view).then_some(NotATable::OwnView);
            assert!(
                matches!(
                    output.cold_codecs.as_slice(),
                    [ColdCodec::Table(table)] if table.codec == name && table.view == printed_view
                ),
                "{name} says other than {printed_view:?} of its view (views: {views}): {:?}",
                output.cold_codecs
            );
            let code = output
                .files
                .into_iter()
                .map(|(_, code)| code)
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(
                code.contains(&format!("pub struct {struct_name}View")),
                views && own_view,
                "{name} has a view of its own otherwise than expected (views: {views}):\n{code}"
            );
            let stores: Vec<&str> = code
                .lines()
                .filter(|line| line.contains("::sce_forge_runtime::codec::TableOut<'f> for "))
                .collect();
            assert!(
                matches!(stores.as_slice(), [one] if one.ends_with(&format!(" for {struct_name}In<S> {{"))),
                "the stores of {name} are other than its value's alone (views: {views}): {stores:?}"
            );
        }
    }
}
