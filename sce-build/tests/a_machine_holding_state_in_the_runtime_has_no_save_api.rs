// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Saving a `datamodel="sce-static"` machine (docs/SCE_ACCEPTED_SUBSET.md §2.15,
// "Saving and restoring"): a machine that keeps part of its state in the
// RUNTIME rather than in its own fields is generated WITHOUT the save API,
// never with one that would write less than the machine holds.
//
// What the runtime keeps is what a saved state does not yet hold — a delayed
// `<send>` still pending in the scheduler, an invoked session. (What a
// `<history>` recorded IS held, and so is the external queue.) The rule is the
// safety of the whole feature: a `save()` that left a pending timer out would
// restore a machine that never fires it, and nothing would say so. It is
// decided by the shape the generator computes (`saved_shape`), from a model
// the ANALYZER has already read — a delayed `<send>` is known to need the
// scheduler only after that pass — so this runs the generator end to end
// rather than asking the lowering about a model nothing analysed. The unit
// test beside `saved_shape` did exactly that and could not see a delayed send.
//
// A machine that does hold everything gets the API, on both backends that
// have one: the control that keeps the refusals above from being a generator
// that never emits it.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// What `sce-codegen generate -l <language>` wrote for `document`, joined, and
/// whether it succeeded.
fn generate(language: &str, document: &str) -> (bool, String) {
    let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "no-save-api-{language}-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join("machine.scxml");
    std::fs::write(&path, document).expect("write document");
    let run = Command::new(sce_codegen_bin())
        .args([
            "generate",
            path.to_str().unwrap(),
            "-l",
            language,
            "-o",
            dir.to_str().unwrap(),
            "--error-format=json",
        ])
        .current_dir(repo_root())
        .output()
        .expect("spawn sce-codegen");
    let mut text = String::from_utf8_lossy(&run.stderr).into_owned();
    for entry in std::fs::read_dir(&dir).expect("read scratch dir") {
        let file = entry.expect("dir entry").path();
        if file
            .extension()
            .is_some_and(|e| e == "json" || e == "scxml")
        {
            continue;
        }
        if let Ok(generated) = std::fs::read_to_string(&file) {
            text.push_str(&generated);
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    (run.status.success(), text)
}

/// A machine whose whole state is its fields and its configuration, with
/// `extra` written into its first state.
fn machine(extra: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="machine">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="idle">
    {extra}
    <transition event="go" target="done">
      <assign location="count" expr="count + 1"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

/// The marker of a generated save API, per backend: the trait the Rust machine
/// implements and the method the Kotlin class declares.
const SAVE_API: [(&str, &str); 2] = [
    ("rust", "Persist for Engine<"),
    ("kotlin", "fun save(): SavedState"),
];

fn has_save_api(language: &str, text: &str) -> bool {
    let marker = SAVE_API
        .iter()
        .find(|(l, _)| *l == language)
        .map(|(_, m)| *m)
        .expect("a backend with a save API");
    text.contains(marker)
}

#[test]
fn a_machine_that_holds_all_of_its_state_is_generated_with_the_save_api() {
    for (language, _) in SAVE_API {
        let (ok, text) = generate(language, &machine(""));
        assert!(
            ok,
            "{language}: the plain machine does not generate:\n{text}"
        );
        assert!(
            has_save_api(language, &text),
            "{language}: a machine of fields alone has a save API"
        );
    }
}

#[test]
fn a_machine_with_a_pending_delayed_send_is_generated_without_it() {
    for (language, _) in SAVE_API {
        let (ok, text) = generate(
            language,
            &machine(r#"<onentry><send event="go" delay="5s"/></onentry>"#),
        );
        assert!(ok, "{language}: the machine does not generate:\n{text}");
        assert!(
            !has_save_api(language, &text),
            "{language}: a timer still pending in the scheduler is not in a saved state, so the \
             machine must not offer to save one"
        );
    }
}

#[test]
fn a_cancel_is_the_scheduler_too() {
    // A `<cancel>` is not a delayed send, and a machine that has one has a
    // scheduler whose entries it names.
    for (language, _) in SAVE_API {
        let (ok, text) = generate(
            language,
            &machine(r#"<onexit><cancel sendid="timer"/></onexit>"#),
        );
        assert!(ok, "{language}: the machine does not generate:\n{text}");
        assert!(
            !has_save_api(language, &text),
            "{language}: a machine that cancels sends by id keeps them in the scheduler"
        );
    }
}

#[test]
fn a_machine_with_a_history_is_generated_with_the_save_api() {
    // What a `<history>` recorded is held by the saved state, so a machine with
    // one saves.
    let with_history = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="outer" datamodel="sce-static" name="machine">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="outer" initial="a">
    <history id="h"><transition target="a"/></history>
    <state id="a"><transition event="next" target="b"/></state>
    <state id="b"/>
    <transition event="away" target="elsewhere"/>
  </state>
  <state id="elsewhere"><transition event="back" target="h"/></state>
</scxml>
"#;
    for (language, _) in SAVE_API {
        let (ok, text) = generate(language, with_history);
        assert!(ok, "{language}: the machine does not generate:\n{text}");
        assert!(
            has_save_api(language, &text),
            "{language}: a machine with a history saves it"
        );
    }
}
