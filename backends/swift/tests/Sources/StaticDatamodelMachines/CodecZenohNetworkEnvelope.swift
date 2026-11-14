// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_zenoh_network_envelope.scxml:60 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohNetworkEnvelope` the tag of a frame selects: the struct of the entry codec the arm names.
/// The catch-all holds the tag it was read for, as no arm names it.
public enum CodecZenohNetworkEnvelopeVariant: Equatable, Sendable {
    case codecZenohInterest(CodecZenohInterest)
    case codecZenohResponseFinal(CodecZenohResponseFinal)
    case codecZenohResponse(CodecZenohResponse)
    case codecZenohRequest(CodecZenohRequest)
    case codecZenohPush(CodecZenohPush)
    case codecZenohDeclare(CodecZenohDeclare)
    case codecZenohOam(CodecZenohOam)
    case `default`(tag: UInt8, body: CodecZenohOam)
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohNetworkEnvelope: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = Swift.min(CodecZenohInterest.frameLength, CodecZenohResponseFinal.frameLength, CodecZenohResponse.frameLength, CodecZenohRequest.frameLength, CodecZenohPush.frameLength, CodecZenohDeclare.frameLength, CodecZenohOam.frameLength)

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohNetworkEnvelopeVariant = .codecZenohOam(CodecZenohOam())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: CodecZenohNetworkEnvelopeVariant) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohNetworkEnvelope {
        var reader = cursor
        var frame = CodecZenohNetworkEnvelope()
        let body_tag = try (reader.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x1F
        switch body_tag {
        case 0x19:
            frame.body = .codecZenohInterest(try CodecZenohInterest.decode(&reader))
        case 0x1A:
            frame.body = .codecZenohResponseFinal(try CodecZenohResponseFinal.decode(&reader))
        case 0x1B:
            frame.body = .codecZenohResponse(try CodecZenohResponse.decode(&reader))
        case 0x1C:
            frame.body = .codecZenohRequest(try CodecZenohRequest.decode(&reader))
        case 0x1D:
            frame.body = .codecZenohPush(try CodecZenohPush.decode(&reader))
        case 0x1E:
            frame.body = .codecZenohDeclare(try CodecZenohDeclare.decode(&reader))
        case 0x1F:
            frame.body = .codecZenohOam(try CodecZenohOam.decode(&reader))
        default:
            frame.body = .default(tag: UInt8(truncatingIfNeeded: body_tag), body: try CodecZenohOam.decode(&reader))
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
            case .codecZenohInterest(let arm):
                try arm.encode(into: &piece)
            case .codecZenohResponseFinal(let arm):
                try arm.encode(into: &piece)
            case .codecZenohResponse(let arm):
                try arm.encode(into: &piece)
            case .codecZenohRequest(let arm):
                try arm.encode(into: &piece)
            case .codecZenohPush(let arm):
                try arm.encode(into: &piece)
            case .codecZenohDeclare(let arm):
                try arm.encode(into: &piece)
            case .codecZenohOam(let arm):
                try arm.encode(into: &piece)
            case .default(_, let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
