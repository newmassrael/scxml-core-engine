// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// NL→IR Item C1 Path A (EventSchema MCU native lowering, RFC §10.4 step 5)
// — Rust compile+run gate, the twin of the C11
// `c11_integration_event_schema_native` test.
//
// The committed SM under `src/integration/event_schema_native/` is
// generated from `sce-build/tests/fixtures/event_schema/statechart_minimal.scxml`
// (regen: `scripts/regen_event_schema_native.sh`). Because this file is
// part of the crate, it is REALLY type-checked — unlike the
// `forge_codegen_event_schema_smoke.rs` gate, which uses `syn::parse_file`
// (syntax only) and therefore cannot catch a Rust semantic error such as
// the orphan rule (an inherent `impl Engine<Policy>` is E0116; the per-event
// inject seam is emitted as an extension trait precisely for this reason).
//
// The transition guard `cond="_event.data.elapsed_ms === 0"` lowers to a
// native `matches!(&self.pending_payload, …)` with NO script engine, so the
// policy is constructed with `Policy::new()` (no `IScriptEngine` argument —
// the MCU-relevant property). The per-event extension-trait inject
// (`raise_job_completed`) binds the event name and payload variant in one
// call.

use sce_rust_tests::integration::event_schema_native::{
    StatechartMinimalInject, StatechartMinimalJobCompletedPayload, StatechartMinimalPolicy,
    StatechartMinimalState,
};

#[test]
fn typed_payload_guard_fires_natively() {
    let mut engine = sce_rust_runtime::Engine::new(StatechartMinimalPolicy::new());
    engine.initialize();
    assert_eq!(
        engine.get_current_state(),
        StatechartMinimalState::Waiting,
        "initial state should be waiting"
    );

    // Per-event typed inject (extension trait). elapsed_ms == 0 satisfies
    // the native typed-payload guard.
    engine.raise_job_completed(StatechartMinimalJobCompletedPayload { elapsed_ms: 0 });
    engine.step();

    assert_eq!(
        engine.get_current_state(),
        StatechartMinimalState::Done,
        "elapsed_ms == 0 must fire the native typed guard to `done`"
    );
}

#[test]
fn typed_payload_guard_misses_on_nonzero() {
    let mut engine = sce_rust_runtime::Engine::new(StatechartMinimalPolicy::new());
    engine.initialize();

    // Same event, a payload the guard rejects — the machine stays put.
    engine.raise_job_completed(StatechartMinimalJobCompletedPayload { elapsed_ms: 5 });
    engine.step();

    assert_eq!(
        engine.get_current_state(),
        StatechartMinimalState::Waiting,
        "elapsed_ms == 5 must leave the machine in `waiting`"
    );
}
