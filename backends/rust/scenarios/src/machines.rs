// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The table that says which committed machine ([`crate::integration`]) replays
//! which scenario.

use crate::replay;
use sce_rust_runtime::{Engine, SceClock};

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
macro_rules! scenarios {
    ($($name:literal => $module:ident::{$policy:ident, $persist:ident} $(@ $manual:ident)?;)*) => {
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
                    use crate::integration::static_datamodel::$module::{$persist, $policy};
                    #[allow(unused_mut)]
                    let mut engine = Engine::new($policy::new());
                    $(engine.set_clock(SceClock::$manual(0));)?
                    replay(
                        engine,
                        |engine| engine.save_at(0).expect("saves"),
                        scenario_text($name).expect("the table names its own scenario"),
                    );
                })*
                other => panic!("`{other}` is not a scenario of this table"),
            }
        }
    };
}

scenarios! {
    "static_block_ends" => static_block_ends_sm::{StaticBlockEndsPolicy, StaticBlockEndsPersist};
    "static_block_ends_list" => static_block_ends_list_sm::{StaticBlockEndsListPolicy, StaticBlockEndsListPersist};
    "static_bytes" => static_bytes_sm::{StaticBytesPolicy, StaticBytesPersist};
    "static_bytes_wire" => static_bytes_wire_sm::{StaticBytesWirePolicy, StaticBytesWirePersist};
    "static_cancel_expr" => static_cancel_expr_sm::{StaticCancelExprPolicy, StaticCancelExprPersist} @ Manual;
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
    "static_invoke" => static_invoke_sm::{StaticInvokePolicy, StaticInvokePersist};
    "static_invoke_abort" => static_invoke_sm::{StaticInvokePolicy, StaticInvokePersist};
    "static_list" => static_list_sm::{StaticListPolicy, StaticListPersist};
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
    "static_record_real" => static_record_real_sm::{StaticRecordRealPolicy, StaticRecordRealPersist};
    "static_record_real32" => static_record_real32_sm::{StaticRecordReal32Policy, StaticRecordReal32Persist};
    "static_record_string" => static_record_string_sm::{StaticRecordStringPolicy, StaticRecordStringPersist};
    "static_send_content" => static_send_content_sm::{StaticSendContentPolicy, StaticSendContentPersist};
    "static_send_delay" => static_send_delay_sm::{StaticSendDelayPolicy, StaticSendDelayPersist} @ Manual;
    "static_send_event" => static_send_event_sm::{StaticSendEventPolicy, StaticSendEventPersist};
    "static_send_idlocation" => static_send_idlocation_sm::{StaticSendIdlocationPolicy, StaticSendIdlocationPersist} @ Manual;
    "static_send_namelist" => static_send_namelist_sm::{StaticSendNamelistPolicy, StaticSendNamelistPersist};
    "static_send_params" => static_send_params_sm::{StaticSendParamsPolicy, StaticSendParamsPersist};
    "static_send_target" => static_send_target_sm::{StaticSendTargetPolicy, StaticSendTargetPersist};
    "static_send_type" => static_send_type_sm::{StaticSendTypePolicy, StaticSendTypePersist};
    "static_string_capacity" => static_string_capacity_sm::{StaticStringCapacityPolicy, StaticStringCapacityPersist};
    "static_timers" => static_timers_sm::{StaticTimersPolicy, StaticTimersPersist} @ Manual;
    "static_timers_stop" => static_timers_sm::{StaticTimersPolicy, StaticTimersPersist} @ Manual;
    "static_whole_payload" => static_whole_payload_sm::{StaticWholePayloadPolicy, StaticWholePayloadPersist};
    "static_wire_enum" => static_wire_enum_sm::{StaticWireEnumPolicy, StaticWireEnumPersist};
    "sync_client" => sync_client_sm::{SyncClientPolicy, SyncClientPersist};
}
