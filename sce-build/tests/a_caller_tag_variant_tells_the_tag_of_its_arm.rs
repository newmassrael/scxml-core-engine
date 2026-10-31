// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A codec whose `<sce:variant>` has no `tag=` takes the tag of the arm as a
//! parameter of `decode`, and nothing it holds said which arm that was: a
//! caller that kept a value but not the tag it was decoded under had to keep
//! the tag too. Each backend's codec now answers it, from the value:
//!
//! - Kotlin `val tag: UByte`, Python `def tag(self) -> int`, Go
//!   `func (s *Codec) Tag() byte`, C11 `<codec>_tag(const <codec>_t *self)`
//!   and C++ `std::uint8_t tag() const`;
//! - the tag the arm declares, from the literal the decode matches on, or,
//!   for the catch-all arm, the unknown tag it holds. Every backend keeps that
//!   tag with the catch-all body, so none has a tag it cannot return;
//! - a variant with a tag field of its own has no accessor, as its tag is a
//!   field of the value;
//! - an arm under a build feature (docs/adr/0072) is a case of the C and C++
//!   accessors inside the guard of its own macro, as it is in the encode.
//!
//! The tests hold the text each backend generates for a variant with a
//! catch-all, one without, and one with a gated arm, for a variant with a tag
//! field, and for the caller-tag variant the broadcast-log fixtures use.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// The module the Go codecs are generated into.
const GO_MODULE: &str = "example.com/sce-caller-tag";

/// A directory with a one-byte body, a two-byte body and three variants over
/// them: `tg_closed` (arms 0x01 and 0x02, no catch-all), `tg_open` (the same
/// two and a catch-all of the one-byte body) and `tg_gated` (the same two,
/// the second under the feature `tg-gate`). The caller removes it.
fn fixture() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "sce_caller_tag_accessor_{}_{}",
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
    let variant = |name: &str, large_feature: &str, catch_all: &str| {
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}">
  <sce:import src="tg_small.scxml" kind="codec" as="tg_small"/>
  <sce:import src="tg_large.scxml" kind="codec" as="tg_large"/>
  <datamodel>
    <sce:variant>
      <sce:arm value="0x01" type="tg_small"/>
      <sce:arm value="0x02" type="tg_large"{large_feature}/>{catch_all}
    </sce:variant>
  </datamodel>
</scxml>"#
        )
    };
    let documents = [
        ("tg_small", body("tg_small", "uint8", 8)),
        ("tg_large", body("tg_large", "uint16", 16)),
        ("tg_closed", variant("tg_closed", "", "")),
        (
            "tg_open",
            variant("tg_open", "", "\n      <sce:default type=\"tg_small\"/>"),
        ),
        ("tg_gated", variant("tg_gated", " feature=\"tg-gate\"", "")),
    ];
    for (name, text) in documents {
        std::fs::write(dir.join(format!("{name}.scxml")), text).expect("write a fixture");
    }
    dir
}

/// Every file `language` writes for the document `stem` of `dir`, joined.
fn generate_in(dir: &Path, stem: &str, language: Language) -> String {
    let text = std::fs::read_to_string(dir.join(format!("{stem}.scxml"))).expect("read a fixture");
    let mut options = ForgeCompileOptions::default();
    if matches!(language, Language::Go) {
        options.go_module_prefix = Some(GO_MODULE.to_string());
    }
    sce_build::compile_forge_with_imports(
        &text,
        DocumentLabel::symmetric(stem),
        language,
        dir,
        &options,
    )
    .unwrap_or_else(|e| panic!("{stem} generates for {language:?}: {e}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// The code of the fixture document `stem` for `language`.
fn code(stem: &str, language: Language) -> String {
    let dir = fixture();
    let code = generate_in(&dir, stem, language);
    let _ = std::fs::remove_dir_all(&dir);
    code
}

/// Assert that `code` holds `wanted`, naming `what` when it does not.
fn assert_holds(code: &str, wanted: &str, what: &str) {
    assert!(
        code.contains(wanted),
        "{what}; wanted\n{wanted}\nin:\n{code}"
    );
}

// ── Kotlin ──────────────────────────────────────────────────────────────

/// The Kotlin getter is a `when` over the sealed variant, which names the
/// catch-all and reads the tag it holds through the `when`'s own binding.
#[test]
fn the_kotlin_codec_returns_the_tag_of_its_arm_and_of_its_catch_all() {
    let text = code("tg_open", Language::Kotlin);
    let getter = [
        "    val tag: UByte",
        "        get() = when (val _b = this.body) {",
        "            is TgOpenVariant.TgSmall -> 1.toUByte()",
        "            is TgOpenVariant.TgLarge -> 2.toUByte()",
        "            is TgOpenVariant.Default -> _b.tag",
        "        }",
    ]
    .join("\n");
    assert_holds(&text, &getter, "the Kotlin codec does not return its tag");
}

/// Without a catch-all the `when` binds nothing, which an unused binding
/// would warn about.
#[test]
fn the_kotlin_codec_without_a_catch_all_binds_nothing() {
    let text = code("tg_closed", Language::Kotlin);
    let getter = [
        "    val tag: UByte",
        "        get() = when (this.body) {",
        "            is TgClosedVariant.TgSmall -> 1.toUByte()",
        "            is TgClosedVariant.TgLarge -> 2.toUByte()",
        "        }",
    ]
    .join("\n");
    assert_holds(&text, &getter, "the Kotlin codec does not return its tag");
}

// ── Python ──────────────────────────────────────────────────────────────

/// The Python method compares the `kind` the decode sets, by the same names,
/// and returns the tag the catch-all keeps in `default_tag`.
#[test]
fn the_python_codec_returns_the_tag_of_its_arm_and_of_its_catch_all() {
    let text = code("tg_open", Language::Python);
    assert_holds(
        &text,
        "    def tag(self) -> int:\n",
        "the Python codec has no `tag` method",
    );
    let chain = [
        "        if self.body.kind == \"TgSmall\":",
        "            return 1",
        "        if self.body.kind == \"TgLarge\":",
        "            return 2",
        "        if self.body.kind == \"Default\":",
        "            return self.body.default_tag",
        "        raise ValueError(f\"no variant arm is named {self.body.kind!r}\")",
    ]
    .join("\n");
    assert_holds(&text, &chain, "the Python codec does not return its tag");
}

// ── Go ──────────────────────────────────────────────────────────────────

/// The Go method switches on which arm's pointer is set, as the encode does,
/// and returns the tag the catch-all's wrapper holds.
#[test]
fn the_go_codec_returns_the_tag_of_its_arm_and_of_its_catch_all() {
    let text = code("tg_open", Language::Go);
    let method = [
        "func (s *TgOpen) Tag() byte {",
        "\tswitch {",
        "\tcase s.Body.TgSmall != nil:",
        "\t\treturn 1",
        "\tcase s.Body.TgLarge != nil:",
        "\t\treturn 2",
        "\tcase s.Body.Default != nil:",
        "\t\treturn s.Body.Default.Tag",
        "\t}",
        "\tpanic(\"TgOpen: the value holds no variant arm\")",
        "}",
    ]
    .join("\n");
    assert_holds(&text, &method, "the Go codec does not return its tag");
}

// ── C11 ─────────────────────────────────────────────────────────────────

/// The C function switches on the kind the decode sets, and returns the tag
/// the catch-all keeps in `default_tag`.
#[test]
fn the_c_codec_returns_the_tag_of_its_arm_and_of_its_catch_all() {
    let text = code("tg_open", Language::C11);
    let function = [
        "static inline uint8_t tg_open_tag(const tg_open_t *self) {",
        "    switch (self->body.kind) {",
        "        case TG_OPEN_BODY_KIND_TG_SMALL:",
        "            return 1;",
        "        case TG_OPEN_BODY_KIND_TG_LARGE:",
        "            return 2;",
        "        case TG_OPEN_BODY_KIND_DEFAULT:",
        "            return self->body.default_tag;",
        "    }",
        "    return 0;",
        "}",
    ]
    .join("\n");
    assert_holds(&text, &function, "the C codec does not return its tag");
}

/// Without a catch-all the switch has no case for one.
#[test]
fn the_c_codec_without_a_catch_all_has_no_case_for_one() {
    let text = code("tg_closed", Language::C11);
    let function = [
        "static inline uint8_t tg_closed_tag(const tg_closed_t *self) {",
        "    switch (self->body.kind) {",
        "        case TG_CLOSED_BODY_KIND_TG_SMALL:",
        "            return 1;",
        "        case TG_CLOSED_BODY_KIND_TG_LARGE:",
        "            return 2;",
        "    }",
        "    return 0;",
        "}",
    ]
    .join("\n");
    assert_holds(&text, &function, "the C codec does not return its tag");
    assert!(
        !text.contains("BODY_KIND_DEFAULT"),
        "a variant with no catch-all names one:\n{text}"
    );
}

/// An arm under a build feature is a case inside the guard of its macro, as
/// in the encode, so a build without it has no case for a kind it lacks.
#[test]
fn the_c_codec_guards_the_case_of_a_gated_arm() {
    let text = code("tg_gated", Language::C11);
    let function = [
        "static inline uint8_t tg_gated_tag(const tg_gated_t *self) {",
        "    switch (self->body.kind) {",
        "        case TG_GATED_BODY_KIND_TG_SMALL:",
        "            return 1;",
        "#if SCE_FEATURE_TG_GATE",
        "        case TG_GATED_BODY_KIND_TG_LARGE:",
        "            return 2;",
        "#endif",
        "    }",
        "    return 0;",
        "}",
    ]
    .join("\n");
    assert_holds(&text, &function, "the C codec does not guard the gated arm");
}

// ── C++ ─────────────────────────────────────────────────────────────────

/// The C++ method finds the alternative by its type, as the encode does, and
/// reads the tag from the catch-all's wrapper.
#[test]
fn the_cpp_codec_returns_the_tag_of_its_arm_and_of_its_catch_all() {
    let text = code("tg_open", Language::Cpp);
    let method = [
        "    [[nodiscard]] std::uint8_t tag() const noexcept {",
        "        if (std::get_if<::SCE::Generated::TgSmall::TgSmall>(&body) != nullptr) return 1;",
        "        if (std::get_if<::SCE::Generated::TgLarge::TgLarge>(&body) != nullptr) return 2;",
        "        if (const auto* _p = std::get_if<TgOpenDefault>(&body)) return _p->tag;",
        "        return 0;",
        "    }",
    ]
    .join("\n");
    assert_holds(&text, &method, "the C++ codec does not return its tag");
}

/// An arm under a build feature is a test inside the guard of its macro.
#[test]
fn the_cpp_codec_guards_the_test_of_a_gated_arm() {
    let text = code("tg_gated", Language::Cpp);
    let method = [
        "    [[nodiscard]] std::uint8_t tag() const noexcept {",
        "        if (std::get_if<::SCE::Generated::TgSmall::TgSmall>(&body) != nullptr) return 1;",
        "#if SCE_FEATURE_TG_GATE",
        "        if (std::get_if<::SCE::Generated::TgLarge::TgLarge>(&body) != nullptr) return 2;",
        "#endif",
        "        return 0;",
        "    }",
    ]
    .join("\n");
    assert_holds(&text, &method, "the C++ codec does not guard the gated arm");
}

// ── the variant the broadcast-log fixtures use ──────────────────────────

/// `broadcast_log_conformance_choice` is a caller-tag variant, and a log that
/// publishes its elements reads the accessor of each backend.
#[test]
fn the_broadcast_log_choice_codec_has_the_accessor_in_every_backend() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/conformance/broadcast_log_context");
    let wanted = [
        (Language::Kotlin, "    val tag: UByte\n"),
        (Language::Python, "    def tag(self) -> int:\n"),
        (
            Language::Go,
            "func (s *BroadcastLogConformanceChoice) Tag() byte {\n",
        ),
        (
            Language::C11,
            "static inline uint8_t broadcast_log_conformance_choice_tag(\
             const broadcast_log_conformance_choice_t *self) {\n",
        ),
        (
            Language::Cpp,
            "    [[nodiscard]] std::uint8_t tag() const noexcept {\n",
        ),
    ];
    for (language, accessor) in wanted {
        let text = generate_in(&dir, "broadcast_log_conformance_choice", language);
        assert_holds(
            &text,
            accessor,
            &format!("the choice codec has no tag accessor in {language:?}"),
        );
    }
}

// ── a variant with a tag field of its own ───────────────────────────────

/// A variant that reads its tag from a field of its own has no accessor in
/// any backend: its tag is a field of the value.
#[test]
fn a_variant_with_a_tag_field_has_no_tag_accessor() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build is at the repository root")
        .join("tests/forge/resources");
    let absent = [
        (Language::Kotlin, "val tag: UByte"),
        (Language::Python, "def tag(self)"),
        (Language::Go, ") Tag() byte"),
        (Language::C11, "codec_variant_dispatch_tag("),
        (Language::Cpp, "std::uint8_t tag() const"),
    ];
    for (language, accessor) in absent {
        let text = generate_in(&dir, "codec_variant_dispatch", language);
        assert!(
            !text.contains(accessor),
            "a variant with a tag field has a tag accessor in {language:?}:\n{text}"
        );
    }
}
