// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A generated C++ machine whose only `<invoke>` is a remote
//! `<invoke type="scxml" src="#peer">` (SCE_MESH.md §9.6) has no pending-invoke
//! queue and no drain for it.
//!
//! # What was wrong
//!
//! A remote invoke starts from onentry and never defers, so nothing fills
//! `pendingInvokes_` in such a machine. The machine still declared the queue
//! and emitted `executePendingInvokes(Engine& engine)`, a drain whose every
//! arm belongs to an invoke that defers (local, hybrid, unsupported type):
//! with none of them the body never reads `engine`, and GCC refused the
//! parameter under `-Werror=unused-parameter` in the Mesh ctest build of
//! `tests/mesh/parent_session_f.scxml`.
//!
//! # What is held
//!
//! The document is the shape of `tests/mesh/parent_session_f.scxml` and its
//! deploy, written here so the test reads no tracked file:
//!
//! * with only the remote invoke, the machine declares no queue and emits no
//!   drain, while it still emits the remote invoke's own machinery
//!   (`failScxmlRemoteInvokesForPeer`), so the deploy did mark it remote;
//! * the control: the same document with an `<invoke>` of a type nobody
//!   declared beside it, which defers, declares the queue and emits the drain.

use std::path::Path;
use std::process::Command;

const REMOTE_ONLY: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="null" initial="waiting" name="parent_session_f">
    <state id="waiting">
        <invoke type="scxml" src="#worker_session_f" id="remote_inv"/>
        <transition event="error.execution" target="pass"/>
    </state>
    <final id="pass"/>
</scxml>
"##;

/// The same document with an invoke that defers: a type no host declared,
/// which is started (and refused) at the end of the macrostep.
const REMOTE_AND_DEFERRED: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="null" initial="waiting" name="parent_session_f">
    <state id="waiting">
        <invoke type="scxml" src="#worker_session_f" id="remote_inv"/>
        <invoke type="x-sce-host" id="host_inv"/>
        <transition event="error.execution" target="pass"/>
    </state>
    <final id="pass"/>
</scxml>
"##;

const WORKER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="null" initial="idle" name="worker_session_f">
    <state id="idle"/>
</scxml>
"#;

/// The parent and its peer as distinct mesh machines on one device, which
/// marks the parent's `src="#worker_session_f"` remote.
const DEPLOY: &str = r#"version: "1.0"
topology:
  ecu1:
    machines:
      parent_session_f:
        source: parent_session_f.scxml
      worker_session_f:
        source: worker_session_f.scxml
"#;

/// What the drain is emitted as, unformatted.
const DRAIN: &str = "void executePendingInvokes(Engine& engine)";

/// What the queue is declared as.
const QUEUE: &str = "std::vector<PendingInvoke> pendingInvokes_;";

/// The remote invoke's own machinery, which no deferring invoke emits.
const REMOTE_MACHINERY: &str = "void failScxmlRemoteInvokesForPeer(";

/// The generated `_sm.h` and `_sm.inl` of `parent` staged beside its peer and
/// the deploy and generated with the deploy, as the CMake rule generates it.
fn generate(parent: &str) -> (String, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = |name: &str| dir.path().join(name);
    std::fs::write(path("parent_session_f.scxml"), parent).expect("write the parent");
    std::fs::write(path("worker_session_f.scxml"), WORKER).expect("write the peer");
    std::fs::write(path("deploy.yaml"), DEPLOY).expect("write the deploy");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("generate")
        .arg(path("parent_session_f.scxml"))
        .args(["-l", "cpp", "--no-format", "-o"])
        .arg(dir.path())
        .arg("--deploy")
        .arg(path("deploy.yaml"))
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "generate of the parent failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    (
        read(&path("parent_session_f_sm.h")),
        read(&path("parent_session_f_sm.inl")),
    )
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn a_machine_whose_invokes_are_all_remote_declares_no_queue_and_no_drain() {
    let (header, inl) = generate(REMOTE_ONLY);
    assert!(
        inl.contains(REMOTE_MACHINERY),
        "the parent emitted no remote invoke machinery, so the deploy did not mark \
         its invoke remote and the rest of this test proves nothing"
    );
    assert!(
        !inl.contains(DRAIN),
        "a machine whose only invoke is remote emitted a pending-invoke drain, \
         which nothing fills and whose body never reads `engine`"
    );
    assert!(
        !header.contains(QUEUE),
        "a machine whose only invoke is remote declared a pending-invoke queue \
         that no invoke defers to"
    );
}

#[test]
fn a_machine_with_an_invoke_that_defers_declares_the_queue_and_its_drain() {
    let (header, inl) = generate(REMOTE_AND_DEFERRED);
    assert!(
        inl.contains(DRAIN),
        "an invoke that defers was given no drain, so it would never start"
    );
    assert!(
        header.contains(QUEUE),
        "an invoke that defers was given no queue to defer to"
    );
}
