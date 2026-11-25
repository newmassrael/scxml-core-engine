// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Rust codec reads a list a gate group holds off the arm the value holds
//! (docs/adr/0065 §6 R4b), as the source trait presents it: such a list has no
//! accessor of its own, so a value that says an input by the list's start
//! (R8) says it through the group's accessor, and a list held to a law that
//! shares its count with another (R5) is held as one `Aligned` of the two.
//!
//! What the codecs do, frame by frame, is held by the byte cases the
//! conformance crate runs over their sources (docs/adr/0081 §5); here the
//! generated text is read where the shape is decided.

use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// Every file the Rust backend generates for the resource `name`, in one text.
fn rust_of(name: &str) -> String {
    let path = resource_dir().join(format!("{name}.scxml"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    sce_build::compile_forge_with_imports(
        &source,
        DocumentLabel::symmetric(name),
        Language::Rust,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{name} generates for Rust: {e}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

fn assert_contains(name: &str, code: &str, needle: &str) {
    assert!(
        code.contains(needle),
        "{name}'s Rust has no `{needle}`:\n{code}"
    );
}

/// The codec's own input, handed to every entry of a list one side of a gate
/// holds, is said by the list's start where the value holds that side, read
/// off the group's accessor, and by nothing where it holds the other side.
#[test]
fn an_input_a_list_a_group_holds_says_is_read_off_the_arm() {
    let name = "codec_repeat_flag_forward_group";
    let code = rust_of(name);
    assert_contains(
        name,
        &code,
        "pub fn input_wide<S: CodecRepeatFlagForwardGroupSource + ?Sized>(src: &S) -> Option<bool>",
    );
    assert_contains(
        name,
        &code,
        "match CodecRepeatFlagForwardGroupSource::head_l(src) { \
         CodecRepeatFlagForwardGroupHeadL::L { entries: _l, .. } => Some(_l.start()), \
         _ => None, }",
    );
    assert!(
        !code.contains("CodecRepeatFlagForwardGroupSource::entries("),
        "{name} reads a list a group holds through an accessor of its own:\n{code}"
    );
}

/// A list always there held to its law, which shares its count with a list a
/// gate decides (docs/adr/0065 §6 R5, R8), is a member of the group in every
/// arm. The encode binds it off the arm the value holds as an `Option`, so the
/// flag its start derives is read through it; the decode fills it from its
/// start (`refill_from`) into a local of its own, from the list's default,
/// before the group is made of it.
#[test]
fn a_list_always_there_held_to_its_law_is_read_off_every_arm() {
    let name = "codec_repeat_flag_aligned_always";
    let code = rust_of(name);
    assert_contains(
        name,
        &code,
        "= _m_links.clone().map_or(0, |_l| usize::from(_l.start()));",
    );
    assert_contains(name, &code, "let mut links: ");
    assert_contains(name, &code, "cursor.hold_with(|cursor| links.refill_from(");
}
