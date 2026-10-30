// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A parent whose carrier holds, in a flag, the tag of a variant it embeds
//! writes for a catch-all body the tag that body holds (ADR 0058: a catch-all
//! presents the tag it could not name), masked to the flag's width.
//!
//! The decode of the parent hands the embedded variant the flag's bits as its
//! tag, and a tag no arm names reaches the catch-all, which keeps it. Bits the
//! encode derives from anything else make a frame that decodes into another
//! arm, or into the catch-all with another tag. The Rust source interface
//! derives them from the tag (`derive_parent_tag`); the C parent once added
//! nothing for a catch-all body, and the C++ parent the bits of its last arm,
//! which the catch-all's wrapper fell through to.
//!
//! The tests hold the term each backend writes.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// The catch-all's wrapper the C++ dispatcher declares next to its struct.
const CPP_WRAPPER: &str = "::SCE::Generated::CaChild::CaChildDefault";

/// A parent that dispatches an embedded tag-less variant on two bits of its
/// header, written with its documents to a fresh directory the caller
/// removes. The variant's arms are 0x00 (one byte; under the feature
/// `local-arm` when `gated`) and 0x01 (two bytes, the default arm), and its
/// catch-all reads one byte.
fn catch_all_fixture(gated: bool) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "sce_catch_all_parent_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("make the fixture directory");
    let local_feature = if gated { r#" feature="local-arm""# } else { "" };
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
            "ca_parent",
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="ca_parent">
  <sce:import src="ca_child.scxml" kind="codec" as="ca_child">
    <sce:variant-dispatch flag="header.K"/>
  </sce:import>
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="mid" bit="0" width="5" value="0x1d"/>
      <sce:flag name="K" bit="5" width="2"/>
    </sce:flags>
    <sce:embed id="key" type="ca_child" sce:byte="1"/>
  </datamodel>
</scxml>"#
                .to_string(),
        ),
        (
            "ca_child",
            format!(
                r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="ca_child">
  <sce:import src="ca_local.scxml" kind="codec" as="ca_local"/>
  <sce:import src="ca_wide.scxml" kind="codec" as="ca_wide"/>
  <datamodel>
    <sce:variant>
      <sce:arm value="0x00" type="ca_local"{local_feature}/>
      <sce:arm value="0x01" type="ca_wide" default="true"/>
      <sce:default type="ca_local"/>
    </sce:variant>
  </datamodel>
</scxml>"#
            ),
        ),
        ("ca_local", body("ca_local", "uint8", 8)),
        ("ca_wide", body("ca_wide", "uint16", 16)),
    ];
    for (name, text) in documents {
        std::fs::write(dir.join(format!("{name}.scxml")), text).expect("write a fixture");
    }
    dir
}

/// Every file `language` writes for the fixture's documents, each once, by
/// its file name.
fn generate_all(dir: &Path, language: Language) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();
    for stem in ["ca_local", "ca_wide", "ca_child", "ca_parent"] {
        let text =
            std::fs::read_to_string(dir.join(format!("{stem}.scxml"))).expect("read a fixture");
        let output = sce_build::compile_forge_with_imports(
            &text,
            DocumentLabel::symmetric(stem),
            language,
            dir,
            &ForgeCompileOptions::default(),
        )
        .unwrap_or_else(|e| panic!("{stem} generates for {language:?}: {e}"));
        for (name, code) in output.files {
            let base = Path::new(&name)
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a generated file has a name")
                .to_string();
            if !files.iter().any(|(seen, _)| *seen == base) {
                files.push((base, code));
            }
        }
    }
    files
}

/// The parent's generated file for `language`.
fn parent_code(gated: bool, language: Language) -> String {
    let dir = catch_all_fixture(gated);
    let files = generate_all(&dir, language);
    let _ = std::fs::remove_dir_all(&dir);
    files
        .into_iter()
        .find(|(name, _)| name.contains("ca_parent"))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("the parent's file for {language:?}"))
}

/// The C++ term of the catch-all: its wrapper, found by type, fills `K` with
/// the tag it holds.
fn cpp_catch_all_term() -> String {
    format!(
        "std::get_if<{CPP_WRAPPER}>(&key.body) != nullptr ? \
         static_cast<std::uint8_t>((std::get_if<{CPP_WRAPPER}>(&key.body)->tag & 0x3u) << 5)"
    )
}

/// The Rust source interface reads the tag a catch-all body holds; the C and
/// C++ terms below are held to the same rule.
#[test]
fn the_rust_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::Rust);
    assert!(
        code.contains("CaChildVariantSource::Default { tag, .. } => u8::from(tag) & 0x3"),
        "the Rust parent does not derive K from the catch-all's tag:\n{code}"
    );
}

/// The C parent's switch on the embedded body's kind has a case for the
/// catch-all, which adds the tag the body keeps in `default_tag`, before the
/// `default` that adds nothing.
#[test]
fn the_c_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::C11);
    let switch = [
        "    uint8_t _derived_header = 0u;",
        "    switch (self->key.body.kind) {",
        "    case CA_CHILD_BODY_KIND_CA_LOCAL: _derived_header |= 0x00u; break;",
        "    case CA_CHILD_BODY_KIND_CA_WIDE: _derived_header |= 0x20u; break;",
        "    case CA_CHILD_BODY_KIND_DEFAULT: _derived_header |= (uint8_t)((self->key.body.default_tag & 0x3u) << 5); break;",
        "    default: break;",
        "    }",
    ]
    .join("\n");
    assert!(
        code.contains(&switch),
        "the C parent does not derive K from the catch-all's tag:\n{code}"
    );
}

/// The C++ parent tests for the catch-all's wrapper ahead of the arms, whose
/// last one is the fallthrough the wrapper reached before.
#[test]
fn the_cpp_parent_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(false, Language::Cpp);
    let derived = format!(
        "        const std::uint8_t _derived_header = static_cast<std::uint8_t>(({} : \
         key.body.index() == 0 ? static_cast<std::uint8_t>(0x00) : \
         static_cast<std::uint8_t>(0x20)));\n",
        cpp_catch_all_term()
    );
    assert!(
        code.contains(&derived),
        "the C++ parent does not derive K from the catch-all's tag:\n{code}"
    );
}

/// Over a variant with an arm under a build feature (docs/adr/0072) the
/// catch-all's term is first, outside every guard, and the arms after it count
/// their index per build as before.
#[test]
fn the_cpp_parent_of_a_gated_variant_writes_the_tag_its_catch_all_holds() {
    let code = parent_code(true, Language::Cpp);
    let derived = [
        "        const std::uint8_t _derived_header = static_cast<std::uint8_t>((".to_string(),
        format!("            {} :", cpp_catch_all_term()),
        "#if SCE_FEATURE_LOCAL_ARM".to_string(),
        "            key.body.index() == static_cast<std::size_t>(0) ? static_cast<std::uint8_t>(0x00) :"
            .to_string(),
        "#endif".to_string(),
        "            static_cast<std::uint8_t>(0x20)));".to_string(),
    ]
    .join("\n");
    assert!(
        code.contains(&derived),
        "the C++ parent of a gated variant does not derive K from the catch-all's tag:\n{code}"
    );
}
