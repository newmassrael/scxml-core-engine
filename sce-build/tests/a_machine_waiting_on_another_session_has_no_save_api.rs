// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Saving a `datamodel="sce-static"` machine (docs/SCE_ACCEPTED_SUBSET.md §2.15,
// "Saving and restoring"): a machine that is waiting on ANOTHER SESSION is
// generated WITHOUT the save API, never with one that would write less than
// the machine holds.
//
// A saved state holds what this session holds: its configuration, its
// variables, what each `<history>` recorded, the delayed `<send>`s it is
// waiting to deliver to its own queues or to a host-served processor, the child
// sessions it is running, static or named by a value among declared candidates
// (a restore starts each again), the invocations a
// declared host invoker is running (a restore starts each again from the request
// it was started with), and its external queue. What it cannot hold is a session
// whose start a restore cannot repeat — a Mesh request, which the peer may
// already have acted on, or a delayed send waiting to be delivered to the
// parent, to an invocation or to a child session. The rule is the safety of the
// whole feature: a `save()` that left a Mesh request out would restore a machine
// that waits for a `done.invoke` nobody will send, and nothing would say so.
//
// It is decided by the shape the generator computes (`saved_shape`), from a
// model the ANALYZER has already read, so this runs the generator end to end
// rather than asking the lowering about a model nothing analysed. The unit
// tests beside `saved_shape` run the analyzer themselves, and this is the
// control that they and the templates agree.
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
/// whether it succeeded. `flags` are handed to the generator as well: the
/// declarations a host makes (`--host-invoker <type>`). `siblings` are
/// `(file name, content)` pairs written next to the document, for one that
/// names others the generator reads from beside it (`sce:candidates`).
fn generate_beside(
    language: &str,
    document: &str,
    flags: &[&str],
    siblings: &[(&str, &str)],
) -> (bool, String) {
    let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "no-save-api-{language}-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = dir.join("machine.scxml");
    std::fs::write(&path, document).expect("write document");
    for (name, content) in siblings {
        std::fs::write(dir.join(name), content).expect("write sibling document");
    }
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
        .args(flags)
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

/// Generate `document` for every backend that has a save API and say, for
/// each, whether it came with one. A document that does not generate at all
/// fails the test, so a refusal is never read as "no API".
fn save_api_per_backend(document: &str) -> Vec<(&'static str, bool)> {
    save_api_per_backend_declaring(document, &[])
}

/// [`save_api_per_backend`] for a document its host declared `flags` for.
fn save_api_per_backend_declaring(document: &str, flags: &[&str]) -> Vec<(&'static str, bool)> {
    save_api_per_backend_beside(document, flags, &[])
}

/// [`save_api_per_backend_declaring`] for a document with `siblings` beside it.
fn save_api_per_backend_beside(
    document: &str,
    flags: &[&str],
    siblings: &[(&str, &str)],
) -> Vec<(&'static str, bool)> {
    SAVE_API
        .iter()
        .map(|(language, _)| {
            let (ok, text) = generate_beside(language, document, flags, siblings);
            assert!(
                ok,
                "{language}: the machine does not generate:\n{text}\n--- document:\n{document}"
            );
            (*language, has_save_api(language, &text))
        })
        .collect()
}

#[test]
fn a_machine_that_holds_all_of_its_state_is_generated_with_the_save_api() {
    for (language, has) in save_api_per_backend(&machine("")) {
        assert!(has, "{language}: a machine of fields alone has a save API");
    }
}

#[test]
fn a_delayed_send_to_this_session_is_generated_with_the_save_api() {
    // The saved state holds each of these as the moment it comes due: an event
    // for the external queue, one for the internal queue (`#_internal`), an act
    // a host-served processor performs, and the `<cancel>` that names any of
    // them.
    for extra in [
        r#"<onentry><send event="go" delay="5s"/></onentry>"#,
        r#"<onentry><send id="timer" event="go" delay="5s"/></onentry>"#,
        r##"<onentry><send event="go" target="#_internal" delay="5s"/></onentry>"##,
        r#"<onentry><send event="go" delayexpr="'5s'"/></onentry>"#,
        r##"<onentry><send event="go" target="#_internal" delayexpr="'5s'"/></onentry>"##,
        r#"<onexit><cancel sendid="timer"/></onexit>"#,
        r#"<onentry><send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
                           target="http://localhost:1/notify" event="go" delay="5s"/></onentry>"#,
    ] {
        for (language, has) in save_api_per_backend(&machine(extra)) {
            assert!(
                has,
                "{language}: a send the saved state holds must not take the save API away: {extra}"
            );
        }
    }
}

#[test]
fn a_delayed_send_to_another_session_is_generated_without_it() {
    // A `delayexpr` is a delay: the time it computes is known only when the send
    // runs, and may be one that waits.
    for target in ["#_parent", "#_scxml_child_session"] {
        for delay in [r#"delay="5s""#, r#"delayexpr="'5s'""#] {
            let extra =
                format!(r#"<onentry><send event="go" target="{target}" {delay}/></onentry>"#);
            for (language, has) in save_api_per_backend(&machine(&extra)) {
                assert!(
                    !has,
                    "{language}: a send waiting on a session the saved state does not carry is \
                     not in it, so the machine must not offer to save one ({target}, {delay})"
                );
            }
        }
    }
}

#[test]
fn a_send_to_another_session_that_is_not_delayed_is_generated_with_it() {
    // Delivered at once, it leaves nothing in the machine to lose.
    let extra = r##"<onentry><send event="go" target="#_parent"/></onentry>"##;
    for (language, has) in save_api_per_backend(&machine(extra)) {
        assert!(
            has,
            "{language}: nothing is waiting, so there is nothing to lose"
        );
    }
}

#[test]
fn a_static_child_session_is_generated_with_the_save_api() {
    // A saved state names the child that is running and a restore starts it
    // again from its beginning, so a machine whose only invoke is a static
    // child session saves.
    let extra = r#"<invoke type="scxml" id="child">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="f"><final id="f"/></scxml>
      </content>
    </invoke>"#;
    for (language, has) in save_api_per_backend(&machine(extra)) {
        assert!(
            has,
            "{language}: a running child session is in a saved state, so the machine saves it"
        );
    }
}

#[test]
fn a_hybrid_invoke_is_generated_with_the_save_api() {
    // A hybrid invoke names its child by a value (`srcexpr`) among the candidates
    // it declares. A saved state names the invocation and holds the fields the
    // value is read from, so a restore starts it again and the restored fields
    // name the candidate: the machine saves.
    let parent = include_str!("fixtures/static_datamodel/static_invoke_hybrid_saved.scxml");
    let watcher = include_str!("fixtures/static_datamodel/static_hybrid_watcher.scxml");
    let holder = include_str!("fixtures/static_datamodel/static_hybrid_holder.scxml");
    for (language, has) in save_api_per_backend_beside(
        parent,
        &[],
        &[
            ("static_hybrid_watcher.scxml", watcher),
            ("static_hybrid_holder.scxml", holder),
        ],
    ) {
        assert!(
            has,
            "{language}: a running hybrid child is in a saved state, so the machine saves it"
        );
    }
}

/// The shape a generated Rust machine binds its saved state to.
fn rust_shape(text: &str) -> String {
    const MARKER: &str = "const SHAPE: &'static str = \"";
    let start = text.find(MARKER).expect("a saved shape in the machine") + MARKER.len();
    let len = text[start..].find('"').expect("the shape ends");
    text[start..start + len].to_string()
}

#[test]
fn a_hybrid_invoke_is_part_of_the_shape_a_saved_state_is_bound_to() {
    // A saved state names the invocation a restore starts again, by its id: a
    // document that renamed it is one a saved state must refuse, so the hybrid
    // invoke is in the shape.
    let parent = include_str!("fixtures/static_datamodel/static_invoke_hybrid_saved.scxml");
    let watcher = include_str!("fixtures/static_datamodel/static_hybrid_watcher.scxml");
    let holder = include_str!("fixtures/static_datamodel/static_hybrid_holder.scxml");
    let siblings = [
        ("static_hybrid_watcher.scxml", watcher),
        ("static_hybrid_holder.scxml", holder),
    ];
    let shape_of = |document: &str| {
        let (ok, text) = generate_beside("rust", document, &[], &siblings);
        assert!(ok, "the machine does not generate:\n{text}");
        rust_shape(&text)
    };
    let renamed = parent
        .replace(r#"id="watch""#, r#"id="watching""#)
        .replace("done.invoke.watch", "done.invoke.watching");
    assert_ne!(renamed, parent, "the rename found the invocation");
    assert_ne!(
        shape_of(&renamed),
        shape_of(parent),
        "the id is what a saved state names the invocation by"
    );
}

#[test]
fn a_host_run_invocation_a_declared_invoker_serves_is_generated_with_the_save_api() {
    // The saved state holds the request the host was handed and the deadline it
    // had left, and a restore starts the invocation again from them.
    let extra = r#"<invoke type="x-sce-host" id="job"/>"#;
    for (language, has) in
        save_api_per_backend_declaring(&machine(extra), &["--host-invoker", "x-sce-host"])
    {
        assert!(
            has,
            "{language}: a host-run invocation is in a saved state, so the machine saves it"
        );
    }
}

#[test]
fn an_invoke_no_host_serves_is_generated_with_the_save_api() {
    // Its start is refused with `error.execution`, so nothing is ever running:
    // a saved state has nothing to say of it, and there is nothing to lose.
    let extra = r#"<invoke type="x-sce-host" id="job"/>"#;
    for (language, has) in save_api_per_backend(&machine(extra)) {
        assert!(
            has,
            "{language}: a refused invoke runs nothing, so it takes nothing from the saved state"
        );
    }
}

#[test]
fn a_mesh_request_is_generated_with_the_save_api() {
    // A Mesh request may already have reached its peer, and starting it again
    // would have the peer act on it twice, so a restore does not start it: the
    // saved state holds it as a call that was in flight, and the document is told
    // on the first macrostep that it was interrupted. Nothing is lost by saving.
    let mesh = r##"<invoke type="sce:mesh-rpc" id="ask" src="#peer">
      <param name="_mesh_event" expr="'service.request'"/>
    </invoke>"##;
    let child = r#"<invoke type="scxml" id="child">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="f"><final id="f"/></scxml>
      </content>
    </invoke>"#;
    for extra in [mesh.to_string(), format!("{child}{mesh}")] {
        for (language, has) in save_api_per_backend(&machine(&extra)) {
            assert!(
                has,
                "{language}: a Mesh request is in a saved state as an interrupted call, so the \
                 machine saves it: {extra}"
            );
        }
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
    for (language, has) in save_api_per_backend(with_history) {
        assert!(has, "{language}: a machine with a history saves it");
    }
}
