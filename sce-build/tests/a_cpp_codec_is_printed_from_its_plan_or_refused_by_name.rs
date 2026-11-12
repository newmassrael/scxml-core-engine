// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which codecs the C++ backend prints from their encode plan, and which it
//! refuses by name (docs/adr/0081 step 4).
//!
//! The C++ backend writes every codec from its plan and has no other encode: a
//! codec whose plan uses a form the C++ printer does not lower is refused by
//! that form (`generate/unsupported-feature`), and so is every codec and every
//! document that holds it. This pins the codecs of the resources it prints and
//! each codec of the repository it refuses, with the clause it refuses it by,
//! so a change to the plan or to `cpp_source::unlowered` that refuses a codec
//! by accident, or prints one it refused, is seen here.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, compile_forge_from_string, cpp_codec_printed_file,
    cpp_codec_unlowered_file, DocumentLabel, ForgeCompileOptions,
};

/// The codecs of `tests/forge/resources` the C++ backend prints, by document
/// stem, in order.
const PRINTED: &[&str] = &[
    "broadcast_log_conformance_event",
    "broadcast_log_conformance_record",
    "codec_cbor_map",
    "codec_chain_depth_entry",
    "codec_chain_depth_gated",
    "codec_chain_depth_plain",
    "codec_chain_flag_carrier",
    "codec_chain_flag_entry",
    "codec_chain_flag_forward",
    "codec_chain_handed",
    "codec_chain_handed_entry",
    "codec_chain_has_envelope",
    "codec_chain_has_input",
    "codec_chain_has_marker",
    "codec_chain_has_marker_slice",
    "codec_chain_has_slice",
    "codec_chain_has_tagged",
    "codec_chain_has_tagged_entry",
    "codec_chain_prev_carrier",
    "codec_chain_prev_entry",
    "codec_chain_prev_gated",
    "codec_chain_prev_tail",
    "codec_chain_prev_tail_gated",
    "codec_content_line_enums",
    "codec_content_line_event",
    "codec_content_line_open",
    "codec_content_line_records",
    "codec_content_line_required_lists",
    "codec_declared_bounds",
    "codec_declared_bounds_gated",
    "codec_declared_bounds_to_end",
    "codec_default_marker_arm_a",
    "codec_default_marker_arm_b",
    "codec_dma_aligned_basic",
    "codec_embed_basic",
    "codec_embed_variant_dispatch",
    "codec_encoded_length_checked",
    "codec_enum_fields",
    "codec_enum_gated",
    "codec_every_arm_says",
    "codec_every_arm_says_far",
    "codec_every_arm_says_near",
    "codec_every_arm_says_parent",
    "codec_ext_attachment",
    "codec_ext_encoding_info",
    "codec_ext_timestamp",
    "codec_fixed_after_lengthref",
    "codec_flags_basic",
    "codec_group_enum",
    "codec_init_cookie_body",
    "codec_init_syn_body",
    "codec_init_syn_envelope",
    "codec_length_ref",
    "codec_length_ref_dotted_basic",
    "codec_length_ref_uint16_be",
    "codec_length_ref_uint16_le",
    "codec_length_ref_uint32_le",
    "codec_length_shared",
    "codec_length_stated",
    "codec_little_endian",
    "codec_nested_body",
    "codec_nested_parent",
    "codec_optional_field_attributes",
    "codec_origin_choice",
    "codec_origin_envelope",
    "codec_origin_leaf",
    "codec_origin_scalar",
    "codec_peek_arm_a",
    "codec_peek_arm_b",
    "codec_present_if_basic",
    "codec_present_if_checked",
    "codec_present_if_disjunction",
    "codec_present_if_length_ref",
    "codec_present_if_negation",
    "codec_present_if_string",
    "codec_present_if_tail",
    "codec_present_if_vle",
    "codec_qos_byte",
    "codec_quantity_temperature",
    "codec_repeat_basic",
    "codec_repeat_elem",
    "codec_repeat_flag_carrier",
    "codec_repeat_flag_gated",
    "codec_repeat_flag_group",
    "codec_repeat_present_if_basic",
    "codec_repeat_unconditional_count",
    "codec_scout_zid_body",
    "codec_simple_frame",
    "codec_some_arms_say",
    "codec_some_arms_say_forward",
    "codec_some_arms_say_parent",
    "codec_some_arms_say_plain",
    "codec_source_leaf",
    "codec_source_parent_tag",
    "codec_subbyte",
    "codec_tail",
    "codec_tlv_chain_basic",
    "codec_tlv_chain_present_if_basic",
    "codec_tlv_entry",
    "codec_transport_envelope",
    "codec_until_eof_basic",
    "codec_variant_default_marker",
    "codec_variant_dispatch",
    "codec_variant_peek_basic",
    "codec_variant_session_close",
    "codec_variant_session_open",
    "codec_vle_zint_u64",
    "codec_zenoh_close",
    "codec_zenoh_decl_ext_keyexpr",
    "codec_zenoh_decl_ext_keyexpr_inner",
    "codec_zenoh_decl_final",
    "codec_zenoh_decl_kexpr",
    "codec_zenoh_decl_queryable",
    "codec_zenoh_decl_subscriber",
    "codec_zenoh_decl_token",
    "codec_zenoh_declaration",
    "codec_zenoh_declare",
    "codec_zenoh_del",
    "codec_zenoh_encoding",
    "codec_zenoh_err",
    "codec_zenoh_ext_entry",
    "codec_zenoh_ext_envelope",
    "codec_zenoh_ext_unit",
    "codec_zenoh_ext_zbuf",
    "codec_zenoh_ext_zint",
    "codec_zenoh_fragment",
    "codec_zenoh_frame",
    "codec_zenoh_hello",
    "codec_zenoh_init_body",
    "codec_zenoh_interest",
    "codec_zenoh_interest_body",
    "codec_zenoh_join",
    "codec_zenoh_keep_alive",
    "codec_zenoh_locator",
    "codec_zenoh_msg_del",
    "codec_zenoh_msg_put",
    "codec_zenoh_network_envelope",
    "codec_zenoh_oam",
    "codec_zenoh_open_body",
    "codec_zenoh_push",
    "codec_zenoh_push_body",
    "codec_zenoh_put",
    "codec_zenoh_query",
    "codec_zenoh_query_value",
    "codec_zenoh_reply",
    "codec_zenoh_request",
    "codec_zenoh_response",
    "codec_zenoh_response_final",
    "codec_zenoh_scout",
    "codec_zenoh_source_info",
    "codec_zenoh_source_info_ext",
    "codec_zenoh_timestamp",
    "codec_zenoh_timestamp_ext",
    "codec_zenoh_undecl_kexpr",
    "codec_zenoh_undecl_queryable",
    "codec_zenoh_undecl_subscriber",
    "codec_zenoh_undecl_token",
    "codec_zenoh_value_entry",
    "codec_zenoh_value_slice",
    "codec_zenoh_value_zbuf",
    "codec_zenoh_wireexpr",
    "keyexpr_entry",
    "queue_conformance_event",
    "subscription_entry",
];

/// The directory of the resources.
fn resources() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources")
}

/// Every document of the resources, by path, in order.
fn resource_documents() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(resources())
        .expect("the resources directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    paths.sort();
    paths
}

/// The directories of the repository that hold codec documents beside the
/// resources: the Rust conformance fixtures, the conformance contexts, the
/// standard library, the test fixtures and the kind examples.
const OTHER_CODEC_ROOTS: &[&str] = &[
    "../backends/rust/forge-conformance/fixtures",
    "../tests/forge/conformance",
    "../stdlib",
    "tests/fixtures",
    "kind-examples",
];

/// Every document under `dir` that declares itself a codec, by path, in order.
fn codec_documents_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            codec_documents_under(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "scxml")
            && std::fs::read_to_string(&path).is_ok_and(|text| text.contains("sce:kind=\"codec\""))
        {
            out.push(path);
        }
    }
}

/// The codecs of `tests/forge/resources` the C++ backend refuses, by document
/// stem, each with the clause it is refused by, in order. None: every codec of
/// the resources is printed.
const REFUSED: &[(&str, &str)] = &[];

/// The codecs of the repository outside the resources that the C++ backend
/// refuses, by path from `sce-build`, each with the clause it is refused by, in
/// order. None: every one of them is printed.
const REFUSED_ELSEWHERE: &[(&str, &str)] = &[];

/// `pairs` as owned pairs, to compare with what a census found.
fn owned(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(what, why)| (what.to_string(), why.to_string()))
        .collect()
}

/// The refusal a generation answers, spelled as its diagnostic prints; a
/// generation that writes its output instead fails the test with `why`. The
/// output is matched rather than taken by `expect_err`, which would need it to
/// print itself.
fn refusal_of<T, E: std::fmt::Debug>(generated: Result<T, E>, why: &str) -> String {
    match generated {
        Ok(_) => panic!("{why}"),
        Err(refusal) => format!("{refusal:?}"),
    }
}

/// Holds that the C++ backend refuses the codec at `path` by `form`, the first
/// form of it the printer does not lower: its generation answers the codec's
/// refusal, and writes nothing.
fn assert_refused_by(path: &Path, form: &str) {
    let refusal = refusal_of(
        compile_forge_file(path, Language::Cpp, &[], &ForgeCompileOptions::default()),
        "a codec the C++ printer does not lower is not written",
    );
    assert!(
        refusal.contains("UnsupportedFeature") && refusal.contains(form),
        "{} is refused by `{form}`: {refusal}",
        path.display()
    );
}

/// The C++ backend prints every codec of the repository outside the resources
/// but [`REFUSED_ELSEWHERE`], which it refuses by name.
#[test]
fn the_cpp_backend_prints_every_codec_of_the_repository_or_refuses_it_by_name() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    for root in OTHER_CODEC_ROOTS {
        codec_documents_under(&manifest.join(root), &mut paths);
    }
    paths.sort();
    let mut refused: Vec<(String, String)> = Vec::new();
    for path in &paths {
        let shown = path
            .strip_prefix(manifest)
            .unwrap_or(path)
            .display()
            .to_string();
        // Every document here is one the repository reads, so a document that
        // cannot be read is a defect of its own and not a codec refused.
        let unlowered = cpp_codec_unlowered_file(path, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{shown}: {e:?}"))
            .expect("a document that declares itself a codec is one");
        if let Some(form) = unlowered.first() {
            assert_refused_by(path, form);
            refused.push((shown, form.clone()));
        }
    }
    eprintln!(
        "{} codec document(s) read; {} refused:",
        paths.len(),
        refused.len()
    );
    for (shown, form) in &refused {
        eprintln!("  {shown}: {form}");
    }
    assert_eq!(
        refused,
        owned(REFUSED_ELSEWHERE),
        "the C++ backend refuses these codecs of the repository by name; a change that \
         refuses one, or prints one it refused, names it in REFUSED_ELSEWHERE \
         (docs/adr/0081 step 4)"
    );
}

/// The C++ backend prints exactly the codecs of the resources whose every form
/// it lowers, [`PRINTED`], and refuses each other one by the first form it does
/// not lower, [`REFUSED`].
#[test]
fn the_cpp_backend_prints_each_codec_of_the_resources_or_refuses_it_by_name() {
    let mut printed: Vec<String> = Vec::new();
    // Each codec the C++ backend refuses, with the first form it does not
    // lower, which its refusal names.
    let mut refused: Vec<(String, String)> = Vec::new();
    for path in resource_documents() {
        let stem = path
            .file_stem()
            .expect("a document has a stem")
            .to_string_lossy()
            .into_owned();
        // A document the generator refuses is printed by no backend, and a
        // document that is no codec has no encode to print.
        let Ok(Some(unlowered)) =
            cpp_codec_unlowered_file(&path, &[], &ForgeCompileOptions::default())
        else {
            continue;
        };
        assert_eq!(
            cpp_codec_printed_file(&path, &[], &ForgeCompileOptions::default()).ok(),
            Some(Some(unlowered.is_empty())),
            "{stem}: printed exactly where no form is unlowered"
        );
        match unlowered.first() {
            None => printed.push(stem),
            Some(form) => {
                assert_refused_by(&path, form);
                refused.push((stem, form.clone()));
            }
        }
    }
    // The census of what each refused codec is refused by, shown with
    // `--nocapture`.
    eprintln!("{} codec(s) refused:", refused.len());
    for (stem, form) in &refused {
        eprintln!("  {stem}: {form}");
    }
    assert_eq!(
        printed, PRINTED,
        "the C++ backend prints these codecs of the resources from their plan \
         (docs/adr/0081 step 4)"
    );
    assert_eq!(
        refused,
        owned(REFUSED),
        "the C++ backend refuses these codecs of the resources by name; a change that \
         refuses one, or prints one it refused, names it in REFUSED (docs/adr/0081 step 4)"
    );
}

/// A codec `name` whose `<datamodel>` holds `fields`, as a document.
fn codec_document(name: &str, imports: &str, fields: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}">
  {imports}
  <datamodel>
    {fields}
  </datamodel>
</scxml>"#
    )
}

/// A codec of a tag and a reading in a `float32`, a number no C++ value has a
/// type for yet.
fn measure_document() -> String {
    codec_document(
        "measure",
        "",
        r#"<sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="reading" sce:type="float32" sce:byte="1" sce:bit-size="32"/>"#,
    )
}

/// What the C++ backend refuses `measure` by.
const MEASURE_REFUSED_BY: &str = "field 'reading': a `float32` field has no C++ type yet";

/// A codec with a field of a type no C++ value presents is refused by that
/// field, and nothing is written for it.
#[test]
fn a_codec_holding_a_number_cpp_has_no_type_for_is_refused_by_its_field() {
    let refusal = refusal_of(
        compile_forge_from_string(
            &measure_document(),
            DocumentLabel {
                identifier: "measure",
                diagnostic_label: "measure.scxml",
            },
            Language::Cpp,
        ),
        "a float field has no C++ type",
    );
    assert!(
        refusal.contains("UnsupportedFeature")
            && refusal.contains(&format!("codec 'measure': {MEASURE_REFUSED_BY}")),
        "{refusal}"
    );
}

/// A codec that embeds a codec the C++ backend refuses is refused for it, by
/// the field that holds it and the reason the held codec is refused.
#[test]
fn a_codec_holding_a_codec_cpp_refuses_is_refused_by_the_field_that_holds_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("measure.scxml"), measure_document())
        .expect("write the measure codec");
    let holder = dir.path().join("holder.scxml");
    std::fs::write(
        &holder,
        codec_document(
            "holder",
            r#"<sce:import src="measure.scxml" kind="codec" as="measure"/>"#,
            r#"<sce:field id="kind" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:embed id="inner" type="measure" sce:byte="1"/>"#,
        ),
    )
    .expect("write the holding codec");
    assert_refused_by(
        &holder,
        &format!(
            "codec 'holder': field 'inner': a body of `measure`, a codec the C++ backend \
             refuses ({MEASURE_REFUSED_BY})"
        ),
    );
}

/// A document of another kind that imports a codec the C++ backend refuses is
/// refused for it, since the codec it names is not written.
#[test]
fn a_document_that_imports_a_codec_cpp_refuses_is_refused_for_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("measure.scxml"), measure_document())
        .expect("write the measure codec");
    let validator = dir.path().join("measure_validator.scxml");
    std::fs::write(
        &validator,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="validator" version="1.0">
  <sce:import src="measure.scxml" kind="codec" as="frame"/>
  <datamodel>
    <data id="tag" sce:type="uint8" sce:direction="in"
          sce:range-min="0" sce:range-max="255"/>
    <data id="valid" sce:type="bool" sce:direction="out"
          sce:plausibility="frame.tag === tag"/>
  </datamodel>
</scxml>"#,
    )
    .expect("write the validator");
    let refusal = refusal_of(
        compile_forge_file(
            &validator,
            Language::Cpp,
            &[],
            &ForgeCompileOptions::default(),
        ),
        "a validator of a codec C++ refuses is not written",
    );
    assert!(
        refusal.contains("UnsupportedFeature")
            && refusal.contains(&format!(
                "the codec `frame` it imports is one the C++ backend refuses \
                 ({MEASURE_REFUSED_BY})"
            )),
        "{refusal}"
    );
}

/// The documents of the resources named by `stems`, copied into `dir`, for a
/// codec written there that imports them.
fn copy_resources(dir: &Path, stems: &[&str]) {
    for stem in stems {
        let name = format!("{stem}.scxml");
        std::fs::copy(resources().join(&name), dir.join(&name))
            .unwrap_or_else(|e| panic!("copy {name}: {e}"));
    }
}

/// A list on one side of a gate whose entries say the codec's own input it
/// hands them is refused by the list's field: the value says the input by the
/// list's start, which the printer reads off a source through the list's own
/// accessor, while a group's member is read off the arm that holds it.
#[test]
fn a_list_a_group_holds_whose_law_is_over_the_codec_s_own_input_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_resources(dir.path(), &["codec_chain_flag_entry"]);
    let holder = dir.path().join("holder.scxml");
    std::fs::write(
        &holder,
        codec_document(
            "holder",
            r#"<sce:import src="codec_chain_flag_entry.scxml" kind="codec" as="codec_chain_flag_entry">
    <sce:flag-bind input="wide" source="handed_wide"/>
  </sce:import>
  <sce:flag-inputs>
    <sce:flag-input name="handed_wide" width="1"/>
  </sce:flag-inputs>"#,
            r#"<sce:flags id="head" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="L" bit="0"/>
    </sce:flags>
    <sce:field id="count" sce:type="uint8" sce:byte="1" sce:bit-size="8"
               sce:present-if="head.L"/>
    <sce:repeat id="entries" type="codec_chain_flag_entry" sce:byte="2"
                count="count" max-count="4" sce:present-if="head.L"/>
    <sce:field id="note" sce:type="uint8" sce:byte="3" sce:bit-size="8"
               sce:present-if="!head.L"/>"#,
        ),
    )
    .expect("write the holding codec");
    assert_refused_by(
        &holder,
        "codec 'holder': field 'entries': a list on one side of a gate, held to the law its \
         entries say the codec's own input `handed_wide` by, has no C++ lowering",
    );
}

/// A list held to a law that shares its count with another list, the two of
/// which an arm holds as one `Aligned` (docs/adr/0065 §6 R5), is refused by the
/// list's field.
#[test]
fn a_list_held_to_a_law_that_shares_its_count_in_a_group_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    copy_resources(dir.path(), &["codec_chain_flag_entry", "codec_repeat_elem"]);
    let holder = dir.path().join("holder.scxml");
    std::fs::write(
        &holder,
        codec_document(
            "holder",
            r#"<sce:import src="codec_repeat_elem.scxml" kind="codec" as="codec_repeat_elem"/>
  <sce:import src="codec_chain_flag_entry.scxml" kind="codec" as="codec_chain_flag_entry">
    <sce:flag-bind input="wide" source="head.W"/>
  </sce:import>"#,
            r#"<sce:flags id="head" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="W" bit="0"/>
      <sce:flag name="H" bit="1"/>
    </sce:flags>
    <sce:field id="links_len" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>
    <sce:repeat id="links" type="codec_repeat_elem" sce:byte="2"
                count="links_len" max-count="4"/>
    <sce:repeat id="weights" type="codec_chain_flag_entry" sce:byte="3"
                count="links_len" max-count="4" sce:present-if="head.H"/>"#,
        ),
    )
    .expect("write the holding codec");
    assert_refused_by(
        &holder,
        "codec 'holder': field 'weights': a list held to the law its entries say their \
         inputs by, sharing its count with another list a gate decides, has no C++ lowering",
    );
}
