// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_zenoh_init_body.scxml:43 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 7 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohInitBody: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// The field at byte 0, little-endian.
    public var version: UInt8 = 0

    /// The field at byte 1, little-endian.
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

    /// On the wire only when `(S & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 3, little-endian.
    public var sn_res: UInt8? = nil

    /// On the wire only when `(S & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 4, little-endian.
    public var batch_size: UInt16? = nil

    /// On the wire only when `(A & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var cookie_len: UInt64? = nil

    /// On the wire only when `(A & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `cookie_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var cookie: [UInt8]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, cbyte: UInt8, zid: [UInt8], sn_res: UInt8?, batch_size: UInt16?, cookie_len: UInt64?, cookie: [UInt8]?) {
        self.version = version
        self.cbyte = cbyte
        self.zid = zid
        self.sn_res = sn_res
        self.batch_size = batch_size
        self.cookie_len = cookie_len
        self.cookie = cookie
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, S: UInt8, A: UInt8) throws(CodecError) -> CodecZenohInitBody {
        var reader = cursor
        var frame = CodecZenohInitBody()
        frame.version = try reader.readInteger(UInt8.self, .little)
        frame.cbyte = try reader.readInteger(UInt8.self, .little)
        frame.zid = Array(try reader.readSlice(Int(clamping: ((frame.cbyte >> 4) & 0xF)) &+ (1)))
        if (S & 0x1) != 0 {
            frame.sn_res = try reader.readInteger(UInt8.self, .little)
        }
        if (S & 0x1) != 0 {
            frame.batch_size = try reader.readInteger(UInt16.self, .little)
        }
        if (A & 0x1) != 0 {
            frame.cookie_len = try reader.readVle(UInt64.self)
        }
        if (A & 0x1) != 0 {
            if (frame.cookie_len ?? 0) > 65535 {
                throw CodecError.lengthOutOfRange
            }
            frame.cookie = Array(try reader.readSlice(Int(clamping: (frame.cookie_len ?? 0))))
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, S: UInt8, A: UInt8) throws(CodecError) {
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
        if (A & 0x1) != 0 {
            if self.cookie == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.cookie != nil {
            throw CodecError.presentIfMismatch
        }
        if self.zid.count < 1 || self.zid.count > 16 {
            throw CodecError.lengthOutOfRange
        }
        if let value = self.cookie, value.count > 65535 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.version, to: &frame, .little)
        appendInteger(self.cbyte, to: &frame, .little)
        frame += self.zid
        if let sn_res = self.sn_res {
            appendInteger(sn_res, to: &frame, .little)
        }
        if let batch_size = self.batch_size {
            appendInteger(batch_size, to: &frame, .little)
        }
        if let cookie_len = self.cookie_len {
            appendVle(cookie_len, to: &frame)
        }
        if let cookie = self.cookie {
            frame += cookie
        }
        try sink.writeBytes(frame)
    }
}
