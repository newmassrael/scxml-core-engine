// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML G.7 `<sce:action>` — Rust compile+run gate for native
// host-trait action dispatch.
//
// The committed SM under `src/integration/native_action/` is generated from
// `sce-build/tests/fixtures/event_schema/statechart_native_action.scxml`
// (regen: `scripts/regen_native_action.sh`). Because the tree is part of the
// crate it is REALLY type-checked: the generated `Policy` is generic over a
// host `Actions` impl and carries NO `IScriptEngine`, so this gate proves the
// engine-free dispatch surface compiles AND that the side effects actually
// fire — the runtime behaviour the form/byte-golden layers cannot give.
//
// `append_fragment_payload` reads two typed `_event.data` fields (a `bytes`
// payload lowered to `&[u8]`, a `uint32` offset lowered to `u32`) bound from
// the event's payload variant; `reset_slot` takes no arguments. `on_idle_entry`
// (a no-argument action in `idle`'s `<onentry>`) and `on_assembling_exit` (in
// `assembling`'s `<onexit>`) appear in no transition — they prove the
// engine-free entry/exit dispatch path AND that an eventless-only action still
// gets a generated trait method. The host records every call through a shared
// `Rc<RefCell<…>>` so the test can assert on them after driving the machine.

use std::cell::RefCell;
use std::rc::Rc;

use sce_rust_tests::integration::native_action::{
    StatechartNativeActionActions, StatechartNativeActionFragmentReceivedPayload,
    StatechartNativeActionInject, StatechartNativeActionPolicy, StatechartNativeActionState,
};

#[derive(Default)]
struct Log {
    appended: Vec<(Vec<u8>, u32)>,
    resets: u32,
    idle_entries: u32,
    assembling_exits: u32,
}

/// Host implementation of the generated `<sce:action>` operations. Records
/// each dispatch so the test can assert the engine-free call path fired with
/// the correct typed arguments.
struct Recorder(Rc<RefCell<Log>>);

impl StatechartNativeActionActions for Recorder {
    fn append_fragment_payload(&mut self, payload: &[u8], offset: u32) {
        self.0
            .borrow_mut()
            .appended
            .push((payload.to_vec(), offset));
    }
    fn reset_slot(&mut self) {
        self.0.borrow_mut().resets += 1;
    }
    fn on_idle_entry(&mut self) {
        self.0.borrow_mut().idle_entries += 1;
    }
    fn on_assembling_exit(&mut self) {
        self.0.borrow_mut().assembling_exits += 1;
    }
}

#[test]
fn native_action_dispatches_typed_payload_to_host_trait() {
    let log = Rc::new(RefCell::new(Log::default()));
    let mut engine =
        sce_rust_runtime::Engine::new(StatechartNativeActionPolicy::new(Recorder(log.clone())));
    engine.initialize();
    assert_eq!(
        engine.get_current_state(),
        StatechartNativeActionState::Idle
    );
    // `<onentry>` of the initial state fires on entry — the engine-free
    // entry-effect path, with no transition having to carry the action.
    assert_eq!(
        log.borrow().idle_entries,
        1,
        "on_idle_entry must fire on the initial entry to idle"
    );

    // Per-event typed inject (extension trait): deliver `fragment.received`
    // with a bytes payload + offset. The transition fires `append_fragment_payload`.
    engine.raise_fragment_received(StatechartNativeActionFragmentReceivedPayload {
        // Portable `SceBytes<64>` built via the SSOT ctor — `N` inferred from
        // the field type (no turbofish / hardcoded cap).
        payload: ::sce_rust_runtime::SceBytes::from_slice(b"abc").unwrap(),
        offset: 7,
    });
    engine.step();

    assert_eq!(
        engine.get_current_state(),
        StatechartNativeActionState::Assembling,
        "fragment.received must move idle -> assembling"
    );
    assert_eq!(
        log.borrow().appended,
        vec![(b"abc".to_vec(), 7)],
        "append_fragment_payload must receive the typed _event.data payload + offset natively"
    );

    // `reset` fires the no-argument `reset_slot()` action and returns to idle.
    // Exiting `assembling` fires its `<onexit>` effect; re-entering `idle`
    // fires `<onentry>` a second time.
    engine.raise_external_by_name("reset", "");
    engine.step();

    assert_eq!(
        engine.get_current_state(),
        StatechartNativeActionState::Idle
    );
    assert_eq!(log.borrow().resets, 1, "reset_slot must have fired once");
    assert_eq!(
        log.borrow().assembling_exits,
        1,
        "on_assembling_exit must fire when leaving assembling"
    );
    assert_eq!(
        log.borrow().idle_entries,
        2,
        "on_idle_entry must fire again on re-entry to idle (entry effect, not per-transition)"
    );
}

/// An event raised by NAME carries no typed payload. The transition still
/// fires — the guard is the event name — but the arg-bearing action has
/// nothing to read, and handing the host a value it would take for data is the
/// one outcome this seam must never produce.
///
/// Asserted on the host's record rather than on the configuration, because the
/// machine reaches `assembling` either way — the record is the half a
/// configuration assertion cannot see. That record is also the ONE observable
/// contract all six channels share: each of the other five asserts exactly it
/// against this same document, because five of the six lower the untyped arm
/// to a branch that simply is not taken.
///
/// The refusal and the proof that the site is live are asserted on ONE engine:
/// the untyped delivery leaves the host untouched, then the SAME machine takes
/// the SAME transition with a typed inject and does call it. An empty record
/// is therefore the effect of the missing payload rather than the effect of a
/// machine that never reached the call site — a check whose only discriminator
/// is an absence reads a broken machine as a pass.
///
/// Compiled only where the record is reachable. This backend is the one of the
/// six whose untyped arm is a `debug_assert!`, so a development build stops at
/// the panic before any of this can be observed; there the same delivery is
/// witnessed by `native_action_debug_build_asserts_on_a_missing_typed_payload`
/// instead. The two are a partition, not a pair — each profile builds exactly
/// one of them, and neither profile is left without a witness.
#[cfg(not(debug_assertions))]
#[test]
fn native_action_does_not_fire_without_its_typed_payload() {
    let log = Rc::new(RefCell::new(Log::default()));
    let mut engine =
        sce_rust_runtime::Engine::new(StatechartNativeActionPolicy::new(Recorder(log.clone())));
    engine.initialize();

    // No typed inject: the payload variant stays unset, which is exactly what
    // a host that reached for `raise_external_by_name` would produce.
    engine.raise_external_by_name("fragment.received", "");
    engine.step();

    assert_eq!(
        engine.get_current_state(),
        StatechartNativeActionState::Assembling,
        "an untyped fragment.received still takes the transition - its guard is the event name"
    );
    assert!(
        log.borrow().appended.is_empty(),
        "append_fragment_payload fired without a typed payload to read: {:?}",
        log.borrow().appended
    );
    assert_eq!(
        log.borrow().idle_entries,
        1,
        "the no-argument entry effect is unaffected by the missing payload"
    );

    // Lower bound, on the same engine: return to `idle` and deliver the event
    // through its generated typed inject. The call site the untyped delivery
    // skipped is now reached and fires, so the empty record above measures the
    // missing payload and not an inert machine.
    engine.raise_external_by_name("reset", "");
    engine.step();
    engine.raise_fragment_received(StatechartNativeActionFragmentReceivedPayload {
        payload: ::sce_rust_runtime::SceBytes::from_slice(b"abc").unwrap(),
        offset: 7,
    });
    engine.step();

    assert_eq!(
        log.borrow().appended,
        vec![(b"abc".to_vec(), 7)],
        "the same action site must fire once the typed payload is present"
    );
}

/// Rust is the only one of the six backends that also says so LOUDLY: the arm
/// an untyped delivery takes is a generated `debug_assert!`, so a development
/// build turns the contract violation into a panic while a release build
/// compiles it out and costs an MCU target nothing.
///
/// Gated on `debug_assertions` so it is ABSENT from a release sweep rather
/// than silently passing there. Ungated, it asserts a panic the profile has
/// already removed — a check built on a deficiency, which is why the release
/// lane reported `test did not panic as expected` while the refusal itself was
/// working. It still executes in CI: `rust-workspace-tests.yml` runs
/// `cargo test --workspace` on the default profile precisely so assertions
/// like this one are compiled in, and `hook_ci_parity` fails if either that
/// lane or the hook stage it mirrors moves onto `--release`.
///
/// The expected message is what makes this a witness and not a smoke test: it
/// pins the arm taken to the refusal arm rather than to any other panic on the
/// path. Paired with `native_action_dispatches_typed_payload_to_host_trait`,
/// which is compiled into both profiles, a development build still gets both
/// halves of the seam — typed delivery reaches the host, untyped delivery
/// takes the arm nobody wrote.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "requires the typed payload of its triggering event")]
fn native_action_debug_build_asserts_on_a_missing_typed_payload() {
    let log = Rc::new(RefCell::new(Log::default()));
    let mut engine =
        sce_rust_runtime::Engine::new(StatechartNativeActionPolicy::new(Recorder(log.clone())));
    engine.initialize();

    engine.raise_external_by_name("fragment.received", "");
    engine.step();
}
