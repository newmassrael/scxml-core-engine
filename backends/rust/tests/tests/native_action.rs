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
/// Asserted on the host's record AND on the configuration, because the two
/// answer different halves: the record says the host was not handed a value it
/// would take for data, and `faulted` says the machine said so instead of
/// swallowing it. §scxml-3.12.2 is what makes the second half a contract rather
/// than a nicety — `error.execution` covers errors "arising from expression
/// evaluation", the processor MUST place it on the internal event queue, and
/// this document answers it.
///
/// The refusal and the proof that the site is live are asserted on ONE engine:
/// the untyped delivery leaves the host untouched, then the SAME machine takes
/// the SAME transition with a typed inject and does call it. An empty record
/// is therefore the effect of the missing payload rather than the effect of a
/// machine that never reached the call site — a check whose only discriminator
/// is an absence reads a broken machine as a pass.
///
/// No `cfg` gate, and that is the point. Until 2026-08-24 this backend lowered
/// the untyped arm to a `debug_assert!`: the same delivery aborted a
/// development build and did nothing at all in `--release`, so no single test
/// could state the contract and the two profiles needed a partition between
/// them. One lowering for six backends replaced it, and one test in one shape
/// covering both profiles is what proves that divergence is gone.
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

    assert!(
        log.borrow().appended.is_empty(),
        "append_fragment_payload fired without a typed payload to read: {:?}",
        log.borrow().appended
    );
    assert_eq!(
        engine.get_current_state(),
        StatechartNativeActionState::Faulted,
        "the unreadable argument must reach the document as error.execution"
    );
    assert_eq!(
        log.borrow().idle_entries,
        1,
        "the no-argument entry effect is unaffected by the missing payload"
    );
    assert_eq!(
        log.borrow().assembling_exits,
        1,
        "answering error.execution leaves assembling, so its <onexit> runs"
    );

    // Lower bound, on the same engine: recover to `idle` and deliver the event
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
        engine.get_current_state(),
        StatechartNativeActionState::Assembling
    );
    assert_eq!(
        log.borrow().appended,
        vec![(b"abc".to_vec(), 7)],
        "the same action site must fire once the typed payload is present"
    );
}

/// The same arm, reached with NO host mistake at all.
///
/// `<raise event="fragment.received"/>` is legal SCXML this generator accepts,
/// and a raise carries no typed payload — so the document alone can put the
/// arg-bearing action in front of a delivery it cannot read. Measured on
/// 2026-08-24, that document killed a development build of this backend and
/// silently did nothing in `--release`; a generator cannot answer a document it
/// accepted by aborting the process.
///
/// The host's only act here is delivering `selftest`. Everything after it is
/// the document's own doing, which is what separates this from
/// `native_action_does_not_fire_without_its_typed_payload` — same arm, an
/// origin the host cannot be blamed for, and the same three facts must hold.
#[test]
fn native_action_answers_a_document_raised_event_with_error_execution() {
    let log = Rc::new(RefCell::new(Log::default()));
    let mut engine =
        sce_rust_runtime::Engine::new(StatechartNativeActionPolicy::new(Recorder(log.clone())));
    engine.initialize();

    engine.raise_external_by_name("selftest", "");
    engine.step();

    assert!(
        log.borrow().appended.is_empty(),
        "a document-raised fragment.received has no payload to hand the host: {:?}",
        log.borrow().appended
    );
    assert_eq!(
        engine.get_current_state(),
        StatechartNativeActionState::Faulted,
        "the document's own raise must come back to it as error.execution"
    );
    assert_eq!(
        log.borrow().idle_entries,
        1,
        "the entry effect ran once and the machine never returned to idle"
    );
}
