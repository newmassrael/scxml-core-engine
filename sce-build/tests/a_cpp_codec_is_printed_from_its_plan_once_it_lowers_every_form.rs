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

use sce_build::{cpp_codec_printed_file, ForgeCompileOptions};

/// The codecs of `tests/forge/resources` the C++ backend prints, by document
/// stem, in order.
const PRINTED: &[&str] = &[
    "broadcast_log_conformance_event",
    "broadcast_log_conformance_record",
    "codec_chain_depth_entry",
    "codec_chain_has_marker_slice",
    "codec_chain_has_slice",
    "codec_chain_has_tagged_entry",
    "codec_default_marker_arm_a",
    "codec_default_marker_arm_b",
    "codec_dma_aligned_basic",
    "codec_ext_attachment",
    "codec_ext_timestamp",
    "codec_fixed_after_lengthref",
    "codec_flags_basic",
    "codec_length_ref",
    "codec_length_ref_uint16_be",
    "codec_length_ref_uint16_le",
    "codec_length_ref_uint32_le",
    "codec_little_endian",
    "codec_origin_leaf",
    "codec_origin_scalar",
    "codec_peek_arm_a",
    "codec_peek_arm_b",
    "codec_qos_byte",
    "codec_repeat_elem",
    "codec_scout_zid_body",
    "codec_simple_frame",
    "codec_subbyte",
    "codec_tail",
    "codec_tlv_entry",
    "codec_variant_session_close",
    "codec_variant_session_open",
    "codec_vle_zint_u64",
    "codec_zenoh_close",
    "codec_zenoh_decl_final",
    "codec_zenoh_del",
    "codec_zenoh_ext_unit",
    "codec_zenoh_ext_zbuf",
    "codec_zenoh_ext_zint",
    "codec_zenoh_fragment",
    "codec_zenoh_frame",
    "codec_zenoh_keep_alive",
    "codec_zenoh_put",
    "codec_zenoh_timestamp",
    "codec_zenoh_undecl_kexpr",
    "codec_zenoh_value_slice",
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
    let printed: Vec<String> = resource_documents()
        .into_iter()
        .filter(|path| {
            // A document the generator refuses is printed by no backend.
            matches!(
                cpp_codec_printed_file(path, &[], &ForgeCompileOptions::default()),
                Ok(Some(true))
            )
        })
        .map(|path| {
            path.file_stem()
                .expect("a document has a stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        printed, PRINTED,
        "the C++ backend prints these codecs of the resources from their plan; a landing \
         that moves a codec names it here (docs/adr/0081 §6)"
    );
}
