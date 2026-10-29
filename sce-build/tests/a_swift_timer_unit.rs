// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// A timer (`sce:kind="timer"`, a period, a fire event and two optional hooks), written for Swift:
// a `final class` that starts the runtime's `ForgeTimer` clock at the document's period, a handler
// protocol the state class implements, and the hooks for the reset event and the state exit.
// The handler is held weakly, so a state class that owns the timer is no cycle.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Swift source `document` (a path under the repository) is written as, or the
/// refusal it is answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

#[test]
fn a_timer_is_a_class_over_the_runtimes_clock_and_a_handler_protocol() {
    let code =
        swift_of("tests/forge/resources/timer_diag_scheduler.scxml").expect("a timer is written");
    assert!(
        code.contains("public protocol TimerDiagSchedulerHandler: AnyObject {"),
        "{code}"
    );
    assert!(code.contains("func fireDiagTick()"), "{code}");
    assert!(
        code.contains("public static let periodMs: Int64 = 2000"),
        "the period is the document's: {code}"
    );
    assert!(
        code.contains("private let timer: SceForgeRuntime.ForgeTimer"),
        "the clock is the runtime's, named by its module: {code}"
    );
}

#[test]
fn the_handler_is_held_weakly_so_that_its_owner_is_no_cycle() {
    let code =
        swift_of("tests/forge/resources/timer_diag_scheduler.scxml").expect("a timer is written");
    assert!(
        code.contains("private weak var handler: (TimerDiagSchedulerHandler)?"),
        "{code}"
    );
    assert!(
        code.contains("[weak handler = self.handler]"),
        "the callback holds the handler and not the timer: {code}"
    );
}

#[test]
fn the_hooks_are_those_the_document_names() {
    let code =
        swift_of("tests/forge/resources/timer_diag_scheduler.scxml").expect("a timer is written");
    assert!(
        code.contains("public func onResetDiagHeartbeat() {"),
        "{code}"
    );
    assert!(
        code.contains("public func onCancelDiagIdleExit() {"),
        "{code}"
    );
}
