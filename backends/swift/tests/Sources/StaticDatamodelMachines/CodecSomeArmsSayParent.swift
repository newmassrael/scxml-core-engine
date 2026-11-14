// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: codec_some_arms_say_parent.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSomeArmsSayParent: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecSomeArmsSay.frameLength

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

    /// A `CodecSomeArmsSay` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var body: CodecSomeArmsSay = CodecSomeArmsSay()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, body: CodecSomeArmsSay) {
        self.header = header
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecSomeArmsSayParent {
        var reader = cursor
        var frame = CodecSomeArmsSayParent()
        frame.header = try reader.readInteger(UInt8.self, .big)
        frame.body = try CodecSomeArmsSay.decode(&reader, N: UInt8(truncatingIfNeeded: (frame.header >> 5) & 0x1), tag: UInt8(truncatingIfNeeded: (frame.header >> 6) & 0x1))
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        let derived_tag_0: UInt8
        switch self.body.body {
        case .codecSomeArmsSayPlain: derived_tag_0 = 0x00
        case .codecEveryArmSaysNear: derived_tag_0 = 0x40
        }
        appendInteger((self.header & 0xBF) | derived_tag_0, to: &frame, .big)
        do {
            var piece = BytesSink()
            try self.body.encode(into: &piece, N: UInt8(truncatingIfNeeded: (self.header >> 5) & 0x1))
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
