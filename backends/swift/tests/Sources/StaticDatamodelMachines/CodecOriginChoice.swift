// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_origin_choice.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecOriginChoice` the tag of a frame selects: the struct of the entry codec the arm names.
/// The catch-all holds the tag it was read for, as no arm names it.
public enum CodecOriginChoiceVariant: Equatable, Sendable {
    case codecOriginLeaf(CodecOriginLeaf)
    case codecOriginScalar(CodecOriginScalar)
    case `default`(tag: UInt8, body: CodecOriginScalar)
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecOriginChoice: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + Swift.min(CodecOriginLeaf.frameLength, CodecOriginScalar.frameLength)

    /// The field at byte 0, big-endian.
    public var tag: UInt8 = 0

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecOriginChoiceVariant = .codecOriginScalar(CodecOriginScalar())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(tag: UInt8, body: CodecOriginChoiceVariant) {
        self.tag = tag
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecOriginChoice {
        var reader = cursor
        var frame = CodecOriginChoice()
        frame.tag = try reader.readInteger(UInt8.self, .big)
        let body_tag = frame.tag
        switch body_tag {
        case 0x1:
            frame.body = .codecOriginLeaf(try CodecOriginLeaf.decode(&reader))
        case 0x2:
            frame.body = .codecOriginScalar(try CodecOriginScalar.decode(&reader))
        default:
            frame.body = .default(tag: body_tag, body: try CodecOriginScalar.decode(&reader))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendInteger(self.tag, to: &frame, .big)
        do {
            var piece = BytesSink()
            switch self.body {
            case .codecOriginLeaf(let arm):
                try arm.encode(into: &piece)
            case .codecOriginScalar(let arm):
                try arm.encode(into: &piece)
            case .default(_, let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
