// SCE-GENERATED — DO NOT EDIT
// source-hash: 8f8a004e39710baf90b28a0fecce4c2e632b2701acfc2fb202bfb0d654ce34e6
// source: codec_zenoh_oam.scxml
// SCE-MAP: codec_zenoh_oam.scxml:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohOam`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecZenohOamArms<A0: CodecZenohExtUnitSource, A1: CodecZenohExtZintSource, A2: CodecZenohExtZbufSource, D: CodecZenohExtUnitSource> {
    case codecZenohExtUnit(A0)
    case codecZenohExtZint(A1)
    case codecZenohExtZbuf(A2)
    case `default`(tag: CodecZenohOamUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecZenohExtUnit: return 0x0
        case .codecZenohExtZint: return 0x1
        case .codecZenohExtZbuf: return 0x2
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecZenohOamArms: Equatable where A0: Equatable, A1: Equatable, A2: Equatable, D: Equatable {}

extension CodecZenohOamArms: Sendable where A0: Sendable, A1: Sendable, A2: Sendable, D: Sendable {}

/// The arm a value of `CodecZenohOam` holds, each arm the value of its own codec.
public typealias CodecZenohOamVariant = CodecZenohOamArms<CodecZenohExtUnit, CodecZenohExtZint, CodecZenohExtZbuf, CodecZenohExtUnit>

/// The tag the catch-all arm of `CodecZenohOam` holds (docs/adr/0065 §6 R3): one no arm names,
/// within the bits the tag is read from. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecZenohOamUnknownTag: Equatable, Hashable, Sendable {
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

/// The elements a value of `CodecZenohOam` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohOam.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohOam`, which conforms itself.
public protocol CodecZenohOamSource {
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    /// What the body of the arm `codecZenohExtUnit` is presented as: a source of `CodecZenohExtUnit`, written by its encode.
    associatedtype CodecZenohExtUnit_Arm: CodecZenohExtUnitSource
    /// What the body of the arm `codecZenohExtZint` is presented as: a source of `CodecZenohExtZint`, written by its encode.
    associatedtype CodecZenohExtZint_Arm: CodecZenohExtZintSource
    /// What the body of the arm `codecZenohExtZbuf` is presented as: a source of `CodecZenohExtZbuf`, written by its encode.
    associatedtype CodecZenohExtZbuf_Arm: CodecZenohExtZbufSource
    /// What the body of the arm `codecZenohExtUnit` is presented as: a source of `CodecZenohExtUnit`, written by its encode.
    associatedtype Default_Arm: CodecZenohExtUnitSource
    var header: UInt8 { get }
    var id: UInt16 { get }
    var extensions: BoundedList<Extensions_Entry, CodecZenohOam.ExtensionsBound>? { get }
    var body: CodecZenohOamArms<CodecZenohExtUnit_Arm, CodecZenohExtZint_Arm, CodecZenohExtZbuf_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohOam: Equatable, Sendable, CodecZenohOamSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2 + Swift.min(CodecZenohExtUnit.frameLength, CodecZenohExtZint.frameLength, CodecZenohExtZbuf.frameLength)

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = nil
    }

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The value presents the body of the arm `codecZenohExtUnit` as a value of its own codec.
    public typealias CodecZenohExtUnit_Arm = CodecZenohExtUnit

    /// The value presents the body of the arm `codecZenohExtZint` as a value of its own codec.
    public typealias CodecZenohExtZint_Arm = CodecZenohExtZint

    /// The value presents the body of the arm `codecZenohExtZbuf` as a value of its own codec.
    public typealias CodecZenohExtZbuf_Arm = CodecZenohExtZbuf

    /// The value presents the body of the arm `codecZenohExtUnit` as a value of its own codec.
    public typealias Default_Arm = CodecZenohExtUnit

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x1F

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
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

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var id: UInt16 = 0

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohOam.ExtensionsBound>? = nil

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohOamVariant = .codecZenohExtUnit(CodecZenohExtUnit())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, id: UInt16, extensions: BoundedList<Extensions_Entry, CodecZenohOam.ExtensionsBound>?, body: CodecZenohOamArms<CodecZenohExtUnit_Arm, CodecZenohExtZint_Arm, CodecZenohExtZbuf_Arm, Default_Arm>) {
        self.header = header
        self.id = id
        self.extensions = extensions
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohOam {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let id = try reader.readVle(UInt16.self)
        var extensions: BoundedList<CodecZenohExtEntry, CodecZenohOam.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            while true {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_more = entry.Z
                extensions_items.append(entry)
                if !extensions_more { break }
            }
            extensions = try BoundedList<CodecZenohExtEntry, CodecZenohOam.ExtensionsBound>(decoding: extensions_items)
        }
        let body: CodecZenohOamVariant
        let body_tag = (header >> 5) & 0x3
        switch body_tag {
        case 0x0:
            body = .codecZenohExtUnit(try CodecZenohExtUnit.decode(&reader))
        case 0x1:
            body = .codecZenohExtZint(try CodecZenohExtZint.decode(&reader))
        case 0x2:
            body = .codecZenohExtZbuf(try CodecZenohExtZbuf.decode(&reader))
        default:
            body = .default(tag: CodecZenohOamUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohExtUnit.decode(&reader))
        }
        cursor = reader
        return CodecZenohOam(header: header, id: id, extensions: extensions, body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohOamSource` is written,
    /// with no copy into `CodecZenohOam`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohOamSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var header_folded = source.header
        header_folded = (header_folded & ~0x80) | ((source.extensions != nil) ? 0x80 : 0)
        header_folded = (header_folded & ~0x60) | ((UInt8(truncatingIfNeeded: source.body.tag) & 0x3) << 5)
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        appendVle(source.id, to: &frame)
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
        case .codecZenohExtUnit(let arm):
            try CodecZenohExtUnit.encodeSource(arm, into: &body_arm)
        case .codecZenohExtZint(let arm):
            try CodecZenohExtZint.encodeSource(arm, into: &body_arm)
        case .codecZenohExtZbuf(let arm):
            try CodecZenohExtZbuf.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecZenohExtUnit.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
