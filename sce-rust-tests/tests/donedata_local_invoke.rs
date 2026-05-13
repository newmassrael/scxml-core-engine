// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.5 + 6.3.1 donedata surfacing — Rust AOT local-invoke path.
//
// Closes the W3C IRP coverage gap: no public IRP test exercises
// `<donedata>` on the invoked child's top-level `<final>` combined
// with `done.invoke.<id>._event.data` readback on the parent. Mirrors
// `tests/integration/DonedataLocalInvokeTest.cpp` (C++ Interpreter,
// commits fb8e3c79 + 00f347cb) and
// `sce-kotlin-tests/src/test/kotlin/com/sce/integration/DonedataLocalInvokeTest.kt`
// (Kotlin AOT, 4d284cb4..b070a7ad) for the Rust AOT code path through
// `sce_rust_runtime::helpers::invoke_processing::raise_done_invoke`.
//
// Fixture: sce-rust-tests/fixtures/donedata_local_invoke.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_donedata_local_invoke.sh
//
// The script wraps the mktemp + per-child --as-child workflow so the
// synth-invoke side files (Mesh §9.6.6 rule 1 pins them adjacent to the
// parent input) do not pollute the tracked `sce-rust-tests/fixtures/`
// directory and the `_sm.rs` artefacts land in this dir with mod.rs
// stitched to match.

use std::time::Duration;

use sce_rust_tests::generated::donedata_local_invoke::{
    DonedataLocalInvokePolicy, DonedataLocalInvokeState,
};

#[test]
fn parent_observes_donedata_on_done_invoke() {
    let _ = sce_rust_lua::register();
    let policy = DonedataLocalInvokePolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    // Children are synchronous (single top-level `<final>`), so the parent
    // reaches `pass`/`fail` within a few microsteps. A brief poll loop
    // mirrors the Kotlin harness and guards against any future async
    // microstep scheduling.
    let completed =
        engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));
    assert!(
        completed,
        "donedata_local_invoke timed out before reaching a final state"
    );

    assert_eq!(
        engine.get_current_state(),
        DonedataLocalInvokeState::Pass,
        "parent reached Fail: `_event.data.result == 42` (param branch) or \
         `_event.data == 'hello_content'` (content branch) failed. An empty \
         `_event.data` means `raise_done_invoke` dropped the child's donedata \
         — mirror the C++ / Kotlin AOT `stashDonedataAtFinal` contract: add \
         `Engine::donedata_at_final` + a top-level `<final>` stash branch in \
         `tools/codegen/templates/rust/entry_exit_actions.rs.jinja2` and \
         thread the stashed payload into `EventMetadata.data` on the emitted \
         `done.invoke.<id>`."
    );
}
