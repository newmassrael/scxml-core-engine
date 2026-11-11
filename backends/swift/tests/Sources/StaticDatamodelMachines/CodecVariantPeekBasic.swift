// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: codec_variant_peek_basic.scxml:29 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecVariantPeekBasic` the tag of a frame selects: the struct of the entry codec the arm names.
public enum CodecVariantPeekBasicVariant: Equatable, Sendable {
    case codecPeekArmA(CodecPeekArmA)
    case codecPeekArmB(CodecPeekArmB)
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecVariantPeekBasic: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = Swift.min(CodecPeekArmA.frameLength, CodecPeekArmB.frameLength)

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecVariantPeekBasicVariant = .codecPeekArmA(CodecPeekArmA())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: CodecVariantPeekBasicVariant) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecVariantPeekBasic {
        var reader = cursor
        var frame = CodecVariantPeekBasic()
        let body_tag = try (reader.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x1
        switch body_tag {
        case 0x0:
            frame.body = .codecPeekArmA(try CodecPeekArmA.decode(&reader))
        default:
            frame.body = .codecPeekArmB(try CodecPeekArmB.decode(&reader))
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
            case .codecPeekArmA(let arm):
                try arm.encode(into: &piece)
            case .codecPeekArmB(let arm):
                try arm.encode(into: &piece)
            }
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
