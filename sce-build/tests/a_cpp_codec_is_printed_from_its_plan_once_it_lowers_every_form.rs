// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which codecs of the resources the C++ backend prints from their encode plan
//! (docs/adr/0081 step 4, §6).
//!
//! The C++ printer lowers the plan's forms one landing at a time, and a codec
//! moves whole, only once its plan uses no form the printer lacks; every other
//! codec keeps the template's encode. This pins the codecs that have moved, so
//! that a landing states which it moves, and a change to the plan or to
//! `cpp_source::unlowered` that moves one by accident, or moves one back, is
//! seen here. When the last codec moves, the old encode is removed and this
//! list is every codec of the resources.

use std::path::{Path, PathBuf};

use sce_build::{cpp_codec_printed_file, cpp_codec_unlowered_file, ForgeCompileOptions};

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
    "codec_chain_has_envelope",
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
    "codec_ext_attachment",
    "codec_ext_encoding_info",
    "codec_ext_timestamp",
    "codec_fixed_after_lengthref",
    "codec_flags_basic",
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
    "codec_repeat_present_if_basic",
    "codec_repeat_unconditional_count",
    "codec_scout_zid_body",
    "codec_simple_frame",
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

/// Every document of the resources, by path, in order.
fn resource_documents() -> Vec<PathBuf> {
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&resources)
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

/// The codecs of the repository outside the resources that the C++ backend
/// keeps on the template's encode, by path from `sce-build`, in order. Until it
/// is empty the template's encode prints a codec somewhere, and so cannot be
/// removed (docs/adr/0081 §6); a landing that moves one removes it here.
const KEPT_ELSEWHERE: &[&str] = &[];

/// The C++ backend prints every codec of the repository outside the resources
/// but [`KEPT_ELSEWHERE`], which it keeps on the template's encode for the
/// forms it does not lower.
#[test]
fn the_cpp_backend_prints_every_codec_of_the_repository_but_the_ones_it_keeps() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    for root in OTHER_CODEC_ROOTS {
        codec_documents_under(&manifest.join(root), &mut paths);
    }
    paths.sort();
    let mut kept: Vec<String> = Vec::new();
    let mut why: Vec<String> = Vec::new();
    for path in &paths {
        let shown = path
            .strip_prefix(manifest)
            .unwrap_or(path)
            .display()
            .to_string();
        // Every document here is one the repository generates, so a refusal
        // is a defect of its own and not a codec kept.
        let unlowered = cpp_codec_unlowered_file(path, &[], &ForgeCompileOptions::default())
            .unwrap_or_else(|e| panic!("{shown}: {e:?}"))
            .expect("a document that declares itself a codec is one");
        if !unlowered.is_empty() {
            why.push(format!("{shown}: {}", unlowered.join("; ")));
            kept.push(shown);
        }
    }
    eprintln!(
        "{} codec document(s) read; {} kept:",
        paths.len(),
        why.len()
    );
    for line in &why {
        eprintln!("  {line}");
    }
    assert_eq!(
        kept,
        KEPT_ELSEWHERE,
        "the C++ backend keeps these codecs of the repository on the template's encode; a \
         landing that moves one removes it from KEPT_ELSEWHERE (docs/adr/0081 §6):\n  {}",
        why.join("\n  ")
    );
}

#[test]
fn the_cpp_backend_prints_exactly_the_codecs_whose_every_form_it_lowers() {
    let mut printed: Vec<String> = Vec::new();
    // Each codec the printer keeps on the template's encode, with every form
    // it does not lower: what a later landing lowers to move it.
    let mut kept: Vec<String> = Vec::new();
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
        if unlowered.is_empty() {
            printed.push(stem);
        } else {
            kept.push(format!("{stem}: {}", unlowered.join("; ")));
        }
    }
    // The census of what keeps each codec, shown with `--nocapture`.
    eprintln!("{} codec(s) kept on the template's encode:", kept.len());
    for line in &kept {
        eprintln!("  {line}");
    }
    assert_eq!(
        printed,
        PRINTED,
        "the C++ backend prints these codecs of the resources from their plan; a landing \
         that moves a codec names it here (docs/adr/0081 §6). Each codec it keeps, with \
         the forms it does not lower:\n  {}",
        kept.join("\n  ")
    );
}
