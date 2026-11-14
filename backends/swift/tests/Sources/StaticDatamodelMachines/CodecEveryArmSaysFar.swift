// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_every_arm_says_far.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecEveryArmSaysFar: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 0

    /// On the wire only when `(bare & 0x1) == 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 0, big-endian.
    public var route: UInt16? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(route: UInt16?) {
        self.route = route
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, bare: UInt8) throws(CodecError) -> CodecEveryArmSaysFar {
        var reader = cursor
        var frame = CodecEveryArmSaysFar()
        if (bare & 0x1) == 0 {
            frame.route = try reader.readInteger(UInt16.self, .big)
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, bare: UInt8) throws(CodecError) {
        if (bare & 0x1) == 0 {
            if self.route == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.route != nil {
            throw CodecError.presentIfMismatch
        }
        var frame: [UInt8] = []
        if let route = self.route {
            appendInteger(route, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }
}
