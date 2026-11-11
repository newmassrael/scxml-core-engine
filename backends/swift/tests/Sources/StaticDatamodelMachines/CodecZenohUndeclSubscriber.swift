// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: codec_zenoh_undecl_subscriber.scxml:46 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohUndeclSubscriber: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var id: UInt32 = 0

    /// On the wire only when `(Z & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A `CodecZenohDeclExtKeyexpr` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var ext_keyexpr: CodecZenohDeclExtKeyexpr? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(id: UInt32, ext_keyexpr: CodecZenohDeclExtKeyexpr?) {
        self.id = id
        self.ext_keyexpr = ext_keyexpr
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, Z: UInt8) throws(CodecError) -> CodecZenohUndeclSubscriber {
        var reader = cursor
        var frame = CodecZenohUndeclSubscriber()
        frame.id = try reader.readVle(UInt32.self)
        if (Z & 0x1) != 0 {
            frame.ext_keyexpr = try CodecZenohDeclExtKeyexpr.decode(&reader)
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, Z: UInt8) throws(CodecError) {
        if (Z & 0x1) != 0 {
            if self.ext_keyexpr == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.ext_keyexpr != nil {
            throw CodecError.presentIfMismatch
        }
        var frame: [UInt8] = []
        appendVle(self.id, to: &frame)
        if let ext_keyexpr = self.ext_keyexpr {
            do {
                var piece = BytesSink()
                try ext_keyexpr.encode(into: &piece)
                frame += piece.bytes
            }
        }
        try sink.writeBytes(frame)
    }
}
