// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceForgeRuntime

// A table is read by exact match: the value at the key's own index, and nothing at a
// key the table does not list. A zero is a key and a value like any other.

@Test func aKeyTheTableListsAnswersItsOwnValue() {
    let keys: [UInt8] = [1, 2, 3]
    let values: [Int32] = [100, 200, 300]
    #expect(Lookup.find(keys, values, 1) == 100)
    #expect(Lookup.find(keys, values, 3) == 300)
}

@Test func aKeyTheTableDoesNotListAnswersNothing() {
    let keys: [UInt8] = [1, 2]
    let values: [Int32] = [100, 200]
    #expect(Lookup.find(keys, values, 0) == nil)
    #expect(Lookup.find(keys, values, 255) == nil)
    #expect(Lookup.find([UInt8](), [Int32](), 1) == nil)
}

@Test func aZeroIsAKeyAndAValueNotAnAbsence() {
    #expect(Lookup.find([0, 5] as [Int32], [0, 7] as [Int32], 0) == 0)
    #expect(Lookup.find([0, 5] as [Int32], [0, 7] as [Int32], 5) == 7)
}

@Test func anyEquatableTypeIsAKey() {
    let keys = ["open", "held"]
    let values = [true, false]
    #expect(Lookup.find(keys, values, "held") == false)
    #expect(Lookup.find(keys, values, "done") == nil)
}
