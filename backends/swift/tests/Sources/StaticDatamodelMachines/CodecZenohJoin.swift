// SCE-GENERATED — DO NOT EDIT
// source-hash: 292f93c83cd96570c6a4bd999df25c5b18dff8e9466c855739810504f6bc4be7
// source: codec_zenoh_join.scxml
// SCE-MAP: codec_zenoh_join.scxml:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecZenohJoin` the gates `S` decide together (docs/adr/0065 §6
/// R4b, R4c): the elements present while the gates are as the one arm says. The encode derives the gates
/// from whether a value holds it, and a decode makes the arm the gates of the frame name.
public struct CodecZenohJoinS: Equatable, Sendable {
    public var sn_res: UInt8
    public var batch_size: UInt16

    public init(sn_res: UInt8, batch_size: UInt16) {
        self.sn_res = sn_res
        self.batch_size = batch_size
    }
}

/// The elements a value of `CodecZenohJoin` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohJoin.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohJoin`, which conforms itself.
public protocol CodecZenohJoinSource {
    var version: UInt8 { get }
    var cbyte: UInt8 { get }
    var zid: BoundedBytes<CodecZenohJoin.ZidBound> { get }
    var s: CodecZenohJoinS? { get }
    var lease: UInt64 { get }
    var next_sn_reliable: UInt64 { get }
    var next_sn_best_effort: UInt64 { get }
}

/// The frame this codec lays on the wire: 8 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohJoin: Equatable, Sendable, CodecZenohJoinSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 5

    /// How many bytes `zid` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ZidBound: SceByteBound {
        public static let least = 1
        public static let most: Int? = 16
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
    public var zid: BoundedBytes<CodecZenohJoin.ZidBound> = .fewest

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var lease: UInt64 = 0

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var next_sn_reliable: UInt64 = 0

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var next_sn_best_effort: UInt64 = 0

    /// The elements the gates `S` decide together, `nil` while they name no arm.
    public var s: CodecZenohJoinS? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, cbyte: UInt8, zid: BoundedBytes<CodecZenohJoin.ZidBound>, s: CodecZenohJoinS?, lease: UInt64, next_sn_reliable: UInt64, next_sn_best_effort: UInt64) {
        self.version = version
        self.cbyte = cbyte
        self.zid = zid
        self.s = s
        self.lease = lease
        self.next_sn_reliable = next_sn_reliable
        self.next_sn_best_effort = next_sn_best_effort
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, S: UInt8) throws(CodecError) -> CodecZenohJoin {
        var reader = cursor
        let version = try reader.readInteger(UInt8.self, .little)
        let cbyte = try reader.readInteger(UInt8.self, .little)
        let zid = try BoundedBytes<CodecZenohJoin.ZidBound>(decoding: Array(try reader.readSlice(Int(clamping: ((cbyte >> 4) & 0xF)) &+ (1))))
        var sn_res: UInt8? = nil
        if (S & 0x1) != 0 {
            sn_res = try reader.readInteger(UInt8.self, .little)
        }
        var batch_size: UInt16? = nil
        if (S & 0x1) != 0 {
            batch_size = try reader.readInteger(UInt16.self, .little)
        }
        let lease = try reader.readVle(UInt64.self)
        let next_sn_reliable = try reader.readVle(UInt64.self)
        let next_sn_best_effort = try reader.readVle(UInt64.self)
        let s_group: CodecZenohJoinS?
        if (S & 0x01) != 0 {
            s_group = CodecZenohJoinS(sn_res: sn_res!, batch_size: batch_size!)
        } else {
            s_group = nil
        }
        cursor = reader
        return CodecZenohJoin(version: version, cbyte: cbyte, zid: zid, s: s_group, lease: lease, next_sn_reliable: next_sn_reliable, next_sn_best_effort: next_sn_best_effort)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohJoinSource` is written,
    /// with no copy into `CodecZenohJoin`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohJoinSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var sn_res_member: UInt8? = nil
        var batch_size_member: UInt16? = nil
        if let held = source.s { sn_res_member = held.sn_res; batch_size_member = held.batch_size }
        var cbyte_folded = source.cbyte
        cbyte_folded = (cbyte_folded & ~0xF0) | ((UInt8(truncatingIfNeeded: (source.zid.byteCount - 1)) & 0xF) << 4)
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
        appendVle(source.lease, to: &frame)
        appendVle(source.next_sn_reliable, to: &frame)
        appendVle(source.next_sn_best_effort, to: &frame)
        try sink.writeBytes(frame)
    }

    /// The flag input `S` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputS<Source: CodecZenohJoinSource>(_ source: Source) -> UInt8 {
        (source.s != nil ? true : false) ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
