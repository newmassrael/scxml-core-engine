// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_transport_envelope.scxml:68 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecTransportEnvelope` the tag of a frame selects: the struct of the entry codec the arm names.
/// The catch-all holds the tag it was read for, as no arm names it.
public enum CodecTransportEnvelopeVariant: Equatable, Sendable {
    case codecZenohInitBody(CodecZenohInitBody)
    case codecZenohOpenBody(CodecZenohOpenBody)
    case codecZenohClose(CodecZenohClose)
    case codecZenohKeepAlive(CodecZenohKeepAlive)
    case codecZenohFrame(CodecZenohFrame)
    case codecZenohFragment(CodecZenohFragment)
    case codecZenohJoin(CodecZenohJoin)
    case `default`(tag: UInt8, body: CodecZenohClose)
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecTransportEnvelope: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + Swift.min(CodecZenohInitBody.frameLength, CodecZenohOpenBody.frameLength, CodecZenohClose.frameLength, CodecZenohKeepAlive.frameLength, CodecZenohFrame.frameLength, CodecZenohFragment.frameLength, CodecZenohJoin.frameLength)

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `A`).
    public var A: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `S`).
    public var S: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecTransportEnvelopeVariant = .codecZenohClose(CodecZenohClose())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, body: CodecTransportEnvelopeVariant) {
        self.header = header
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecTransportEnvelope {
        var reader = cursor
        var frame = CodecTransportEnvelope()
        frame.header = try reader.readInteger(UInt8.self, .big)
        let body_tag = (frame.header >> 0) & 0x1F
        switch body_tag {
        case 0x1:
            frame.body = .codecZenohInitBody(try CodecZenohInitBody.decode(&reader, S: UInt8(truncatingIfNeeded: (frame.header >> 6) & 0x1), A: UInt8(truncatingIfNeeded: (frame.header >> 5) & 0x1)))
        case 0x2:
            frame.body = .codecZenohOpenBody(try CodecZenohOpenBody.decode(&reader, A: UInt8(truncatingIfNeeded: (frame.header >> 5) & 0x1)))
        case 0x3:
            frame.body = .codecZenohClose(try CodecZenohClose.decode(&reader))
        case 0x4:
            frame.body = .codecZenohKeepAlive(try CodecZenohKeepAlive.decode(&reader))
        case 0x5:
            frame.body = .codecZenohFrame(try CodecZenohFrame.decode(&reader))
        case 0x6:
            frame.body = .codecZenohFragment(try CodecZenohFragment.decode(&reader))
        case 0x7:
            frame.body = .codecZenohJoin(try CodecZenohJoin.decode(&reader, S: UInt8(truncatingIfNeeded: (frame.header >> 6) & 0x1)))
        default:
            frame.body = .default(tag: UInt8(truncatingIfNeeded: body_tag), body: try CodecZenohClose.decode(&reader))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        do {
            var piece = BytesSink()
            switch self.body {
            case .codecZenohInitBody(let arm):
                try arm.encode(into: &piece, S: UInt8(truncatingIfNeeded: (self.header >> 6) & 0x1), A: UInt8(truncatingIfNeeded: (self.header >> 5) & 0x1))
            case .codecZenohOpenBody(let arm):
                try arm.encode(into: &piece, A: UInt8(truncatingIfNeeded: (self.header >> 5) & 0x1))
            case .codecZenohClose(let arm):
                try arm.encode(into: &piece)
            case .codecZenohKeepAlive(let arm):
                try arm.encode(into: &piece)
            case .codecZenohFrame(let arm):
                try arm.encode(into: &piece)
            case .codecZenohFragment(let arm):
                try arm.encode(into: &piece)
            case .codecZenohJoin(let arm):
                try arm.encode(into: &piece, S: UInt8(truncatingIfNeeded: (self.header >> 6) & 0x1))
            case .default(_, let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
