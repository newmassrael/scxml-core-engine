// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An arm that names a build feature is compiled only where the feature is on
//! (docs/adr/0072).
//!
//! The Rust codec gates every item that names the arm on
//! `#[cfg(feature = "<name>")]`, and the C and C++ codecs put it inside
//! `#if SCE_FEATURE_<NAME>`, which the header defines to 1 unless the build
//! defined it. A build without the feature has no such arm, and its decode
//! refuses the arm's tag (`CodecError::VariantArmNotBuilt`,
//! `SCE_FORGE_CODEC_VARIANT_ARM_NOT_BUILT`, and in C++ `std::nullopt` with
//! `CodecError::VariantArmNotBuilt` recorded on the cursor) rather than read
//! the frame as the catch-all's. The C++ variant's alternatives move where one
//! is left out, so such a variant names them by type, and a parent's carrier
//! bits count their index per build. Go, Kotlin and Python keep the arm in
//! every build, which is the feature on.
//!
//! What a document without the attribute renders is held by the committed
//! goldens of every codec under `tests/forge/expected/`, which this change
//! leaves as they were; the six `codec_zenoh_request` cases of
//! `forge_conformance.rs` (`forge_codec_zenoh_request_rust`,
//! `forge_c11_codec_zenoh_request`, `forge_codec_zenoh_request_cpp` and the
//! three others) hold the request below without its features.
//!
//! The decode of each Rust build is run by
//! `backends/rust/forge-conformance/tests/forge_feature_gated_arm.rs`, and of
//! each C++ build by the `feature_gated_arm_on` and `feature_gated_arm_off`
//! tests of `backends/cpp/forge-runtime/tests/conformance`.

use std::path::PathBuf;

use sce_build::forge::diagnostic::SingleDiagnostic;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// The network request of `tests/forge/resources/codec_zenoh_request.scxml`,
/// with its put and delete arms under one feature and the query arm as the
/// default and the catch-all.
const GATED_REQUEST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encode-may-fail="true" sce:default-endian="big" name="codec_zenoh_request">
  <sce:import src="codec_zenoh_wireexpr.scxml" kind="codec" as="codec_zenoh_wireexpr">
    <sce:flag-bind input="N" source="header.N"/>
  </sce:import>
  <sce:import src="codec_zenoh_ext_entry.scxml" kind="codec" as="codec_zenoh_ext_entry"/>
  <sce:import src="codec_zenoh_msg_put.scxml" kind="codec" as="codec_zenoh_msg_put"/>
  <sce:import src="codec_zenoh_msg_del.scxml" kind="codec" as="codec_zenoh_msg_del"/>
  <sce:import src="codec_zenoh_query.scxml"   kind="codec" as="codec_zenoh_query"/>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="mid" bit="0" width="5" value="0x1c"/>
      <sce:flag name="N"   bit="5"/>
      <sce:flag name="M"   bit="6"/>
      <sce:flag name="Z"   bit="7"/>
    </sce:flags>
    <sce:field id="rid" sce:type="uint64" sce:byte="1" sce:bit-size="vle"/>
    <sce:embed id="keyexpr" type="codec_zenoh_wireexpr" sce:byte="2"/>
    <sce:tlv-chain id="extensions" type="codec_zenoh_ext_entry" sce:byte="3"
                   max-depth="4" on-overflow="reject"
                   sce:present-if="header.Z"
                   terminate-on="entry-flag" entry-flag-name="Z"/>
    <sce:variant tag="peek.mid">
      <sce:peek-byte id="peek" sce:type="uint8">
        <sce:flag name="mid" bit="0" width="5"/>
      </sce:peek-byte>
      <sce:arm value="0x01" type="codec_zenoh_msg_put" feature="request-put-del"/>
      <sce:arm value="0x02" type="codec_zenoh_msg_del" feature="request-put-del"/>
      <sce:arm value="0x03" type="codec_zenoh_query" default="true"/>
      <sce:default type="codec_zenoh_query"/>
    </sce:variant>
  </datamodel>
</scxml>"#;

const CFG: &str = "#[cfg(feature = \"request-put-del\")]";
const CFG_OFF: &str = "#[cfg(not(feature = \"request-put-del\"))]";
const GATED: [&str; 2] = ["CodecZenohMsgPut", "CodecZenohMsgDel"];
const UNGATED: &str = "CodecZenohQuery";

fn generate(language: Language, options: &ForgeCompileOptions) -> String {
    let output = sce_build::compile_forge_with_imports(
        GATED_REQUEST,
        DocumentLabel::symmetric("codec_zenoh_request"),
        language,
        &resource_dir(),
        options,
    )
    .unwrap_or_else(|e| panic!("the gated request generates for {language:?}: {e}"));
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `line` is an item of its own for the arm whose variant is `arm`:
/// a variant of an enum, or the head of a match arm.
fn heads_arm(line: &str, arm: &str) -> bool {
    let t = line.trim_start();
    t.starts_with(&format!("{arm}("))
        || t.contains(&format!("::{arm}(_b) =>"))
        || t.contains(&format!("::{arm}(_) =>"))
}

/// The arms of the decode's `match` on the peeked tag, by their value.
fn heads_tag(line: &str, value: u8) -> bool {
    line.trim() == format!("{value}u8 => {{")
}

/// Every item of the Rust codec that names the put or the delete arm carries
/// the feature's `#[cfg]` on the line before it, and nothing that names the
/// query arm does: the variant of each of the three variant enums, each
/// `match` over them, the decode's arm on the tag, the source interface's
/// variant enum and its parts, and the encode.
#[test]
fn every_rust_item_of_a_gated_arm_carries_its_cfg_and_no_other_does() {
    let code = generate(
        Language::Rust,
        &ForgeCompileOptions {
            views: true,
            ..ForgeCompileOptions::default()
        },
    );
    let lines: Vec<&str> = code.lines().collect();
    let mut gated_heads = 0usize;
    let mut query_heads = 0usize;
    for pair in lines.windows(2) {
        let (previous, line) = (pair[0].trim(), pair[1]);
        let gated = GATED.iter().any(|arm| heads_arm(line, arm))
            || heads_tag(line, 0x01)
            || heads_tag(line, 0x02);
        if gated {
            gated_heads += 1;
            assert_eq!(
                previous, CFG,
                "an item of a gated arm is not under its feature:\n{line}"
            );
        }
        if heads_arm(line, UNGATED) || heads_tag(line, 0x03) {
            query_heads += 1;
            assert!(
                !previous.starts_with("#[cfg"),
                "an item of the query arm, which every build holds, is under a cfg:\n{previous}\n{line}"
            );
        }
    }
    // Two gated arms in: three variant enums, two decodes (the builder's and
    // the view's), three projections, the source enum, its parts, and the
    // `body` accessors of the parts and the three forms, once per build.
    assert!(
        gated_heads >= 30,
        "only {gated_heads} item(s) of a gated arm found; the walk above checked too little"
    );
    assert!(
        query_heads >= 15,
        "only {query_heads} item(s) of the query arm found"
    );

    // Every `#[cfg]` of the feature stands in front of something the gated
    // arms alone use.
    for pair in lines.windows(2) {
        if pair[0].trim() != CFG {
            continue;
        }
        let next = pair[1];
        let follows_a_gated_item = GATED.iter().any(|arm| heads_arm(next, arm))
            || heads_tag(next, 0x01)
            || heads_tag(next, 0x02)
            || next.starts_with("use super::codec_zenoh_msg_put::")
            || next.starts_with("use super::codec_zenoh_msg_del::")
            || next.trim_start().starts_with("fn body(&self)")
            || next
                .trim_start()
                .starts_with("/// The arm of the variant this value holds.");
        assert!(
            follows_a_gated_item,
            "the feature's cfg stands in front of something every build holds:\n{next}"
        );
    }
}

/// The imports the gated arms alone use are under the feature, in each form a
/// codec imports a body by; the query's are not.
#[test]
fn an_import_only_a_gated_arm_uses_is_under_its_feature() {
    let code = generate(
        Language::Rust,
        &ForgeCompileOptions {
            views: true,
            ..ForgeCompileOptions::default()
        },
    );
    let lines: Vec<&str> = code.lines().collect();
    for import in [
        "use super::codec_zenoh_msg_put::CodecZenohMsgPut;",
        "use super::codec_zenoh_msg_del::CodecZenohMsgDel;",
        "use super::codec_zenoh_msg_put::CodecZenohMsgPutView;",
        "use super::codec_zenoh_msg_put::CodecZenohMsgPutOwned;",
    ] {
        assert_eq!(
            line_before(&lines, import),
            CFG,
            "`{import}` is not under the feature"
        );
    }
    for import in [
        "use super::codec_zenoh_query::CodecZenohQuery;",
        "use super::codec_zenoh_query::CodecZenohQueryView;",
        "use super::codec_zenoh_wireexpr::CodecZenohWireexpr;",
    ] {
        assert!(
            !line_before(&lines, import).starts_with("#[cfg"),
            "`{import}`, which every build uses, is under a cfg"
        );
    }
}

/// The line before the one that is `wanted`, trimmed.
fn line_before<'a>(lines: &[&'a str], wanted: &str) -> &'a str {
    let at = lines
        .iter()
        .position(|l| *l == wanted)
        .unwrap_or_else(|| panic!("the codec has no line `{wanted}`"));
    lines[at - 1].trim()
}

/// The source interface's variant enum declares a type parameter per arm, and
/// the gated arms' parameters are under the feature; the accessor that names
/// the enum is written once per build, with the arms that build holds.
#[test]
fn the_source_interface_names_the_arms_each_build_holds() {
    let code = generate(Language::Rust, &ForgeCompileOptions::default());
    assert!(
        code.contains(&format!(
            "pub enum CodecZenohRequestVariantSource<'s, {CFG} A0: ?Sized, {CFG} A1: ?Sized, A2: ?Sized, D: ?Sized> {{"
        )),
        "the gated arms' type parameters are not under the feature:\n{code}"
    );
    let lines: Vec<&str> = code.lines().collect();
    let mut on = 0usize;
    let mut off = 0usize;
    for (at, line) in lines.iter().enumerate() {
        if !line.contains("fn body(&self) -> CodecZenohRequestVariantSource<'_, ") {
            continue;
        }
        // The `#[cfg]` is the line before, or the one before the trait's doc.
        let cfg = lines[at.saturating_sub(2)..at]
            .iter()
            .map(|l| l.trim())
            .find(|l| l.starts_with("#[cfg"))
            .unwrap_or_else(|| panic!("an accessor of the variant without a cfg:\n{line}"));
        match line.matches("impl ").count() {
            4 => {
                assert_eq!(cfg, CFG, "the accessor naming all four arms:\n{line}");
                on += 1;
            }
            2 => {
                assert_eq!(
                    cfg, CFG_OFF,
                    "the accessor naming the query and the catch-all:\n{line}"
                );
                off += 1;
            }
            n => panic!("an accessor naming {n} arm(s):\n{line}"),
        }
    }
    // The trait, the reference impl, the parts and the builder; the view and
    // the owned form when the codec has them.
    assert!(on >= 4, "{on} accessor(s) for the build with the feature");
    assert_eq!(on, off, "every accessor is written for both builds");
}

/// In a build without the feature, each decode's `match` on the tag refuses
/// the gated arm's tag in the arm's place, under `cfg(not(..))`, and the query
/// arm's tag has no such refusal.
#[test]
fn a_rust_build_without_the_feature_refuses_the_gated_arms_tag() {
    let code = generate(
        Language::Rust,
        &ForgeCompileOptions {
            views: true,
            ..ForgeCompileOptions::default()
        },
    );
    let lines: Vec<&str> = code.lines().collect();
    let mut refusals: Vec<String> = Vec::new();
    for pair in lines.windows(2) {
        let next = pair[1].trim();
        if next.contains("CodecError::VariantArmNotBuilt") {
            assert_eq!(
                pair[0].trim(),
                CFG_OFF,
                "a refusal of a gated arm's tag stands in every build:\n{next}"
            );
            refusals.push(next.to_string());
        }
    }
    // The builder's decode and the view's, each refusing tags 1 and 2.
    let expected: Vec<String> = ["1u8", "2u8", "1u8", "2u8"]
        .iter()
        .map(|tag| format!("{tag} => return Err(CodecError::VariantArmNotBuilt),"))
        .collect();
    assert_eq!(refusals, expected);
}

/// Every item of the C codec that names the put or the delete arm sits inside
/// `#if SCE_FEATURE_REQUEST_PUT_DEL`, which the header defines to 1 before its
/// first use unless the build defined it; the query arm's items do not.
#[test]
fn every_c_item_of_a_gated_arm_is_inside_its_macro() {
    let code = generate(Language::C11, &ForgeCompileOptions::default());
    let default_on =
        "#ifndef SCE_FEATURE_REQUEST_PUT_DEL\n#define SCE_FEATURE_REQUEST_PUT_DEL 1\n#endif\n";
    assert_eq!(
        code.matches(default_on).count(),
        1,
        "the macro is defined to 1 once, unless the build defined it:\n{code}"
    );
    let guard = "#if SCE_FEATURE_REQUEST_PUT_DEL";
    let first_use = code.find(guard).expect("the gated arms are guarded");
    assert!(
        code.find(default_on).expect("defined") < first_use,
        "the macro is used before the header defines it"
    );

    let lines: Vec<&str> = code.lines().collect();
    let gated_item = |line: &str| {
        let t = line.trim();
        t.contains("CODEC_ZENOH_MSG_PUT")
            || t.contains("CODEC_ZENOH_MSG_DEL")
            || t.starts_with("codec_zenoh_msg_put_t ")
            || t.starts_with("codec_zenoh_msg_del_t ")
            || t == "case 1:"
            || t == "case 2:"
    };
    // The kind constant, the union member, the decode's case and the encode's,
    // for each of the two arms; the default initializer names the query arm.
    let guards: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim() == guard)
        .map(|(at, _)| at)
        .collect();
    assert_eq!(guards.len(), 8, "four items of each gated arm:\n{code}");
    for at in guards {
        assert!(
            gated_item(lines[at + 1]),
            "`{guard}` stands in front of something every build holds:\n{}",
            lines[at + 1]
        );
        let close = lines[at + 1..]
            .iter()
            .position(|l| l.trim() == "#endif")
            .expect("the guard is closed");
        assert!(
            lines[at + 1..at + 1 + close]
                .iter()
                .all(|l| !l.contains("CODEC_ZENOH_QUERY") && !l.contains("codec_zenoh_query")),
            "the query arm is inside the gated arm's guard"
        );
    }
    // The decode's case of each gated arm has its refusal in the other branch
    // of the same guard: a build without the feature refuses the tag.
    let refusals: Vec<(&str, &str)> = lines
        .windows(3)
        .filter(|w| w[0].trim() == "#else" && w[1].trim().starts_with("case "))
        .map(|w| (w[1].trim(), w[2].trim()))
        .collect();
    assert_eq!(
        refusals,
        vec![
            ("case 1:", "return SCE_FORGE_CODEC_VARIANT_ARM_NOT_BUILT;"),
            ("case 2:", "return SCE_FORGE_CODEC_VARIANT_ARM_NOT_BUILT;"),
        ],
        "each gated arm's decode case is refused where its macro is 0"
    );
    for (at, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t == "CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_QUERY," || t == "case 3:" {
            assert_ne!(
                lines[at - 1].trim(),
                guard,
                "the query arm, which every build holds, is guarded:\n{t}"
            );
        }
    }
}

/// The C++ macro of the request's gated arms.
const CPP_GUARD: &str = "#if SCE_FEATURE_REQUEST_PUT_DEL";
const CPP_PUT: &str = "::SCE::Generated::CodecZenohMsgPut::CodecZenohMsgPut";
const CPP_DEL: &str = "::SCE::Generated::CodecZenohMsgDel::CodecZenohMsgDel";
const CPP_QUERY: &str = "::SCE::Generated::CodecZenohQuery::CodecZenohQuery";

/// Whether `code` names a `std::variant` alternative by a fixed index: an
/// `in_place_index`, or `.index() == ` followed by a number. Where an
/// alternative before it is under a feature, that number is the alternative's
/// index only in the build where the feature is on.
fn names_an_alternative_by_a_fixed_index(code: &str) -> bool {
    code.contains("in_place_index")
        || code
            .match_indices(".index() == ")
            .any(|(at, needle)| code[at + needle.len()..].starts_with(|c: char| c.is_ascii_digit()))
}

/// Every item of the C++ codec that names the put or the delete arm sits
/// inside `#if SCE_FEATURE_REQUEST_PUT_DEL`, which the header defines to 1
/// before its first use unless the build defined it: the arm's alternative of
/// the `std::variant`, its decode case and its encode branch. The query arm's
/// items do not.
#[test]
fn every_cpp_item_of_a_gated_arm_is_inside_its_macro() {
    let code = generate(Language::Cpp, &ForgeCompileOptions::default());
    let default_on =
        "#ifndef SCE_FEATURE_REQUEST_PUT_DEL\n#define SCE_FEATURE_REQUEST_PUT_DEL 1\n#endif\n";
    assert_eq!(
        code.matches(default_on).count(),
        1,
        "the macro is defined to 1 once, unless the build defined it:\n{code}"
    );
    let first_use = code.find(CPP_GUARD).expect("the gated arms are guarded");
    assert!(
        code.find(default_on).expect("defined") < first_use,
        "the macro is used before the header defines it"
    );

    // The alternatives: each gated one with its separator inside its guard,
    // so the list is well formed in both builds.
    let alternatives = [
        "using CodecZenohRequestVariant = std::variant<".to_string(),
        CPP_GUARD.to_string(),
        format!("    {CPP_PUT},"),
        "#endif".to_string(),
        CPP_GUARD.to_string(),
        format!("    {CPP_DEL},"),
        "#endif".to_string(),
        format!("    {CPP_QUERY},"),
        "    CodecZenohRequestDefault".to_string(),
        ">;".to_string(),
    ]
    .join("\n");
    assert!(
        code.contains(&alternatives),
        "the variant's alternatives are not the gated list:\n{code}"
    );

    let lines: Vec<&str> = code.lines().collect();
    let gated_item = |line: &str| {
        let t = line.trim();
        t.contains("CodecZenohMsgPut")
            || t.contains("CodecZenohMsgDel")
            || t == "case 1: {"
            || t == "case 2: {"
    };
    // The alternative, the decode case and the encode branch of each of the
    // two arms.
    let guards: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim() == CPP_GUARD)
        .map(|(at, _)| at)
        .collect();
    assert_eq!(guards.len(), 6, "three items of each gated arm:\n{code}");
    for at in guards {
        assert!(
            gated_item(lines[at + 1]),
            "`{CPP_GUARD}` stands in front of something every build holds:\n{}",
            lines[at + 1]
        );
        let close = lines[at + 1..]
            .iter()
            .position(|l| matches!(l.trim(), "#endif" | "#else"))
            .expect("the guard is closed");
        assert!(
            lines[at + 1..at + 1 + close]
                .iter()
                .all(|l| !l.contains("CodecZenohQuery")),
            "the query arm is inside the gated arm's guard"
        );
    }
    let query_alternative = format!("{CPP_QUERY},");
    let query_encode = format!("if (auto _p = std::get_if<{CPP_QUERY}>");
    let mut query_items = 0usize;
    for (at, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t == query_alternative || t == "case 3: {" || t.starts_with(query_encode.as_str()) {
            query_items += 1;
            assert_ne!(
                lines[at - 1].trim(),
                CPP_GUARD,
                "the query arm, which every build holds, is guarded:\n{t}"
            );
        }
    }
    assert_eq!(query_items, 3, "the query's alternative, case and encode");
}

/// In a C++ build without the feature, the decode's case for each gated arm's
/// tag is the other branch of the arm's guard, and it records the refusal on
/// the cursor before it returns, so a caller tells it from a frame short of
/// bytes.
#[test]
fn a_cpp_build_without_the_feature_refuses_the_gated_arms_tag() {
    let code = generate(Language::Cpp, &ForgeCompileOptions::default());
    let lines: Vec<&str> = code.lines().collect();
    let refusals: Vec<[&str; 4]> = lines
        .windows(5)
        .filter(|w| w[0].trim() == "#else")
        .map(|w| [w[1].trim(), w[2].trim(), w[3].trim(), w[4].trim()])
        .collect();
    let refusal = |case: &'static str| {
        [
            case,
            "cursor.refuse(::SCE::Forge::CodecError::VariantArmNotBuilt);",
            "return std::nullopt;",
            "#endif",
        ]
    };
    assert_eq!(
        refusals,
        vec![refusal("case 1:"), refusal("case 2:")],
        "each gated arm's decode case is refused where its macro is 0:\n{code}"
    );
}

/// A C++ variant with a gated arm names no alternative by a fixed index: its
/// default value is the default arm chosen by type, which every build holds
/// at a different index (2 with the feature, 0 without).
#[test]
fn a_cpp_variant_with_a_gated_arm_names_its_alternatives_by_type() {
    let code = generate(Language::Cpp, &ForgeCompileOptions::default());
    assert!(
        code.contains(&format!(
            "    CodecZenohRequestVariant body{{std::in_place_type_t<{CPP_QUERY}>{{}}}};\n"
        )),
        "the default value is not the query arm chosen by type:\n{code}"
    );
    assert!(
        !names_an_alternative_by_a_fixed_index(&code),
        "an alternative is named by an index that moves with the feature:\n{code}"
    );
}

/// When the last arm is under a feature and no catch-all follows it, its
/// separator goes before it, inside its guard, so the build without it does
/// not end the list with a comma.
#[test]
fn a_cpp_gated_last_alternative_carries_its_separator_before_it() {
    let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="gated_last">
  <sce:import src="codec_peek_arm_a.scxml" kind="codec" as="codec_peek_arm_a"/>
  <sce:import src="codec_peek_arm_b.scxml" kind="codec" as="codec_peek_arm_b"/>
  <datamodel>
    <sce:variant tag="peek.kind">
      <sce:peek-byte id="peek" sce:type="uint8">
        <sce:flag name="kind" bit="0" width="1"/>
      </sce:peek-byte>
      <sce:arm value="0x00" type="codec_peek_arm_a" default="true"/>
      <sce:arm value="0x01" type="codec_peek_arm_b" feature="gated-arm"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let code = sce_build::compile_forge_with_imports(
        document,
        DocumentLabel::symmetric("gated_last"),
        Language::Cpp,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("the gated variant generates for C++: {e}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n");
    let alternatives = [
        "using GatedLastVariant = std::variant<",
        "    ::SCE::Generated::CodecPeekArmA::CodecPeekArmA",
        "#if SCE_FEATURE_GATED_ARM",
        "    , ::SCE::Generated::CodecPeekArmB::CodecPeekArmB",
        "#endif",
        ">;",
    ]
    .join("\n");
    assert!(
        code.contains(&alternatives),
        "the gated last alternative does not carry its separator before it:\n{code}"
    );
}

/// A parent whose carrier bits an embedded variant's arm fills compares the
/// embedded body's index with one counted per build: the term of the gated
/// arm is under its macro, and the arm after it adds 1 where the macro is
/// non-zero. The last arm every build holds is the fallthrough.
#[test]
fn a_cpp_parent_counts_a_gated_variants_index_per_build() {
    let dir = gated_dispatch_fixture();
    let parent = std::fs::read_to_string(dir.join("gated_parent.scxml")).expect("read parent");
    let output = sce_build::compile_forge_with_imports(
        &parent,
        DocumentLabel::symmetric("gated_parent"),
        Language::Cpp,
        &dir,
        &ForgeCompileOptions::default(),
    );
    let _ = std::fs::remove_dir_all(&dir);
    let code = output
        .unwrap_or_else(|e| panic!("the parent of a gated variant generates for C++: {e}"))
        .files
        .into_iter()
        .find(|(name, _)| name.contains("gated_parent"))
        .map(|(_, code)| code)
        .expect("the parent's header");
    let default_on = "#ifndef SCE_FEATURE_LOCAL_ARM\n#define SCE_FEATURE_LOCAL_ARM 1\n#endif\n";
    assert_eq!(
        code.matches(default_on).count(),
        1,
        "the parent defines the embedded arm's macro to 1 once:\n{code}"
    );
    let derived = [
        "        const std::uint8_t _derived_header = static_cast<std::uint8_t>((",
        "#if SCE_FEATURE_LOCAL_ARM",
        "            key.body.index() == static_cast<std::size_t>(0) ? static_cast<std::uint8_t>(0x00) :",
        "#endif",
        "            key.body.index() == static_cast<std::size_t>(0 + (SCE_FEATURE_LOCAL_ARM ? 1 : 0)) ? static_cast<std::uint8_t>(0x20) :",
        "            static_cast<std::uint8_t>(0x40)));",
    ]
    .join("\n");
    assert!(
        code.contains(&derived),
        "the carrier bits do not count the embedded index per build:\n{code}"
    );
    assert!(
        code.find(default_on).expect("defined") < code.find(&derived).expect("derived"),
        "the macro is used before the parent defines it"
    );
    assert!(
        !names_an_alternative_by_a_fixed_index(&code),
        "the parent names an embedded alternative by an index that moves:\n{code}"
    );
}

/// A parent that dispatches an embedded tag-less variant on two bits of its
/// header, the variant's first arm under the feature `local-arm`, its second
/// the default arm and its third an arm every build holds, written to a fresh
/// directory the caller removes.
fn gated_dispatch_fixture() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "sce_gated_dispatch_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("make the fixture directory");
    let body = |name: &str, ty: &str, bits: u32| {
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}">
  <datamodel>
    <sce:field id="payload" sce:type="{ty}" sce:byte="0" sce:bit-size="{bits}"/>
  </datamodel>
</scxml>"#
        )
    };
    let documents = [
        (
            "gated_parent",
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="gated_parent">
  <sce:import src="gated_child.scxml" kind="codec" as="gated_child">
    <sce:variant-dispatch flag="header.K"/>
  </sce:import>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="mid" bit="0" width="5" value="0x1d"/>
      <sce:flag name="K" bit="5" width="2"/>
    </sce:flags>
    <sce:embed id="key" type="gated_child" sce:byte="1"/>
  </datamodel>
</scxml>"#
                .to_string(),
        ),
        (
            "gated_child",
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="gated_child">
  <sce:import src="gated_local.scxml" kind="codec" as="gated_local"/>
  <sce:import src="gated_nonlocal.scxml" kind="codec" as="gated_nonlocal"/>
  <sce:import src="gated_wide.scxml" kind="codec" as="gated_wide"/>
  <datamodel>
    <sce:variant>
      <sce:arm value="0x00" type="gated_local" feature="local-arm"/>
      <sce:arm value="0x01" type="gated_nonlocal" default="true"/>
      <sce:arm value="0x02" type="gated_wide"/>
    </sce:variant>
  </datamodel>
</scxml>"#
                .to_string(),
        ),
        ("gated_local", body("gated_local", "uint8", 8)),
        ("gated_nonlocal", body("gated_nonlocal", "uint16", 16)),
        ("gated_wide", body("gated_wide", "uint32", 32)),
    ];
    for (name, text) in documents {
        std::fs::write(dir.join(format!("{name}.scxml")), text).expect("write a fixture");
    }
    dir
}

/// A Rust build without the feature keeps only the arms without one, and the
/// variant enum takes a lifetime when one of those arms borrows. When only a
/// gated arm borrows, no one declaration fits both builds, and the generator
/// says so rather than emit a codec one build cannot compile.
#[test]
fn a_lifetime_only_a_gated_arm_needs_is_refused() {
    let document = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:encode-may-fail="true" sce:default-endian="big" name="gated_borrow">
  <sce:import src="codec_variant_session_open.scxml" kind="codec" as="codec_variant_session_open"/>
  <sce:import src="codec_length_ref.scxml" kind="codec" as="codec_length_ref"/>
  <datamodel>
    <data id="msg_id" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:variant tag="msg_id">
      <sce:arm value="0x01" type="codec_variant_session_open" default="true"/>
      <sce:arm value="0x02" type="codec_length_ref" feature="length-ref-arm"/>
      <sce:default type="codec_variant_session_open"/>
    </sce:variant>
  </datamodel>
</scxml>"#;
    let err = match sce_build::compile_forge_with_imports(
        document,
        DocumentLabel::symmetric("gated_borrow"),
        Language::Rust,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("a lifetime only the gated arm needs must be refused"),
        Err(e) => e,
    };
    assert_eq!(
        err.diagnostic_payload().code.as_str(),
        "generate/unsupported-feature"
    );
    assert!(
        err.to_string().contains(
            "the arm `codec_length_ref` (value 0x2) names the build feature 'length-ref-arm'"
        ),
        "the refusal names the arm and its feature; got: {err}"
    );
}
