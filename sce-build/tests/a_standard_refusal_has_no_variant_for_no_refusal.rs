// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// No refusal is an absent record, and no variant of the standard enum of reasons
// (docs/adr/0055, section 7).
//
// `record_refusal` names why a shared record list refused a change of its own. It
// once declared `none` = 0 for "no change was refused", a value the list never told
// and a parent held its last refusal in before there was one. The enum now starts at
// `notWriter` = 1, and a parent keeps its last refusal in an
// `optional<record:Refused>` that starts `null` and takes the payload whole, so the
// record holds the four fields of one refusal or is absent.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
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

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()))
}

/// The value of the attribute `name` in the element text `tag`, which writes it as
/// `name="..."`.
fn attribute<'t>(tag: &'t str, name: &str) -> &'t str {
    let key = format!("{name}=\"");
    let start = tag
        .find(&key)
        .unwrap_or_else(|| panic!("{tag} has no {name}"))
        + key.len();
    let end = tag[start..]
        .find('"')
        .unwrap_or_else(|| panic!("{tag} ends its {name}"))
        + start;
    &tag[start..end]
}

/// Every variant `stdlib/shared/record_refusal.scxml` declares, as `(name, value)` in
/// the order it declares them. The comment above the root is prose and is not read.
fn variants() -> Vec<(String, u32)> {
    read(&repo_root().join("stdlib/shared/record_refusal.scxml"))
        .lines()
        .filter(|line| line.trim_start().starts_with("<sce:variant "))
        .map(|line| {
            (
                attribute(line, "name").to_string(),
                attribute(line, "value")
                    .parse()
                    .expect("a variant's value is a number"),
            )
        })
        .collect()
}

/// `sce-codegen <args...> <path>` with JSON diagnostics: `(exit_ok, stdout + stderr)`.
fn run_path(args: &[&str], path: &Path) -> (bool, String) {
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// A statechart that imports the standard enum as `Refusal` and holds `variable`, a
/// value of it, starting at `start`.
fn machine_starting_at(start: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <sce:import kind="enum" src="sce:std/shared/record_refusal.scxml" as="Refusal"/>
  <datamodel>
    <data id="last" sce:type="enum:Refusal" expr="{start}"/>
  </datamodel>
  <state id="s"/>
</scxml>
"##
    )
}

fn check_probe(text: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, text).expect("write probe");
    run_path(&["check", "-l", "rust"], &path)
}

#[test]
fn record_refusal_declares_no_variant_for_no_refusal_and_starts_at_not_writer() {
    let variants = variants();
    let names: Vec<&str> = variants.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "notWriter",
            "noRoomForValue",
            "noRoomForHistory",
            "noRoomForDigest",
            "textTooLong",
            "changeTooLarge",
            "registerNamedTwice",
            "clockExhausted",
        ],
        "every variant is a reason, and none stands for no refusal"
    );
    assert!(
        !names.contains(&"none"),
        "`none` is retired: a parent keeps no refusal as an absent record"
    );
    assert_eq!(
        variants.first(),
        Some(&("notWriter".to_string(), 1)),
        "the first variant is notWriter, at the number it always had"
    );
    // Every other number is the one it had before `none` was removed: one more than
    // its place, so nothing a saved state or a peer holds was renumbered.
    for (place, (name, value)) in variants.iter().enumerate() {
        assert_eq!(
            *value,
            u32::try_from(place + 1).expect("a few variants"),
            "{name} keeps its number"
        );
    }
}

#[test]
fn a_variable_of_record_refusal_starts_at_a_reason_and_never_at_none() {
    let (ok, out) = check_probe(&machine_starting_at("Refusal.notWriter"));
    assert!(ok, "a variant the enum declares is a start:\n{out}");
    let (ok, out) = check_probe(&machine_starting_at("Refusal.none"));
    assert!(
        !ok,
        "`Refusal.none` names no variant of the enum, and is refused:\n{out}"
    );
    assert!(
        out.contains("none"),
        "the refusal names what it refused:\n{out}"
    );
}

#[test]
fn the_parent_keeps_its_last_refusal_as_an_optional_record_that_starts_absent() {
    let text = read(&fixtures().join("record_list_parent.scxml"));
    let declaration = text
        .lines()
        .find(|line| line.contains(r#"<data id="refused""#))
        .expect("the parent declares `refused`");
    assert_eq!(
        attribute(declaration, "sce:type"),
        "optional&lt;record:Refused&gt;",
        "{declaration}"
    );
    assert_eq!(attribute(declaration, "expr"), "null", "{declaration}");
    assert!(
        !declaration.contains("<sce:set") && declaration.trim_end().ends_with("/>"),
        "an optional record is built from no <sce:set>: {declaration}"
    );
    // The enum has no `none` to start a reason at, so no variant of either enum is
    // named, and the parent imports neither.
    for gone in ["Refusal.none", "Command.edit", "kind=\"enum\""] {
        assert!(!text.contains(gone), "the parent no longer writes {gone}");
    }
}

#[test]
fn the_parent_takes_a_refusal_whole_from_the_payload() {
    let text = read(&fixtures().join("record_list_parent.scxml"));
    assert!(
        text.contains(r#"<assign location="refused" expr="_event.data"/>"#),
        "the handler of record_list.refused puts the whole record into `refused`"
    );
    assert!(
        !text.contains(r#"location="refused."#),
        "a field of an absent record is never written: `refused` is a record or absent"
    );
}

#[test]
fn the_parent_and_the_enum_probe_generate_on_every_backend_that_runs_them() {
    // The parent is the machine the Rust, Go, Kotlin, Python and C lanes replay; the
    // unimported-payload fixture holds the enum as an optional that starts absent.
    for fixture in [
        "record_list_parent.scxml",
        "static_payload_enum_unimported.scxml",
    ] {
        for language in ["rust", "go", "kotlin", "python", "c11"] {
            let out_dir = tempdir().expect("tempdir");
            let (ok, out) = run_path(
                &[
                    "generate",
                    "-l",
                    language,
                    "--go-module-prefix",
                    "x/y",
                    "-o",
                    out_dir.path().to_str().expect("a UTF-8 path"),
                ],
                &fixtures().join(fixture),
            );
            assert!(ok, "{language}: {fixture} generates:\n{out}");
        }
    }
}

#[test]
fn the_unimported_payload_fixture_holds_the_enum_as_an_optional_that_starts_absent() {
    let text = read(&fixtures().join("static_payload_enum_unimported.scxml"));
    let declaration = text
        .lines()
        .find(|line| line.contains(r#"<data id="last""#))
        .expect("the fixture declares `last`");
    assert_eq!(
        attribute(declaration, "sce:type"),
        "optional&lt;enum:Why&gt;",
        "{declaration}"
    );
    assert_eq!(attribute(declaration, "expr"), "null", "{declaration}");
}

#[test]
fn the_scenarios_first_step_expects_no_refusal_as_null_and_later_steps_a_whole_record() {
    let path = fixtures().join("scenarios/record_list_parent.json");
    let scenario: Value = sce_build::strict_json::from_str(&read(&path))
        .unwrap_or_else(|e| panic!("{} is JSON with no key twice: {e}", path.display()));
    let steps = scenario["steps"].as_array().expect("steps");
    assert_eq!(
        steps[0]["expect"]["variables"].get("refused"),
        Some(&Value::Null),
        "before the first refusal the parent holds none: `refused` is stated, and is null"
    );
    let mut refusals = 0;
    for step in &steps[1..] {
        let Some(refused) = step["expect"]["variables"].get("refused") else {
            continue;
        };
        refusals += 1;
        let record = refused.as_object().expect("a later step expects a record");
        let mut fields: Vec<&str> = record.keys().map(String::as_str).collect();
        fields.sort_unstable();
        assert_eq!(
            fields,
            ["field", "id", "kind", "reason"],
            "a present refusal states all four fields: {step}"
        );
        assert_ne!(
            record["reason"], "none",
            "no refusal has a reason that stands for none: {step}"
        );
    }
    assert!(
        refusals >= 7,
        "the scenario states the refusal of each reason it provokes: {refusals}"
    );
}
