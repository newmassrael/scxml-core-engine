// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_zenoh_hello.scxml:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohHello: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// The field at byte 0, big-endian.
    public var version: UInt8 = 0

    /// The field at byte 1, big-endian.
    public var cbyte: UInt8 = 0

    /// The 2 bits of `cbyte` from bit 0 (flag `whatami`). A value
    /// wider than the flag is cut to it when written.
    public var whatami: UInt8 {
        get { UInt8(truncatingIfNeeded: (cbyte >> 0) & 0x3) }
        set { cbyte = (cbyte & ~(0x3 << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 0) }
    }

    /// The 4 bits of `cbyte` from bit 4 (flag `zid_len_m1`). A value
    /// wider than the flag is cut to it when written.
    public var zidLenM1: UInt8 {
        get { UInt8(truncatingIfNeeded: (cbyte >> 4) & 0xF) }
        set { cbyte = (cbyte & ~(0xF << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 4) }
    }

    /// The bytes, as many as `cbyte.zid_len_m1` says, plus 1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var zid: [UInt8] = []

    /// On the wire only when `(L & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var num_locators: UInt64? = nil

    /// On the wire only when `(L & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// As many entries as `num_locators` says, each a `CodecZenohLocator`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var locators: [CodecZenohLocator]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, cbyte: UInt8, zid: [UInt8], num_locators: UInt64?, locators: [CodecZenohLocator]?) {
        self.version = version
        self.cbyte = cbyte
        self.zid = zid
        self.num_locators = num_locators
        self.locators = locators
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, L: UInt8) throws(CodecError) -> CodecZenohHello {
        var reader = cursor
        var frame = CodecZenohHello()
        frame.version = try reader.readInteger(UInt8.self, .big)
        frame.cbyte = try reader.readInteger(UInt8.self, .big)
        frame.zid = Array(try reader.readSlice(Int(clamping: ((frame.cbyte >> 4) & 0xF)) &+ (1)))
        if (L & 0x1) != 0 {
            frame.num_locators = try reader.readVle(UInt64.self)
        }
        if (L & 0x1) != 0 {
            var locators_items: [CodecZenohLocator] = []
            for _ in 0..<Int(clamping: (frame.num_locators ?? 0)) {
                locators_items.append(try CodecZenohLocator.decode(&reader))
            }
            frame.locators = locators_items
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, L: UInt8) throws(CodecError) {
        if (L & 0x1) != 0 {
            if self.locators == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.locators != nil {
            throw CodecError.presentIfMismatch
        }
        if self.zid.count < 1 || self.zid.count > 16 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.version, to: &frame, .big)
        appendInteger(self.cbyte, to: &frame, .big)
        frame += self.zid
        if let num_locators = self.num_locators {
            appendVle(num_locators, to: &frame)
        }
        if let locators = self.locators {
            for entry in locators {
                var piece = BytesSink()
                try entry.encode(into: &piece)
                frame += piece.bytes
            }
        }
        try sink.writeBytes(frame)
    }
}
