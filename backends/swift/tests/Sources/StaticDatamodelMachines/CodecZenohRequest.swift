// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_request.scxml:73 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohRequest`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecZenohRequestArms<A0: CodecZenohMsgPutSource, A1: CodecZenohMsgDelSource, A2: CodecZenohQuerySource, D: CodecZenohQuerySource> {
    case codecZenohMsgPut(A0)
    case codecZenohMsgDel(A1)
    case codecZenohQuery(A2)
    case `default`(tag: CodecZenohRequestUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecZenohMsgPut: return 0x1
        case .codecZenohMsgDel: return 0x2
        case .codecZenohQuery: return 0x3
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecZenohRequestArms: Equatable where A0: Equatable, A1: Equatable, A2: Equatable, D: Equatable {}

extension CodecZenohRequestArms: Sendable where A0: Sendable, A1: Sendable, A2: Sendable, D: Sendable {}

/// The arm a value of `CodecZenohRequest` holds, each arm the value of its own codec.
public typealias CodecZenohRequestVariant = CodecZenohRequestArms<CodecZenohMsgPut, CodecZenohMsgDel, CodecZenohQuery, CodecZenohQuery>

/// The tag the catch-all arm of `CodecZenohRequest` holds (docs/adr/0065 §6 R3): one no arm names,
/// within the bits the tag is read from. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecZenohRequestUnknownTag: Equatable, Hashable, Sendable {
    /// The tag.
    public let value: UInt8

    /// The tag `value`, or `nil` where an arm names it or its bits do not hold it.
    public init?(_ value: UInt8) {
        guard Self.admits(value) else { return nil }
        self.value = value
    }

    /// A tag the decode read where no arm names one, or the least one the catch-all admits.
    fileprivate init(admitted value: UInt8) {
        self.value = value
    }

    /// Whether the catch-all may hold `value`.
    public static func admits(_ value: UInt8) -> Bool {
        switch value {
        case 0x1...0x3: return false
        default: return value <= 0x1F
        }
    }
}

/// The elements a value of `CodecZenohRequest` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohRequest.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohRequest`, which conforms itself.
public protocol CodecZenohRequestSource {
    /// What `keyexpr` is presented as: a source of `CodecZenohWireexpr`, written by its encode.
    associatedtype Keyexpr_Source: CodecZenohWireexprSource
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    /// What the body of the arm `codecZenohMsgPut` is presented as: a source of `CodecZenohMsgPut`, written by its encode.
    associatedtype CodecZenohMsgPut_Arm: CodecZenohMsgPutSource
    /// What the body of the arm `codecZenohMsgDel` is presented as: a source of `CodecZenohMsgDel`, written by its encode.
    associatedtype CodecZenohMsgDel_Arm: CodecZenohMsgDelSource
    /// What the body of the arm `codecZenohQuery` is presented as: a source of `CodecZenohQuery`, written by its encode.
    associatedtype CodecZenohQuery_Arm: CodecZenohQuerySource
    /// What the body of the arm `codecZenohQuery` is presented as: a source of `CodecZenohQuery`, written by its encode.
    associatedtype Default_Arm: CodecZenohQuerySource
    var header: UInt8 { get }
    var rid: UInt64 { get }
    var keyexpr: Keyexpr_Source { get }
    var extensions: BoundedList<Extensions_Entry, CodecZenohRequest.ExtensionsBound>? { get }
    var body: CodecZenohRequestArms<CodecZenohMsgPut_Arm, CodecZenohMsgDel_Arm, CodecZenohQuery_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohRequest: Equatable, Sendable, CodecZenohRequestSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2 + CodecZenohWireexpr.frameLength + Swift.min(CodecZenohMsgPut.frameLength, CodecZenohMsgDel.frameLength, CodecZenohQuery.frameLength)

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = nil
    }

    /// The value presents `keyexpr` as a value of its own codec.
    public typealias Keyexpr_Source = CodecZenohWireexpr

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The value presents the body of the arm `codecZenohMsgPut` as a value of its own codec.
    public typealias CodecZenohMsgPut_Arm = CodecZenohMsgPut

    /// The value presents the body of the arm `codecZenohMsgDel` as a value of its own codec.
    public typealias CodecZenohMsgDel_Arm = CodecZenohMsgDel

    /// The value presents the body of the arm `codecZenohQuery` as a value of its own codec.
    public typealias CodecZenohQuery_Arm = CodecZenohQuery

    /// The value presents the body of the arm `codecZenohQuery` as a value of its own codec.
    public typealias Default_Arm = CodecZenohQuery

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x1C

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `N`).
    public var N: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `M`).
    public var M: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var rid: UInt64 = 0

    /// A `CodecZenohWireexpr` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var keyexpr: CodecZenohWireexpr = CodecZenohWireexpr()

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohRequest.ExtensionsBound>? = nil

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohRequestVariant = .codecZenohMsgPut(CodecZenohMsgPut())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, rid: UInt64, keyexpr: Keyexpr_Source, extensions: BoundedList<Extensions_Entry, CodecZenohRequest.ExtensionsBound>?, body: CodecZenohRequestArms<CodecZenohMsgPut_Arm, CodecZenohMsgDel_Arm, CodecZenohQuery_Arm, Default_Arm>) {
        self.header = header
        self.rid = rid
        self.keyexpr = keyexpr
        self.extensions = extensions
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohRequest {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let rid = try reader.readVle(UInt64.self)
        let keyexpr = try CodecZenohWireexpr.decode(&reader, N: UInt8(truncatingIfNeeded: (header >> 5) & 0x1))
        var extensions: BoundedList<CodecZenohExtEntry, CodecZenohRequest.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            while true {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_more = entry.Z
                extensions_items.append(entry)
                if !extensions_more { break }
            }
            extensions = try BoundedList<CodecZenohExtEntry, CodecZenohRequest.ExtensionsBound>(decoding: extensions_items)
        }
        let body: CodecZenohRequestVariant
        let body_tag = try (reader.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x1F
        switch body_tag {
        case 0x1:
            body = .codecZenohMsgPut(try CodecZenohMsgPut.decode(&reader))
        case 0x2:
            body = .codecZenohMsgDel(try CodecZenohMsgDel.decode(&reader))
        case 0x3:
            body = .codecZenohQuery(try CodecZenohQuery.decode(&reader))
        default:
            body = .default(tag: CodecZenohRequestUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohQuery.decode(&reader))
        }
        cursor = reader
        return CodecZenohRequest(header: header, rid: rid, keyexpr: keyexpr, extensions: extensions, body: body)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecZenohRequest` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `M` flag of `header`, where a parent sets it.
        public var headerM: Bool?

        public init(headerM: Bool? = nil) {
            self.headerM = headerM
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohRequestSource` is written,
    /// with no copy into `CodecZenohRequest`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohRequestSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var header_folded = source.header
        header_folded = (header_folded & ~0x20) | ((UInt8(truncatingIfNeeded: Int(CodecZenohWireexpr.inputN(source.keyexpr))) & 0x1) << 5)
        header_folded = (header_folded & ~0x80) | ((source.extensions != nil) ? 0x80 : 0)
        if let set = overrides.headerM { header_folded = set ? (header_folded | 0x40) : (header_folded & ~0x40) }
        var keyexpr_piece = BytesSink()
        try CodecZenohWireexpr.encodeSource(source.keyexpr, into: &keyexpr_piece)
        let keyexpr_bytes = keyexpr_piece.bytes
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        appendVle(source.rid, to: &frame)
        frame += keyexpr_bytes
        if let held = source.extensions {
            var extensions_entries = BytesSink()
            var extensions_left = held.count
            for entry in held {
                extensions_left -= 1
                try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_left != 0))
            }
            frame += extensions_entries.bytes
        }
        var body_arm = BytesSink()
        switch source.body {
        case .codecZenohMsgPut(let arm):
            try CodecZenohMsgPut.encodeSource(arm, into: &body_arm)
        case .codecZenohMsgDel(let arm):
            try CodecZenohMsgDel.encodeSource(arm, into: &body_arm)
        case .codecZenohQuery(let arm):
            try CodecZenohQuery.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecZenohQuery.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
