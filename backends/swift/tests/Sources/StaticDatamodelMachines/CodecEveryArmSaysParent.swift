// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_every_arm_says_parent.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecEveryArmSaysParent` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecEveryArmSaysParent.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecEveryArmSaysParent`, which conforms itself.
public protocol CodecEveryArmSaysParentSource {
    /// What `body` is presented as: a source of `CodecEveryArmSays`, written by its encode.
    associatedtype Body_Source: CodecEveryArmSaysSource
    var header: UInt8 { get }
    var body: Body_Source { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecEveryArmSaysParent: Equatable, Sendable, CodecEveryArmSaysParentSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecEveryArmSays.frameLength

    /// The value presents `body` as a value of its own codec.
    public typealias Body_Source = CodecEveryArmSays

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

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

    /// A `CodecEveryArmSays` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var body: CodecEveryArmSays = CodecEveryArmSays()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, body: Body_Source) {
        self.header = header
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecEveryArmSaysParent {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let body = try CodecEveryArmSays.decode(&reader, N: UInt8(truncatingIfNeeded: (header >> 5) & 0x1), tag: UInt8(truncatingIfNeeded: (header >> 6) & 0x1))
        cursor = reader
        return CodecEveryArmSaysParent(header: header, body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecEveryArmSaysParentSource` is written,
    /// with no copy into `CodecEveryArmSaysParent`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecEveryArmSaysParentSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var header_folded = source.header
        header_folded = (header_folded & ~0x20) | ((UInt8(truncatingIfNeeded: Int(CodecEveryArmSays.inputN(source.body))) & 0x1) << 5)
        var header_M_filled: UInt64 = 0
        header_M_filled |= UInt64(source.body.body.tag) & 0x1
        header_folded = (header_folded & ~0x40) | ((UInt8(truncatingIfNeeded: header_M_filled) & 0x1) << 6)
        var body_piece = BytesSink()
        try CodecEveryArmSays.encodeSource(source.body, into: &body_piece)
        let body_bytes = body_piece.bytes
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        frame += body_bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
