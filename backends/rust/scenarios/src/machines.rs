// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The table that says which committed machine ([`crate::integration`]) replays
//! which scenario.

use crate::hosts::ChildHosts;
use crate::{hosts, replay_with_hosts, Hosts};
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock};

/// The session id every replayed machine is given (docs/adr/0045, "Questions the
/// owner answered" 2). A replay reads the machine through its saved state, which
/// names the session, and `wasm32-unknown-unknown` has no source of randomness to
/// draw one from, so its host gives it where it builds the machine, as every host
/// there does (a machine there has no constructor that does not take one).
const REPLAY_SESSION_ID: &str = "replay";

/// The machine's policy as a row builds it: a machine with `<sce:action>`s takes
/// the host that records them, and a machine whose children are given hosts takes
/// the host `hosts::<builder>` makes, which writes each child's into the book.
/// Each is built under [`REPLAY_SESSION_ID`].
macro_rules! policy {
    ($policy:ident) => {
        $policy::new_with_session_id(REPLAY_SESSION_ID)
            .expect("the replay's session id is an Nmtoken")
    };
    ($policy:ident, host $host:ident) => {
        $policy::new_with_session_id(REPLAY_SESSION_ID, $host::default())
            .expect("the replay's session id is an Nmtoken")
    };
    // A machine that states a duty refuses a host that only simulates it, and the
    // recording host simulates, so a replay that drives one with it says so
    // (docs/adr/0038).
    ($policy:ident, simulated $host:ident) => {
        $policy::new_simulated_with_session_id(REPLAY_SESSION_ID, $host::default())
            .expect("the replay's session id is an Nmtoken")
    };
    ($policy:ident, children $builder:ident, $book:ident) => {
        $policy::new_with_session_id(REPLAY_SESSION_ID, hosts::$builder(&$book))
            .expect("the replay's session id is an Nmtoken")
    };
}

/// The clock a row's machine is restored on: the manual one at zero for a row
/// marked `@ Manual`, which the saved state's waiting sends are measured from as
/// they were saved at it, and the engine's own default for every other.
macro_rules! restore_clock {
    () => {
        SceClock::default()
    };
    ($manual:ident) => {
        SceClock::$manual(0)
    };
}

/// A row's replay: with the reader of what its host recorded, or of what its
/// children's hosts did, or without; and with `$restore`, which builds the
/// machine again from a saved state for a step that restores (docs/adr/0045).
macro_rules! replay_row {
    ($engine:ident, $restore:expr, $text:expr) => {
        replay_with_hosts(
            $engine,
            |engine| engine.save_at(0).expect("saves"),
            Hosts::none().restorer($restore),
            $text,
        )
    };
    ($engine:ident, $restore:expr, $text:expr, calls $calls:ident) => {{
        let mut seen = 0usize;
        replay_with_hosts(
            $engine,
            |engine| engine.save_at(0).expect("saves"),
            Hosts::none()
                .host_calls(|engine| hosts::$calls(engine, &mut seen))
                .restorer($restore),
            $text,
        )
    }};
    // The builder is named by the row so that the repetition that carries the book
    // is the one that carries it; the policy was built with it already.
    ($engine:ident, $restore:expr, $text:expr, children $book:ident $builder:ident) => {
        replay_with_hosts(
            $engine,
            |engine| engine.save_at(0).expect("saves"),
            Hosts::none()
                .child_hosts(|| $book.take())
                .restorer($restore),
            $text,
        )
    };
}

/// One row per scenario: the file it is read from, and the machine that replays
/// it. A machine may serve several scenarios (`static_counter_bound` replays the
/// machine `static_counter` does), which is why this is a table and not a rule on
/// the names.
///
/// A row marked `@ Manual` is replayed on a manual clock at zero, which the
/// scenario's `advance_ms` steps move on: a scenario of a delayed send waits the
/// time it names and not the time the replay happened to take. Every other row
/// keeps the clock the engine starts on, and that is the point of leaving it
/// alone: on `wasm32-unknown-unknown` the engine's own default is host-owned
/// time, and a replay that set a clock for every engine would never use it.
///
/// A row marked `+ <Host> => <reader>` is a machine with `<sce:action>`s: it takes
/// the host of that name, which records what it is asked, and `hosts::<reader>`
/// turns that into the calls a scenario's `host_calls` states.
///
/// A row marked `~ <Host> => <reader>` is the same for a machine that states a duty
/// (docs/adr/0038): the host records and so only simulates, and the machine is built
/// through `new_simulated` to say that it means to run on one.
///
/// A row marked `& <builder>` is a machine whose children are given hosts: its
/// policy takes the host `hosts::<builder>` makes of the book the row replays with,
/// and the scenario's `child_hosts` are read from that book.
///
/// What a machine sends its Mesh peers needs no mark: the replay records it for
/// every machine, and a scenario's `sent` is held to what was recorded.
macro_rules! scenarios {
    ($($name:literal => $module:ident::{$policy:ident, $persist:ident} $(@ $manual:ident)? $(+ $host:ident => $calls:ident)? $(~ $shost:ident => $scalls:ident)? $(& $builder:ident)?;)*) => {
        /// The scenarios this crate can replay, by file name.
        pub const NAMES: &[&str] = &[$($name),*];

        /// The text of the scenario `name`, from the file it is committed in.
        pub fn scenario_text(name: &str) -> Option<&'static str> {
            match name {
                $($name => Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../sce-build/tests/fixtures/static_datamodel/scenarios/",
                    $name,
                    ".json"
                ))),)*
                _ => None,
            }
        }

        /// Replay the scenario `name` against its machine. Panics, with the step
        /// and what differed, where the machine does not do what the scenario
        /// holds it to; and for a name that is not a scenario of this table.
        pub fn run(name: &str) {
            match name {
                $($name => {
                    use crate::integration::static_datamodel::$module::{$persist, $policy $(, $host)? $(, $shost)?};
                    #[allow(unused_variables)]
                    let book = ChildHosts::default();
                    #[allow(unused_mut)]
                    let mut engine = Engine::new(policy!($policy $(, host $host)? $(, simulated $shost)? $(, children $builder, book)?));
                    $(engine.set_clock(SceClock::$manual(0));)?
                    // docs/adr/0045: the machine a step that restores takes in
                    // place of the one it saved, built as this one was.
                    let restore = |saved: &SavedState| {
                        <Engine<_> as $persist>::restore_with(
                            policy!($policy $(, host $host)? $(, simulated $shost)? $(, children $builder, book)?),
                            saved,
                            restore_clock!($($manual)?),
                            0,
                        )
                        .expect("the saved machine restores")
                    };
                    replay_row!(
                        engine,
                        restore,
                        scenario_text($name).expect("the table names its own scenario")
                        $(, calls $calls)?
                        $(, calls $scalls)?
                        $(, children book $builder)?
                    );
                })*
                other => panic!("`{other}` is not a scenario of this table"),
            }
        }
    };
}

scenarios! {
    "presence_peer_alice" => presence_peer_sm::{PresencePeerPolicy, PresencePeerPersist} @ Manual;
    "presence_peer_bob" => presence_peer_sm::{PresencePeerPolicy, PresencePeerPersist} @ Manual;
    "presence_peer_server" => presence_peer_sm::{PresencePeerPolicy, PresencePeerPersist} @ Manual;
    "record_list_alone" => record_list_sm::{RecordListPolicy, RecordListPersist};
    "record_list_change" => record_list_sm::{RecordListPolicy, RecordListPersist};
    "record_list_hub" => record_list_sm::{RecordListPolicy, RecordListPersist};
    "record_list_offline" => record_list_sm::{RecordListPolicy, RecordListPersist};
    "record_list_parent" => record_list_parent_sm::{RecordListParentPolicy, RecordListParentPersist};
    "record_list_person" => record_list_sm::{RecordListPolicy, RecordListPersist};
    "record_list_retire" => record_list_sm::{RecordListPolicy, RecordListPersist};
    "sheet_peer_alice" => sheet_peer_sm::{SheetPeerPolicy, SheetPeerPersist};
    "sheet_peer_bob" => sheet_peer_sm::{SheetPeerPolicy, SheetPeerPersist};
    "sheet_peer_server" => sheet_peer_sm::{SheetPeerPolicy, SheetPeerPersist};
    "static_block_ends" => static_block_ends_sm::{StaticBlockEndsPolicy, StaticBlockEndsPersist};
    "static_block_ends_list" => static_block_ends_list_sm::{StaticBlockEndsListPolicy, StaticBlockEndsListPersist};
    "static_bytes" => static_bytes_sm::{StaticBytesPolicy, StaticBytesPersist};
    "static_bytes_wire" => static_bytes_wire_sm::{StaticBytesWirePolicy, StaticBytesWirePersist};
    "static_cancel_expr" => static_cancel_expr_sm::{StaticCancelExprPolicy, StaticCancelExprPersist} @ Manual;
    "static_child_host" => static_child_host_sm::{StaticChildHostPolicy, StaticChildHostPersist} & static_child_host;
    "static_child_host_hybrid" => static_child_host_hybrid_sm::{StaticChildHostHybridPolicy, StaticChildHostHybridPersist} & static_child_host_hybrid;
    "static_child_mesh" => static_child_mesh_sm::{StaticChildMeshPolicy, StaticChildMeshPersist};
    "static_counter" => static_counter_sm::{StaticCounterPolicy, StaticCounterPersist};
    "static_counter_bound" => static_counter_sm::{StaticCounterPolicy, StaticCounterPersist};
    "static_donedata" => static_donedata_sm::{StaticDonedataPolicy, StaticDonedataPersist};
    "static_donedata_content" => static_donedata_content_sm::{StaticDonedataContentPolicy, StaticDonedataContentPersist};
    "static_donedata_content_lost" => static_donedata_content_sm::{StaticDonedataContentPolicy, StaticDonedataContentPersist};
    "static_donedata_content_text" => static_donedata_content_sm::{StaticDonedataContentPolicy, StaticDonedataContentPersist};
    "static_donedata_content_value" => static_donedata_content_sm::{StaticDonedataContentPolicy, StaticDonedataContentPersist};
    "static_donedata_record" => static_donedata_record_sm::{StaticDonedataRecordPolicy, StaticDonedataRecordPersist};
    "static_enum" => static_enum_sm::{StaticEnumPolicy, StaticEnumPersist};
    "static_event_arrival" => static_event_arrival_sm::{StaticEventArrivalPolicy, StaticEventArrivalPersist};
    "static_event_wildcard" => static_event_wildcard_sm::{StaticEventWildcardPolicy, StaticEventWildcardPersist};
    "static_foreach" => static_foreach_sm::{StaticForeachPolicy, StaticForeachPersist};
    "static_history" => static_history_sm::{StaticHistoryPolicy, StaticHistoryPersist};
    "static_host_call" => static_host_call_sm::{StaticHostCallPolicy, StaticHostCallPersist} + RecordingStaticHostCallActions => static_host_call;
    "static_host_duty" => static_host_duty_sm::{StaticHostDutyPolicy, StaticHostDutyPersist} ~ RecordingStaticHostDutyActions => static_host_duty;
    "static_invoke" => static_invoke_sm::{StaticInvokePolicy, StaticInvokePersist};
    "static_invoke_abort" => static_invoke_sm::{StaticInvokePolicy, StaticInvokePersist};
    "static_invoke_params" => static_invoke_params_sm::{StaticInvokeParamsPolicy, StaticInvokeParamsPersist};
    "static_invoke_string" => static_invoke_string_sm::{StaticInvokeStringPolicy, StaticInvokeStringPersist};
    "static_invoke_tree" => static_invoke_tree_sm::{StaticInvokeTreePolicy, StaticInvokeTreePersist} @ Manual;
    "static_list" => static_list_sm::{StaticListPolicy, StaticListPersist};
    "static_list_assign" => static_list_assign_sm::{StaticListAssignPolicy, StaticListAssignPersist};
    "static_list_index" => static_list_index_sm::{StaticListIndexPolicy, StaticListIndexPersist};
    "static_overflow" => static_overflow_sm::{StaticOverflowPolicy, StaticOverflowPersist};
    "static_payload" => static_payload_sm::{StaticPayloadPolicy, StaticPayloadPersist};
    "static_payload_bytes" => static_payload_bytes_sm::{StaticPayloadBytesPolicy, StaticPayloadBytesPersist};
    "static_payload_enum" => static_payload_enum_sm::{StaticPayloadEnumPolicy, StaticPayloadEnumPersist};
    "static_payload_relay" => static_payload_relay_sm::{StaticPayloadRelayPolicy, StaticPayloadRelayPersist};
    "static_real" => static_real_sm::{StaticRealPolicy, StaticRealPersist};
    "static_real32" => static_real32_sm::{StaticReal32Policy, StaticReal32Persist};
    "static_record" => static_record_sm::{StaticRecordPolicy, StaticRecordPersist};
    "static_record_bytes" => static_record_bytes_sm::{StaticRecordBytesPolicy, StaticRecordBytesPersist};
    "static_record_enum" => static_record_enum_sm::{StaticRecordEnumPolicy, StaticRecordEnumPersist};
    "static_record_fields" => static_record_fields_sm::{StaticRecordFieldsPolicy, StaticRecordFieldsPersist};
    "static_record_list" => static_record_list_sm::{StaticRecordListPolicy, StaticRecordListPersist};
    "static_record_list_index" => static_record_list_index_sm::{StaticRecordListIndexPolicy, StaticRecordListIndexPersist};
    "static_record_real" => static_record_real_sm::{StaticRecordRealPolicy, StaticRecordRealPersist};
    "static_record_real32" => static_record_real32_sm::{StaticRecordReal32Policy, StaticRecordReal32Persist};
    "static_record_string" => static_record_string_sm::{StaticRecordStringPolicy, StaticRecordStringPersist};
    "static_schema_revision" => static_schema_revision_sm::{StaticSchemaRevisionPolicy, StaticSchemaRevisionPersist};
    "static_send_content" => static_send_content_sm::{StaticSendContentPolicy, StaticSendContentPersist};
    "static_send_delay" => static_send_delay_sm::{StaticSendDelayPolicy, StaticSendDelayPersist} @ Manual;
    "static_send_event" => static_send_event_sm::{StaticSendEventPolicy, StaticSendEventPersist};
    "static_send_idlocation" => static_send_idlocation_sm::{StaticSendIdlocationPolicy, StaticSendIdlocationPersist} @ Manual;
    "static_send_namelist" => static_send_namelist_sm::{StaticSendNamelistPolicy, StaticSendNamelistPersist};
    "static_send_params" => static_send_params_sm::{StaticSendParamsPolicy, StaticSendParamsPersist};
    "static_send_target" => static_send_target_sm::{StaticSendTargetPolicy, StaticSendTargetPersist};
    "static_send_type" => static_send_type_sm::{StaticSendTypePolicy, StaticSendTypePersist};
    "static_state_algorithm" => static_state_algorithm_sm::{StaticStateAlgorithmPolicy, StaticStateAlgorithmPersist};
    "static_state_algorithm_enum" => static_state_algorithm_enum_sm::{StaticStateAlgorithmEnumPolicy, StaticStateAlgorithmEnumPersist};
    "static_string_capacity" => static_string_capacity_sm::{StaticStringCapacityPolicy, StaticStringCapacityPersist};
    "static_text_record" => static_text_record_sm::{StaticTextRecordPolicy, StaticTextRecordPersist};
    "static_timers" => static_timers_sm::{StaticTimersPolicy, StaticTimersPersist} @ Manual;
    "static_timers_stop" => static_timers_sm::{StaticTimersPolicy, StaticTimersPersist} @ Manual;
    "static_whole_payload" => static_whole_payload_sm::{StaticWholePayloadPolicy, StaticWholePayloadPersist};
    "static_wire_enum" => static_wire_enum_sm::{StaticWireEnumPolicy, StaticWireEnumPersist};
    "static_wire_wide" => static_wire_wide_sm::{StaticWireWidePolicy, StaticWireWidePersist};
    "sync_client" => sync_client_sm::{SyncClientPolicy, SyncClientPersist};
    "todo_peer_alice" => todo_peer_sm::{TodoPeerPolicy, TodoPeerPersist};
    "todo_peer_bob" => todo_peer_sm::{TodoPeerPolicy, TodoPeerPersist};
    "todo_peer_server" => todo_peer_sm::{TodoPeerPolicy, TodoPeerPersist};
}
