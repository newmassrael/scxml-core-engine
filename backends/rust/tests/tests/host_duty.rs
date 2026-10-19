// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A machine that states a duty (docs/adr/0038), as a Rust host meets it.
//!
//! `static_host_duty` takes the duty `start_keepalive` on entering `engaged` and
//! gives it back, through `released-by`, with `stop_keepalive` on that state's
//! `<onexit>`; `note_engaged` is a notice. Its host trait declares a
//! `REALIZATION` that has no default, and `Policy::new` evaluates it where the
//! machine is built with a host type: a host that only simulates the duties does
//! not compile there, so the refusal itself is a compile error and is held by
//! `sce-build/tests/a_host_duty_is_refused_where_the_machine_is_built.rs`, which
//! reads the generated source. What runs here is the other half: the host that
//! performs is taken, the generated recorder says it simulates, and the one way to
//! run the machine on a recorder is the entry that says so.

use sce_rust_runtime::{ActionRealization, Engine};
use sce_rust_tests::integration::static_datamodel::static_host_duty_sm::{
    RecordingStaticHostDutyActions, StaticHostDutyActions, StaticHostDutyActionsCall,
    StaticHostDutyPolicy,
};

/// A host that does what the machine's duties name: it counts, which stands for
/// starting and stopping a real keepalive.
#[derive(Default)]
struct KeepaliveHost {
    started: u32,
    stopped: u32,
    noted: u32,
}

impl StaticHostDutyActions for KeepaliveHost {
    const REALIZATION: ActionRealization = ActionRealization::Performs;

    fn note_engaged(&mut self) {
        self.noted += 1;
    }

    fn start_keepalive(&mut self) {
        self.started += 1;
    }

    fn stop_keepalive(&mut self) {
        self.stopped += 1;
    }
}

#[test]
fn a_host_that_performs_is_taken_by_new_and_given_the_duty_and_its_release() {
    let mut engine = Engine::new(StaticHostDutyPolicy::new(KeepaliveHost::default()));
    engine.initialize();
    assert_eq!(engine.policy().actions().started, 0);

    engine.raise_external_by_name("engage", "");
    engine.step();
    assert_eq!(
        engine.policy().actions().started,
        1,
        "entering takes the duty"
    );
    assert_eq!(engine.policy().actions().noted, 1, "and gives the notice");
    assert_eq!(engine.policy().actions().stopped, 0);

    engine.raise_external_by_name("release", "");
    engine.step();
    assert_eq!(
        engine.policy().actions().stopped,
        1,
        "leaving the state gives the duty back"
    );
}

#[test]
fn the_generated_recorder_says_it_simulates() {
    assert_eq!(
        <RecordingStaticHostDutyActions as StaticHostDutyActions>::REALIZATION,
        ActionRealization::Simulates
    );
    assert!(
        !<RecordingStaticHostDutyActions as StaticHostDutyActions>::REALIZATION.is_performing()
    );
    assert!(<KeepaliveHost as StaticHostDutyActions>::REALIZATION.is_performing());
}

#[test]
fn a_recorder_runs_the_machine_only_through_the_entry_that_says_so() {
    let mut engine = Engine::new(StaticHostDutyPolicy::new_simulated(
        RecordingStaticHostDutyActions::default(),
    ));
    engine.initialize();
    engine.raise_external_by_name("engage", "");
    engine.step();
    engine.raise_external_by_name("release", "");
    engine.step();
    assert_eq!(
        engine.policy().actions().calls(),
        &[
            StaticHostDutyActionsCall::StartKeepalive,
            StaticHostDutyActionsCall::NoteEngaged,
            StaticHostDutyActionsCall::StopKeepalive,
        ]
    );
}

#[test]
fn a_host_that_performs_may_also_be_built_through_the_simulated_entry() {
    // The opt-in is permission, not a requirement: it accepts a host that
    // performs as well, so a harness that always builds this way is not wrong
    // for a host that is real.
    let mut engine = Engine::new(StaticHostDutyPolicy::new_simulated(KeepaliveHost::default()));
    engine.initialize();
    engine.raise_external_by_name("engage", "");
    engine.step();
    assert_eq!(engine.policy().actions().started, 1);
}
