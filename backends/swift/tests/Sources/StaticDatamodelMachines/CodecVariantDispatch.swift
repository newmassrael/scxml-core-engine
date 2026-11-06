// SCE-GENERATED — DO NOT EDIT
// source-hash: a657d6d390827c00a8c65cc79818ec59a776eff477a713478c6b36adae5a7d61
// SCE-MAP: codec_variant_dispatch.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecVariantDispatch` the tag of a frame selects: the struct of the entry codec the arm names.
/// The catch-all holds the tag it was read for, as no arm names it.
public enum CodecVariantDispatchVariant: Equatable, Sendable {
    case codecVariantSessionOpen(CodecVariantSessionOpen)
    case codecVariantSessionClose(CodecVariantSessionClose)
    case `default`(tag: UInt8, body: CodecVariantSessionClose)
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecVariantDispatch: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + Swift.min(CodecVariantSessionOpen.frameLength, CodecVariantSessionClose.frameLength)

    /// The field at byte 0, big-endian.
    public var msg_id: UInt8 = 0

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecVariantDispatchVariant = .codecVariantSessionClose(CodecVariantSessionClose())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(msg_id: UInt8, body: CodecVariantDispatchVariant) {
        self.msg_id = msg_id
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecVariantDispatch {
        var reader = cursor
        var frame = CodecVariantDispatch()
        frame.msg_id = try reader.readInteger(UInt8.self, .big)
        let body_tag = frame.msg_id
        switch body_tag {
        case 0x1:
            frame.body = .codecVariantSessionOpen(try CodecVariantSessionOpen.decode(&reader))
        case 0x2:
            frame.body = .codecVariantSessionClose(try CodecVariantSessionClose.decode(&reader))
        default:
            frame.body = .default(tag: body_tag, body: try CodecVariantSessionClose.decode(&reader))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendInteger(self.msg_id, to: &frame, .big)
        do {
            var piece = BytesSink()
            switch self.body {
            case .codecVariantSessionOpen(let arm):
                try arm.encode(into: &piece)
            case .codecVariantSessionClose(let arm):
                try arm.encode(into: &piece)
            case .default(_, let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
