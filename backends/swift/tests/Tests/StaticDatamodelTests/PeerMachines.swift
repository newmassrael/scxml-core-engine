// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import SceRuntime

@testable import StaticDatamodelMachines

// The machines that talk to Mesh peers, as their scenarios see them: the shared list copies
// (presence, to-do, sheet, the standard record list) and the machine whose children do. Each is
// driven with the recorders of its peers (`PeerRecorder`), so that what it sends them is
// held to the scenario; the copy a scenario says has no router is driven with none.
//
// A scenario writes a 64-bit integer as the text of its digits and a smaller one as a number,
// and the members of a record by name: `ScalarScenarioTests` replays the scenarios of these.

/// A 64-bit integer as a scenario writes one held in a record: the text of its digits.
private func digits<T: BinaryInteger>(_ value: T) -> JSONValue {
    .text(String(value))
}

/// A hybrid logical clock reading.
private func hlc(_ wallTime: Int64, _ counter: UInt32, _ nodeId: UInt64) -> JSONValue {
    .object([
        (key: "counter", value: integer(counter)),
        (key: "nodeId", value: digits(nodeId)),
        (key: "wallTime", value: digits(wallTime)),
    ])
}

/// An element of a list that tracks adds and removals, with the clock reading that made it.
private func listEntry(_ element: UInt64, _ wallTime: Int64, _ counter: UInt32, _ nodeId: UInt64) -> JSONValue {
    .object([
        (key: "counter", value: integer(counter)),
        (key: "element", value: digits(element)),
        (key: "nodeId", value: digits(nodeId)),
        (key: "wallTime", value: digits(wallTime)),
    ])
}

/// A value a field of a record of the list holds, with the clock reading that wrote it.
private func fieldValue(
    _ id: UInt64, _ field: UInt64, _ value: Int64, _ text: String, _ wallTime: Int64,
    _ counter: UInt32, _ nodeId: UInt64
) -> JSONValue {
    .object([
        (key: "counter", value: integer(counter)),
        (key: "field", value: digits(field)),
        (key: "id", value: digits(id)),
        (key: "nodeId", value: digits(nodeId)),
        (key: "text", value: .text(text)),
        (key: "value", value: digits(value)),
        (key: "wallTime", value: digits(wallTime)),
    ])
}

/// A node of the digest tree two copies compare.
private func digestNode(_ depth: UInt8, _ prefix: UInt64, _ hash: UInt32) -> JSONValue {
    .object([
        (key: "depth", value: integer(depth)),
        (key: "hash", value: integer(hash)),
        (key: "prefix", value: digits(prefix)),
    ])
}

/// A copy of the presence list.
func presencePeer() -> Machine {
    let policy = PresencePeerPolicy()
    return drive(
        policy,
        variables: [
            "hub": { .bool(policy.hub) },
            "lease": { integer(policy.lease) },
            "lost": { integer(policy.lost) },
            "node": { integer(policy.node) },
            "online": {
                .array(
                    policy.online.map {
                        .object([
                            (key: "client", value: integer($0.client)),
                            (key: "gen", value: integer($0.gen)),
                            (key: "last", value: digits($0.last)),
                        ])
                    })
            },
            "refusals": { integer(policy.refusals) },
            "serial": { integer(policy.serial) },
            "ttl": { integer(policy.ttl) },
        ],
        peers: PeerRecorder()
    )
}

/// A copy of the to-do list.
func todoPeer() -> Machine {
    let policy = TodoPeerPolicy()
    return drive(
        policy,
        variables: [
            "adds": { .array(policy.adds.map { listEntry($0.element, $0.wallTime, $0.counter, $0.nodeId) }) },
            "agenda": {
                .array(
                    policy.agenda.map {
                        .object([
                            (key: "conflict", value: integer($0.conflict)),
                            (key: "end", value: digits($0.end)),
                            (key: "id", value: digits($0.id)),
                            (key: "nth", value: integer($0.nth)),
                            (key: "start", value: digits($0.start)),
                        ])
                    })
            },
            "agendaNext": { integer(policy.agendaNext) },
            "clock": { hlc(policy.clock.wallTime, policy.clock.counter, policy.clock.nodeId) },
            "denied": { integer(policy.denied) },
            "digest": { .array(policy.digest.map { digestNode($0.depth, $0.prefix, $0.hash) }) },
            "floor": { digits(policy.floor) },
            "full": { integer(policy.full) },
            "hub": { .bool(policy.hub) },
            "late": { integer(policy.late) },
            "node": { digits(policy.node) },
            "refusals": { integer(policy.refusals) },
            "stale": { .bool(policy.stale) },
            "tombstones": {
                .array(policy.tombstones.map { listEntry($0.element, $0.wallTime, $0.counter, $0.nodeId) })
            },
            "values": {
                .array(
                    policy.values.map {
                        fieldValue($0.id, $0.field, $0.value, $0.text, $0.wallTime, $0.counter, $0.nodeId)
                    })
            },
        ],
        peers: PeerRecorder()
    )
}

/// A copy of the sheet.
func sheetPeer() -> Machine {
    let policy = SheetPeerPolicy()
    return drive(
        policy,
        variables: [
            "cells": {
                .array(
                    policy.cells.map {
                        .object([
                            (key: "aCol", value: digits($0.aCol)),
                            (key: "aRef", value: .bool($0.aRef)),
                            (key: "aRow", value: digits($0.aRow)),
                            (key: "aVal", value: digits($0.aVal)),
                            (key: "bCol", value: digits($0.bCol)),
                            (key: "bRef", value: .bool($0.bRef)),
                            (key: "bRow", value: digits($0.bRow)),
                            (key: "bVal", value: digits($0.bVal)),
                            (key: "col", value: digits($0.col)),
                            (key: "counter", value: integer($0.counter)),
                            (key: "nodeId", value: digits($0.nodeId)),
                            (key: "op", value: integer($0.op)),
                            (key: "row", value: digits($0.row)),
                            (key: "value", value: digits($0.value)),
                            (key: "wallTime", value: digits($0.wallTime)),
                        ])
                    })
            },
            "clock": { hlc(policy.clock.wallTime, policy.clock.counter, policy.clock.nodeId) },
            "cols": { .array(policy.cols.map { ordered($0.key, after: $0.after) }) },
            "gone": { .array(policy.gone.map { removed($0.key, by: $0.by) }) },
            "goneCols": { .array(policy.goneCols.map { removed($0.key, by: $0.by) }) },
            "hub": { .bool(policy.hub) },
            "lamport": { digits(policy.lamport) },
            "node": { digits(policy.node) },
            "orphans": { integer(policy.orphans) },
            "refusals": { integer(policy.refusals) },
            "rows": { .array(policy.rows.map { ordered($0.key, after: $0.after) }) },
            "shown": {
                .array(
                    policy.shown.map {
                        .object([
                            (key: "col", value: digits($0.col)),
                            (key: "fault", value: integer($0.fault)),
                            (key: "row", value: digits($0.row)),
                            (key: "value", value: digits($0.value)),
                        ])
                    })
            },
        ],
        peers: PeerRecorder()
    )
}

/// An element of a sequence that is ordered by the element it follows.
private func ordered(_ key: UInt64, after: UInt64) -> JSONValue {
    .object([(key: "after", value: digits(after)), (key: "key", value: digits(key))])
}

/// An element of a sequence that was removed, and by whom.
private func removed(_ key: UInt64, by: UInt64) -> JSONValue {
    .object([(key: "by", value: digits(by)), (key: "key", value: digits(key))])
}

/// A copy of the standard shared record list, given a Mesh router when `routed` and none when
/// it is not (a scenario that says `"router": false`).
func sharedRecordList(routed: Bool = true) -> Machine {
    let policy = RecordListPolicy()
    return drive(
        policy,
        variables: [
            "adds": { .array(policy.adds.map { listEntry($0.element, $0.wallTime, $0.counter, $0.nodeId) }) },
            "announced": { integer(policy.announced) },
            "clock": { hlc(policy.clock.wallTime, policy.clock.counter, policy.clock.nodeId) },
            "denied": { integer(policy.denied) },
            "full": { integer(policy.full) },
            "node": { digits(policy.node) },
            "refusals": { integer(policy.refusals) },
            "values": {
                .array(
                    policy.values.map {
                        fieldValue($0.id, $0.field, $0.value, $0.text, $0.wallTime, $0.counter, $0.nodeId)
                    })
            },
        ],
        peers: routed ? PeerRecorder() : nil
    )
}

/// The statechart that keeps a shared record list by invoking the standard one and publishes
/// what the list told it. No Mesh route is given to the list, so the machine is driven with no
/// peers: the change the list would send its peers is refused where it is made, after it is
/// taken in, and the edit runs whole.
func recordListParent() -> Machine {
    let policy = RecordListParentPolicy()
    return drive(
        policy,
        variables: [
            "told": { integer(policy.told) },
            "shown": {
                let shown = policy.shown
                return .object([
                    (key: "count", value: integer(shown.count)),
                    (key: "counter", value: integer(shown.counter)),
                    (key: "field", value: integer(shown.field)),
                    (key: "id", value: integer(shown.id)),
                    (key: "nodeId", value: digits(shown.nodeId)),
                    (key: "text", value: .text(shown.text)),
                    (key: "value", value: digits(shown.value)),
                    (key: "wallTime", value: digits(shown.wallTime)),
                ])
            },
            "others": {
                .array(
                    policy.others.map {
                        .object([
                            (key: "counter", value: integer($0.counter)),
                            (key: "field", value: integer($0.field)),
                            (key: "id", value: integer($0.id)),
                            (key: "nodeId", value: digits($0.nodeId)),
                            (key: "ordinal", value: integer($0.ordinal)),
                            (key: "text", value: .text($0.text)),
                            (key: "value", value: digits($0.value)),
                            (key: "wallTime", value: digits($0.wallTime)),
                        ])
                    })
            },
            "refused": {
                guard let refused = policy.refused else {
                    return .null
                }
                return .object([
                    (key: "field", value: integer(refused.field)),
                    (key: "id", value: integer(refused.id)),
                    (key: "kind", value: .text(refused.kind.declaredName)),
                    (key: "reason", value: .text(refused.reason.declaredName)),
                ])
            },
            "refusedCount": { integer(policy.refusedCount) },
        ]
    )
}

/// The machine whose child sessions talk to Mesh peers through routers of their own.
func childMesh() -> Machine {
    let policy = StaticChildMeshPolicy()
    return drive(
        policy,
        variables: ["refusals": { integer(policy.refusals) }],
        peers: PeerRecorder()
    )
}
