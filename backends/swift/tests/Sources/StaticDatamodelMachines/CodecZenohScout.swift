// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_zenoh_scout.scxml:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohScout` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohScout.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohScout`, which conforms itself.
public protocol CodecZenohScoutSource {
    var version: UInt8 { get }
    var cbyte: UInt8 { get }
    var zid: BoundedBytes<CodecZenohScout.ZidBound>? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohScout: Equatable, Sendable, CodecZenohScoutSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `zid` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ZidBound: SceByteBound {
        public static let least = 1
        public static let most: Int? = 16
    }

    /// The field at byte 0, big-endian.
    public var version: UInt8 = 0

    /// The field at byte 1, big-endian.
    public var cbyte: UInt8 = 0

    /// The 3 bits of `cbyte` from bit 0 (flag `what`). A value
    /// wider than the flag is cut to it when written.
    public var what: UInt8 {
        get { UInt8(truncatingIfNeeded: (cbyte >> 0) & 0x7) }
        set { cbyte = (cbyte & ~(0x7 << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x7) << 0) }
    }

    /// Bit 3 of `cbyte` (flag `I`).
    public var I: Bool {
        get { (cbyte & 0x8) != 0 }
        set { cbyte = newValue ? (cbyte | 0x8) : (cbyte & ~0x8) }
    }

    /// The 4 bits of `cbyte` from bit 4 (flag `zid_len_m1`). A value
    /// wider than the flag is cut to it when written.
    public var zidLenM1: UInt8 {
        get { UInt8(truncatingIfNeeded: (cbyte >> 4) & 0xF) }
        set { cbyte = (cbyte & ~(0xF << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 4) }
    }

    /// On the wire only when `(cbyte & 0x8) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `cbyte.zid_len_m1` says, plus 1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var zid: BoundedBytes<CodecZenohScout.ZidBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, cbyte: UInt8, zid: BoundedBytes<CodecZenohScout.ZidBound>?) {
        self.version = version
        self.cbyte = cbyte
        self.zid = zid
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohScout {
        var reader = cursor
        let version = try reader.readInteger(UInt8.self, .big)
        let cbyte = try reader.readInteger(UInt8.self, .big)
        var zid: BoundedBytes<CodecZenohScout.ZidBound>? = nil
        if (cbyte & 0x8) != 0 {
            zid = try BoundedBytes<CodecZenohScout.ZidBound>(decoding: Array(try reader.readSlice(Int(clamping: ((cbyte >> 4) & 0xF)) &+ (1))))
        }
        cursor = reader
        return CodecZenohScout(version: version, cbyte: cbyte, zid: zid)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohScoutSource` is written,
    /// with no copy into `CodecZenohScout`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohScoutSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var cbyte_folded = source.cbyte
        if source.zid != nil { cbyte_folded = (cbyte_folded & ~0xF0) | ((UInt8(truncatingIfNeeded: ((source.zid?.byteCount ?? 0) - 1)) & 0xF) << 4) }
        cbyte_folded = (cbyte_folded & ~0x8) | ((source.zid != nil) ? 0x8 : 0)
        var frame: [UInt8] = []
        appendInteger(source.version, to: &frame, .big)
        appendInteger(cbyte_folded, to: &frame, .big)
        if let held = source.zid {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
