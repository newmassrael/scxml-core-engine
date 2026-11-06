# ADR 0090 step 6 — saved-state format 3: the change on each backend

**Temporary.** This file is a working list for the round that moves every backend to format 3. Delete it in the
commit that lands the last backend. It is not a decision record. The decision and the byte form are in
`docs/adr/0090-an-event-origin-is-the-reply-address-its-processor-writes-and-one-value-holds-both-fields.md`,
section 7 and its "Text form". When this list and the ADR disagree, the ADR is right.

## What every backend does

1. **The format constant.** Add format 3 beside formats 1 and 2, and make it the format the backend writes. Every
   test of the form `format == 2` that means "a tree of sessions" must also admit 3. The grammar of the tree is the
   same in both formats, and only the origin members differ. Keep the format-2 shape constants: format 3 uses the
   format-2 shape, and no generated tree moves because of this step.
2. **The writer.** In every element of `external`, write `origin` (the pair, processor then address, or `null`) and
   then `sender` (the id, or `null`), in the place of `origin` and `origintype`. In every `raise`, `parent` and
   `child` act, write the session's SCXML pair and its id after `sendid`. In every `internal` act, write
   `"origin":null,"sender":null`. A `host` act, a `hostinvokes` entry and an `invokes` entry carry neither member.
   The SCXML address is `scxmlLocation(id)`, the existing codec.
3. **The reader.**
   - Read format 3 as written, deriving neither member from the other.
   - Refuse each case the ADR lists:
     - an origin that is neither `null` nor a pair;
     - a member missing, empty or added;
     - a sender beside an origin other than its SCXML pair;
     - a session's act with no sender;
     - an `internal` act that holds either member.
   - Read formats 1 and 2 through the stored-origin reader you already have (`readStoredOrigin` and
     `readStoredSender`, or the port). A format-1 or format-2 `internal` act is read as no origin and no sender.
   - Refuse any other format with this reason:
     `the saved state is format <as written>, and this runtime reads formats 1, 2 and 3`.
4. **Writing a state that was read.** A state read from format 2 is written back as format 3. A state read from
   format 1 and written without a machine stays format 1, with `origin` and `origintype` written by the format-2
   rule (`storedOriginText` and the pair's processor).
5. **The shared instances.**
   - The texts in `sce-build/tests/fixtures/static_datamodel/saved/` and `sce-build/tests/fixtures/host_processor/saved/`
     are format 3 now. Every save test that compares bytes compares against them as they are.
   - Add one test that restores each text in `saved/format2/`, at the wall-clock moment `1700000000000` that the
     shared fixtures use, with a machine of `static_invoke_tree` or `static_timers`. Save at once, then compare the
     result byte for byte with `saved/format2/read_as/<same name>`.
   - `saved/format2/README.md` lists what each record must be read as, if a test asks for a pair directly.
6. **Tests that use format 3 as "a format of the future".** Move them to `{"format":4}` and the new reason.
7. **Status lockstep.** Where a test reads the schema headers to match the status constant, add
   `schemas/sce-saved-state.v3.schema.json` to the files it reads.
8. **Mutation casefiles.**
   - If you edit text that a casefile quotes, run `scripts/mutate --check <casefile>` and re-aim the anchor.
   - The casefiles named below anchor the files listed for each backend. Run `--check` on each of them, whether or
     not you think the anchor moved.
9. **Inline texts in tests.** Search the backend's tests for `origintype` and for `"origin":"`, and move each saved
   text that a test writes inline to format 3. The shared-fixture tests are not the only place a format-2 text lives.

Search terms that find the rest: `origintype`, `"origin"`, `FORMAT_2` / `SavedFormat2` / `kSavedFormat2` /
`SAVED_FORMAT_2` / `SCE_SAVED_FORMAT_2` / `format2`, `storedOriginText` / `stored_origin_text`, `readStoredOrigin`
/ `read_stored_origin`, `readStoredSender` / `read_stored_sender`, `SavedAct`, `SavedEvent`, `Until step 6`.

## Rust

- Format constants and status: `backends/rust/runtime/src/saved_state.rs`, `FORMAT_1`, `FORMAT_2`, `FORMAT`,
  `SCHEMA_STATUS`. Also `SavedState::format`, `bound_session_id`, `members`, the `FORMAT_2` gates in
  `SavedSend::from_value`, and the refusal reason in `SavedState::from_json`.
- Writer: `saved_state.rs`.
  - `SavedEvent` (`origin` and `origin_type`, `FIELDS`, `to_value`);
  - the `origin` of each `SavedAct` variant and `SavedSend::to_value`;
  - the queue save that calls `stored_origin_text`;
  - `save_pending`, which stores `sender` as `origin`.
- Reader: `saved_state.rs`.
  - `SavedEvent::from_value` and `SavedSend::from_value`;
  - the restore of `external` that calls `read_stored_origin`;
  - `ReadSend` and `arm_pending`, which call `read_stored_sender`.
- Codec: `backends/rust/runtime/src/helpers/io_processors.rs` (`stored_origin_text`, `StoredOrigin`,
  `read_stored_origin`, `read_stored_sender`, and their tests).
- Unit tests in `saved_state.rs`:
  - `schema_file_declares_status`: add the v3 `include_str!`;
  - `another_format_is_refused`: it uses `{"format":3}`;
  - the inline texts with `origin` near the tree and refusal tests.
- Tests that read the shared instances: `backends/rust/tests/tests/` `static_datamodel.rs`,
  `a_saved_tree_holds_every_session_where_it_stood.rs`, `a_saved_child_session_is_resumed_after_a_restore.rs`
  (inline `origintype`), `a_generated_send_id_survives_a_restore.rs`,
  `a_saved_machine_carries_on_the_send_ids_it_generated.rs`, `a_saved_host_invocation_is_started_again_after_a_restore.rs`,
  `a_saved_mesh_request_is_not_sent_again_after_a_restore.rs`, `a_generated_machines_waiting_host_send_survives_a_restore.rs`.
- Casefiles on these files: `a_saved_single_is_the_double_it_widens_to.cases`,
  `a_static_rust_byte_string_is_held_within_its_bound_rust.cases`,
  `a_static_rust_record_bytes_field_is_held_within_its_bound_rust.cases`,
  `a_static_rust_record_string_field_is_held_within_its_bound_rust.cases`.
- Templates: none writes an origin. `rust/state_machine.rs.jinja2` holds only the shape constants.

## C++ (generated engine) and the Interpreter

- The Interpreter does not save (ADR 0045, section 7). It needs no change for this step.
- Format constants and status: `sce/include/saved/SavedState.h`, `kSavedFormat1`, `kSavedFormat2`, `kSavedFormat`,
  `kSavedSchemaStatus`, the `kSavedFormat2` gates, and the refusal reason near the format check.
- Writer and reader of the records: `SavedState.h`.
  - the event struct (`origin` and `originType`) and the act struct (`origin`);
  - the `members.set("origin"...)` and `"origintype"` writers;
  - the `memberText(... "origin")` and `"origintype"` readers, and the act key lists.
- Engine save and restore: `sce/include/saved/SavedEngine.h`.
  - the queue save (`storedOriginText`, `originProcessorOf`);
  - the act save (`act.origin = sessionId`, five places);
  - the queue restore (`readStoredOrigin`);
  - the act restore (`readStoredSender`, three places).
- Codec: `sce/include/common/IOProcessorHelper.h` (`storedOriginText`, `StoredOrigin`, `readStoredOrigin`,
  `readStoredSender`). Its cites are ledger-bound: keep the `§` tokens on the symbols you keep.
- Tests:
  - `tests/saved/SavedStateTest.cpp`: the sweep of `saved/`, and the `{"format":3}` refusal row;
  - `tests/integration/AStateAnotherBackendSavedIsRestoredTest.cpp`, `ASavedTreeHoldsEverySessionWhereItStoodTest.cpp`,
    `ASavedChildSessionIsResumedAfterARestoreTest.cpp`, `ASavedHostInvocationIsStartedAgainAfterARestoreTest.cpp`,
    `ASavedWaitingSendSurvivesARestoreTest.cpp`;
  - the fixture wiring in `tests/CMakeLists.txt`.
- Casefile: `a_saved_cpp_state_is_restored_as_the_run_it_was_ctest.cases` (`SavedEngine.h`).
- Templates: none writes an origin. `state_machine.jinja2` holds only the shape constants. `sce/include` is an
  embed-manifest input: the merge regenerates `embed/MANIFEST.*`.

## Go

- Format constants and status: `backends/go/runtime/saved_state.go`, `SavedFormat1`, `SavedFormat2`, `SavedFormat`,
  `SavedSchemaStatus`. Also `Format()`, the `SavedFormat2` gates, and the refusal reason in `SavedStateFromJSON`.
- Writer and reader of the records: `saved_state.go`.
  - the event members `origin` and `origintype`, written and read;
  - the act `origin`, written and read;
  - the act key lists near `"event", "data", "sendid", "origin"`.
- Engine save and restore: `backends/go/runtime/engine_saved.go`.
  - the queue save (`StoredOriginText`);
  - the act save (`Origin: act.Sender`, four places, the `internal` one among them);
  - the queue restore (`ReadStoredOrigin`);
  - the act restore (`ReadStoredSender`).
- Codec: `backends/go/runtime/io_processors.go` (`StoredOriginText`, `ReadStoredOrigin`, `ReadStoredSender`).
- Tests:
  - `backends/go/runtime/saved_state_test.go`: `sharedSavedTexts`, and the `{"format":3}` "format of the future"
    row;
  - `backends/go/tests/integration/static_datamodel/` `a_state_another_backend_saved_is_restored_test.go`,
    `a_saved_tree_holds_every_session_where_it_stood_test.go`, `a_saved_child_session_is_resumed_after_a_restore_test.go`
    (inline `origintype`), `a_generated_send_id_survives_a_restore_test.go`;
  - the three `statechart_static_*` restore tests.
- Casefiles:
  - `a_saved_go_state_is_the_text_every_backend_writes_go.cases`. It quotes `Origin: act.Sender` on the `raise` and
    `internal` acts and `ScheduleEventAt(..., sender)`, so the `internal` anchor moves with the null rule.
  - `a_saved_go_tree_carries_its_queues_ids_and_routes_go.cases`, `a_saved_go_tree_is_judged_before_anything_is_built_go.cases`,
    `a_saved_go_child_session_is_resumed_where_it_stood_go.cases`, `a_saved_go_child_sessions_host_invocation_is_reached_through_the_tree_go.cases`,
    `a_saved_go_host_invocation_is_started_again_go.cases`, `a_saved_go_host_invocation_is_written_as_it_ran_go.cases`,
    `a_saved_go_mesh_request_is_not_sent_again_go.cases`, `a_saved_go_waiting_host_send_is_carried_with_its_request_go.cases`.
- Templates: none writes an origin. `go/state_machine.go.jinja2` holds only the shape constants.

## Python

- Format constants and status: `backends/python/runtime/sce_runtime/saved_state.py`, `SAVED_FORMAT_1`,
  `SAVED_FORMAT_2`, `SAVED_FORMAT`, `SAVED_SCHEMA_STATUS`. Also `format()`, the `SAVED_FORMAT_2` gates, the refusal
  reason, and the exports in `sce_runtime/__init__.py`.
- Writer and reader of the records: `saved_state.py`.
  - the event `origin` and `origintype`, written and read;
  - the act `origin`, written and read;
  - the act key lists (`["event", "data", "sendid", "origin"]`).
- Engine save and restore: `backends/python/runtime/sce_runtime/saved_engine.py`.
  - the queue save (`stored_origin_text`, `origin_processor_of`);
  - the act save (`origin=session_id`, four places);
  - the queue restore (`read_stored_origin`).
- Codec: `backends/python/runtime/sce_runtime/io_processors.py` (`stored_origin_text`, `read_stored_origin`, and
  the sender reader).
- Tests:
  - `backends/python/tests/saved_state/test_saved_state.py`: the sweep, and the `{"format":3}` row;
  - `backends/python/tests/integration/static_datamodel/test_a_state_another_backend_saved_is_restored.py`,
    `test_a_saved_tree_holds_every_session_where_it_stood.py`, `test_a_saved_child_session_is_resumed_after_a_restore.py`
    (inline `origintype`), `test_a_generated_send_id_survives_a_restore.py`;
  - `backends/python/tests/integration/host_processor/test_a_saved_host_invocation_is_started_again_after_a_restore.py`
    (inline `origintype`), and the two other host restore tests.
- Casefiles:
  - `a_saved_python_state_is_the_text_every_backend_reads_python.cases`, which quotes `if form != SAVED_FORMAT_2:`;
  - `a_saved_python_tree_holds_every_session_where_it_stood_python.cases`,
    `a_saved_python_child_session_is_resumed_where_it_stood_python.cases`,
    `a_saved_python_host_invocation_is_started_again_after_a_restore_python.cases`,
    `a_saved_python_waiting_host_send_survives_a_restore_python.cases`.
- Templates: none writes an origin. `python/saved_functions.py.jinja2` and `python/state_machine.py.jinja2` hold
  only the shape constants.

## Kotlin

- Format constants and status: `backends/kotlin/runtime/src/commonMain/kotlin/com/sce/runtime/SavedState.kt`,
  `FORMAT_1`, `FORMAT_2`, `FORMAT`, `SCHEMA_STATUS`. Also `format`, `boundSessionId`, the `FORMAT_2` gates, and the
  refusal reason.
- Writer and reader of the records: `SavedState.kt`.
  - the event `origin` and `originType` (`"origintype"`), written and read;
  - the `origin` of each `SavedAct`, written and read.
- Engine save and restore: `backends/kotlin/runtime/src/commonMain/kotlin/com/sce/runtime/StateMachineEngine.kt`.
  - the queue save (`storedOriginText`);
  - `pendingSends`: `Raise` and `Internal` call `storedOriginText`, and `Parent` and `Child` write
    `sessionIdentity`. The internal entry keeps a sender that format 3 does not write.
  - the act restore (`readStoredSender`, `EventMetadata.sentBy`; the `Internal` entry is built with a sender);
  - the queue restore (`readStoredOrigin`).
- Codec: `backends/kotlin/runtime/src/commonMain/kotlin/com/sce/runtime/IoProcessors.kt` (`StoredOrigin`,
  `storedOriginText`, `readStoredOrigin`, `readStoredSender`).
- Tests:
  - `backends/kotlin/tests/src/test/kotlin/com/sce/integration/StaticDatamodelTest.kt`. Its
    `theSavedStateSchemaFileDeclaresTheStatusThisRuntimeDoes` gets the v3 path, and its shared-instance tests stay;
  - `ASavedTreeHoldsEverySessionWhereItStoodTest.kt`, `ASavedChildSessionIsResumedAfterARestoreTest.kt` (inline
    `origintype`), `AGeneratedSendIdSurvivesARestoreTest.kt`, `ASavedMachineCarriesOnTheSendIdsItGeneratedTest.kt`,
    `ASavedHostInvocationIsStartedAgainAfterARestoreTest.kt`, `ASavedMeshRequestIsNotSentAgainAfterARestoreTest.kt`,
    `AGeneratedMachinesWaitingHostSendSurvivesARestoreTest.kt`.
- Casefiles: `a_static_kotlin_byte_string_is_held_within_its_bound_kotlin.cases`,
  `a_static_kotlin_record_string_field_is_held_within_its_bound_kotlin.cases` (`SavedState.kt`),
  `a_static_kotlin_hybrid_invoke_is_started_again_by_a_restore_kotlin.cases`.
- Templates: none writes an origin. `kotlin/state_machine.kt.jinja2` holds only the shape constants.

## Swift

- Format constants and status: `backends/swift/runtime/Sources/SceRuntime/SavedState.swift`, `format1`, `format2`,
  `format`, `schemaStatus`. Also the `format2` gates and the refusal reason in `fromJSON`.
- Writer and reader of the records: `SavedState.swift`.
  - the event `origin` and `originType`, written and read;
  - the `origin` of `SavedAct` (`.raise`, `.internal`, `.parent`, `.child`), written and read.
- Engine save and restore:
  - `backends/swift/runtime/Sources/SceRuntime/SavedMachine.swift`: the queue save (`storedOriginText`) and the
    queue restore (`readStoredOrigin`);
  - `backends/swift/runtime/Sources/SceRuntime/Engine.swift`: `savedPendingSends` (`origin: act.sender`, the
    `.internalQueue` case among them) and the act restore (`readStoredSender`).
- Codec: `backends/swift/runtime/Sources/SceRuntime/IoProcessors.swift` (`storedOriginText`, `readStoredOrigin`,
  `readStoredSender`).
- Tests:
  - `backends/swift/runtime/Tests/SceRuntimeTests/SavedStateTests.swift`: the sweep, and the `{"format":3}`
    refusal;
  - `backends/swift/tests/Tests/StaticDatamodelTests/SavedTreeTests.swift`, `SavedInvokeTests.swift`,
    `SavedSendTests.swift`, `SavedCompositeTests.swift`, `SavedEnumTests.swift`, `SavedHistoryTests.swift`,
    `SavedRevisionTests.swift`, and `SavedStateMachineTests.swift` (an inline `"format":2` text).
- Casefiles: none anchors a Swift saved-state file today.
- Templates: none writes an origin. `swift/saved_state.swift.jinja2` holds only the shape constants.

## C11

- Format constants and status: `backends/c/runtime/include/sce/saved_state.h`, `SCE_SAVED_FORMAT_1`,
  `SCE_SAVED_FORMAT_2`, `SCE_SAVED_FORMAT`, `SCE_SAVED_SCHEMA_STATUS`. Also the `SCE_SAVED_FORMAT_2` gates and the
  refusal reason in the format check of `sce_saved_read`.
- Writer and reader of the records: `saved_state.h`.
  - the send reader (`"origin"`), the event reader (`sce_saved_event_read`, `"origin"`, `"origintype"`) and the
    `sce_saved_event_t` and act structs;
  - `sce_saved_origin_text` and `sce_saved_origin_read`;
  - the act writer (`sce_saved_write_member_text(w, "origin", send->origin)`) and the event writer
    (`sce_saved_write_event`, `sce_saved_event_out_t`).
  - The pair is an object in format 3. `sce_saved_write_member_text` is for texts, so the writer needs a small pair
    writer and a `null`.
- Generated save and restore: `tools/codegen/templates/c/saved.c.jinja2`. This is the one template that writes an
  origin.
  - the `external` restore (`sce_saved_origin_read`, with the `SCE_MAX_URI_LEN` and `SCE_MAX_ID_LEN` buffers);
  - the act save (`send.origin = sm->session_id`);
  - the queue save (`sce_saved_origin_text`, `sce_event_origin_processor`);
  - the `SCE_SAVED_FORMAT_2` gates of the tree.
  - The act restore reads `origin` into the send struct but does not use it today. A format-3 reader must still
    judge both members as the ADR lists.
  - The comment in `tools/codegen/templates/c/state_machine.h.jinja2` near `_init` says "format 2".
  - Every template edit moves the C11 generated trees: the merge regenerates them.
- Codec: `backends/c/runtime/include/sce/types.h` and the event-origin helpers that `sce_saved_origin_read` calls
  (`sce_event_origin_scxml`, `sce_event_origin_set`, `sce_session_id_from_scxml_location`,
  `SCE_SCXML_LOCATION_PREFIX`).
- Tests:
  - `backends/c/tests/unit/saved_state_test.c`: `check_directory` asserts `session.format == format`, and the
    `{"format":3}` refusal;
  - `backends/c/tests/unit/saved_json_test.c`: the sweep of `saved/`. Its `TEXT_CAP` is 16384, and the largest
    reading fixture is about 3.4 KB;
  - `backends/c/tests/unit/event_origin_test.c`: the stored-origin table rows;
  - `backends/c/tests/integration/test_a_state_another_backend_saved_is_restored.c` (inline `origintype`, and the
    `format 9` reason), `test_a_saved_child_session_is_resumed_after_a_restore.c` (inline `origintype`),
    `test_a_saved_tree_holds_every_session_where_it_stood.c`,
    `test_a_saved_host_invocation_is_started_again_after_a_restore.c`,
    `test_a_saved_mesh_request_is_not_sent_again_after_a_restore.c`,
    `test_a_generated_machines_waiting_host_send_survives_a_restore.c`;
  - the `SCE_SAVED_FIXTURES_DIR` wiring in `backends/c/tests/CMakeLists.txt`.
- Casefiles: `a_saved_c11_child_session_is_resumed_where_it_stood_ctest.cases`,
  `a_saved_c11_host_work_is_restored_as_the_call_it_was_ctest.cases` (`saved.c.jinja2`).

## Amendment (2026-11-06): the sender rule

ADR 0090 section 1 and the section 7 "Text form" are amended. These items override the lists above where they
disagree.

1. **`sender` and `origin` are independent.** `sender` is the id of the session that sent the event. The SCXML
   processor writes it for a local session, and the Mesh transport for a remote child or parent (§mesh-9.6.3).
   The address beside it is the reply address that processor wrote: a local location, or a session endpoint
   `mesh://<device>/<machine>/<session>`. The C++ remote child is `sender` `<parent machine>:<invokeid>` beside
   `{SCXML type, mesh://<device>/<machine>/<parent machine>%3A<invokeid>}` (`MeshDispatch.h`,
   `meshChildSessionOrigin`).
2. **Queued-event refusals.**
   - Remove "a sender beside an origin other than its SCXML pair".
   - Add "a sender beside a null origin".
   - Keep every pair-shape refusal and the Nmtoken check on `sender`.
   - A reader reads a sender beside any pair as written.
3. **Acts are unchanged.**
   - `raise`, `parent` and `child` carry `scxmlOrigin(sender)` and a sender; anything else is refused.
   - `internal` carries null and null.
4. **Format-2 act with an empty stored origin.** Read a `raise`, `parent` or `child` act that stored an empty
   `origin` as the own send of the session that holds it, by the `sessionid` of its session record. A format-1 act
   with an empty origin is armed as the restoring machine's own send.
5. **Format-2 act stored as the prefix `sce://scxml/` alone** names no session: refuse it. A queued event stored so
   is row 4 of the table, read as `{its type or SCXML, "sce://scxml/"}` with no sender. Go's `ReadStoredSender`
   returns that text as a sender today; it must not.
6. **The writer never refuses** a queued event for its sender and origin. Remove the C++ save refusal near
   `sce/include/saved/SavedEngine.h:327`.
7. **C11 only.** C11 derives a sender from an SCXML location and cannot hold a sender beside another address. It
   runs no Mesh remote child, so it refuses a text with a `sender` beside an address other than
   `scxmlLocation(sender)`, and names that as the reason.
8. **Fixtures.**
   - `saved/format2/static_invoke_tree_working_origins.json` now stores both of its session-to-session acts with an
     empty `origin`. It also holds a tenth root event, `go`, stored as the prefix alone. Its `read_as/` twin gained
     that event's reading; the acts read as before.
   - Restore it and compare as before.
   - Any new reading fixture must be named `static_invoke_tree_working*` or `static_timers*`.

## What the round merges and runs once

- `cargo test -p sce-build --features cli --test saved_state_schema` and `--test wire_surface_stability`: the format
  itself.
- Each backend's saved-state lanes, after every backend has moved. Until then, a backend that still writes format 2
  fails its byte-for-byte save tests against the format-3 shared instances.
- The C11 generated trees and `embed/MANIFEST.*` are regenerated once, after the merge.
