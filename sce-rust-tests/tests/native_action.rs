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
// the event's payload variant; `reset_slot` takes no arguments. The host
// records every call through a shared `Rc<RefCell<…>>` so the test can assert
// on them after driving the machine.

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

    // Per-event typed inject (extension trait): deliver `fragment.received`
    // with a bytes payload + offset. The transition fires `append_fragment_payload`.
    engine.raise_fragment_received(StatechartNativeActionFragmentReceivedPayload {
        payload: b"abc".to_vec(),
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
    engine.raise_external_by_name("reset", "");
    engine.step();

    assert_eq!(
        engine.get_current_state(),
        StatechartNativeActionState::Idle
    );
    assert_eq!(log.borrow().resets, 1, "reset_slot must have fired once");
}
