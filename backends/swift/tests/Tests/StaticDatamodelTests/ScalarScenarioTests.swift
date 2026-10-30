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

private func wrappingCounter() -> Machine {
    let policy = StaticWrappingCounterPolicy()
    return drive(
        policy,
        variables: [
            "ticks": { integer(policy.ticks) },
            "fuel": { integer(policy.fuel) },
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

private func invoke() -> Machine {
    let policy = StaticInvokePolicy()
    return drive(policy, variables: ["completed": { integer(policy.completed) }])
}

private func forgeCalls() -> Machine {
    let policy = StaticForgeCallsPolicy()
    return drive(
        policy,
        variables: [
            "gear": { integer(policy.gear) },
            "ratio": { integer(policy.ratio) },
            "code": { integer(policy.code) },
            "misses": { integer(policy.misses) },
            "level": { integer(policy.level) },
            "offset": { integer(policy.offset) },
            "accepted": { .bool(policy.accepted) },
            "rpm": { integer(policy.rpm) },
            "limit": { real(policy.limit) },
            "highs": { integer(policy.highs) },
            "phase": { .text(policy.phase.declaredName) },
            "weight": { .text(policy.weight.declaredName) },
            "customer": { .text(policy.customer) },
            "known": { .bool(policy.known) },
        ]
    )
}

/// A value that may be absent as a scenario writes a host argument: the value, or `null`.
private func optionalBool(_ value: Bool?) -> JSONValue {
    value.map { .bool($0) } ?? .null
}

private func hostCalling() -> Machine {
    let host = RecordingStaticHostCallActions()
    let policy = StaticHostCallPolicy(actions: host)
    return drive(
        policy,
        variables: [:],
        hostCalls: {
            defer { host.clear() }
            return host.calls.map { call in
                switch call {
                case .showAttempts(let count, let exhausted):
                    return hostCall("showAttempts", [integer(count), .bool(exhausted)])
                case .showDue(let day, let urgent):
                    return hostCall("showDue", [optionalInteger(day), optionalBool(urgent)])
                }
            }
        }
    )
}

private func hostDuty() -> Machine {
    let host = RecordingStaticHostDutyActions()
    // The recorder simulates the duties, so the machine is built for one on purpose.
    let policy = StaticHostDutyPolicy(actions: host, acceptSimulatedHost: true)
    return drive(
        policy,
        variables: [:],
        hostCalls: {
            defer { host.clear() }
            return host.calls.map { call in
                switch call {
                case .startKeepalive: return hostCall("start_keepalive")
                case .noteEngaged: return hostCall("note_engaged")
                case .stopKeepalive: return hostCall("stop_keepalive")
                }
            }
        }
    )
}

private func invokeParams() -> Machine {
    let policy = StaticInvokeParamsPolicy()
    return drive(policy, variables: ["completed": { integer(policy.completed) }])
}

private func invokeOptional() -> Machine {
    let policy = StaticInvokeOptionalPolicy()
    return drive(
        policy,
        variables: [
            "completed": { integer(policy.completed) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func invokeString() -> Machine {
    let policy = StaticInvokeStringPolicy()
    return drive(
        policy,
        variables: [
            "completed": { integer(policy.completed) },
            "errors": { integer(policy.errors) },
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

private func listAssign() -> Machine {
    let policy = StaticListAssignPolicy()
    return drive(
        policy,
        variables: [
            "shown": { integers(policy.shown) },
            "first": { integer(policy.first) },
            "few": { integers(policy.few) },
            "count": { integer(policy.count) },
            "size": { integer(policy.size) },
            "refusals": { integer(policy.refusals) },
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

// A ledger line as a scenario writes it: its id and amount, in the schema's order.
private func entry(_ id: UInt32, _ amount: Int32) -> JSONValue {
    .object([
        (key: "id", value: integer(id)),
        (key: "amount", value: integer(amount)),
    ])
}

private func recordCall() -> Machine {
    let policy = StaticRecordCallPolicy()
    return drive(
        policy,
        variables: [
            "best": { entry(policy.best.id, policy.best.amount) },
            "large": { integer(policy.large) },
            "debts": { integer(policy.debts) },
            "peaks": { integer(policy.peaks) },
        ]
    )
}

private func syncClient() -> Machine {
    let policy = SyncClientPolicy()
    return drive(
        policy,
        variables: [
            "byToken": { .bool(policy.byToken) },
            "fullListing": { .bool(policy.fullListing) },
            "outcome": { integer(policy.outcome) },
            "retryAt": { integer(policy.retryAt) },
            "deleted": { integer(policy.deleted) },
            "uploaded": { integer(policy.uploaded) },
            "discarded": { integer(policy.discarded) },
            "pages": { integer(policy.pages) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

// A running total as a scenario writes it: its count and sum, in the schema's order.
private func totalled(_ count: UInt32, _ sum: Int32) -> JSONValue {
    .object([
        (key: "count", value: integer(count)),
        (key: "sum", value: integer(sum)),
    ])
}

private func stateAlgorithm() -> Machine {
    let policy = StaticStateAlgorithmPolicy()
    return drive(
        policy,
        variables: [
            "entries": { .array(policy.entries.map { entry($0.id, $0.amount) }) },
            "total": { totalled(policy.total.count, policy.total.sum) },
            "last": { entry(policy.last.id, policy.last.amount) },
            "ids": { integers(policy.ids) },
            "idsum": { integer(policy.idsum) },
            "large": { integer(policy.large) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

// An item as a scenario writes it: its id and the declared name of its status, in the
// schema's order.
private func statusItem(_ id: UInt32, _ status: String) -> JSONValue {
    .object([
        (key: "id", value: integer(id)),
        (key: "status", value: .text(status)),
    ])
}

private func stateAlgorithmEnum() -> Machine {
    let policy = StaticStateAlgorithmEnumPolicy()
    return drive(
        policy,
        variables: [
            "items": {
                .array(policy.items.map { statusItem($0.id, $0.status.declaredName) })
            },
            "last": { .text(policy.last.declaredName) },
            "openCount": { integer(policy.openCount) },
            "startedCount": { integer(policy.startedCount) },
            "doneCount": { integer(policy.doneCount) },
            "checks": { integer(policy.checks) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

// A text slot as a scenario writes it: its slot, version and text, in the schema's
// order. The version is 64 bits wide, so it is the text of its digits, exactly.
private func textSlot(_ slot: UInt32, _ version: Int64, _ text: String) -> JSONValue {
    .object([
        (key: "slot", value: integer(slot)),
        (key: "version", value: .text(String(version))),
        (key: "text", value: .text(text)),
    ])
}

private func textRecord() -> Machine {
    let policy = StaticTextRecordPolicy()
    return drive(
        policy,
        variables: [
            "slots": { .array(policy.slots.map { textSlot($0.slot, $0.version, $0.text) }) },
            "held": { textSlot(policy.held.slot, policy.held.version, policy.held.text) },
            "incoming": {
                textSlot(policy.incoming.slot, policy.incoming.version, policy.incoming.text)
            },
            "known": { integer(policy.known) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func listCall() -> Machine {
    let policy = StaticListCallPolicy()
    return drive(
        policy,
        variables: [
            "entries": { .array(policy.entries.map { entry($0.id, $0.amount) }) },
            "ids": { integers(policy.ids) },
            "repeats": { integer(policy.repeats) },
            "high": { integer(policy.high) },
        ]
    )
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

private func record() -> Machine {
    let policy = StaticRecordPolicy()
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

private func donedata() -> Machine {
    let policy = StaticDonedataPolicy()
    return drive(policy, variables: ["count": { integer(policy.count) }])
}

private func donedataContent() -> Machine {
    let policy = StaticDonedataContentPolicy()
    return drive(policy, variables: ["count": { integer(policy.count) }])
}

private func donedataRecord() -> Machine {
    let policy = StaticDonedataRecordPolicy()
    return drive(
        policy,
        variables: ["shown": { view(policy.shown.layout.declaredName, policy.shown.zoom) }]
    )
}

private func bytesWire() -> Machine {
    let policy = StaticBytesWirePolicy()
    return drive(
        policy,
        variables: [
            "held": { bytesText(policy.held) },
            "echo": { bytesText(policy.echo) },
            "relays": { integer(policy.relays) },
            "errors": { integer(policy.errors) },
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

private func optionalWidened() -> Machine {
    let policy = StaticOptionalWidenedPolicy()
    return drive(
        policy,
        variables: [
            "entry": { integer(policy.entry) },
            "due": { optionalInteger(policy.due) },
            "remind": { optionalInteger(policy.remind) },
            "taken": { integer(policy.taken) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func pickedItem(_ item: StaticOptionalRecordItemRecord) -> JSONValue {
    .object([
        (key: "id", value: integer(item.id)),
        (key: "title", value: .text(item.title)),
        (key: "count", value: integer(item.count)),
    ])
}

private func optionalRecord() -> Machine {
    let policy = StaticOptionalRecordPolicy()
    return drive(
        policy,
        variables: [
            "pick": { policy.pick.map(pickedItem) ?? .null },
            "found": { policy.found.map(pickedItem) ?? .null },
            "draft": { pickedItem(policy.draft) },
            "items": { .array(policy.items.map(pickedItem)) },
            "title": { .text(policy.title) },
            "count": { integer(policy.count) },
            "shown": { integer(policy.shown) },
            "known": { .bool(policy.known) },
            "absent": { .bool(policy.absent) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func goal(_ goal: StaticStateAlgorithmOptionalGoalRecord) -> JSONValue {
    .object([
        (key: "id", value: integer(goal.id)),
        (key: "day", value: optionalDigits(goal.day)),
        (key: "weight", value: integer(goal.weight)),
    ])
}

private func stateAlgorithmOptional() -> Machine {
    let policy = StaticStateAlgorithmOptionalPolicy()
    return drive(
        policy,
        variables: [
            "goals": { .array(policy.goals.map(goal)) },
            "first": { optionalInteger(policy.first) },
            "left": { optionalInteger(policy.left) },
            "undated": { integer(policy.undated) },
            "total": { integer(policy.total) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func dueEntry(_ entry: StaticOptionalDueRecord) -> JSONValue {
    .object([
        (key: "id", value: integer(entry.id)),
        (key: "due", value: optionalDigits(entry.due)),
        (key: "title", value: optionalText(entry.title)),
        (key: "status", value: optionalText(entry.status?.declaredName)),
    ])
}

private func optionalValues() -> Machine {
    let policy = StaticOptionalPolicy()
    return drive(
        policy,
        variables: [
            "due": { optionalInteger(policy.due) },
            "other": { optionalInteger(policy.other) },
            "title": { optionalText(policy.title) },
            "status": { optionalText(policy.status?.declaredName) },
            "item": { dueEntry(policy.item) },
            "items": { .array(policy.items.map(dueEntry)) },
            "days": { integer(policy.days) },
            "following": { integer(policy.following) },
            "late": { .bool(policy.late) },
            "early": { .bool(policy.early) },
            "label": { .text(policy.label) },
            "plain": { .text(policy.plain.declaredName) },
            "kept": { integer(policy.kept) },
            "listed": { integer(policy.listed) },
            "missing": { integer(policy.missing) },
            "same": { .bool(policy.same) },
            "absent": { .bool(policy.absent) },
            "statusDone": { .bool(policy.statusDone) },
            "echoed": { integer(policy.echoed) },
            "echoTitle": { optionalText(policy.echoTitle) },
            "echoStatus": { optionalText(policy.echoStatus?.declaredName) },
            "fallbacks": { integer(policy.fallbacks) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func recordEnum() -> Machine {
    let policy = StaticRecordEnumPolicy()
    return drive(
        policy,
        variables: [
            "shown": { view(policy.shown.layout.declaredName, policy.shown.zoom) },
            "seen": { .array(policy.seen.map { view($0.layout.declaredName, $0.zoom) }) },
            "weeks": { integer(policy.weeks) },
            "flips": { integer(policy.flips) },
            "leads": { integer(policy.leads) },
        ]
    )
}

private func recordListIndex() -> Machine {
    let policy = StaticRecordListIndexPolicy()
    return drive(
        policy,
        variables: [
            "picked": { day(policy.picked.year, policy.picked.month, policy.picked.dayOfMonth) },
            "cursor": { integer(policy.cursor) },
            "day": { integer(policy.day) },
            "year": { integer(policy.year) },
            "sum": { integer(policy.sum) },
            "ordered": { .bool(policy.ordered) },
            "refusals": { integer(policy.refusals) },
        ]
    )
}

private func labelled(_ sensor: UInt8, _ label: String) -> JSONValue {
    .object([(key: "sensor", value: integer(sensor)), (key: "label", value: .text(label))])
}

private func recordString() -> Machine {
    let policy = StaticRecordStringPolicy()
    return drive(
        policy,
        variables: [
            "last": { labelled(policy.last.sensor, policy.last.label) },
            "note": { .text(policy.note) },
            "labels": { .array(policy.labels.map { labelled($0.sensor, $0.label) }) },
            "errors": { integer(policy.errors) },
        ]
    )
}

// A byte string as a scenario writes it: its byte-exact Latin-1 text.
private func bytesText(_ bytes: [UInt8]) -> JSONValue {
    .text(bytesAsPayloadText(bytes))
}

private func framed(_ sensor: UInt8, _ frame: [UInt8]) -> JSONValue {
    .object([(key: "sensor", value: integer(sensor)), (key: "frame", value: bytesText(frame))])
}

private func bytesVariables() -> Machine {
    let policy = StaticBytesPolicy()
    return drive(
        policy,
        variables: [
            "frame": { bytesText(policy.frame) },
            "tail": { bytesText(policy.vTail) },
            "size": { integer(policy.size) },
            "matches": { integer(policy.matches) },
            "misses": { integer(policy.misses) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func payloadBytes() -> Machine {
    let policy = StaticPayloadBytesPolicy()
    return drive(
        policy,
        variables: [
            "last": { framed(policy.last.sensor, policy.last.frame) },
            "held": { bytesText(policy.held) },
            "frames": { .array(policy.frames.map { framed($0.sensor, $0.frame) }) },
            "size": { integer(policy.size) },
            "matches": { integer(policy.matches) },
            "misses": { integer(policy.misses) },
            "errors": { integer(policy.errors) },
        ]
    )
}

private func recordBytes() -> Machine {
    let policy = StaticRecordBytesPolicy()
    return drive(
        policy,
        variables: [
            "last": { framed(policy.last.sensor, policy.last.frame) },
            "spare": { bytesText(policy.spare) },
            "frames": { .array(policy.frames.map { framed($0.sensor, $0.frame) }) },
            "size": { integer(policy.size) },
            "matches": { integer(policy.matches) },
            "misses": { integer(policy.misses) },
            "leads": { integer(policy.leads) },
            "errors": { integer(policy.errors) },
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

// A wrapping or a saturating integer operation never fails: the counter wraps past
// its width, the level holds at its bound, and neither raises `error.execution`.
@Test func aWrappingOrSaturatingOperationRaisesNothing() {
    replay("static_wrapping_counter", wrappingCounter())
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

// A guard calls an imported algorithm with a record's fields, and the bound it answers
// is computed from them on every step: the algorithm is a unit of the machine's own
// module, generated from the same document.
@Test func aGuardCallsAnImportedAlgorithmWithTheRecordsFields() {
    replay("static_record", record())
}

// A record is handed to an imported algorithm and the record it answers is taken back
// whole: the machine's own struct crosses to the one the algorithm's schema unit
// declares and back by field, for a record the machine holds and for an event's payload.
@Test func aRecordIsHandedToAnImportedAlgorithmAndTakenBackWhole() {
    replay("static_record_call", recordCall())
}

// A sync run composed of the standard sync rules, each an imported algorithm that may
// fail, called from a guard or an assignment: a failure is thrown by the algorithm,
// recorded by the machine, and ends its block as error.execution.
@Test func aSyncRunIsComposedOfTheStandardSyncRules() {
    replay("sync_client", syncClient())
}

// A list the machine keeps is handed to an imported algorithm to be read, a list of
// records mapped into the schema's struct and a list of numbers as it is, and the
// algorithm cannot write the list it was given.
@Test func aListIsHandedToAnImportedAlgorithmToBeRead() {
    replay("static_list_call", listCall())
}

// A list and a record the machine keeps are handed to imported algorithms that may
// fail, and the list or record each answers is taken back whole: the adapter throws
// what the algorithm throws, the machine records it, and the block ends as
// error.execution where an id sum does not fit its width.
@Test func aListAndARecordAreHandedToAlgorithmsThatMayFail() {
    replay("static_state_algorithm", stateAlgorithm())
}

// A record with text and a list of them are handed to the standard text-slot rules,
// two of which answer a record and a list and one of which only reads.
@Test func aTextSlotIsHandedToTheStandardTextRules() {
    replay("static_text_record", textRecord())
}

// A list of records whose field is a value of an enum, and a value of the enum itself,
// are handed to imported algorithms and taken back: each crosses between the machine's
// own enum and the unit of the enum document as the variant of the same declared name,
// and the values are the document's, not the variants' positions.
@Test func aValueOfAnEnumIsHandedToAnAlgorithmAndTakenBack() {
    replay("static_state_algorithm_enum", stateAlgorithmEnum())
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

// A record that holds an enum is built, compared and copied whole, and a list of them
// is walked and read.
@Test func aRecordHoldsAnEnumField() {
    replay("static_record_enum", recordEnum())
}

// An element of a list of records is read by its index and then by its field, and an
// index outside the list is refused rather than read: the read fails as a whole.
@Test func aListElementOfRecordsIsReadByItsIndexAndItsField() {
    replay("static_record_list_index", recordListIndex())
}

// A record holds bounded text, which a list copies with the record and which an
// assignment that does not fit refuses.
@Test func aRecordHoldsBoundedText() {
    replay("static_record_string", recordString())
}

// MARK: - Byte strings

// A byte string holds no more than its bound, is compared by its contents and measured
// in bytes, and an assignment that does not fit is refused.
@Test func aByteStringIsHeldToItsBoundAndComparedByItsContents() {
    replay("static_bytes", bytesVariables())
}

// A payload's byte-string field is read at its bound and compared as the variable's is.
@Test func aPayloadsByteStringFieldIsHeldToItsBound() {
    replay("static_payload_bytes", payloadBytes())
}

// A record holds a byte string, which a list copies with the record.
@Test func aRecordHoldsAByteString() {
    replay("static_record_bytes", recordBytes())
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

// A list an imported algorithm answers is taken into a list variable whole, held to the
// variable's bound: a list that does not fit is refused as `error.execution` and leaves
// the variable as it was.
@Test func aListAnAlgorithmAnswersIsTakenWholeAndHeldToTheBound() {
    replay("static_list_assign", listAssign())
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

// A top-level final hands its done event the pairs of its `<donedata>`, each read from the
// machine's fields as the state is entered; a pair whose value does not fit is left out
// and the others cross (§scxml-5.7.1).
@Test func aFinalHandsItsDoneEventTheValuesOfItsParams() {
    replay("static_donedata", donedata())
}

// A `<content>` is the done event's whole data: inline text finished at build time, or
// the one value an `expr` computes. An `expr` that cannot be computed is the evaluation
// that failed, so the data is the empty string and error.execution is raised.
@Test func aFinalHandsItsDoneEventTheValueOfItsContent() {
    replay("static_donedata_content", donedataContent())
    replay("static_donedata_content_text", donedataContent())
    replay("static_donedata_content_value", donedataContent())
    replay("static_donedata_content_lost", donedataContent())
}

// A record crosses a `<param>` as the object its fields spell.
@Test func aRecordCrossesAFinalsParamAsItsFields() {
    replay("static_donedata_record", donedataRecord())
}

// A byte string crosses the wire as the text of its bytes.
@Test func aByteStringCrossesTheWireAsTheTextOfItsBytes() {
    replay("static_bytes_wire", bytesWire())
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

// A schema that widened a field to one that may be absent: an entry of the first
// revision reads present where it carries the day and absent where it lacks the
// reminder, and an entry that lacks a field of the first revision is refused.
@Test func aWidenedFieldReadsAbsentWhereAnOlderEntryLacksIt() {
    replay("static_optional_widened", optionalWidened())
}

// A value that may be absent is held by a variable, a record's field and a list's
// element, read with a default and proven present by a test, handed on as a send's
// params and a final's donedata, and compared: an absent value is a value, not a
// zero.
@Test func aValueThatMayBeAbsentIsHeldReadAndHandedOn() {
    replay("static_optional", optionalValues())
}

// A record that may be absent is taken whole from a payload, a record variable and a
// list's element, and a field of it is read only where a test proves it present.
@Test func aRecordThatMayBeAbsentIsTakenWholeAndReadWhereProvenPresent() {
    replay("static_optional_record", optionalRecord())
}

// An imported algorithm is handed a list of records whose field may be absent and a
// value that may be absent, and the value that may be absent it answers is held.
@Test func anAlgorithmIsHandedValuesThatMayBeAbsentAndAnswersOne() {
    replay("static_state_algorithm_optional", stateAlgorithmOptional())
}

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

@Test func aChildSessionIsStartedDrivenAndEndedByItsParent() {
    replay("static_invoke", invoke())
}

@Test func aMachineCallsTheKindsThatAreFunctionsOfTheirInputs() {
    replay("static_forge_calls", forgeCalls())
}

@Test func aMachineAsksItsHostForWhatItCannotDoItself() {
    replay("static_host_call", hostCalling())
}

@Test func aDutyIsTakenOnEntryAndGivenBackOnExitThroughTheHost() {
    replay("static_host_duty", hostDuty())
}

@Test func aChildIsHandedTheValuesItsInvokeNames() {
    replay("static_invoke_params", invokeParams())
}

@Test func aChildIsHandedAValueThatMayBeAbsentWhole() {
    replay("static_invoke_optional", invokeOptional())
}

@Test func aChildIsHandedATextHeldToTheBoundItDeclared() {
    replay("static_invoke_string", invokeString())
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
