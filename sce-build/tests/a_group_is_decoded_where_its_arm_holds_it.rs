// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Rust codec's eager decode builds each list, embedded body and chain of a
//! group where the arm that holds it lives (docs/adr/0065 §6 R4b, R5, R6;
//! docs/adr/0077 §7).
//!
//! A list held inline moves by copying its whole capacity, so a decode that
//! read a group's elements into locals and assembled the arm by value at the
//! end carried a copy of each list in its frame, and more for each move the
//! assembly made. The decode now installs the arm the gates name before the
//! first element it builds and reads each element into the arm's field; the
//! chain whose memberships are the gates is read through `decode_chain_in` or
//! `decode_placed_in`, which install the arm and check the chain where it
//! lives. Here the eager `store_into` of each such codec is read for a local of
//! an element's type and for the writes that build the elements in place.
//!
//! What the decode does, frame by frame, is held by the round trips the
//! conformance crate runs over the committed goldens of these codecs.

use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// The eager decode of the codec `name`, generated for Rust from its resource,
/// with its view: its `store_into`, or its `decode_into` where the codec holds
/// nothing a storage profile answers for and has no `store_into`.
fn store_into(name: &str) -> String {
    let path = resource_dir().join(format!("{name}.scxml"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    let output = sce_build::compile_forge_with_imports(
        &source,
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        &ForgeCompileOptions {
            views: true,
            ..ForgeCompileOptions::default()
        },
    )
    .unwrap_or_else(|e| panic!("{name} generates for Rust: {e}"));
    let code = output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n");
    let start = code
        .find("fn store_into")
        .or_else(|| code.find("pub fn decode_into"))
        .unwrap_or_else(|| panic!("{name} has an eager decode"));
    let body = &code[start..];
    let end = body
        .find("\n    }\n")
        .unwrap_or_else(|| panic!("{name}'s eager decode closes"));
    body[..end].to_string()
}

/// No local of the element `id` is declared in `body`.
fn assert_no_local(name: &str, body: &str, id: &str) {
    for declared in [format!("let mut {id}:"), format!("let {id}:")] {
        assert!(
            !body.contains(&declared),
            "{name} declares a local `{id}` its group should hold in place:\n{body}"
        );
    }
}

fn assert_contains(name: &str, body: &str, needle: &str) {
    assert!(
        body.contains(needle),
        "{name}'s store_into has no `{needle}`:\n{body}"
    );
}

/// The chain decides the arm by whether it holds an entry of kind 0x2; the
/// slices are in the arm where it does, the payload where it does not. The
/// chain is read through the door that installs the arm and checks it, the
/// slices where that arm holds them, and nothing is assembled by value.
#[test]
fn a_chain_whose_kinds_decide_the_arm_is_read_where_the_arm_holds_it() {
    let name = "codec_chain_has_envelope";
    let body = store_into(name);
    assert_no_local(name, &body, "extensions");
    assert_no_local(name, &body, "slices");
    assert_contains(
        name,
        &body,
        "::sce_forge_runtime::codec::decode_chain_in(&mut _out.extensions_has_0x2,",
    );
    assert_contains(name, &body, "_p.refill(|_vec|");
    // The old assembly moved the chain out of its local into the arm.
    assert!(
        !body.contains(".into_inner()"),
        "{name} still moves a list into its arm:\n{body}"
    );
    assert!(
        !body.contains("Chain::new("),
        "{name} still refines a chain it built elsewhere:\n{body}"
    );
    // A chain of the other kind is read again into the arm it names.
    assert_contains(name, &body, "*cursor = _extensions_at;");
    // The payload, which the decode does not build, is stored in the arm.
    assert_contains(name, &body, "*_p_payload = payload.unwrap_or_default();");
}

/// A bare entry's one membership: the arm where it holds is the chain placed at
/// its first entry of the kind, read through `decode_placed_in`.
#[test]
fn a_bare_entrys_chain_is_placed_where_its_arm_holds_it() {
    let name = "codec_chain_has_marker";
    let body = store_into(name);
    assert_no_local(name, &body, "extensions");
    assert_no_local(name, &body, "slices");
    assert_contains(
        name,
        &body,
        "::sce_forge_runtime::codec::decode_placed_in(&mut _out.extensions_has_0x12,",
    );
    assert_contains(
        name,
        &body,
        "::sce_forge_runtime::codec::decode_chain_in(&mut _out.extensions_has_0x12,",
    );
    assert!(
        !body.contains("place_first"),
        "{name} still places a chain moved out of a local:\n{body}"
    );
}

/// Two memberships and a flag decide eight arms; the chain is read into the one
/// the flag and the expected kinds name, and the scalars are stored in it.
#[test]
fn a_chain_of_several_memberships_is_read_into_the_arm_they_name() {
    let name = "codec_chain_has_tagged";
    let body = store_into(name);
    assert_no_local(name, &body, "entries");
    assert_contains(
        name,
        &body,
        "decode_chain_in(&mut _out.entries_has_0x7_has_0x9_header_wide,",
    );
    assert_contains(name, &body, "let mut _has_entries_7 = false;");
    assert_contains(name, &body, "let mut _has_entries_9 = false;");
    assert!(
        !body.contains(".into_inner()"),
        "{name} still moves a list into its arm:\n{body}"
    );
}

/// A flag-input decides the arm before any element is read, so the arm is
/// installed first and the slices are read into it.
#[test]
fn a_list_of_an_arm_a_flag_input_names_is_read_into_that_arm() {
    let name = "codec_zenoh_value_zbuf";
    let body = store_into(name);
    assert_no_local(name, &body, "slices");
    assert_contains(
        name,
        &body,
        "_out.after_shm = if (after_shm & 0x01u8) != 0 {",
    );
    assert_contains(name, &body, "_p.refill(|_vec|");
    assert_contains(name, &body, "*_p_value = value.unwrap_or_default();");
}

/// An embedded body the arms of a disjunction hold is decoded into the arm the
/// flags name, the value holding no arm where neither is set.
#[test]
fn an_embedded_body_of_an_arm_is_decoded_into_that_arm() {
    let name = "codec_zenoh_interest";
    let body = store_into(name);
    assert_no_local(name, &body, "body");
    assert_contains(name, &body, "_out.header_current_or_future = match (");
    assert_contains(name, &body, "::decode_into(cursor, _p)?;");
}

/// Two lists of one count are filled in their pair where the arm holds both, and
/// the first alone where the arm holds it alone.
#[test]
fn two_lists_of_one_count_are_filled_where_their_arm_holds_them() {
    let name = "codec_repeat_unconditional_count";
    let body = store_into(name);
    assert_no_local(name, &body, "links");
    assert_no_local(name, &body, "weights");
    assert_contains(name, &body, "_p.refill_first(|_vec|");
    assert_contains(name, &body, "_p.refill_second(|_vec|");
    assert!(
        !body.contains("Aligned::new("),
        "{name} still pairs two lists moved out of locals:\n{body}"
    );
}

/// A group of scalars builds nothing in place, so its arm is assembled from its
/// elements' locals as before.
#[test]
fn a_group_of_scalars_is_still_assembled_from_its_locals() {
    let name = "codec_present_if_disjunction";
    let body = store_into(name);
    assert_contains(name, &body, "_out.flags_wants_a_or_wants_b = match (");
    assert_contains(name, &body, "seq.unwrap_or_default()");
}
