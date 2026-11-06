// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// A payload value of any bounded type is read at its path, or refused there before
// anything of it is kept (docs/adr/0086, sections 4 and 14.3): every element kind, at
// list depths 1 to 3, each bound 2, with an empty value, a maximal one, one past the
// bound at the innermost and the outermost list, a bad element, `null` where the
// element is not optional, an unknown key and an older revision. The oracle is the
// construction: each case is built with the value or the path it must come to. A
// maximal value is written at exactly the size the calculus gives it, and reads back
// as itself; a value a host built is refused before a raise encodes it.

private let crossBound = 2
private let textBound = 3
private let controls = String(repeating: "\u{1}", count: textBound)
private let enumNames = ["low", "medium", "high"]

/// The element kinds. `rec` is `{a: uint8, b: string[3]}`, whose schema's later
/// revision added `c: uint8` with the default 9; `recList` is `{tags:
/// list<string[3]>[2]}`.
private enum Kind: CaseIterable {
    case u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, bool, enumeration, text, bytes, opt, rec, recList
}

private indirect enum Ty {
    case kind(Kind)
    case list(Ty)
}

private struct Member: Equatable {
    let id: String
    let value: Held
}

private enum Held: Equatable {
    case u(UInt64)
    case i(Int64)
    case f(Double)
    case b(Bool)
    case s(String)
    case bytes([UInt8])
    case absent
    case rec([Member])
    case list([Held])
}

private func nested(_ kind: Kind, _ depth: Int) -> Ty {
    var ty = Ty.kind(kind)
    for _ in 0..<depth {
        ty = .list(ty)
    }
    return ty
}

/// The reader of `ty`, one call per constructor, as the generator writes it.
private func read(_ ty: Ty, _ v: PayloadValue) throws -> Held {
    switch ty {
    case .list(let elem):
        return .list(try v.list(bound: crossBound) { try read(elem, $0) })
    case .kind(let kind):
        switch kind {
        case .u8: return .u(UInt64(try v.unsigned() as UInt8))
        case .u16: return .u(UInt64(try v.unsigned() as UInt16))
        case .u32: return .u(UInt64(try v.unsigned() as UInt32))
        case .u64: return .u(try v.unsigned64())
        case .i8: return .i(Int64(try v.signed() as Int8))
        case .i16: return .i(Int64(try v.signed() as Int16))
        case .i32: return .i(Int64(try v.signed() as Int32))
        case .i64: return .i(try v.signed64())
        case .f32: return .f(Double(try v.float32()))
        case .f64: return .f(try v.float64())
        case .bool: return .b(try v.truth())
        case .enumeration:
            let name = try v.name()
            guard enumNames.contains(name) else {
                throw v.unknownVariant(name, alias: "Level")
            }
            return .s(name)
        case .text: return .s(try v.text(bound: textBound))
        case .bytes: return .bytes(try v.bytes(bound: textBound))
        case .opt: return try v.optional { Held.u(UInt64(try $0.unsigned() as UInt8)) } ?? .absent
        case .rec:
            return try v.record(defaults: #"{"c":9}"#) { f in
                .rec([
                    Member(id: "a", value: .u(UInt64(try f.field("a").unsigned() as UInt8))),
                    Member(id: "b", value: .s(try f.field("b").text(bound: textBound))),
                    Member(id: "c", value: .u(UInt64(try f.field("c").unsigned() as UInt8))),
                ])
            }
        case .recList:
            return try v.record { f in
                .rec([Member(id: "tags", value: try read(.list(.kind(.text)), f.field("tags")))])
            }
        }
    }
}

/// The maximal value of `ty`: every list full, every text at its bound of control
/// characters, every number at its longest spelling.
private func maximal(_ ty: Ty) -> Held {
    switch ty {
    case .list(let elem):
        return .list(Array(repeating: maximal(elem), count: crossBound))
    case .kind(let kind):
        switch kind {
        case .u8: return .u(UInt64(UInt8.max))
        case .u16: return .u(UInt64(UInt16.max))
        case .u32: return .u(UInt64(UInt32.max))
        case .u64: return .u(UInt64.max)
        case .i8: return .i(Int64(Int8.min))
        case .i16: return .i(Int64(Int16.min))
        case .i32: return .i(Int64(Int32.min))
        case .i64: return .i(Int64.min)
        // Seventeen digits after `0.00000`: the longest spelling of a number, which only the
        // decimal band from 1e-6 to 1e-5 reaches.
        case .f32: return .f(Double(-Float(bitPattern: 0x35a5_b36d)))
        case .f64: return .f(-Double(bitPattern: 0x3eb4_b66d_c01e_c6fb))
        case .bool: return .b(false)
        case .enumeration: return .s("medium")
        case .text: return .s(controls)
        case .bytes: return .bytes([1, 1, 1])
        // `null` (4 bytes) is longer than any uint8's digits.
        case .opt: return .absent
        case .rec:
            return .rec([
                Member(id: "a", value: .u(255)), Member(id: "b", value: .s(controls)), Member(id: "c", value: .u(255)),
            ])
        case .recList:
            return .rec([Member(id: "tags", value: .list([.s(controls), .s(controls)]))])
        }
    }
}

private func fieldType(_ id: String) -> Ty {
    switch id {
    case "b": return .kind(.text)
    case "tags": return .list(.kind(.text))
    default: return .kind(.u8)
    }
}

/// `held` as JSON, written by the runtime's writers, one call per constructor; a
/// record's members are handed in reverse to show that the writer orders them. A
/// single is written as the double it widens to, as the C11 and Rust writers write it.
private func write(_ ty: Ty, _ held: Held) -> String {
    if case .list(let elem) = ty, case .list(let items) = held {
        return listJSON(items) { write(elem, $0) }
    }
    guard case .kind(let kind) = ty else {
        return "?"
    }
    switch (kind, held) {
    case (.u64, .u(let n)): return WireValue.unsigned64(n).json
    case (.i64, .i(let n)): return WireValue.signed64(n).json
    case (_, .u(let n)): return WireValue.integer(Int64(n)).json
    case (_, .i(let n)): return WireValue.integer(n).json
    case (_, .f(let x)): return WireValue.real(x).json
    case (_, .b(let b)): return WireValue.truth(b).json
    case (_, .s(let s)): return WireValue.text(s).json
    case (_, .bytes(let bytes)): return WireValue.bytes(bytes).json
    case (_, .absent): return WireValue.null.json
    case (_, .rec(let members)):
        return recordJSON(members.reversed().map { PayloadPair.json($0.id, rawJSON: write(fieldType($0.id), $0.value)) })
    case (_, .list): return "?"
    }
}

/// The largest JSON text of `ty` (docs/adr/0086, section 2.1).
private func largest(_ ty: Ty) -> Int {
    func list(_ size: Int) -> Int {
        2 + crossBound * size + (crossBound - 1)
    }
    func record(_ fields: [(String, Int)]) -> Int {
        var total = 2 + fields.count - 1
        for (id, size) in fields {
            total += id.utf8.count + 2 + 1 + size
        }
        return total
    }
    let text = 2 + 6 * textBound
    switch ty {
    case .list(let elem):
        return list(largest(elem))
    case .kind(let kind):
        switch kind {
        case .u8: return 3
        case .u16: return 5
        case .u32: return 10
        case .u64, .i64: return 22
        case .i8: return 4
        case .i16: return 6
        case .i32: return 11
        case .f32, .f64: return 25
        case .bool: return 5
        case .enumeration: return 2 + "medium".utf8.count
        case .text, .bytes: return text
        case .opt: return 4
        case .rec: return record([("a", 3), ("b", text), ("c", 3)])
        case .recList: return record([("tags", list(text))])
        }
    }
}

private func data(_ list: String) -> String {
    #"{"v":"# + list + #","zz":{"unknown":[1,null]}}"#
}

/// The path of the innermost list along the last index at each depth.
private func innermost(_ depth: Int) -> String {
    "v" + String(repeating: "/\(crossBound - 1)", count: depth - 1)
}

/// `held` with the element at the end of the last index at every depth replaced by
/// `leaf`.
private func withLastLeaf(_ ty: Ty, _ held: Held, _ leaf: String) -> String {
    guard case .list(let elem) = ty, case .list(let items) = held else {
        return leaf
    }
    var parts = items.map { write(elem, $0) }
    parts[parts.count - 1] = withLastLeaf(elem, items[items.count - 1], leaf)
    return "[" + parts.joined(separator: ",") + "]"
}

/// The maximal value with one more element in the list at `level` (1 the
/// outermost) along the last index.
private func onePast(_ ty: Ty, _ held: Held, _ level: Int) -> String {
    guard case .list(let elem) = ty, case .list(let items) = held else {
        return "?"
    }
    var parts = items.map { write(elem, $0) }
    if level == 1 {
        parts.append(write(elem, items[0]))
    } else {
        parts[parts.count - 1] = onePast(elem, items[items.count - 1], level - 1)
    }
    return "[" + parts.joined(separator: ",") + "]"
}

private func setLastLeaf(_ held: Held, _ leaf: Held) -> Held {
    guard case .list(var items) = held else {
        return leaf
    }
    items[items.count - 1] = setLastLeaf(items[items.count - 1], leaf)
    return .list(items)
}

/// A bad element of `kind`, the path below the element it is refused at, and the
/// sentence it is refused with.
private func bad(_ kind: Kind) -> (String, String, String) {
    switch kind {
    case .u8, .u16, .u32, .opt: return ("256000000000", "", "does not fit the width")
    case .u64: return (#""-1""#, "", "is not a whole number written as digits")
    case .i8, .i16, .i32: return ("1.5", "", "is not a whole number")
    case .i64: return ("9007199254740992", "", "past 2^53 - 1")
    case .f32: return ("1e39", "", "does not fit the width")
    case .f64: return ("true", "", "is not a number")
    case .bool: return ("0", "", "is not a truth value")
    case .enumeration: return (#""lowest""#, "", "is not a variant of Level")
    case .text: return (#""abcd""#, "", "is 4 bytes, past the 3 its type declares")
    case .bytes: return (#""Ā""#, "", "carries a character above U+00FF")
    case .rec: return (#"{"a":"x","b":""}"#, "/a", "is not a number")
    case .recList: return (#"{"tags":["","","x"]}"#, "/tags", "holds 3 elements, past the 2")
    }
}

private func lift(_ ty: Ty, _ text: String) throws -> Held {
    try read(ty, PayloadFields.decode(text).field("v"))
}

/// What a case must come to.
private enum Want {
    case reads(Held)
    case refusedAt(String, String)
}

@Test func everyElementKindAtEveryDepthIsReadOrRefusedAtItsPath() {
    var failures: [String] = []
    var cases = 0
    func expect(_ label: String, _ want: Want, _ run: () throws -> Held) {
        cases += 1
        let got = Result { try run() }
        switch (want, got) {
        case (.reads(let value), .success(let held)) where held == value:
            return
        case (.refusedAt(let path, let sentence), .failure(let error)):
            if let refusal = error as? PayloadRefusal, refusal.reason.contains("'\(path)'"),
                refusal.reason.contains(sentence)
            {
                return
            }
        default:
            break
        }
        failures.append("\(label): read \(got), the oracle says \(want)")
    }
    let pastBound = "holds 3 elements, past the 2 its type declares"
    for kind in Kind.allCases {
        for depth in 1...3 {
            let ty = nested(kind, depth)
            let full = maximal(ty)
            let leaf = "\(innermost(depth))/\(crossBound - 1)"
            let label = { (name: String) in "\(kind) at depth \(depth), \(name)" }
            expect(label("empty"), .reads(.list([]))) { try lift(ty, data("[]")) }
            expect(label("maximal"), .reads(full)) { try lift(ty, data(write(ty, full))) }
            expect(label("one past the innermost bound"), .refusedAt(innermost(depth), pastBound)) {
                try lift(ty, data(onePast(ty, full, depth)))
            }
            expect(label("one past the outermost bound"), .refusedAt("v", pastBound)) {
                try lift(ty, data(onePast(ty, full, 1)))
            }
            if kind == .opt {
                // The maximal list of optionals is every element absent; a present one
                // among them is read as present.
                expect(label("a present element among absent ones"), .reads(setLastLeaf(full, .u(7)))) {
                    try lift(ty, data(withLastLeaf(ty, full, "7")))
                }
            } else {
                expect(label("null where the element is not optional"),
                       .refusedAt(leaf, "is null, and its type is not optional")) {
                    try lift(ty, data(withLastLeaf(ty, full, "null")))
                }
            }
            let (text, below, sentence) = bad(kind)
            expect(label("a bad element"), .refusedAt(leaf + below, sentence)) {
                try lift(ty, data(withLastLeaf(ty, full, text)))
            }
            // A key no schema declares, inside a record element and beside the field,
            // is stepped over.
            let unknown =
                kind == .rec
                ? withLastLeaf(ty, full, #"{"zz":[{"q":1}],"c":255,"b":"\u0001\u0001\u0001","a":255}"#)
                : write(ty, full)
            expect(label("an unknown key"), .reads(full)) { try lift(ty, data(unknown)) }
            // An older revision: a record element without the field its schema's later
            // revision added reads that field's default, and a payload without a list a
            // later revision added reads it empty.
            if kind == .rec {
                let older = setLastLeaf(
                    full,
                    .rec([Member(id: "a", value: .u(1)), Member(id: "b", value: .s("")), Member(id: "c", value: .u(9))])
                )
                expect(label("an element of an older revision"), .reads(older)) {
                    try lift(ty, data(withLastLeaf(ty, full, #"{"a":1,"b":""}"#)))
                }
            } else {
                expect(label("a delivery of the revision before the list"), .reads(.list([]))) {
                    try read(ty, PayloadFields.decode(#"{"zz":1}"#, defaults: #"{"v":[]}"#).field("v"))
                }
            }
        }
    }
    #expect(cases == 17 * 3 * 8)
    #expect(failures.isEmpty, "\(failures.joined(separator: "\n"))")
}

@Test func aMaximalValueIsWrittenAtExactlyItsLargestSize() {
    var failures: [String] = []
    for kind in Kind.allCases {
        for depth in 0...3 {
            let ty = nested(kind, depth)
            let full = maximal(ty)
            let text = write(ty, full)
            let size = largest(ty)
            let written = text.utf8.count
            if written != size {
                failures.append("\(kind) at depth \(depth): \(written) bytes, the calculus says \(size): \(text)")
            }
            if (try? lift(ty, data(text))) != full {
                failures.append("\(kind) at depth \(depth): written and not read back as itself: \(text)")
            }
        }
    }
    #expect(failures.isEmpty, "\(failures.joined(separator: "\n"))")
}

@Test func aTopLevelTextAndByteStringAreHeldToTheirBoundWhenRead() throws {
    let fields = try PayloadFields.decode(#"{"note": "abcde", "blob": "abcde", "t": "éé", "n": null}"#)
    let note = #expect(throws: PayloadRefusal.self) { try fields.field("note").text(bound: 4) }
    #expect(note?.reason == "'note' is 5 bytes, past the 4 its type declares")
    let blob = #expect(throws: PayloadRefusal.self) { try fields.bytes("blob", capacity: 4) }
    #expect(blob?.reason == "'blob' is 5 bytes, past the 4 its type declares")
    // A text's bound counts UTF-8 bytes: two characters, four bytes.
    #expect(throws: PayloadRefusal.self) { try fields.field("t").text(bound: 3) }
    #expect(try fields.field("t").text(bound: 4) == "éé")
    let null = #expect(throws: PayloadRefusal.self) { try fields.field("n").signed64() }
    #expect(null?.reason == "'n' is null, and its type is not optional")
    let gone = #expect(throws: PayloadRefusal.self) { try fields.field("gone") }
    #expect(gone?.reason == "the event's data has no 'gone'")
}

@Test func aListPastItsBoundIsRefusedBeforeAnElementIsRead() throws {
    let fields = try PayloadFields.decode(#"{"v": [1, 2, "x"], "s": "x"}"#)
    var asked = 0
    var refusal: PayloadRefusal?
    do {
        _ = try fields.field("v").list(bound: 2) { (element: PayloadValue) -> UInt8 in
            asked += 1
            return try element.unsigned()
        }
    } catch let error as PayloadRefusal {
        refusal = error
    }
    #expect(refusal?.reason == "'v' holds 3 elements, past the 2 its type declares")
    #expect(asked == 0)
    let notList = #expect(throws: PayloadRefusal.self) {
        try fields.field("s").list(bound: 2) { try $0.unsigned() as UInt8 }
    }
    #expect(notList?.reason == "'s' is not a list")
}

/// A list short of its lower count is refused as `tooFew`, naming the count, before an
/// element is read (docs/adr/0086, section 15.5).
@Test func aListShortOfItsLowerCountIsRefusedBeforeAnElementIsRead() throws {
    let fields = try PayloadFields.decode(#"{"v": ["x"], "w": [1, 2], "e": []}"#)
    var asked = 0
    var refusal: PayloadRefusal?
    do {
        _ = try fields.field("v").list(least: 2, bound: 3) { (element: PayloadValue) -> UInt8 in
            asked += 1
            return try element.unsigned()
        }
    } catch let error as PayloadRefusal {
        refusal = error
    }
    #expect(refusal?.reason == "'v' holds 1 elements, fewer than the 2 its type declares")
    #expect(refusal?.code == .tooFew)
    #expect(refusal?.pointer == "/v")
    #expect(refusal?.bound == 2)
    #expect(asked == 0)
    #expect(try fields.field("w").list(least: 2, bound: 3) { try $0.unsigned() as UInt8 } == [1, 2])
    let empty = #expect(throws: PayloadRefusal.self) {
        try fields.field("e").list(least: 1, bound: 3) { try $0.unsigned() as UInt8 }
    }
    #expect(empty?.code == .tooFew)
    #expect(try fields.field("e").list(bound: 3) { try $0.unsigned() as UInt8 } == [])
}

/// The writer's check refuses a list short of the place's lower count, the path in front
/// (docs/adr/0086, section 15.6).
@Test func aListWrittenShortOfItsLowerCountIsRefused() throws {
    try checkList("items", [1, 2], least: 2, bound: 3) { _, _ in }
    let short = #expect(throws: BoundExceeded.self) {
        try checkList("items/1/tags", [Int](), least: 1, bound: 3) { _, _ in }
    }
    #expect(short?.reason == "'items/1/tags' holds 0 elements, fewer than the 1 its type declares")
    let past = #expect(throws: BoundExceeded.self) {
        try checkList("items", [1, 2, 3, 4], least: 2, bound: 3) { _, _ in }
    }
    #expect(past?.reason == "'items' holds 4 elements, past the 3 its type declares")
}

@Test func aRecordValueNamesItsMissingFieldAndReadsItsLaterFieldsFromDefaults() throws {
    let fields = try PayloadFields.decode(#"{"r": {"a": 1, "zz": [null]}, "s": 5}"#)
    let missing = #expect(throws: PayloadRefusal.self) { try fields.field("r").record { try $0.field("b") } }
    #expect(missing?.reason == "the event's data has no 'r/b'")
    #expect(try fields.field("r").record(defaults: #"{"b": 9}"#) { try $0.field("b").unsigned() as UInt8 } == 9)
    let notRecord = #expect(throws: PayloadRefusal.self) { try fields.field("s").record { try $0.field("a") } }
    #expect(notRecord?.reason == "'s' is not a record")
    #expect(throws: PayloadRefusal.self) { try fields.field("r").record(defaults: "[]") { try $0.field("a") } }
}

@Test func aListAndARecordAreWrittenInTheirOrderAndByTheirKeyBytes() {
    #expect(listJSON([UInt8]()) { String($0) } == "[]")
    #expect(listJSON([UInt8(2), 1]) { String($0) } == "[2,1]")
    #expect(
        recordJSON([PayloadPair.integer("b", 2), PayloadPair.integer("aa", 1), PayloadPair.integer("B", 3)])
            == #"{"B":3,"aa":1,"b":2}"#
    )
}

@Test func aValueAHostBuiltIsCheckedAtEveryDepthBeforeItIsRaised() throws {
    let tags = { (path: String, items: [String]) throws in
        try checkList(path, items, bound: 2) { try checkText($0, $1, bound: 3) }
    }
    // A value within every bound is checked and not refused.
    try checkList("items", [["a"], ["abc", "ab"]], bound: 3, tags)
    let outer = #expect(throws: BoundExceeded.self) { try checkList("items", [[], [], [], []], bound: 3, tags) }
    #expect(outer?.reason == "'items' holds 4 elements, past the 3 its type declares")
    let inner = #expect(throws: BoundExceeded.self) { try checkList("items", [["a"], ["a", "b", "c"]], bound: 3, tags) }
    #expect(inner?.reason == "'items/1' holds 3 elements, past the 2 its type declares")
    let text = #expect(throws: BoundExceeded.self) { try checkList("items", [["a"], ["a", "abcd"]], bound: 3, tags) }
    #expect(text?.reason == "'items/1/1' is 4 bytes, past the 3 its type declares")
    let blob = #expect(throws: BoundExceeded.self) { try checkBytes("blob", [1, 2, 3, 4, 5], bound: 4) }
    #expect(blob?.reason == "'blob' is 5 bytes, past the 4 its type declares")
    var checked: [String] = []
    try checkOptional("due", String?.none) { path, _ in checked.append(path) }
    try checkOptional("due", "x") { path, _ in checked.append(path) }
    #expect(checked == ["due"])
}
