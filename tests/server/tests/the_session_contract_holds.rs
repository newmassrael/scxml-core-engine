// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The contract of a keyed pool's session machine (docs/adr/0112), held by its one kit
//! (`sce_rust_mesh::session_conformance`) for every machine the suite serves: the ledger, written
//! by hand, and both versions of the upgraded document, generated and served by the library's
//! adapter (`sce_rust_mesh::served`). The next version is held to restoring a state the first
//! saved through the migration it declares (docs/adr/0109, D11).

use sce_rust_mesh::peers::Machine;
use sce_rust_mesh::served::Served;
use sce_rust_mesh::session_conformance::{self as kit, Request, Subject, T0};
use sce_rust_runtime::saved_state::{PersistentPolicy, SavedState};
use sce_rust_runtime::{Engine, SceClock};
use sce_server_suite::ledger::{self, Held, Ledgers, PIPE};
use sce_server_suite::machines::generated::server_upgrade_1_sm::ServerUpgrade1Policy;
use sce_server_suite::machines::generated::server_upgrade_2_sm::ServerUpgrade2Policy;
use sce_server_suite::upgrade::{self, key_data, take_data, Notes, UpgradeHost};

/// A state of the ledger's shape, which no version of the upgraded document restores.
fn ledger_state(key: &str) -> SavedState {
    Held {
        taken: vec!["a".to_string()],
        ..Held::default()
    }
    .to_state(key)
}

/// The ledger.
struct LedgerSubject;

impl Subject for LedgerSubject {
    type Factory = Ledgers;

    fn machine(&self) -> Machine {
        ledger::machine(PIPE, 600_000)
    }

    fn factory(&self) -> Ledgers {
        Ledgers::default()
    }

    fn steps(&self, key: &str) -> Vec<Request> {
        (0..4)
            .map(|n| Request {
                event: ledger::TAKE.to_string(),
                data: ledger::take_data(key, &format!("t{n}"), &[T0 + 60_000]),
            })
            .collect()
    }

    fn ending(&self, _key: &str) -> Vec<Request> {
        Vec::new()
    }

    fn greeting(&self) -> Option<String> {
        None
    }

    fn foreign(&self, key: &str) -> SavedState {
        SavedState {
            shape: "another document".to_string(),
            ..ledger_state(key)
        }
    }

    fn earlier(&self, _key: &str) -> Option<SavedState> {
        None
    }
}

/// A version of the upgraded document, served by the library's adapter.
struct Upgraded<P> {
    earlier: fn(&str) -> Option<SavedState>,
    policy: std::marker::PhantomData<fn() -> P>,
}

fn upgraded<P>(earlier: fn(&str) -> Option<SavedState>) -> Upgraded<P> {
    Upgraded {
        earlier,
        policy: std::marker::PhantomData,
    }
}

impl<P> Subject for Upgraded<P>
where
    P: PersistentPolicy,
    Engine<P>: Send,
    UpgradeHost: sce_rust_mesh::served::SessionHost<P>,
{
    type Factory = Served<P, UpgradeHost>;

    fn machine(&self) -> Machine {
        upgrade::machine(PIPE, 600_000)
    }

    fn factory(&self) -> Served<P, UpgradeHost> {
        Served::new(UpgradeHost::new(&Notes::read()))
    }

    fn steps(&self, key: &str) -> Vec<Request> {
        let mut steps: Vec<Request> = (0..3)
            .map(|n| Request {
                event: upgrade::TAKE.to_string(),
                data: take_data(key, &format!("t{n}")),
            })
            .collect();
        steps.push(Request {
            event: upgrade::NOTE.to_string(),
            data: upgrade::object_data(key, &Notes::read().signed),
        });
        steps.push(Request {
            event: upgrade::READ.to_string(),
            data: key_data(key),
        });
        steps
    }

    fn ending(&self, key: &str) -> Vec<Request> {
        vec![
            Request {
                event: upgrade::TAKE.to_string(),
                data: take_data(key, "t"),
            },
            Request {
                event: upgrade::CLOSE.to_string(),
                data: key_data(key),
            },
            Request {
                event: upgrade::END.to_string(),
                data: key_data(key),
            },
        ]
    }

    fn greeting(&self) -> Option<String> {
        Some("opened".to_string())
    }

    fn foreign(&self, key: &str) -> SavedState {
        ledger_state(key)
    }

    fn earlier(&self, key: &str) -> Option<SavedState> {
        (self.earlier)(key)
    }
}

/// A state the first version saved for `key`: a session that took a tag, standing in `active`
/// with its reminder waiting, as a server of the first build leaves it at its stop.
fn saved_by_the_first_version(key: &str) -> Option<SavedState> {
    let mut engine = Engine::new(ServerUpgrade1Policy::new());
    engine.set_clock(SceClock::Manual(T0));
    // What the machine sends goes nowhere: only its state is wanted.
    engine.register_mesh_router(|_| Vec::new());
    let Ok(()) = engine.initialize();
    engine.raise_external_by_name(upgrade::TAKE, &take_data(key, "before"));
    engine.tick();
    let saved = ServerUpgrade1Policy::save_at(&engine, T0).expect("a running machine saves");
    Some(SavedState::from_json(&saved.to_json()).expect("a saved state reads back"))
}

fn no_earlier(_key: &str) -> Option<SavedState> {
    None
}

macro_rules! the_kit {
    ($module:ident, $subject:expr) => {
        mod $module {
            use super::*;

            #[test]
            fn a_restored_session_runs_on_as_one_never_saved() {
                kit::a_restored_session_runs_on_as_one_never_saved(&$subject);
            }

            #[test]
            fn a_restore_runs_no_initial_content() {
                kit::a_restore_runs_no_initial_content(&$subject);
            }

            #[test]
            fn a_finished_session_leaves_no_checkpoint() {
                kit::a_finished_session_leaves_no_checkpoint(&$subject);
            }

            #[test]
            fn a_state_of_another_document_is_refused_and_left_as_it_is() {
                kit::a_state_of_another_document_is_refused_and_left_as_it_is(&$subject);
            }

            #[test]
            fn a_state_an_earlier_version_saved_is_carried_into_this_one() {
                kit::a_state_an_earlier_version_saved_is_carried_into_this_one(&$subject);
            }
        }
    };
}

the_kit!(the_ledger, LedgerSubject);
the_kit!(
    the_first_version,
    upgraded::<ServerUpgrade1Policy>(no_earlier)
);
the_kit!(
    the_next_version,
    upgraded::<ServerUpgrade2Policy>(saved_by_the_first_version)
);
