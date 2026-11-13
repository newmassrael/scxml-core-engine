// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A Rust codec names a group whose elements vary once for each form that
//! holds it: `{Group}Owned<S>` for the owned value, `{Group}Parts<'a>` for the
//! parts a caller already has, `{Group}View<'a>` for the view of a frame
//! (docs/adr/0065 §6 R4; docs/adr/0077 §7).
//!
//! A form's field that spelt the whole instantiation grew with each element's
//! type: the two lists of one count whose second is held to its law spelt a
//! bounded list and a lawful bounded list inside the group's type, which
//! clippy refuses as a field's type (`type_complexity`). Here each form's field
//! is read for the name and each name for its definition.

use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// The Rust source of the codec `name`, with its view.
fn rust_source(name: &str) -> String {
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
    output
        .files
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_group_whose_elements_vary_is_named_once_in_each_form() {
    let name = "codec_repeat_flag_aligned_group";
    let code = rust_source(name);
    let group = "CodecRepeatFlagAlignedGroupHeadH";
    for (form, lt) in [("Owned", "<S>"), ("Parts", "<'a>"), ("View", "<'a>")] {
        let alias = format!("{group}{form}{lt}");
        let definition = format!("pub type {alias} = {group}<");
        assert!(
            code.contains(&definition),
            "{name}: the {form} form names the group once (`{definition}`)"
        );
        let field = format!("pub head_h: {alias},");
        assert!(
            code.contains(&field),
            "{name}: the {form} form's field reads as the name (`{field}`)"
        );
    }
    let spelt: Vec<&str> = code
        .lines()
        .filter(|line| {
            line.trim_start().starts_with("pub head_h: ") && line.contains(&format!("{group}<"))
        })
        .collect();
    assert!(
        spelt.is_empty(),
        "{name}: a form's field spells the whole instantiation of the group: {spelt:#?}"
    );
}
