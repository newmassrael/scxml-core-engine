// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_zenoh_init_body.scxml:43 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecZenohInitBody` the gates `S` decide together (docs/adr/0065 §6
/// R4b, R4c): the elements present while the gates are as the one arm says. The encode derives the gates
/// from whether a value holds it, and a decode makes the arm the gates of the frame name.
public struct CodecZenohInitBodyS: Equatable, Sendable {
    public var sn_res: UInt8
    public var batch_size: UInt16

    public init(sn_res: UInt8, batch_size: UInt16) {
        self.sn_res = sn_res
        self.batch_size = batch_size
    }
}

/// The elements a value of `CodecZenohInitBody` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohInitBody.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohInitBody`, which conforms itself.
public protocol CodecZenohInitBodySource {
    var version: UInt8 { get }
    var cbyte: UInt8 { get }
    var zid: BoundedBytes<CodecZenohInitBody.ZidBound> { get }
    var s: CodecZenohInitBodyS? { get }
    var cookie: BoundedBytes<CodecZenohInitBody.CookieBound>? { get }
}

/// The frame this codec lays on the wire: 7 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohInitBody: Equatable, Sendable, CodecZenohInitBodySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `zid` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ZidBound: SceByteBound {
        public static let least = 1
        public static let most: Int? = 16
    }

    /// How many bytes `cookie` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum CookieBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 65535
    }

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
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var zid: BoundedBytes<CodecZenohInitBody.ZidBound> = .fewest

    /// On the wire only when `(A & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `cookie_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var cookie: BoundedBytes<CodecZenohInitBody.CookieBound>? = nil

    /// The elements the gates `S` decide together, `nil` while they name no arm.
    public var s: CodecZenohInitBodyS? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, cbyte: UInt8, zid: BoundedBytes<CodecZenohInitBody.ZidBound>, s: CodecZenohInitBodyS?, cookie: BoundedBytes<CodecZenohInitBody.CookieBound>?) {
        self.version = version
        self.cbyte = cbyte
        self.zid = zid
        self.s = s
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
        let version = try reader.readInteger(UInt8.self, .little)
        let cbyte = try reader.readInteger(UInt8.self, .little)
        let zid = try BoundedBytes<CodecZenohInitBody.ZidBound>(decoding: Array(try reader.readSlice(Int(clamping: ((cbyte >> 4) & 0xF)) &+ (1))))
        var sn_res: UInt8? = nil
        if (S & 0x1) != 0 {
            sn_res = try reader.readInteger(UInt8.self, .little)
        }
        var batch_size: UInt16? = nil
        if (S & 0x1) != 0 {
            batch_size = try reader.readInteger(UInt16.self, .little)
        }
        var cookie_len: UInt64? = nil
        if (A & 0x1) != 0 {
            cookie_len = try reader.readVle(UInt64.self)
        }
        var cookie: BoundedBytes<CodecZenohInitBody.CookieBound>? = nil
        if (A & 0x1) != 0 {
            if (cookie_len ?? 0) > 65535 {
                throw CodecError.lengthOutOfRange
            }
            cookie = try BoundedBytes<CodecZenohInitBody.CookieBound>(decoding: Array(try reader.readSlice(Int(clamping: (cookie_len ?? 0)))))
        }
        let s_group: CodecZenohInitBodyS?
        if (S & 0x01) != 0 {
            s_group = CodecZenohInitBodyS(sn_res: sn_res!, batch_size: batch_size!)
        } else {
            s_group = nil
        }
        cursor = reader
        return CodecZenohInitBody(version: version, cbyte: cbyte, zid: zid, s: s_group, cookie: cookie)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohInitBodySource` is written,
    /// with no copy into `CodecZenohInitBody`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohInitBodySource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var sn_res_member: UInt8? = nil
        var batch_size_member: UInt16? = nil
        if let held = source.s { sn_res_member = held.sn_res; batch_size_member = held.batch_size }
        let A: UInt8 = source.cookie != nil ? 1 : 0
        var cbyte_folded = source.cbyte
        cbyte_folded = (cbyte_folded & ~0xF0) | ((UInt8(truncatingIfNeeded: (source.zid.byteCount - 1)) & 0xF) << 4)
        let cookie_len_derived: UInt64 = source.cookie != nil ? UInt64((source.cookie?.byteCount ?? 0)) : 0
        var frame: [UInt8] = []
        appendInteger(source.version, to: &frame, .little)
        appendInteger(cbyte_folded, to: &frame, .little)
        source.zid.append(to: &frame)
        if let held = sn_res_member {
            appendInteger(held, to: &frame, .little)
        }
        if let held = batch_size_member {
            appendInteger(held, to: &frame, .little)
        }
        if A != 0 {
            appendVle(cookie_len_derived, to: &frame)
        }
        if let held = source.cookie {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `S` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputS<Source: CodecZenohInitBodySource>(_ source: Source) -> UInt8 {
        (source.s != nil ? true : false) ? 1 : 0
    }

    /// The flag input `A` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputA<Source: CodecZenohInitBodySource>(_ source: Source) -> UInt8 {
        source.cookie != nil ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
