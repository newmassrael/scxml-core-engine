// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime
import Testing

@testable import StaticDatamodelMachines

// The scalar core of `datamodel="sce-static"`: a variable is a property of the policy,
// and guards, `<assign>`, `<if>`, `<elseif>`, `<log>`, `<raise>` and `In()` are native.
//
// Each scenario is replayed by one line, `replay("<scenario>", <machine>())`, which is
// what `every_scenario_is_replayed_on_every_backend_that_lowers_its_machine` reads
// this file for.

// MARK: - The machines, with the variables their scenarios state

private func counter() -> Machine {
    let policy = StaticCounterPolicy()
    return drive(
        policy,
        variables: [
            "count": { integer(policy.vCount) },
            "ready": { .bool(policy.vReady) },
        ]
    )
}

private func eventArrival() -> Machine {
    let policy = StaticEventArrivalPolicy()
    return drive(
        policy,
        variables: [
            "requests": { integer(policy.requests) },
            "specials": { integer(policy.specials) },
        ]
    )
}

private func eventWildcard() -> Machine {
    let policy = StaticEventWildcardPolicy()
    return drive(
        policy,
        variables: [
            "requests": { integer(policy.requests) },
            "strays": { integer(policy.strays) },
        ]
    )
}

private func overflow() -> Machine {
    let policy = StaticOverflowPolicy()
    return drive(
        policy,
        variables: [
            "level": { integer(policy.level) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func stringCapacity() -> Machine {
    let policy = StaticStringCapacityPolicy()
    return drive(
        policy,
        variables: [
            "title": { .text(policy.title) },
            "body": { .text(policy.body) },
            "copied": { integer(policy.copied) },
            "measured": { integer(policy.measured) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func blockEnds() -> Machine {
    let policy = StaticBlockEndsPolicy()
    return drive(
        policy,
        variables: [
            "a": { integer(policy.a) },
            "b": { integer(policy.b) },
            "afterAssign": { integer(policy.afterAssign) },
            "thenRan": { integer(policy.thenRan) },
            "elseRan": { integer(policy.elseRan) },
            "afterIf": { integer(policy.afterIf) },
            "inBranch": { integer(policy.inBranch) },
            "afterBranch": { integer(policy.afterBranch) },
            "afterOk": { integer(policy.afterOk) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func history() -> Machine {
    let policy = StaticHistoryPolicy()
    return drive(policy, variables: ["resumed": { integer(policy.resumed) }])
}

private func timers() -> Machine {
    let policy = StaticTimersPolicy()
    return drive(policy, variables: ["trace": { integer(policy.trace) }])
}

private func sendDelay() -> Machine {
    let policy = StaticSendDelayPolicy()
    return drive(
        policy,
        variables: [
            "wait": { integer(policy.wait) },
            "beats": { integer(policy.beats) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func sendEvent() -> Machine {
    let policy = StaticSendEventPolicy()
    return drive(
        policy,
        variables: [
            "pings": { integer(policy.pings) },
            "pongs": { integer(policy.pongs) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func cancelExpr() -> Machine {
    let policy = StaticCancelExprPolicy()
    return drive(
        policy,
        variables: [
            "a_fired": { integer(policy.aFired) },
            "b_fired": { integer(policy.bFired) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func sendIdLocation() -> Machine {
    let policy = StaticSendIdlocationPolicy()
    return drive(
        policy,
        variables: [
            "first_fired": { integer(policy.firstFired) },
            "second_fired": { integer(policy.secondFired) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func sendTarget() -> Machine {
    let policy = StaticSendTargetPolicy()
    return drive(
        policy,
        variables: [
            "landed": { integer(policy.landed) },
            "refused": { integer(policy.refused) },
        ]
    )
}

private func sendType() -> Machine {
    let policy = StaticSendTypePolicy()
    return drive(
        policy,
        variables: [
            "landed": { integer(policy.landed) },
            "refused": { integer(policy.refused) },
        ]
    )
}

private func payload() -> Machine {
    let policy = StaticPayloadPolicy()
    return drive(
        policy,
        variables: [
            "day": { integer(policy.day) },
            "late": { .bool(policy.late) },
            "sinceEpoch": { integer(policy.sinceEpoch) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func enumeration() -> Machine {
    let policy = StaticEnumPolicy()
    return drive(
        policy,
        variables: [
            "layout": { .text(policy.layout.declaredName) },
            "previous": { .text(policy.vPrevious.declaredName) },
            "changes": { integer(policy.changes) },
        ]
    )
}

private func list() -> Machine {
    let policy = StaticListPolicy()
    return drive(
        policy,
        variables: [
            "picked": { integers(policy.picked) },
            "refusals": { integer(policy.refusals) },
            "count": { integer(policy.count) },
        ]
    )
}

private func listIndex() -> Machine {
    let policy = StaticListIndexPolicy()
    return drive(
        policy,
        variables: [
            "picked": { integers(policy.picked) },
            "cursor": { integer(policy.cursor) },
            "under": { integer(policy.under) },
            "first": { integer(policy.first) },
            "last": { integer(policy.last) },
            "ordered": { .bool(policy.ordered) },
            "room": { .bool(policy.room) },
            "count": { integer(policy.count) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func foreachList() -> Machine {
    let policy = StaticForeachPolicy()
    return drive(
        policy,
        variables: [
            "picked": { integers(policy.picked) },
            "total": { integer(policy.total) },
            "weighted": { integer(policy.weighted) },
            "small": { integer(policy.small) },
            "crossings": { integer(policy.crossings) },
            "visited": { integer(policy.visited) },
            "finished": { integer(policy.finished) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func blockEndsList() -> Machine {
    let policy = StaticBlockEndsListPolicy()
    return drive(
        policy,
        variables: [
            "picked": { integers(policy.picked) },
            "afterAppend": { integer(policy.afterAppend) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func reals64() -> Machine {
    let policy = StaticRealPolicy()
    return drive(
        policy,
        variables: [
            "level": { real(policy.level) },
            "total": { real(policy.total) },
            "drift": { real(policy.drift) },
            "samples": { reals(policy.samples) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func reals32() -> Machine {
    let policy = StaticReal32Policy()
    return drive(
        policy,
        variables: [
            "level": { real(policy.level) },
            "drift": { real(policy.drift) },
            "wide": { real(policy.wide) },
            "samples": { reals(policy.samples) },
            "total": { real(policy.total) },
        ]
    )
}

// A record as a scenario writes it: an object of its fields, in the schema's order.
private func day(_ year: UInt16, _ month: UInt8, _ dayOfMonth: UInt8) -> JSONValue {
    .object([
        (key: "year", value: integer(year)),
        (key: "month", value: integer(month)),
        (key: "dayOfMonth", value: integer(dayOfMonth)),
    ])
}

private func recordFields() -> Machine {
    let policy = StaticRecordFieldsPolicy()
    return drive(
        policy,
        variables: [
            "shown": {
                day(policy.shown.year, policy.shown.month, policy.shown.dayOfMonth)
            },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func recordList() -> Machine {
    let policy = StaticRecordListPolicy()
    return drive(
        policy,
        variables: [
            "draft": { day(policy.draft.year, policy.draft.month, policy.draft.dayOfMonth) },
            "last": { day(policy.last.year, policy.last.month, policy.last.dayOfMonth) },
            "days": { .array(policy.days.map { day($0.year, $0.month, $0.dayOfMonth) }) },
            "copies": { .array(policy.copies.map { day($0.year, $0.month, $0.dayOfMonth) }) },
            "total": { integer(policy.total) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func reading(_ sensor: UInt8, _ value: Double) -> JSONValue {
    .object([(key: "sensor", value: integer(sensor)), (key: "value", value: real(value))])
}

private func recordReal() -> Machine {
    let policy = StaticRecordRealPolicy()
    return drive(
        policy,
        variables: [
            "last": { reading(policy.last.sensor, policy.last.value) },
            "sum": { real(policy.sum) },
        ]
    )
}

private func recordReal32() -> Machine {
    let policy = StaticRecordReal32Policy()
    return drive(
        policy,
        variables: [
            "last": { reading(policy.last.sensor, Double(policy.last.value)) },
            "sum": { real(policy.sum) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func enumNumber() -> Machine {
    let policy = StaticEnumNumberPolicy()
    return drive(
        policy,
        variables: [
            "number": { integer(policy.number) },
            "status": { .text(policy.status.declaredName) },
            "checks": { integer(policy.checks) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func payloadEnum() -> Machine {
    let policy = StaticPayloadEnumPolicy()
    return drive(
        policy,
        variables: [
            "layout": { .text(policy.layout.declaredName) },
            "zoom": { integer(policy.zoom) },
            "agenda": { .bool(policy.agenda) },
            "shown": { integer(policy.shown) },
        ]
    )
}

private func view(_ layout: String, _ zoom: UInt8) -> JSONValue {
    .object([(key: "layout", value: .text(layout)), (key: "zoom", value: integer(zoom))])
}

private func wholePayload() -> Machine {
    let policy = StaticWholePayloadPolicy()
    return drive(
        policy,
        variables: [
            "shown": { view(policy.shown.layout.declaredName, policy.shown.zoom) },
            "seen": { .array(policy.seen.map { view($0.layout.declaredName, $0.zoom) }) },
            "updates": { integer(policy.updates) },
            "agendas": { integer(policy.agendas) },
            "others": { integer(policy.others) },
        ]
    )
}

private func schemaRevision() -> Machine {
    let policy = StaticSchemaRevisionPolicy()
    return drive(
        policy,
        variables: [
            "last": {
                .object([
                    (key: "level", value: integer(policy.last.level)),
                    (key: "layout", value: .text(policy.last.layout.declaredName)),
                    (key: "urgent", value: .bool(policy.last.urgent)),
                ])
            },
            "taken": { integer(policy.taken) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func payloadRelay() -> Machine {
    let policy = StaticPayloadRelayPolicy()
    return drive(
        policy,
        variables: [
            "layout": { .text(policy.layout.declaredName) },
            "zoom": { integer(policy.zoom) },
            "relays": { integer(policy.relays) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func sendContent() -> Machine {
    let policy = StaticSendContentPolicy()
    return drive(
        policy,
        variables: [
            "shown": { view(policy.shown.layout.declaredName, policy.shown.zoom) },
            "received": { .text(policy.received.declaredName) },
            "level": { integer(policy.level) },
            "relays": { integer(policy.relays) },
        ]
    )
}

private func sendNamelist() -> Machine {
    let policy = StaticSendNamelistPolicy()
    return drive(
        policy,
        variables: [
            "layout": { .text(policy.layout.declaredName) },
            "zoom": { integer(policy.zoom) },
            "received": { .text(policy.received.declaredName) },
            "level": { integer(policy.level) },
            "deliveries": { integer(policy.deliveries) },
        ]
    )
}

private func sendParams() -> Machine {
    let policy = StaticSendParamsPolicy()
    return drive(
        policy,
        variables: [
            "total": { integer(policy.total) },
            "ok": { .bool(policy.ok) },
            "tag": { .text(policy.tag) },
            "partialTotal": { integer(policy.partialTotal) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func wireEnum() -> Machine {
    let policy = StaticWireEnumPolicy()
    return drive(
        policy,
        variables: [
            "layout": { .text(policy.layout.declaredName) },
            "shown": { view(policy.shown.layout.declaredName, policy.shown.zoom) },
            "received": { .text(policy.received.declaredName) },
            "deliveries": { integer(policy.deliveries) },
        ]
    )
}

private func wireWide() -> Machine {
    let policy = StaticWireWidePolicy()
    return drive(
        policy,
        variables: [
            "low": { integer(policy.low) },
            "smallLow": { integer(policy.smallLow) },
            "large": { .bool(policy.large) },
            "negative": { .bool(policy.negative) },
            "relays": { integer(policy.relays) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

// MARK: - The scenarios

// A counter counts to its flag and lets go: guards, `<assign>`, `<if>` and `<elseif>`
// are native, and `In()` asks the machine's own configuration.
@Test func theCounterCountsToItsFlagAndLetsGo() {
    replay("static_counter", counter())
}

@Test func theCounterStopsAtItsBoundAndRefusesGo() {
    replay("static_counter_bound", counter())
}

// An event arrives by name from outside the document, so the names it can arrive
// under are not the ones the document writes: a name that extends one the document
// has is that event, and one it does not is dropped (§scxml-3.12.1).
@Test func anEventArrivesUnderANameTheDocumentDoesNotWrite() {
    replay("static_event_arrival", eventArrival())
}

@Test func anEventArrivesUnderANameOnlyTheWildcardTakes() {
    replay("static_event_wildcard", eventWildcard())
}

// A checked integer operation that overflows fails instead of wrapping: the
// statement writes nothing, `error.execution` is raised, and the block ends.
@Test func anOverflowingOperationFailsInsteadOfWrapping() {
    replay("static_overflow", overflow())
}

// A string variable holds no more than its bound, counted in UTF-8 bytes: the text
// that does not fit is refused, not cut.
@Test func aStringHoldsNoMoreThanItsBound() {
    replay("static_string_capacity", stringCapacity())
}

// A failing statement ends its block, and only its block: what stands after it in the
// same block does not run, and the next block does (§scxml-4.9).
@Test func aFailingStatementEndsItsBlockAndOnlyItsBlock() {
    replay("static_block_ends", blockEnds())
}

// A history remembers what its parent held when it was left, and a machine that
// resumes through it finds the variables it kept.
@Test func aHistoryResumesWhereItsParentWasLeft() {
    replay("static_history", history())
}

// MARK: - Records

// A record is built whole from its `<sce:set>`s, read field by field in a guard and an
// assignment, and written a field at a time.
@Test func aRecordIsBuiltWholeAndReadAndWrittenAFieldAtATime() {
    replay("static_record_fields", recordFields())
}

// A record is taken into a list by name, and the list holds a copy of it: a record
// written after it was taken does not move the element.
@Test func aRecordListHoldsCopiesOfTheRecordsItTook() {
    replay("static_record_list", recordList())
}

@Test func aRecordWithA64BitRealFieldIsReadAndWrittenAFieldAtATime() {
    replay("static_record_real", recordReal())
}

@Test func aRecordWithA32BitRealFieldIsHeldToBinary32() {
    replay("static_record_real32", recordReal32())
}

// MARK: - Reals

// A real is a double, conversions between an integer and a real are written where
// they are made, and `round` and `floor` answer the integer the context declares.
@Test func aRealVariableIsComputedAsADouble() {
    replay("static_real", reals64())
}

// A 32-bit real is a binary32, and an operation on one is rounded to binary32 where it
// is made: `16777216 + 1` is `16777216`, where a double makes `16777217`.
@Test func aSingleIsRoundedToBinary32WhereAnOperationIsMade() {
    replay("static_real32", reals32())
}

// MARK: - Lists

// A list is filled to its bound and emptied: the append that would pass the bound is
// refused as `error.execution` and leaves the list as it was.
@Test func aListIsFilledToItsBoundAndEmptied() {
    replay("static_list", list())
}

// An element is read by its index, and an index outside the list is refused rather
// than read.
@Test func aListElementIsReadByItsIndex() {
    replay("static_list_index", listIndex())
}

// A `<foreach>` walks the list as it was when the loop began, so a body that appends to
// the list or empties it does not move it.
@Test func aForeachWalksAListVariable() {
    replay("static_foreach", foreachList())
}

// An append that fails ends its block, and only its block.
@Test func aFailingAppendEndsItsBlockAndOnlyItsBlock() {
    replay("static_block_ends_list", blockEndsList())
}

// MARK: - Enums

// An enum variable starts at a variant, is compared with `===` and `!==` to a variant
// or to another variable of the same enum, takes a conditional of two variants, and is
// read by the host as the name its enum document declares.
@Test func anEnumVariableHoldsAVariantOfItsEnum() {
    replay("static_enum", enumeration())
}

// MARK: - Enums in a payload

// An enum field of a payload is held in the machine's own enum, lifted from the
// variant's declared name, and an enum is read as the number its variant declares and
// back, the number of a closed enum no variant declares being refused.
@Test func anEnumIsReadAsItsNumberAndBack() {
    replay("static_enum_number", enumNumber())
}

@Test func anEventsPayloadCarriesAnEnumField() {
    replay("static_payload_enum", payloadEnum())
}

// A payload is taken into a record whole, and a list takes the record by name.
@Test func aWholePayloadIsTakenIntoARecord() {
    replay("static_whole_payload", wholePayload())
}

// A schema that grew is read with the defaults of the fields a later revision added
// where the delivery lacks them.
@Test func aSchemaThatGrewIsReadWithTheDefaultsOfItsLaterFields() {
    replay("static_schema_revision", schemaRevision())
}

// MARK: - What a send carries

// A `<param>` is the typed value of the data model crossed to the event's JSON: the pair
// a failed evaluation would have written is left out, the message still goes, and the
// block ends.
@Test func aSendCarriesItsParamsAsTypedValues() {
    replay("static_send_params", sendParams())
}

// A `namelist` names variables, and each is a pair like a `<param>`.
@Test func aSendCarriesTheVariablesItsNamelistNames() {
    replay("static_send_namelist", sendNamelist())
}

// A `<content expr>` is the one value the event carries as its data.
@Test func aSendCarriesTheValueItsContentComputes() {
    replay("static_send_content", sendContent())
}

// An enum crosses a `<param>` as the name its document declares and arrives as the
// machine's own enum.
@Test func anEnumCrossesASendAsItsDeclaredName() {
    replay("static_wire_enum", wireEnum())
}

// A 64-bit integer crosses as the text of its digits, exactly.
@Test func a64BitIntegerCrossesASendAsItsDigits() {
    replay("static_wire_wide", wireWide())
}

// A payload received is relayed on as the params of another send.
@Test func aPayloadIsCarriedOnAsParams() {
    replay("static_payload_relay", payloadRelay())
}

// MARK: - Event payloads

// An event's typed payload is read in a guard and in assignments, at the width its
// schema declares: a value that does not fit is refused as `error.execution`, and
// the guard that read it does not fire.
@Test func anEventsTypedPayloadIsReadInAGuardAndInAssignments() {
    replay("static_payload", payload())
}

// MARK: - Sends and cancels

// A delayed `<send>` still waiting is part of a machine's state: armed on entering a
// state, delivered when its time comes, and removed by the `<cancel>` that names it.
@Test func aDelayedSendIsDeliveredWhenItsTimeComesAndACancelRemovesIt() {
    replay("static_timers", timers())
}

@Test func aCancelStopsTheTimersOfTheStateItLeaves() {
    replay("static_timers_stop", timers())
}

// The delay of a `<send>` is a string the machine computes, read as the CSS2 time it
// must be: a value that is no time, or an operation that fails, is refused.
@Test func aSendWaitsForADelayTheMachineComputes() {
    replay("static_send_delay", sendDelay())
}

@Test func aSendDeliversTheEventTheMachineNames() {
    replay("static_send_event", sendEvent())
}

@Test func aCancelNamesTheSendItRemovesByAComputedId() {
    replay("static_cancel_expr", cancelExpr())
}

@Test func aSendHandsItsIdToTheVariableItNames() {
    replay("static_send_idlocation", sendIdLocation())
}

@Test func aSendGoesOnlyByARouteTheDocumentDeclares() {
    replay("static_send_target", sendTarget())
}

@Test func aSendOfAComputedTypeGoesByAProcessorTheDocumentDeclares() {
    replay("static_send_type", sendType())
}
