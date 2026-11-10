// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_init_syn_body.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecInitSynBody: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var version: UInt8 = 0

    /// On the wire only when `(S & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var sn_res: UInt8? = nil

    /// On the wire only when `(S & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 2, big-endian.
    public var batch_size: UInt16? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, sn_res: UInt8?, batch_size: UInt16?) {
        self.version = version
        self.sn_res = sn_res
        self.batch_size = batch_size
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, S: UInt8) throws(CodecError) -> CodecInitSynBody {
        var reader = cursor
        var frame = CodecInitSynBody()
        frame.version = try reader.readInteger(UInt8.self, .big)
        if (S & 0x1) != 0 {
            frame.sn_res = try reader.readInteger(UInt8.self, .big)
        }
        if (S & 0x1) != 0 {
            frame.batch_size = try reader.readInteger(UInt16.self, .big)
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, S: UInt8) throws(CodecError) {
        if (S & 0x1) != 0 {
            if self.sn_res == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.sn_res != nil {
            throw CodecError.presentIfMismatch
        }
        if (S & 0x1) != 0 {
            if self.batch_size == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.batch_size != nil {
            throw CodecError.presentIfMismatch
        }
        var frame: [UInt8] = []
        appendInteger(self.version, to: &frame, .big)
        if let sn_res = self.sn_res {
            appendInteger(sn_res, to: &frame, .big)
        }
        if let batch_size = self.batch_size {
            appendInteger(batch_size, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }
}
