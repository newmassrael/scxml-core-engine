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
        _ = try v.list(bound: bound) { try readDescribed(inner, $0) }
    } else if let fields = described["record"] as? [String: Any] {
        try v.record { record in
            for (id, field) in fields {
                try readDescribed(field, try record.field(id))
            }
        }
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
            try readDescribed(type, try PayloadFields.decode(data).field("v"))
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

/// What a generated reader of an enum field answers for a name its enum does not
/// declare: the sentence it always wrote, and the code and pointer beside it.
@Test func aNameNoVariantDeclaresIsRefusedAsUnknownVariant() throws {
    let refusal = try PayloadFields.decode(#"{"status":"x"}"#).field("status").unknownVariant("x", alias: "Status")
    #expect(refusal.reason == "'status' (x) is not a variant of Status")
    #expect(refusal.code == .unknownVariant)
    #expect(refusal.pointer == "/status")
    #expect(refusal.bound == nil)
}
