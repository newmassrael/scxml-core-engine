// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust encoder writes each header, flag and one-byte field with the
//! sink's own `write_u8` and each VLE with `write_vle_u*`, never a slice of
//! bytes it built for the write (docs/adr/0130 §3): a sink stores a byte
//! itself, and a VLE is assembled once and handed to the sink in one write.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn resource_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources")
}

/// The Rust the codec document at `path` generates, every file joined, or
/// `None` where the document is one the generator refuses.
fn rust_of(path: &Path) -> Option<String> {
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    let name = path.file_stem()?.to_str()?.to_string();
    let output = sce_build::compile_forge_with_imports(
        &source,
        DocumentLabel::symmetric(&name),
        Language::Rust,
        &resource_dir(),
        &ForgeCompileOptions::default(),
    )
    .ok()?;
    Some(
        output
            .files
            .into_iter()
            .map(|(_, code)| code)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// No codec of the resources writes a slice it built for the write: a byte is
/// `write_u8`, a VLE `write_vle_u*`, and `write_bytes` is left the bytes a
/// field holds.
#[test]
fn no_rust_encoder_writes_a_slice_it_built() {
    let mut generated = 0usize;
    let mut paths: Vec<PathBuf> = std::fs::read_dir(resource_dir())
        .expect("the resources are readable")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "scxml"))
        .collect();
    paths.sort();
    for path in paths {
        let source = std::fs::read_to_string(&path).expect("readable");
        if !source.contains("sce:kind=\"codec\"") {
            continue;
        }
        let Some(code) = rust_of(&path) else {
            continue;
        };
        generated += 1;
        for line in code.lines() {
            assert!(
                !line.contains("write_bytes(&["),
                "{} writes a slice it built:\n{line}",
                path.display()
            );
        }
    }
    assert!(
        generated >= 100,
        "only {generated} codec(s) generated, so the sweep read too little"
    );
}

/// A VLE field is written by the sink's VLE write of its width, and the header
/// before it by `write_u8`.
#[test]
fn a_vle_is_one_write_of_its_width_and_a_header_one_byte() {
    for (name, vle) in [
        ("codec_zenoh_oam", "w.write_vle_u16("),
        ("codec_zenoh_ext_zint", "w.write_vle_u64("),
    ] {
        let code = rust_of(&resource_dir().join(format!("{name}.scxml")))
            .unwrap_or_else(|| panic!("{name} generates for Rust"));
        assert!(
            code.contains(vle),
            "{name} does not write its VLE by `{vle}`:\n{code}"
        );
    }
    let oam = rust_of(&resource_dir().join("codec_zenoh_oam.scxml")).expect("generates");
    assert!(
        oam.contains("w.write_u8("),
        "the OAM header is not written by `write_u8`:\n{oam}"
    );
}
