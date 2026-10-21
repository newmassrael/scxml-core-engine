// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `sce-static` statechart imports a transform, a condition, a lookup, an
//! interpolation and a validator and calls each as `Alias(args)`, as it calls an
//! algorithm (docs/adr/0048).
//!
//! # What was wrong
//!
//! Read in the code on 2026-10-21: the statechart read the imports of
//! `kind="algorithm"` and no other, so a guard that called an imported lookup or
//! validator was refused as calling a name nothing provides, and the only way to
//! call one was an algorithm that restated its table or its ranges.
//! `scenarios/static_forge_calls.json` replays a call of each kind on the six
//! generated backends; these cases hold the rule, each refusal and what each
//! backend lowers a call to.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel")
}

/// The machine of the fixtures, and the documents it calls, beside it.
const MACHINE: &str = "static_forge_calls.scxml";
const SIBLINGS: [&str; 11] = [
    "forge_call_high.scxml",
    "forge_call_offset.scxml",
    "forge_call_ratio.scxml",
    "forge_call_code.scxml",
    "forge_call_limit.scxml",
    "forge_call_range.scxml",
    "forge_call_status.scxml",
    "forge_call_level.scxml",
    "forge_call_next.scxml",
    "forge_call_weight.scxml",
    "forge_call_named.scxml",
];

/// The fixture document `name`, with `from` replaced by `to` when given.
fn fixture(name: &str, replace: Option<(&str, &str)>) -> String {
    let text = std::fs::read_to_string(fixtures().join(name)).expect("a fixture");
    match replace {
        Some((from, to)) => {
            assert!(text.contains(from), "{name} no longer holds `{from}`");
            text.replacen(from, to, 1)
        }
        None => text,
    }
}

/// A directory holding the machine and its siblings, `documents` written over them.
fn staged(documents: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir");
    for name in SIBLINGS.iter().chain(std::iter::once(&MACHINE)) {
        std::fs::copy(fixtures().join(name), dir.path().join(name)).expect("copy a fixture");
    }
    for (name, text) in documents {
        std::fs::write(dir.path().join(name), text).expect("write a document");
    }
    dir
}

/// `sce-codegen <args…>` on the machine, beside its siblings and `documents`.
/// Returns `(exit_ok, stdout + stderr)`.
fn run(args: &[&str], documents: &[(&str, &str)]) -> (bool, String) {
    let dir = staged(documents);
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(dir.path().join(MACHINE))
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// The `check` arguments of each of the six generated backends.
fn every_backend() -> Vec<Vec<&'static str>> {
    vec![
        vec!["check", "-l", "rust"],
        vec!["check", "-l", "kotlin"],
        vec!["check", "-l", "python"],
        vec!["check", "-l", "c"],
        vec!["check", "-l", "cpp"],
        vec![
            "check",
            "-l",
            "go",
            "--go-module-prefix",
            "example.test/generated",
        ],
    ]
}

/// What `generate -l <language>` writes for the machine, every file of it in one
/// text.
fn generated(language: &str) -> String {
    let dir = staged(&[]);
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("generate")
        .arg(dir.path().join(MACHINE))
        .args([
            "-l",
            language,
            "--go-module-prefix",
            "example.test/generated",
        ])
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "generate -l {language}:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut text = String::new();
    let mut pending = vec![out_dir.path().to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the output directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
    text
}

/// The machine, with `documents` written over its siblings or itself, is refused
/// for Rust, naming `reason`.
fn refused(documents: &[(&str, &str)], reason: &str) {
    let (ok, out) = run(&["check", "-l", "rust"], documents);
    assert!(!ok, "expected a refusal naming `{reason}`:\n{out}");
    assert!(out.contains(reason), "expected `{reason}` in:\n{out}");
}

#[test]
fn every_backend_lowers_the_machine_that_calls_each_kind() {
    for args in every_backend() {
        let (ok, out) = run(&args, &[]);
        assert!(ok, "{args:?} must lower the machine:\n{out}");
    }
}

/// Each backend calls the function each document's own generation emits, through
/// the import a forge kind writes for it; a validator is built for the call and its
/// validity read; a lookup whose miss is an error is read through the runtime's
/// `found`, and on C through the reading the machine declares beside the call.
#[test]
fn each_backend_calls_the_function_each_documents_generation_emits() {
    for (language, fragments) in [
        (
            "rust",
            &[
                "use super::forge_call_ratio;",
                "forge_call_ratio::lookup_ratio(",
                "forge_call_offset::compute_offset(",
                "forge_call_high::forge_call_high(",
                "forge_call_limit::ForgeCallLimit::lookup(",
                "forge_call_range::ForgeCallRange::new().validate(",
                ").valid",
                "sce_forge_runtime::algorithm::found(forge_call_code::lookup_code(",
            ][..],
        ),
        (
            "kotlin",
            &[
                "import com.sce.generated.forge_call_ratio.*",
                "lookupRatio(",
                "computeOffset(",
                "ForgeCallLimit.lookup(",
                "ForgeCallRange().validate(",
                ").valid",
                "com.sce.forge.runtime.SceChecked.found(lookupCode(",
            ][..],
        ),
        (
            "cpp",
            &[
                "#include \"forge_call_ratio.h\"",
                "SCE::Generated::ForgeCallRatio::lookupRatio(",
                "SCE::Generated::ForgeCallOffset::computeOffset(",
                "SCE::Generated::ForgeCallLimit::ForgeCallLimit::lookup(",
                "SCE::Generated::ForgeCallRange::ForgeCallRange{}.validate(",
                ").valid",
                "SCE::Forge::Checked::found(SCE::Generated::ForgeCallCode::lookupCode(",
            ][..],
        ),
        (
            "go",
            &[
                ".LookupRatio(",
                ".ComputeOffset(",
                ".NewForgeCallRange().Validate(",
                ").Valid",
                "scealgorithm.TakeOutcome(&sceFailure, scealgorithm.Found(",
            ][..],
        ),
        (
            "python",
            &[
                ".lookup_ratio(",
                ".compute_offset(",
                ".ForgeCallRange().validate(",
                ").valid",
                "sce_algorithm.found(",
            ][..],
        ),
        (
            "c11",
            &[
                "#include \"forge_call_ratio.h\"",
                "forge_call_ratio_ratio(",
                "forge_call_offset_compute_offset(",
                "forge_call_high_check(",
                "forge_call_limit_lookup(",
                "forge_call_range_validate(",
                ").valid",
                "SCE_FOUND_FORGE_CALL_CODE_CODE_FOUND",
                "forge_call_code_code_found_take(&sce_failure_, forge_call_code_code_found(",
            ][..],
        ),
    ] {
        let out = generated(language);
        for fragment in fragments {
            assert!(
                out.contains(fragment),
                "{language}: the generated machine holds `{fragment}`:\n{out}"
            );
        }
    }
}

/// A lookup with a default answers it, and is no call that may fail: it is called
/// as the function itself.
#[test]
fn a_lookup_with_a_default_is_called_as_it_is() {
    let out = generated("rust");
    assert!(
        !out.contains("sce_forge_runtime::algorithm::found(forge_call_ratio::"),
        "a lookup with a default cannot fail:\n{out}"
    );
}

#[test]
fn a_transform_of_two_outputs_is_refused_where_it_is_imported() {
    let two = fixture(
        "forge_call_offset.scxml",
        Some((
            "  </datamodel>",
            "    <data id=\"twice\" sce:type=\"int32\" sce:direction=\"out\" expr=\"level + level\"/>\n  </datamodel>",
        )),
    );
    refused(
        &[("forge_call_offset.scxml", &two)],
        "it computes 2 outputs",
    );
}

#[test]
fn a_transform_that_keeps_a_value_is_refused_where_it_is_imported() {
    let kept = fixture(
        "forge_call_offset.scxml",
        Some(("expr=\"level - 10\"", "expr=\"level - previous(level)\"")),
    );
    let kept = kept.replacen(
        "sce:type=\"int32\" sce:direction=\"in\"/>",
        "sce:type=\"int32\" sce:direction=\"in\" sce:initial=\"0\"/>",
        1,
    );
    refused(
        &[("forge_call_offset.scxml", &kept)],
        "reads `previous(level)`",
    );
}

#[test]
fn a_lookup_of_a_string_is_refused_where_it_is_imported() {
    let named = fixture(
        "forge_call_ratio.scxml",
        Some(("sce:type=\"int32\"", "sce:type=\"string\"")),
    )
    .replace("value=\"35\"", "value=\"LOW\"")
    .replace("value=\"20\"", "value=\"MID\"")
    .replace("value=\"14\"", "value=\"HIGH\"")
    .replace("sce:default=\"0\"", "sce:default=\"LOW\"");
    refused(&[("forge_call_ratio.scxml", &named)], "is a string");
}

#[test]
fn an_interpolation_that_does_not_clamp_is_refused_where_it_is_imported() {
    let extrapolated = fixture(
        "forge_call_limit.scxml",
        Some((
            "sce:out-of-bounds=\"clamp\"",
            "sce:out-of-bounds=\"extrapolate\"",
        )),
    );
    refused(
        &[("forge_call_limit.scxml", &extrapolated)],
        "sce:out-of-bounds=\\\"extrapolate\\\"",
    );
}

#[test]
fn a_validator_that_keeps_what_it_read_is_refused_where_it_is_imported() {
    let rated = fixture(
        "forge_call_range.scxml",
        Some((
            "sce:range-max=\"30\"",
            "sce:range-max=\"30\" sce:max-delta=\"5\" sce:sample-interval=\"100ms\"",
        )),
    );
    refused(
        &[("forge_call_range.scxml", &rated)],
        "its input `level` declares a rate of change",
    );
}

/// A byte string is no slot a statechart hands such a document (docs/adr/0051).
#[test]
fn a_slot_of_bytes_is_refused_where_it_is_imported() {
    let text = fixture(
        "forge_call_high.scxml",
        Some((
            "<data id=\"level\" sce:type=\"int32\" sce:direction=\"in\"/>",
            "<data id=\"level\" sce:type=\"int32\" sce:direction=\"in\"/>\n    \
             <data id=\"frame\" sce:type=\"bytes\" sce:direction=\"in\"/>",
        )),
    );
    refused(
        &[("forge_call_high.scxml", &text)],
        "its input `frame` is a byte string",
    );
}

/// A text is an input a statechart hands over as it is (docs/adr/0051), and an
/// output it does not take: a call answers a text with no bound a statechart holds.
#[test]
fn a_text_is_an_input_and_no_output() {
    let output = fixture(
        "forge_call_offset.scxml",
        Some((
            "<data id=\"offset\" sce:type=\"int32\" sce:direction=\"out\" expr=\"level - 10\"/>",
            "<data id=\"offset\" sce:type=\"string\" sce:direction=\"out\" expr=\"'x'\"/>",
        )),
    );
    refused(
        &[("forge_call_offset.scxml", &output)],
        "its output `offset` is a string",
    );
    // A lookup is keyed by numbers or by an enum's variants, which every backend's
    // table compares alike, and not by a text.
    let keyed = fixture(
        "forge_call_code.scxml",
        Some((
            "<data id=\"gear\" sce:type=\"uint8\" sce:direction=\"in\"/>",
            "<data id=\"gear\" sce:type=\"string\" sce:direction=\"in\"/>",
        )),
    );
    refused(
        &[("forge_call_code.scxml", &keyed)],
        "its input `gear` is a string",
    );
}

/// A value of an enum crosses by the name its variant declares, which only one
/// document gives both sides: a slot of an enum the statechart does not import from
/// the same document is refused where the callee is imported (docs/adr/0051).
#[test]
fn a_slot_of_an_enum_the_statechart_does_not_import_is_refused() {
    let machine = fixture(
        MACHINE,
        Some((
            "  <sce:import kind=\"enum\" src=\"forge_call_level.scxml\" as=\"Level\"/>\n",
            "",
        )),
    )
    .replace(
        "<data id=\"weight\" sce:type=\"enum:Level\" expr=\"Level.light\" sce:direction=\"out\"/>",
        "",
    )
    .replace(
        "<assign location=\"weight\" expr=\"Weight(phase)\"/>",
        "<assign location=\"misses\" expr=\"misses\"/>",
    );
    refused(
        &[(MACHINE, &machine)],
        "which the statechart does not import as an enum",
    );
}

/// A value of an enum is handed to a callee whose slot holds that enum, and taken
/// back where a value of it stands: a number is no status (docs/adr/0051).
#[test]
fn an_argument_of_an_enum_slot_is_a_value_of_that_enum() {
    let machine = fixture(
        MACHINE,
        Some((
            "<assign location=\"phase\" expr=\"Next(phase)\"/>",
            "<assign location=\"phase\" expr=\"Next(gear)\"/>",
        )),
    );
    refused(
        &[(MACHINE, &machine)],
        "a value that is not of the enum `Status` handed to `Next`",
    );
}

/// A lookup names each key and value by the variant's declared name, and a name the
/// enum does not declare is refused where the lookup is generated.
#[test]
fn a_lookup_entry_names_a_variant_its_enum_declares() {
    let dir = staged(&[(
        "forge_call_weight.scxml",
        &fixture(
            "forge_call_weight.scxml",
            Some(("key=\"held\"", "key=\"waiting\"")),
        ),
    )]);
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .args(["check", "-l", "rust"])
        .arg(dir.path().join("forge_call_weight.scxml"))
        .output()
        .expect("invoke sce-codegen");
    let text =
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "an unknown key was taken:\n{text}");
    assert!(
        text.contains("waiting") && text.contains("no variant of the enum `Status`"),
        "{text}"
    );
}

/// Each backend crosses a value of an enum into the type the enum document declares
/// through a function its machine declares, and takes it back through the other; C
/// holds the document's own type and passes it as it is (docs/adr/0051).
#[test]
fn each_backend_crosses_a_value_of_an_enum_into_the_callee_and_back() {
    for (language, fragments) in [
        (
            "rust",
            &[
                "fn StaticForgeCallsStatusEnumToDocument(",
                "fn StaticForgeCallsLevelEnumFromDocument(",
                "forge_call_status::ForgeCallStatus",
                "forge_call_next::compute_after(",
                "StaticForgeCallsStatusEnumToDocument(self.phase)",
                "forge_call_weight::lookup_weight(",
                "forge_call_named::ForgeCallNamed::new().validate(",
                "&(self.customer)",
            ][..],
        ),
        (
            "kotlin",
            &[
                "fun StaticForgeCallsStatusEnumToDocument(v: StaticForgeCallsStatusEnum): ForgeCallStatus",
                "fun StaticForgeCallsLevelEnumFromDocument(v: ForgeCallLevel): StaticForgeCallsLevelEnum",
                "computeAfter(StaticForgeCallsStatusEnumToDocument(",
            ][..],
        ),
        (
            "cpp",
            &[
                "StaticForgeCallsStatusEnumToDocument(",
                "StaticForgeCallsLevelEnumFromDocument(",
            ][..],
        ),
        (
            "go",
            &[
                "func StaticForgeCallsStatusEnumToDocument(v StaticForgeCallsStatusEnum)",
                "func StaticForgeCallsLevelEnumFromDocument(",
            ][..],
        ),
        (
            "python",
            &[
                "def StaticForgeCallsStatusEnumToDocument(v):",
                "def StaticForgeCallsLevelEnumFromDocument(v):",
            ][..],
        ),
        (
            "c11",
            &[
                "forge_call_next_compute_after(",
                "forge_call_weight_weight_found(",
                "forge_call_named_validate(",
            ][..],
        ),
    ] {
        let out = generated(language);
        for fragment in fragments {
            assert!(
                out.contains(fragment),
                "{language}: the generated machine holds `{fragment}`:\n{out}"
            );
        }
    }
    // C's machine holds the document's own type, so nothing crosses there.
    let c = generated("c11");
    assert!(!c.contains("ToDocument"), "{c}");
}

/// An import nothing calls is refused by its own kind, as an unused algorithm is.
#[test]
fn an_import_nothing_calls_is_refused_by_its_kind() {
    let machine = fixture(
        MACHINE,
        Some((
            "    <transition event=\"probe\" cond=\"High(level)\" type=\"internal\">",
            "    <transition event=\"probe\" cond=\"level &gt; 20\" type=\"internal\">",
        )),
    );
    refused(
        &[(MACHINE, &machine)],
        "an imported condition is called by the document, and nothing calls `High`",
    );
}

/// A call is judged by the document's own signature: a validity is a truth value,
/// and no number a transform's input may be. Every import stays called, so the
/// refusal is the argument's and not that of an import nothing calls.
#[test]
fn a_validity_is_a_truth_value() {
    let machine = fixture(
        MACHINE,
        Some((
            "<assign location=\"offset\" expr=\"Offset(level)\"/>",
            "<assign location=\"offset\" expr=\"Offset(Range(level, offset))\"/>",
        )),
    );
    let (ok, out) = run(&["check", "-l", "rust"], &[(MACHINE, &machine)]);
    assert!(
        !ok && out.contains("expression/type-mismatch"),
        "a validity handed to a number's slot is refused as a type mismatch:\n{out}"
    );
}

/// A lookup whose miss is an error may fail, so it is refused where no failure can
/// be received: a `<data expr>`, which runs before there is a session to raise in.
#[test]
fn a_lookup_that_may_fail_is_refused_where_no_failure_is_received() {
    let machine = fixture(
        MACHINE,
        Some((
            "<data id=\"code\" sce:type=\"int32\" expr=\"0\" sce:direction=\"out\"/>",
            "<data id=\"code\" sce:type=\"int32\" expr=\"Code(1)\" sce:direction=\"out\"/>",
        )),
    );
    refused(
        &[(MACHINE, &machine)],
        "an initial value that can overflow or fail",
    );
}

/// The Interpreter has no lowering of these kinds yet, and refuses a call of one by
/// its kind rather than read it as an algorithm.
#[test]
fn the_interpreter_refuses_the_call_by_its_kind() {
    let (ok, out) = run(&["lower"], &[]);
    assert!(!ok, "the Interpreter does not lower the call yet:\n{out}");
    assert!(
        out.contains("a call of an imported condition has no ecmascript lowering yet"),
        "{out}"
    );
}
