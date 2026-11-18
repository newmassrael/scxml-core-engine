// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_zenoh_value_entry.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohValueEntry`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecZenohValueEntryArms<A0: CodecZenohExtUnitSource, A1: CodecZenohExtZintSource, A2: CodecZenohValueZbufSource, D: CodecZenohExtUnitSource> {
    case codecZenohExtUnit(A0)
    case codecZenohExtZint(A1)
    case codecZenohValueZbuf(A2)
    case `default`(tag: CodecZenohValueEntryUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecZenohExtUnit: return 0x0
        case .codecZenohExtZint: return 0x1
        case .codecZenohValueZbuf: return 0x2
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecZenohValueEntryArms: Equatable where A0: Equatable, A1: Equatable, A2: Equatable, D: Equatable {}

extension CodecZenohValueEntryArms: Sendable where A0: Sendable, A1: Sendable, A2: Sendable, D: Sendable {}

/// The arm a value of `CodecZenohValueEntry` holds, each arm the value of its own codec.
public typealias CodecZenohValueEntryVariant = CodecZenohValueEntryArms<CodecZenohExtUnit, CodecZenohExtZint, CodecZenohValueZbuf, CodecZenohExtUnit>

/// The tag the catch-all arm of `CodecZenohValueEntry` holds (docs/adr/0065 §6 R3): one no arm names,
/// within the bits the tag is read from. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecZenohValueEntryUnknownTag: Equatable, Hashable, Sendable {
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
        case 0x0...0x2: return false
        default: return value <= 0x3
        }
    }
}

/// The elements a value of `CodecZenohValueEntry` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohValueEntry.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohValueEntry`, which conforms itself.
public protocol CodecZenohValueEntrySource {
    /// What the body of the arm `codecZenohExtUnit` is presented as: a source of `CodecZenohExtUnit`, written by its encode.
    associatedtype CodecZenohExtUnit_Arm: CodecZenohExtUnitSource
    /// What the body of the arm `codecZenohExtZint` is presented as: a source of `CodecZenohExtZint`, written by its encode.
    associatedtype CodecZenohExtZint_Arm: CodecZenohExtZintSource
    /// What the body of the arm `codecZenohValueZbuf` is presented as: a source of `CodecZenohValueZbuf`, written by its encode.
    associatedtype CodecZenohValueZbuf_Arm: CodecZenohValueZbufSource
    /// What the body of the arm `codecZenohExtUnit` is presented as: a source of `CodecZenohExtUnit`, written by its encode.
    associatedtype Default_Arm: CodecZenohExtUnitSource
    var header: UInt8 { get }
    var body: CodecZenohValueEntryArms<CodecZenohExtUnit_Arm, CodecZenohExtZint_Arm, CodecZenohValueZbuf_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohValueEntry: Equatable, Sendable, CodecZenohValueEntrySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + Swift.min(CodecZenohExtUnit.frameLength, CodecZenohExtZint.frameLength, CodecZenohValueZbuf.frameLength)

    /// The value presents the body of the arm `codecZenohExtUnit` as a value of its own codec.
    public typealias CodecZenohExtUnit_Arm = CodecZenohExtUnit

    /// The value presents the body of the arm `codecZenohExtZint` as a value of its own codec.
    public typealias CodecZenohExtZint_Arm = CodecZenohExtZint

    /// The value presents the body of the arm `codecZenohValueZbuf` as a value of its own codec.
    public typealias CodecZenohValueZbuf_Arm = CodecZenohValueZbuf

    /// The value presents the body of the arm `codecZenohExtUnit` as a value of its own codec.
    public typealias Default_Arm = CodecZenohExtUnit

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 4 bits of `header` from bit 0 (flag `ext_id`). A value
    /// wider than the flag is cut to it when written.
    public var extId: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0xF) }
        set { header = (header & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// Bit 4 of `header` (flag `M`).
    public var M: Bool {
        get { (header & 0x10) != 0 }
        set { header = newValue ? (header | 0x10) : (header & ~0x10) }
    }

    /// The 2 bits of `header` from bit 5 (flag `enc`). A value
    /// wider than the flag is cut to it when written.
    public var enc: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 5) & 0x3) }
        set { header = (header & ~(0x3 << 5)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 5) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohValueEntryVariant = .codecZenohExtUnit(CodecZenohExtUnit())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, body: CodecZenohValueEntryArms<CodecZenohExtUnit_Arm, CodecZenohExtZint_Arm, CodecZenohValueZbuf_Arm, Default_Arm>) {
        self.header = header
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, after_shm: UInt8) throws(CodecError) -> CodecZenohValueEntry {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let body: CodecZenohValueEntryVariant
        let body_tag = (header >> 5) & 0x3
        switch body_tag {
        case 0x0:
            body = .codecZenohExtUnit(try CodecZenohExtUnit.decode(&reader))
        case 0x1:
            body = .codecZenohExtZint(try CodecZenohExtZint.decode(&reader))
        case 0x2:
            body = .codecZenohValueZbuf(try CodecZenohValueZbuf.decode(&reader, after_shm: after_shm))
        default:
            body = .default(tag: CodecZenohValueEntryUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohExtUnit.decode(&reader))
        }
        cursor = reader
        return CodecZenohValueEntry(header: header, body: body)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecZenohValueEntry` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `M` flag of `header`, where a parent sets it.
        public var headerM: Bool?
        /// The `Z` flag of `header`, where a parent sets it.
        public var headerZ: Bool?

        public init(headerM: Bool? = nil, headerZ: Bool? = nil) {
            self.headerM = headerM
            self.headerZ = headerZ
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohValueEntrySource` is written,
    /// with no copy into `CodecZenohValueEntry`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohValueEntrySource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var header_folded = source.header
        header_folded = (header_folded & ~0x60) | ((UInt8(truncatingIfNeeded: source.body.tag) & 0x3) << 5)
        if let set = overrides.headerM { header_folded = set ? (header_folded | 0x10) : (header_folded & ~0x10) }
        if let set = overrides.headerZ { header_folded = set ? (header_folded | 0x80) : (header_folded & ~0x80) }
        var frame: [UInt8] = []
        storeInteger(header_folded, into: &frame, at: 0, .big)
        var body_arm = BytesSink()
        switch source.body {
        case .codecZenohExtUnit(let arm):
            try CodecZenohExtUnit.encodeSource(arm, into: &body_arm)
        case .codecZenohExtZint(let arm):
            try CodecZenohExtZint.encodeSource(arm, into: &body_arm)
        case .codecZenohValueZbuf(let arm):
            try CodecZenohValueZbuf.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecZenohExtUnit.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// The flag input `after_shm` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0, and `nil` where the value says nothing of it (§6 R4e). A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputAfterShm<Source: CodecZenohValueEntrySource>(_ source: Source) -> UInt8? {
        switch source.body {
        case .codecZenohExtUnit: return nil
        case .codecZenohExtZint: return nil
        case .codecZenohValueZbuf(let arm): return CodecZenohValueZbuf.inputAfterShm(arm)
        case .default: return nil
        }
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
