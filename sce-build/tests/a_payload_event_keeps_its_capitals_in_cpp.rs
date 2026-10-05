// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: an event that carries a typed payload and whose name
// has a capital after its first letter (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// The C++ `Event` enum spells an event name as a state is: first letter up, the
// rest as written (`mem.hF` is `Mem_hF`). The payload code that lifts
// `_event.data` names the same member, and used to spell it with its own rule —
// first letter up, the REST DOWN — so the member it asked for, `Mem_hf`, was not
// in the enum and the generated header did not compile. A document whose event
// names were all lower case never showed it; one that wrote `mem.hF` for the
// stored hour of a refuel did, and only when it was built into a product.

use sce_build::toolchain;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The payload of `mem.hF`: one number, `value`.
const SCHEMA: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_mem" sce:event-name="mem.hF">
  <datamodel>
    <data id="value" sce:type="uint16" sce:direction="in"/>
  </datamodel>
</scxml>
"##;

/// A statechart that takes the number the event carries into a variable.
const MACHINE: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="probe" initial="s" datamodel="sce-static">
  <sce:import kind="event-schema" src="schema_mem.scxml" as="Mem"/>
  <datamodel>
    <data id="total" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="mem.hF" type="internal">
      <assign location="total" expr="_event.data.value"/>
    </transition>
  </state>
</scxml>
"##;

/// Scratch dir keyed by the calling test, so the tests that run side by side in
/// one process never share a directory.
fn out_dir(scope: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("a_payload_event_keeps_its_capitals_in_cpp")
        .join(scope);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Generate the C++ for the probe into a fresh directory and return it.
fn generate(scope: &str) -> PathBuf {
    let dir = out_dir(scope);
    std::fs::write(dir.join("schema_mem.scxml"), SCHEMA).expect("write schema");
    let doc = dir.join("probe.scxml");
    std::fs::write(&doc, MACHINE).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(&doc)
        .arg("-o")
        .arg(&dir)
        .arg("-l")
        .arg("cpp")
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "sce-codegen generate -l cpp refused the probe:\n{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    dir
}

#[test]
fn the_payload_code_names_the_enum_member_as_the_enum_declares_it() {
    let dir = generate("form");
    let header = std::fs::read_to_string(dir.join("probe_sm.h")).expect("read generated header");
    assert!(
        header.contains("case Event::Mem_hF:"),
        "the payload lift must name the member the enum declares, `Mem_hF`"
    );
    assert!(
        !header.contains("Event::Mem_hf"),
        "no member `Mem_hf` is declared; the payload code asked for one"
    );
}

// The form check says which spelling was written; this one asks the compiler
// whether it is a member. Skipped, not failed, where g++ is absent.
#[test]
fn the_generated_header_with_a_capital_in_an_event_name_compiles() {
    let dir = generate("compile");
    let Some(gpp) = toolchain::locate("g++") else {
        toolchain::skipped(
            "the_generated_header_with_a_capital_in_an_event_name_compiles: g++ not on PATH",
        );
        return;
    };
    let driver = dir.join("driver.cpp");
    std::fs::write(
        &driver,
        "#include \"probe_sm.h\"\nint main() { return 0; }\n",
    )
    .expect("write driver");
    let compile = Command::new(&gpp)
        .arg("-std=c++17")
        .arg("-fsyntax-only")
        .arg("-I")
        .arg(repo_root().join("sce/include"))
        .arg("-I")
        .arg(repo_root().join("backends/cpp/forge-runtime/include"))
        .arg("-I")
        .arg(&dir)
        .arg(&driver)
        .output()
        .expect("run g++");
    assert!(
        compile.status.success(),
        "the generated C++ for an event named `mem.hF` did not compile\nstderr: {}",
        String::from_utf8_lossy(&compile.stderr),
    );
}
