// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_zenoh_open_body.scxml:42 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohOpenBody: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var lease: UInt64 = 0

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var initial_sn: UInt64 = 0

    /// On the wire only when `(A & 0x1) == 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var cookie_len: UInt64? = nil

    /// On the wire only when `(A & 0x1) == 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `cookie_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var cookie: [UInt8]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(lease: UInt64, initial_sn: UInt64, cookie_len: UInt64?, cookie: [UInt8]?) {
        self.lease = lease
        self.initial_sn = initial_sn
        self.cookie_len = cookie_len
        self.cookie = cookie
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, A: UInt8) throws(CodecError) -> CodecZenohOpenBody {
        var reader = cursor
        var frame = CodecZenohOpenBody()
        frame.lease = try reader.readVle(UInt64.self)
        frame.initial_sn = try reader.readVle(UInt64.self)
        if (A & 0x1) == 0 {
            frame.cookie_len = try reader.readVle(UInt64.self)
        }
        if (A & 0x1) == 0 {
            frame.cookie = Array(try reader.readSlice(Int(clamping: (frame.cookie_len ?? 0))))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, A: UInt8) throws(CodecError) {
        if (A & 0x1) == 0 {
            if self.cookie == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.cookie != nil {
            throw CodecError.presentIfMismatch
        }
        var frame: [UInt8] = []
        appendVle(self.lease, to: &frame)
        appendVle(self.initial_sn, to: &frame)
        if let cookie_len = self.cookie_len {
            appendVle(cookie_len, to: &frame)
        }
        if let cookie = self.cookie {
            frame += cookie
        }
        try sink.writeBytes(frame)
    }
}
