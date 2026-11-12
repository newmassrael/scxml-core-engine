// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_source_leaf.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecSourceLeaf` the tag of a frame selects: the struct of the entry codec the arm names.
public enum CodecSourceLeafVariant: Equatable, Sendable {
    case codecVariantSessionOpen(CodecVariantSessionOpen)
    case codecVariantSessionClose(CodecVariantSessionClose)
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSourceLeaf: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = Swift.min(CodecVariantSessionOpen.frameLength, CodecVariantSessionClose.frameLength)

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecSourceLeafVariant = .codecVariantSessionClose(CodecVariantSessionClose())

    /// The tag of the arm this value holds: the one a decode is given to read it back, which a
    /// holder that stores the bytes (a broadcast log) stores beside them. A frame carries none, so
    /// the arm that is held is what says it.
    public var tag: UInt8 {
        switch body {
        case .codecVariantSessionOpen: return 0x0
        case .codecVariantSessionClose: return 0x1
        }
    }

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: CodecSourceLeafVariant) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, tag: UInt8) throws(CodecError) -> CodecSourceLeaf {
        var reader = cursor
        var frame = CodecSourceLeaf()
        let body_tag = tag
        switch body_tag {
        case 0x0:
            frame.body = .codecVariantSessionOpen(try CodecVariantSessionOpen.decode(&reader))
        default:
            frame.body = .codecVariantSessionClose(try CodecVariantSessionClose.decode(&reader))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        do {
            var piece = BytesSink()
            switch self.body {
            case .codecVariantSessionOpen(let arm):
                try arm.encode(into: &piece)
            case .codecVariantSessionClose(let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
