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
    "codec_chain_depth_entry",
    "codec_chain_depth_gated",
    "codec_chain_depth_plain",
    "codec_chain_flag_carrier",
    "codec_chain_flag_entry",
    "codec_chain_flag_forward",
    "codec_chain_has_marker_slice",
    "codec_chain_has_slice",
    "codec_chain_has_tagged_entry",
    "codec_chain_prev_carrier",
    "codec_chain_prev_entry",
    "codec_chain_prev_gated",
    "codec_chain_prev_tail",
    "codec_chain_prev_tail_gated",
    "codec_declared_bounds",
    "codec_declared_bounds_gated",
    "codec_declared_bounds_to_end",
    "codec_default_marker_arm_a",
    "codec_default_marker_arm_b",
    "codec_dma_aligned_basic",
    "codec_embed_basic",
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
    "codec_little_endian",
    "codec_nested_body",
    "codec_nested_parent",
    "codec_origin_choice",
    "codec_origin_envelope",
    "codec_origin_leaf",
    "codec_origin_scalar",
    "codec_peek_arm_a",
    "codec_peek_arm_b",
    "codec_present_if_basic",
    "codec_present_if_disjunction",
    "codec_present_if_length_ref",
    "codec_present_if_negation",
    "codec_present_if_string",
    "codec_present_if_tail",
    "codec_present_if_vle",
    "codec_qos_byte",
    "codec_repeat_basic",
    "codec_repeat_elem",
    "codec_repeat_flag_carrier",
    "codec_repeat_flag_gated",
    "codec_repeat_present_if_basic",
    "codec_scout_zid_body",
    "codec_simple_frame",
    "codec_subbyte",
    "codec_tail",
    "codec_tlv_chain_basic",
    "codec_tlv_chain_present_if_basic",
    "codec_tlv_entry",
    "codec_transport_envelope",
    "codec_until_eof_basic",
    "codec_variant_dispatch",
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
    "codec_zenoh_oam",
    "codec_zenoh_open_body",
    "codec_zenoh_push",
    "codec_zenoh_push_body",
    "codec_zenoh_put",
    "codec_zenoh_query",
    "codec_zenoh_query_value",
    "codec_zenoh_reply",
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
