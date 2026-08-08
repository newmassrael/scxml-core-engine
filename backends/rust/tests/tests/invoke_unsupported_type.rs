// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1: an `<invoke>` naming an unsupported `type` raises
// `error.execution` — Rust AOT path.
//
// The spec defines the case ("the processor MUST place error.execution in the
// internal event queue"), so the document is valid SCXML with one observable:
// that raise. No child session starts and `done.invoke.<id>` never fires.
//
// Both engines were silent here in different ways before this landed — the
// Interpreter substituted an SCXML handler for the unknown type, and AOT
// dropped the `<invoke>` from the model entirely. A backend that renders this
// fixture without the raise reproduces the AOT form, and the machine then
// rests in `probe` instead of reaching `pass`.
//
// Fixture: integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_invoke_unsupported_type.sh

use std::time::Duration;

use sce_rust_tests::integration::invoke_unsupported_type::{
    InvokeUnsupportedTypePolicy, InvokeUnsupportedTypeState,
};

#[test]
fn an_unsupported_invoke_type_raises_error_execution() {
    let policy = InvokeUnsupportedTypePolicy::new();
    let mut engine = sce_rust_runtime::Engine::new(policy);
    engine.initialize();

    let completed = engine.run_until_completion(Duration::from_secs(2), Duration::from_millis(10));

    assert!(
        completed,
        "the machine never completed (parked in {:?}). W3C SCXML 6.4.1 requires an \
         `<invoke>` whose `type` names no supported processor to place \
         `error.execution` on the internal queue; parking in `probe` means the \
         `<invoke>` was dropped rather than lowered",
        engine.get_current_state()
    );
    assert_eq!(
        engine.get_current_state(),
        InvokeUnsupportedTypeState::Pass,
        "the machine completed somewhere other than the `error.execution` target"
    );
}
