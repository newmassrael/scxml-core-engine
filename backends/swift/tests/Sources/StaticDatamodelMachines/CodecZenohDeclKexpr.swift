// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: codec_zenoh_decl_kexpr.scxml:47 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohDeclKexpr: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecZenohWireexpr.frameLength

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var id: UInt16 = 0

    /// A `CodecZenohWireexpr` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var wireexpr: CodecZenohWireexpr = CodecZenohWireexpr()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(id: UInt16, wireexpr: CodecZenohWireexpr) {
        self.id = id
        self.wireexpr = wireexpr
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, N: UInt8) throws(CodecError) -> CodecZenohDeclKexpr {
        var reader = cursor
        var frame = CodecZenohDeclKexpr()
        frame.id = try reader.readVle(UInt16.self)
        frame.wireexpr = try CodecZenohWireexpr.decode(&reader, N: N)
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, N: UInt8) throws(CodecError) {
        var frame: [UInt8] = []
        appendVle(self.id, to: &frame)
        do {
            var piece = BytesSink()
            try self.wireexpr.encode(into: &piece, N: N)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
