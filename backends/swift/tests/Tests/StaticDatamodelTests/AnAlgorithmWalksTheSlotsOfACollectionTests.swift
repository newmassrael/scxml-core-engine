// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import StaticDatamodelMachines

// A Swift algorithm that imports a bounded collection takes it as a parameter and walks its occupied slots (RFC
// section synth-5-A line 311, item C7), run here over the committed unit
// `AlgorithmBcIterMinimal.swift`, generated from `tests/forge/resources/algorithm_bc_iter_minimal.scxml` beside the
// collection `LocalSubTable` and the codec it holds. The algorithm answers `0` when an element's `callback_id` is the
// target and `0xFFFF` when none is, which the document's two `<sce:return>`s say.

private let found: UInt16 = 0
private let absent: UInt16 = 0xFFFF

private func holding(_ ids: [UInt32]) -> (table: LocalSubTable, handles: [LocalSubTableHandle]) {
    var table = LocalSubTable()
    var handles: [LocalSubTableHandle] = []
    for id in ids {
        guard case .ok(let handle) = table.insert(SubscriptionEntry(callback_id: id)) else {
            preconditionFailure("the table took \(handles.count) of its \(LocalSubTable.capacity) slots")
        }
        handles.append(handle)
    }
    return (table, handles)
}

private func answer(_ target: UInt32, _ table: LocalSubTable) -> UInt16 {
    AlgorithmBcIterMinimal.algorithmBcIterMinimal(target, table)
}

@Test func anEmptyCollectionHoldsNoElementOfTheTarget() {
    #expect(answer(7, LocalSubTable()) == absent)
}

@Test func anElementOfTheTargetIsFoundAndOneOfAnotherIsNot() {
    let (table, _) = holding([3, 7, 9])
    #expect(answer(7, table) == found)
    #expect(answer(3, table) == found)
    #expect(answer(9, table) == found)
    #expect(answer(4, table) == absent)
}

// A slot a removal left empty is passed over, and the elements after it are still walked.
@Test func aHoleLeftByARemovalIsPassedOverAndTheSlotsAfterItAreWalked() {
    let built = holding([3, 7])
    var table = built.table
    let removed = table.remove(built.handles[0])
    #expect(removed)
    #expect(table.len == 1)
    #expect(answer(7, table) == found)
    #expect(answer(3, table) == absent)
}

@Test func aRemovedElementIsNotWalked() {
    let built = holding([5])
    var table = built.table
    #expect(answer(5, table) == found)
    let removed = table.remove(built.handles[0])
    #expect(removed)
    #expect(answer(5, table) == absent)
}

// The walk reaches the last slot of a full collection, which the bound of the loop is the capacity of.
@Test func theLastSlotOfAFullCollectionIsWalked() {
    let capacity = LocalSubTable.capacity
    let ids = (0..<capacity).map { UInt32(100 + $0) }
    let (table, _) = holding(ids)
    #expect(table.len == capacity)
    #expect(answer(ids[capacity - 1], table) == found)
    #expect(answer(ids[0], table) == found)
    #expect(answer(1, table) == absent)
}
