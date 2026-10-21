// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Format 2: a tree of sessions" (docs/adr/0045,
// section 8): the tree fixture saved byte for byte, restored from that text, and
// refused for a grandchild of another shape and a child of a later revision.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml.
// The shared text, sce-build/tests/fixtures/static_datamodel/saved/
// static_invoke_tree_working.json, is the tree `working` leaves:
//
// - `worker` was started twice (`rest`, `back`), and has taken `go`: it counted
//   a step, holds a delayed `report` to its parent, asks its host for the time
//   (a host-run invocation that has not answered), and runs a grandchild under
//   the hybrid candidate `static_hybrid_watcher`, whose `pick` was changed to the
//   other candidate after it started (`swap`); it keeps the id the grandchild was
//   started under in `helper_id` (`idlocation`).
// - The parent holds a delayed `nudge` to the child (`arm`) and the child's
//   `answer` in its queue; the child holds the parent's `swap` in its own.
//
// The text is the one every backend saves of that run. Rust is the first to
// write it, so it is the run's own save: with `UPDATE_EXPECT=1` this test writes
// it, and without, it holds the run to it.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sce_rust_runtime::json::Value;
use sce_rust_runtime::saved_state::{SavedAct, SavedInvokes, SavedState, StateRefusal};
use sce_rust_runtime::{Engine, HostInvokeEvent, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_tree_sm::{
    StaticInvokeTreePersist, StaticInvokeTreePolicy,
};

/// The session id the shared format-2 fixtures give their root.
const ROOT: &str = "fixture-root";

/// The wall-clock moment the tree is saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// The standard host type the child asks.
const CLOCK: &str = "sce-host-clock-now";

fn shared_text_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../../sce-build/tests/fixtures/static_datamodel/saved/static_invoke_tree_working.json",
    )
}

/// The run the shared text is the save of. The host serves the clock and never
/// answers, so the invocation is running when the tree is saved.
fn working() -> Engine<StaticInvokeTreePolicy> {
    let mut engine =
        Engine::new(StaticInvokeTreePolicy::new_with_session_id(ROOT).expect("an Nmtoken"));
    engine.set_clock(SceClock::Manual(0));
    let asked = Arc::new(Mutex::new(0usize));
    let seen = Arc::clone(&asked);
    engine.register_invoker(CLOCK, move |event| {
        if let HostInvokeEvent::Start(_) = event {
            *seen.lock().unwrap() += 1;
        }
        None
    });
    engine.initialize();
    // Started twice: left and entered again.
    for event in ["rest", "back"] {
        engine.raise_external_by_name(event, "");
        engine.step();
    }
    // `go`, forwarded in one tick and taken by the child in the next.
    engine.raise_external_by_name("go", "");
    engine.tick();
    engine.tick();
    assert_eq!(*asked.lock().unwrap(), 1, "the child asked its host once");
    // `swap`, forwarded and taken: `pick` names the other candidate now.
    engine.raise_external_by_name("swap", "");
    engine.tick();
    engine.tick();
    // A delayed send to the child.
    engine.raise_external_by_name("arm", "");
    engine.step();
    // `ask`, then `swap`, taken two to a call: the child answers `question` at
    // once, behind the host's `swap`, and the call ends after the parent takes
    // `swap`, which it forwards to the child, so the child's answer waits in the
    // parent's queue and the parent's `swap` in the child's.
    engine.set_max_external_events_per_call(NonZeroU32::new(2).expect("two"));
    engine.raise_external_by_name("ask", "");
    engine.raise_external_by_name("swap", "");
    engine.step();
    engine
}

fn saved(engine: &Engine<StaticInvokeTreePolicy>) -> SavedState {
    engine
        .save_at(SAVED_AT_MS)
        .unwrap_or_else(|refusal| panic!("the tree saves: {refusal}"))
}

fn restored(saved: &SavedState) -> Result<Engine<StaticInvokeTreePolicy>, StateRefusal> {
    Engine::<StaticInvokeTreePolicy>::restore_with(
        StaticInvokeTreePolicy::new(),
        saved,
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
}

fn variable<'s>(session: &'s SavedState, id: &str) -> &'s Value {
    session
        .variables
        .iter()
        .find(|(name, _)| name == id)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("the session holds `{id}`"))
}

fn children(session: &SavedState) -> &[sce_rust_runtime::saved_state::SavedChild] {
    match &session.invokes {
        SavedInvokes::Resumed(children) => children,
        SavedInvokes::Restarted(_) => panic!("a format-2 session"),
    }
}

#[test]
fn the_tree_holds_every_session_where_it_stood() {
    let root = saved(&working());
    assert_eq!(root.session_id, ROOT);
    assert_eq!(root.current, "working");
    assert_eq!(
        root.invoke_starts,
        vec![("worker".to_string(), 2)],
        "the worker was started twice"
    );
    let queued: Vec<&str> = root.external.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        queued,
        ["answer"],
        "the child's answer waits in the parent's queue"
    );
    let to_child: Vec<&str> = root
        .pending
        .iter()
        .filter_map(|send| match &send.act {
            SavedAct::Child { event, .. } => Some(event.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        to_child,
        ["nudge"],
        "the parent holds a delayed send to the child"
    );

    let [worker] = children(&root) else {
        panic!("one child runs");
    };
    assert_eq!(worker.id, "worker");
    assert_eq!(worker.start, 2);
    assert_eq!(worker.invoke_id, "working.2.worker");
    let child = &worker.session;
    assert_eq!(child.session_id, "fixture-root.working.2.worker");
    assert_eq!(child.current, "second", "the child took `go`");
    assert_eq!(variable(child, "steps"), &Value::Number("1".to_string()));
    assert_eq!(
        variable(child, "pick"),
        &Value::Text("static_hybrid_holder.scxml".to_string()),
        "`pick` was changed after the grandchild started"
    );
    let queued: Vec<&str> = child.external.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        queued,
        ["swap"],
        "the parent's `swap` waits in the child's queue"
    );
    let to_parent: Vec<&str> = child
        .pending
        .iter()
        .filter_map(|send| match &send.act {
            SavedAct::Parent { event, .. } => Some(event.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        to_parent,
        ["report"],
        "the child holds a delayed send to its parent"
    );
    let hosted: Vec<(&str, &str)> = child
        .host_invokes
        .iter()
        .map(|host| (host.processor_type.as_str(), host.invoke_id.as_str()))
        .collect();
    assert_eq!(
        hosted,
        [(CLOCK, "clock")],
        "the child's host-run invocation runs"
    );

    let [helper] = children(child) else {
        panic!("one grandchild runs");
    };
    assert_eq!(
        helper.document, "static_hybrid_watcher",
        "the candidate that was started, whatever `pick` names now"
    );
    assert_eq!(helper.start, 1);
    assert!(
        helper.invoke_id.starts_with("second.1."),
        "{}",
        helper.invoke_id
    );
    assert_eq!(
        variable(child, "helper_id"),
        &Value::Text(helper.invoke_id.clone()),
        "the idlocation holds the id the grandchild was started under"
    );
}

#[test]
fn the_tree_saves_the_shared_text_byte_for_byte() {
    let text = saved(&working()).to_json();
    let path = shared_text_path();
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        std::fs::write(&path, format!("{text}\n")).expect("the shared text is writable");
        return;
    }
    let shared = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing: the first backend to save the tree writes it, with \
             `UPDATE_EXPECT=1 cargo test -p sce-rust-tests --test \
             a_saved_tree_holds_every_session_where_it_stood`",
            path.display()
        )
    });
    assert_eq!(text, shared.trim_end(), "the tree's text is the shared one");
}

#[test]
fn the_shared_text_restores_and_saves_again_as_it_was() {
    let text = saved(&working()).to_json();
    let engine = restored(&SavedState::from_json(&text).expect("reads"))
        .unwrap_or_else(|refusal| panic!("the tree restores: {refusal}"));
    assert_eq!(
        saved(&engine).to_json(),
        text,
        "a restored tree stands where the saved one stood"
    );
}

#[test]
fn a_grandchild_of_another_shape_refuses_the_tree_by_its_path() {
    let mut tree = saved(&working());
    let SavedInvokes::Resumed(children) = &mut tree.invokes else {
        panic!("a format-2 tree");
    };
    let SavedInvokes::Resumed(grandchildren) = &mut children[0].session.invokes else {
        panic!("a format-2 child");
    };
    grandchildren[0].session.shape = "0".repeat(64);
    let refusal = restored(&tree)
        .err()
        .expect("a grandchild of another shape");
    assert!(
        refusal
            .reason()
            .contains("invokes[0].session.invokes[0].session"),
        "the refusal names the grandchild's path: {refusal}"
    );
    assert!(refusal.reason().contains("shape"), "{refusal}");
}

#[test]
fn a_child_of_a_later_revision_refuses_the_tree_by_its_path() {
    // The child holds a record of `schema_revised.scxml`, saved at its third
    // revision; a fourth this build does not know is refused, naming the child.
    let text = saved(&working()).to_json();
    assert!(
        text.contains(r#""revision":3"#),
        "the child names the revision of its record: {text}"
    );
    let later = text.replace(r#""revision":3"#, r#""revision":4"#);
    let refusal = restored(&SavedState::from_json(&later).expect("reads"))
        .err()
        .expect("a child of a later revision");
    assert!(
        refusal.reason().contains("invokes[0].session"),
        "the refusal names the child's path: {refusal}"
    );
    assert!(refusal.reason().contains("later than"), "{refusal}");
}
