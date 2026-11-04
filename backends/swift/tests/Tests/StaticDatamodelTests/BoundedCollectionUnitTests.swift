// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import XCTest

@testable import StaticDatamodelMachines

// A bounded collection (RFC section synth-5-L) over a codec element
// (`tests/forge/conformance/bounded_collection/`): a fixed number of slots whose occupancy varies, and
// a handle that packs a slot with its generation so a handle taken before a removal finds nothing.
// `BcIndexedTable` has capacity 3, an index on `callback_id` and the `reject` overflow policy;
// `BcRecentTable` has capacity 2 and `oldest-wins`.
final class BoundedCollectionUnitTests: XCTestCase {
    private func entry(_ id: UInt32) -> BcEntry { BcEntry(callback_id: id) }

    private func handle(_ result: BcIndexedTableInsertResult) throws -> BcIndexedTableHandle {
        guard case .ok(let handle) = result else {
            throw XCTSkip("the insert was refused: \(result)")
        }
        return handle
    }

    func testAnInsertTakesTheFirstFreeSlotAndIsReadBackByItsHandle() throws {
        var table = BcIndexedTable()
        XCTAssertEqual(table.capacity, 3)
        XCTAssertTrue(table.isEmpty)
        let first = try handle(table.insert(entry(10)))
        let second = try handle(table.insert(entry(20)))
        XCTAssertEqual(first.slot, 0)
        XCTAssertEqual(second.slot, 1)
        XCTAssertEqual(table.len, 2)
        XCTAssertEqual(table.get(first), entry(10))
        XCTAssertEqual(table.get(second), entry(20))
        XCTAssertEqual(table.getBySlot(1), entry(20))
        XCTAssertNil(table.getBySlot(2))
        XCTAssertNil(table.getBySlot(99))
    }

    func testAFullCollectionRefusesAnInsertAndKeepsWhatItHolds() throws {
        var table = BcIndexedTable()
        for id in 1...3 { _ = try handle(table.insert(entry(UInt32(id)))) }
        XCTAssertEqual(table.insert(entry(4)), .full)
        XCTAssertEqual(table.len, 3)
        XCTAssertEqual(table.map(\.callback_id), [1, 2, 3])
    }

    func testARemovedEntryIsGoneAndItsHandleIsStaleOnceTheSlotIsReused() throws {
        var table = BcIndexedTable()
        let old = try handle(table.insert(entry(10)))
        XCTAssertTrue(table.remove(old))
        XCTAssertFalse(table.remove(old), "a slot already free is not removed twice")
        XCTAssertNil(table.get(old))
        XCTAssertEqual(table.len, 0)
        let reused = try handle(table.insert(entry(11)))
        XCTAssertEqual(reused.slot, old.slot)
        XCTAssertNotEqual(reused.generation, old.generation, "a reused slot is a new generation")
        XCTAssertNil(table.get(old), "the handle taken before the removal finds nothing")
        XCTAssertFalse(table.remove(old))
        XCTAssertEqual(table.get(reused), entry(11))
    }

    func testAHandleIsItsSlotAndItsGenerationPacked() {
        let handle = BcIndexedTableHandle.pack(slot: 0x0102, generation: 0x0304)
        XCTAssertEqual(handle.raw, 0x0304_0102)
        XCTAssertEqual(handle.slot, 0x0102)
        XCTAssertEqual(handle.generation, 0x0304)
        // A part past 16 bits is cut, not carried into the other.
        XCTAssertEqual(BcIndexedTableHandle.pack(slot: 0x1_0001, generation: 0x1_0002).raw, 0x0002_0001)
    }

    func testTheIndexFindsTheFirstOccupiedSlotWhoseFieldIsTheKey() throws {
        var table = BcIndexedTable()
        _ = try handle(table.insert(entry(7)))
        let wanted = try handle(table.insert(entry(8)))
        _ = try handle(table.insert(entry(8)))
        XCTAssertEqual(table.findByIndex(8), wanted)
        XCTAssertNil(table.findByIndex(9))
        table.remove(wanted)
        XCTAssertNotEqual(table.findByIndex(8), wanted, "the next slot that holds the key is found")
    }

    func testIterationWalksTheOccupiedSlotsInSlotOrder() throws {
        var table = BcIndexedTable()
        let a = try handle(table.insert(entry(1)))
        _ = try handle(table.insert(entry(2)))
        _ = try handle(table.insert(entry(3)))
        table.remove(a)
        XCTAssertEqual(table.map(\.callback_id), [2, 3])
        _ = try handle(table.insert(entry(4)))
        XCTAssertEqual(table.map(\.callback_id), [4, 2, 3], "the freed slot is the first taken")
    }

    func testACollectionIsAValue() throws {
        var table = BcIndexedTable()
        let first = try handle(table.insert(entry(1)))
        var copy = table
        copy.remove(first)
        XCTAssertEqual(table.len, 1)
        XCTAssertEqual(copy.len, 0)
    }

    func testOldestWinsEvictsTheOldestEntryAndNeverRefuses() {
        var table = BcRecentTable()
        guard case .ok(let first) = table.insert(entry(1)),
            case .ok(let second) = table.insert(entry(2)),
            case .ok(let third) = table.insert(entry(3))
        else {
            return XCTFail("an insert under oldest-wins is always ok")
        }
        XCTAssertEqual(table.len, 2)
        XCTAssertEqual(third.slot, first.slot, "the oldest entry's slot is taken")
        XCTAssertNotEqual(third.generation, first.generation)
        XCTAssertNil(table.get(first), "the evicted entry's handle is stale")
        XCTAssertEqual(table.get(second), entry(2))
        XCTAssertEqual(table.get(third), entry(3))
        XCTAssertEqual(table.map(\.callback_id), [3, 2])
    }
}
