// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_source_info.scxml:57 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohSourceInfo` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohSourceInfo.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohSourceInfo`, which conforms itself.
public protocol CodecZenohSourceInfoSource {
    var header: UInt8 { get }
    var zid: BoundedBytes<CodecZenohSourceInfo.ZidBound> { get }
    var eid: UInt32 { get }
    var sn: UInt32 { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohSourceInfo: Equatable, Sendable, CodecZenohSourceInfoSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 3

    /// How many bytes `zid` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ZidBound: SceByteBound {
        public static let least = 1
        public static let most: Int? = 16
    }

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 4 bits of `header` from bit 4 (flag `zidlen_m1`). A value
    /// wider than the flag is cut to it when written.
    public var zidlenM1: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 4) & 0xF) }
        set { header = (header & ~(0xF << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 4) }
    }

    /// The bytes, as many as `header.zidlen_m1` says, plus 1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var zid: BoundedBytes<CodecZenohSourceInfo.ZidBound> = .fewest

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var eid: UInt32 = 0

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var sn: UInt32 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, zid: BoundedBytes<CodecZenohSourceInfo.ZidBound>, eid: UInt32, sn: UInt32) {
        self.header = header
        self.zid = zid
        self.eid = eid
        self.sn = sn
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohSourceInfo {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let zid = try BoundedBytes<CodecZenohSourceInfo.ZidBound>(decoding: Array(try reader.readSlice(Int(clamping: ((header >> 4) & 0xF)) &+ (1))))
        let eid = try reader.readVle(UInt32.self)
        let sn = try reader.readVle(UInt32.self)
        cursor = reader
        return CodecZenohSourceInfo(header: header, zid: zid, eid: eid, sn: sn)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohSourceInfoSource` is written,
    /// with no copy into `CodecZenohSourceInfo`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohSourceInfoSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var header_folded = source.header
        header_folded = (header_folded & ~0xF0) | ((UInt8(truncatingIfNeeded: (source.zid.byteCount - 1)) & 0xF) << 4)
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        source.zid.append(to: &frame)
        appendVle(source.eid, to: &frame)
        appendVle(source.sn, to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
