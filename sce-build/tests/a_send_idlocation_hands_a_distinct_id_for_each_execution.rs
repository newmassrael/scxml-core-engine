// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A `<send idlocation>` hands the document an id, and the document may name it in
// a later `<cancel>`: so the id belongs to one EXECUTION of the element, not to
// the element. A backend that wrote the id into its code at build time handed the
// same text to every execution of one site — a send inside a loop, or a
// transition taken twice — and one `<cancel>` then removed all of them.
//
// Each backend generates the id when the send runs, from a count its machine
// keeps: Rust, Go, Kotlin and C from the one the engine or the machine carries,
// Python from its engine's, and the C++ AOT from its generator. A send that asks
// for no `idlocation` is known to nobody and keeps the id the build gave it.
//
// What each backend generates is read from its code, because a runtime check of
// "two executions, two ids" needs a document per engine; the W3C tests 178, 183
// and 332 run on every engine and fail if the id a send is known by and the id the
// document was handed ever part.

use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

/// A document whose one transition sends with the attributes `send`.
fn sending(send: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s" datamodel="ecmascript">
  <datamodel><data id="a" expr="''"/></datamodel>
  <state id="s">
    <transition event="go" type="internal"><send {send}/></transition>
  </state>
  <final id="done"/>
</scxml>
"##
    )
}

/// Everything `sce-codegen generate -l language` writes for `doc`, joined.
fn generated(language: &str, doc: &str) -> String {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("probe.scxml");
    std::fs::write(&source, doc).expect("write the probe");
    let out_dir = dir.path().join("out");
    std::fs::create_dir(&out_dir).expect("an output directory");
    let run = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen")))
        .args([
            "generate",
            "-l",
            language,
            "--go-module-prefix",
            "x/y",
            "-o",
        ])
        .arg(&out_dir)
        .arg(&source)
        .output()
        .expect("invoke sce-codegen");
    assert!(
        run.status.success(),
        "{language}: the document generates:\n{}{}",
        String::from_utf8_lossy(&run.stderr),
        String::from_utf8_lossy(&run.stdout)
    );
    let mut text = String::new();
    for entry in std::fs::read_dir(&out_dir).expect("the output directory") {
        let path = entry.expect("an entry").path();
        if path.file_name().is_some_and(|n| n == "sce_sourcemap.json") {
            continue;
        }
        text.push_str(&std::fs::read_to_string(&path).expect("a generated file"));
        text.push('\n');
    }
    text
}

#[test]
fn a_send_that_hands_its_id_to_the_document_has_it_generated_when_it_runs() {
    let document = sending(r#"idlocation="a" event="e" delay="1s""#);
    for (language, generates) in [
        ("rust", "engine.next_auto_send_id()"),
        ("go", "engine.NextAutoSendID()"),
        ("kotlin", "nextAutoSendId()"),
        ("c", "sce_next_auto_send_id(&sm->auto_send_seq"),
        ("python", "engine._next_auto_sendid()"),
        ("cpp", "SendHelper::generateSendId()"),
    ] {
        let code = generated(language, &document);
        assert!(
            code.contains(generates),
            "{language}: the id is generated when the send runs (`{generates}`)"
        );
        assert!(
            !code.contains("\"__send_0\""),
            "{language}: no id is written into the code for every execution to share"
        );
    }
}

#[test]
fn a_c_machine_that_hands_out_ids_carries_the_count_they_come_from() {
    let code = generated("c", &sending(r#"idlocation="a" event="e" delay="1s""#));
    assert!(
        code.contains("uint64_t auto_send_seq;") && code.contains("char auto_send_id["),
        "the machine carries a count and the buffer an id is written into"
    );
}

#[test]
fn a_send_that_hands_its_id_to_nobody_keeps_the_id_the_build_gave_it() {
    // Nobody can name it, so nothing distinguishes one execution's id from
    // another's, and a count would be kept for nothing.
    let document = sending(r#"event="e" delay="1s""#);
    for language in ["rust", "go", "kotlin"] {
        let code = generated(language, &document);
        assert!(
            code.contains("\"__send_0\""),
            "{language}: the send keeps its build-time id"
        );
    }
    let c = generated("c", &document);
    assert!(
        !c.contains("auto_send_seq"),
        "a C machine that hands out no id carries no count"
    );
}

#[test]
fn a_send_with_an_id_of_its_own_keeps_it() {
    // An `id` the author wrote is the send's whatever else is asked of it.
    let document = sending(r#"id="mine" idlocation="a" event="e" delay="1s""#);
    for language in ["rust", "go", "kotlin"] {
        let code = generated(language, &document);
        assert!(
            code.contains("\"mine\""),
            "{language}: the id the author wrote is the send's"
        );
    }
}
