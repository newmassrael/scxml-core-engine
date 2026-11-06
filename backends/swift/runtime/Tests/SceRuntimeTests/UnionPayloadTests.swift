// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import Testing

@testable import SceRuntime

// A union value is read tag first, then the record of the case its tag names, from the
// same object, and written as one object with its tag among the fields of its case
// (docs/adr/0092, sections 2 and 4).

/// The tag of `UnionShape`, read as a generated reader of an enum reads it.
private enum UnionShapeKind {
    case dot, circle, disc
}

/// A union over `UnionShapeKind` whose `circle` and `disc` both hold one record.
private enum UnionShape: Equatable {
    case dot
    case circle(UInt16)
    case disc(UInt16)
}

private func readUnionShape(_ data: String) throws -> UnionShape {
    try PayloadFields.decode(data).field("s").union(
        tag: "kind",
        { tag -> UnionShapeKind in
            let name = try tag.name()
            switch name {
            case "dot": return .dot
            case "circle": return .circle
            case "disc": return .disc
            default: throw tag.unknownVariant(name, alias: "ShapeKind")
            }
        },
        { kind, object -> UnionShape in
            switch kind {
            case .dot: return .dot
            case .circle: return .circle(try object.record { try $0.unsigned("radius") })
            case .disc: return .disc(try object.record { try $0.unsigned("radius") })
            }
        })
}

/// A union's tag is read first, wherever it stands, and names the case whose record is
/// read from the same object: two cases that hold one record stay apart by their tag, a
/// key or a `null` the case does not declare is stepped over, and each refusal points at
/// the tag or at the case's field.
@Test func aUnionReadsItsTagFirstAndThenTheRecordOfItsCase() throws {
    #expect(try readUnionShape(#"{"s":{"kind":"dot"}}"#) == .dot)
    #expect(try readUnionShape(#"{"s":{"radius":2,"kind":"disc"}}"#) == .disc(2))
    #expect(try readUnionShape(#"{"s":{"kind":"circle","radius":2,"width":null,"text":"x"}}"#) == .circle(2))
    #expect(try readUnionShape(#"{"s":{"kind":"dot","radius":null}}"#) == .dot)
    let refused: [(String, RefusalCode, String)] = [
        (#"{"s":[]}"#, .wrongKind, "/s"),
        (#"{"s":null}"#, .nullNotOptional, "/s"),
        (#"{"s":{"radius":2}}"#, .missing, "/s/kind"),
        (#"{"s":{"kind":null,"radius":null}}"#, .nullNotOptional, "/s/kind"),
        (#"{"s":{"kind":3,"radius":"x"}}"#, .wrongKind, "/s/kind"),
        (#"{"s":{"kind":"oval","radius":2}}"#, .unknownVariant, "/s/kind"),
        (#"{"s":{"kind":"circle","width":2}}"#, .missing, "/s/radius"),
        (#"{"s":{"kind":"disc","radius":null}}"#, .nullNotOptional, "/s/radius"),
    ]
    for (data, code, pointer) in refused {
        do {
            _ = try readUnionShape(data)
            Issue.record("\(data): read, it is refused as \(code)")
        } catch let refusal as PayloadRefusal {
            #expect(refusal.code == code, "\(data): \(refusal.reason)")
            #expect(refusal.pointer == pointer, "\(data): \(refusal.reason)")
        }
    }
}

/// A union is written as one object, its tag among the fields of its case, every key
/// ascending by its bytes.
@Test func aUnionIsWrittenWithItsTagAmongTheFieldsOfItsCase() {
    #expect(unionJSON(tag: "kind", variant: "none", []) == #"{"kind":"none"}"#)
    #expect(
        unionJSON(tag: "kind", variant: "relativeWeeks", [.integer("count", 2)])
            == #"{"count":2,"kind":"relativeWeeks"}"#)
    #expect(
        unionJSON(tag: "kind", variant: "absolute", [.integer("year", 2027), .integer("month", 3), .integer("day", 3)])
            == #"{"day":3,"kind":"absolute","month":3,"year":2027}"#)
}
