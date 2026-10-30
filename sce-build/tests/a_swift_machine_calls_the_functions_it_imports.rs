// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend lets a `sce-static` statechart call the kinds that are
//! functions of their inputs, as `Alias(args)` the way an algorithm is called
//! (docs/adr/0048): a condition, a lookup, a transform, an interpolation and a
//! validator, each a static member of the caseless enum its unit declares, in the
//! machine's own module.
//!
//! `scenarios/static_forge_calls.json` replays the machine the build generates
//! (`StaticDatamodelTests`); this file holds the text the generator writes, which
//! that reads.
//!
//! # What was wrong
//!
//! Measured in the code on 2026-10-30: the Swift generator called only an
//! algorithm and a condition, so a machine that called a transform, a lookup, an
//! interpolation or a validator was refused by name although a Swift unit of each
//! was written, and a value of an enum the machine held in its own type had no
//! function to cross into the type the unit of the enum document declares.

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

/// The Swift machine file `sce-codegen generate -l swift` writes for `machine`, a
/// document of the fixture directory whose imports are copied beside it.
fn machine_of(machine: &str, file: &str) -> String {
    let dir = tempdir().expect("tempdir");
    for entry in std::fs::read_dir(fixtures()).expect("the fixture directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            let name = path.file_name().expect("a file name").to_owned();
            std::fs::copy(&path, dir.path().join(name)).expect("copy a fixture");
        }
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "swift"])
        .arg(dir.path().join(machine))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "{machine} is written for Swift: {}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    std::fs::read_to_string(out_dir.path().join(file))
        .unwrap_or_else(|_| panic!("{machine} writes {file}"))
}

/// `written` holds each of `needles`, or the test names the one it lacks and shows
/// what was written.
fn holds_each(written: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            written.contains(needle),
            "the generated Swift holds `{needle}`:\n{written}"
        );
    }
}

/// Each kind is called as a static member of its unit: a lookup that answers a default
/// where it misses is the value itself, and one whose miss is an error is read through
/// `Checked.found`, so the miss fails the assignment (§scxml-4.9).
#[test]
fn a_lookup_a_transform_and_an_interpolation_are_static_members_of_their_units() {
    let code = machine_of("static_forge_calls.scxml", "static_forge_calls_sm.swift");
    holds_each(
        &code,
        &[
            "self.vRatio = ForgeCallRatio.lookupRatio(self.vGear)",
            "Checked.found(ForgeCallCode.lookupCode(self.vGear))",
            "self.vOffset = ForgeCallOffset.computeOffset(self.vLevel)",
            "self.vLimit = ForgeCallLimit.lookup(self.vRpm)",
        ],
    );
}

/// A condition guards a transition by the unit's own function.
#[test]
fn a_condition_guards_a_transition_by_its_unit() {
    let code = machine_of("static_forge_calls.scxml", "static_forge_calls_sm.swift");
    holds_each(&code, &["ForgeCallHigh.forgeCallHigh(self.vLevel)"]);
}

/// The machine holds an enum in a type of its own, and the unit of the enum document
/// declares another: a value crosses into it and back through two functions of the
/// machine's file, one pair for each enum a called function holds.
#[test]
fn a_value_of_an_enum_crosses_into_the_units_type_and_back() {
    let code = machine_of("static_forge_calls.scxml", "static_forge_calls_sm.swift");
    holds_each(
        &code,
        &[
            "private func StaticForgeCallsStatusEnumToDocument(_ v: StaticForgeCallsStatusEnum) -> ForgeCallStatus {",
            "private func StaticForgeCallsStatusEnumFromDocument(_ v: ForgeCallStatus) -> StaticForgeCallsStatusEnum {",
            "self.vPhase = StaticForgeCallsStatusEnumFromDocument(ForgeCallNext.computeAfter(StaticForgeCallsStatusEnumToDocument(self.vPhase)))",
        ],
    );
}

/// A machine that calls nothing of the kind holds none of the crossing functions.
#[test]
fn a_machine_that_calls_no_function_holds_no_enum_crossing() {
    let code = machine_of("static_enum.scxml", "static_enum_sm.swift");
    assert!(
        !code.contains("ToDocument") && !code.contains("FromDocument"),
        "a machine that calls no imported function crosses no enum:\n{code}"
    );
}
