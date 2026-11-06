// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Foundation
import Testing

@testable import SceRuntime

// tests/json_text/payload_refusal.json, read where it is: every case read through
// this runtime's payload reader, the code, pointer and bound of each refusal held to
// what the table says, and each case it accepts read (docs/adr/0087, section 9.1,
// item 3). The table is the one every runtime's reader is held to.

/// `v` read as the type `type` the table describes, one call per constructor, as a
/// generated reader reads it.
private func readDescribed(_ type: Any, _ v: PayloadValue) throws {
    if let scalar = type as? String {
        switch scalar {
        case "uint8": _ = try v.unsigned() as UInt8
        case "uint16": _ = try v.unsigned() as UInt16
        case "uint32": _ = try v.unsigned() as UInt32
        case "uint64": _ = try v.unsigned64()
        case "int8": _ = try v.signed() as Int8
        case "int16": _ = try v.signed() as Int16
        case "int32": _ = try v.signed() as Int32
        case "int64": _ = try v.signed64()
        case "float32": _ = try v.float32()
        case "float64": _ = try v.float64()
        case "bool": _ = try v.truth()
        case "string": _ = try v.name()
        default: Issue.record("the table names no type \(scalar)")
        }
        return
    }
    if let names = type as? [String] {
        let name = try v.name()
        guard names.contains(name) else {
            throw v.unknownVariant(name, alias: "Described")
        }
        return
    }
    guard let described = type as? [String: Any] else {
        Issue.record("a type of the table is a name, a list or an object: \(type)")
        return
    }
    if let inner = described["optional"] {
        _ = try v.optional { try readDescribed(inner, $0) }
    } else if let bound = described["string"] as? Int {
        _ = try v.text(bound: bound)
    } else if let bound = described["bytes"] as? Int {
        _ = try v.bytes(bound: bound)
    } else if let inner = described["list"], let bound = described["max"] as? Int {
        // The lower count, where the case states one (docs/adr/0086, section 15).
        let least = described["min"] as? Int ?? 0
        _ = try v.list(least: least, bound: bound) { try readDescribed(inner, $0) }
    } else if let fields = described["record"] as? [String: Any] {
        try v.record { record in
            for (id, field) in fields {
                try readDescribed(field, try record.field(id))
            }
        }
    } else if let union = described["union"] as? [String: Any], let tag = union["tag"] as? String,
        let cases = union["cases"] as? [String: Any]
    {
        // The tag's enum is the cases' names; an empty case is null, any other the
        // record it holds (docs/adr/0092, section 4).
        try v.union(
            tag: tag,
            { named in
                let name = try named.name()
                guard cases[name] != nil else {
                    throw named.unknownVariant(name, alias: "Described")
                }
                return name
            },
            { name, object in
                if let record = cases[name], !(record is NSNull) {
                    try readDescribed(record, object)
                }
            })
    } else {
        Issue.record("a type of the table names no constructor: \(described)")
    }
}

@Test func everyCaseOfTheSharedTableIsRefusedWithItsCodePointerAndBound() throws {
    let cases = try sharedCases("json_text/payload_refusal.json")
    #expect(cases.count >= 40, "the table lost cases: \(cases.count)")
    for c in cases {
        let name = try #require(c["name"] as? String)
        let data = try #require(c["data"] as? String)
        let type = try #require(c["type"])
        var got: PayloadRefusal?
        do {
            // A case that gives a depth is read as a `<sce:lift>` of the record of the
            // one field `v` reads it (docs/adr/0087, section 2.1), its pointer held to
            // `pathBound` bytes, the standard refusal's 256 where the case gives none.
            let pathBound = c["pathBound"] as? Int ?? 256
            let fields =
                if let depth = c["depth"] as? Int {
                    try PayloadFields.lift(data, depth: depth, pathBound: pathBound, defaults: nil)
                } else {
                    try PayloadFields.decode(data)
                }
            try readDescribed(type, try fields.field("v"))
        } catch let refusal as PayloadRefusal {
            got = refusal
        }
        guard let want = c["refusal"] as? [String: Any] else {
            #expect(got == nil, "\(name): refused (\(got?.reason ?? "")), the table reads it")
            continue
        }
        let refusal = try #require(got, "\(name): read, the table refuses it")
        let codeName = try #require(want["code"] as? String)
        let code = try #require(RefusalCode(declaredName: codeName))
        let pointer = try #require(want["pointer"] as? String)
        let bound = (want["bound"] as? Int).map { UInt32($0) }
        #expect(refusal.code == code, "\(name): \(refusal.reason)")
        #expect(refusal.pointer == pointer, "\(name): \(refusal.reason)")
        #expect(refusal.bound == bound, "\(name): \(refusal.reason)")
    }
}

/// The codes are the variants `sce:std/lift/refusal_code.scxml` declares, by the same
/// names and at the same numbers.
@Test func everyCodeIsTheVariantTheStandardEnumDeclares() throws {
    let document = try String(
        contentsOf: repositoryRoot().appendingPathComponent("stdlib/lift/refusal_code.scxml"),
        encoding: .utf8)
    #expect(document.components(separatedBy: "<sce:variant ").count - 1 == RefusalCode.allCases.count)
    for code in RefusalCode.allCases {
        let variant = "<sce:variant name=\"\(code.declaredName)\" value=\"\(code.rawValue)\"/>"
        #expect(document.contains(variant), "\(code): \(variant)")
        #expect(RefusalCode(declaredName: code.declaredName) == code)
    }
}

/// RFC 6901: `/` before each token, `~` written `~0` and `/` written `~1`, `~`
/// escaped first so that `~1` in a name is not read back as `/`.
@Test func aPointerEscapesTildeAndSolidusInEachToken() {
    #expect(pointerBelow("", "items") == "/items")
    #expect(pointerBelow("/items", "2") == "/items/2")
    #expect(pointerBelow("", "a~b/c") == "/a~0b~1c")
    #expect(pointerBelow("", "~1") == "/~01")
    #expect(pointerBelow("", "") == "/")
}

/// A lift refuses a container past its depth over the brackets alone, before the
/// text is read as JSON, and names the container it refused, held to the room of the
/// refusal's path by whole tokens (docs/adr/0087, sections 2.1 and 2.4).
@Test func aLiftRefusesAContainerPastItsDepthBeforeItReadsTheText() {
    let cases: [(String, Int, Int, String?)] = [
        (#"{"title":"a","count":1}"#, 1, 256, nil),
        (#"{"title":["a"]}"#, 1, 256, "/title"),
        (#"{"extra":{"a":1}}"#, 1, 256, "/extra"),
        ("[[1]]", 1, 256, "/0"),
        ("[1,2,[3]]", 1, 256, "/2"),
        (#"{"extra":[[[[[[["#, 1, 256, "/extra"),
        (#"{"title":"[[{\"[","count":1}"#, 1, 256, nil),
        (#"{"title":{"a":1}}"#, 2, 256, nil),
        (#"{"title":"[["#, 1, 256, nil),
        (#"{"title":"x","extra":[1]}"#, 1, 256, "/extra"),
        (#"{"a":{"b":[1]}}"#, 2, 256, "/a/b"),
        (#"{"a":[0,{"b":[1]}]}"#, 3, 256, "/a/1/b"),
        (#"{"a/b~c":[1]}"#, 1, 256, "/a~1b~0c"),
        ("{[1]}", 1, 256, ""),
        (#"{"a":{"abcdef":[1]}}"#, 2, 4, "/a"),
        (#"{"a":{"bc":[1]}}"#, 2, 5, "/a/bc"),
    ]
    for (text, depth, pathBound, pointer) in cases {
        let refusal = liftTooDeep(text, depth: depth, pathBound: pathBound)
        guard let pointer else {
            #expect(refusal == nil, "\(text): \(refusal?.reason ?? "")")
            continue
        }
        #expect(refusal?.code == .tooDeep, "\(text)")
        #expect(refusal?.pointer == pointer, "\(text)")
        #expect(refusal?.bound == UInt32(depth), "\(text)")
    }
}

/// A lift reads its record as a delivery's is read, or answers its refusal as data.
@Test func aLiftReadsARecordOrAnswersItsRefusal() throws {
    let read = { (fields: PayloadFields) throws -> [UInt8] in
        [try fields.unsigned("v"), try fields.unsigned("w")]
    }
    let lifted = liftRecord(#"{"v":1}"#, schema: "Pair", depth: 1, pathBound: 256, defaults: #"{"w":7}"#, read: read)
    #expect((try? lifted.get()) == [1, 7])
    let refused: [(String, RefusalCode)] = [
        (" \t", .notJson),
        ("[[", .tooDeep),
        ("[1]", .notObject),
        (#"{"v":1,"v":2}"#, .notJson),
        (#"{"v":256,"w":1}"#, .outOfRange),
        (#"{"w":1}"#, .missing),
    ]
    for (text, code) in refused {
        let outcome = liftRecord(text, schema: "Pair", depth: 1, pathBound: 256, defaults: nil, read: read)
        guard case .failure(let refusal) = outcome else {
            Issue.record("\(text): read, the lift refuses it as \(code)")
            continue
        }
        #expect(refusal.code == code, "\(text): \(refusal.reason)")
    }
}

/// What a generated reader of an enum field answers for a name its enum does not
/// declare: the sentence it always wrote, and the code and pointer beside it.
@Test func aNameNoVariantDeclaresIsRefusedAsUnknownVariant() throws {
    let refusal = try PayloadFields.decode(#"{"status":"x"}"#).field("status").unknownVariant("x", alias: "Status")
    #expect(refusal.reason == "'status' (x) is not a variant of Status")
    #expect(refusal.code == .unknownVariant)
    #expect(refusal.pointer == "/status")
    #expect(refusal.bound == nil)
}
