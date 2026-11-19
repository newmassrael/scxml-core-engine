// SCE-GENERATED — DO NOT EDIT
// source-hash: 76e23f8a137b0a0686621bb892dc579b70649bc8690b2d7b3dbc63ff5b8c6beb
// source: codec_ext_timestamp.scxml
// SCE-MAP: codec_ext_timestamp.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecExtTimestamp` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecExtTimestamp.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecExtTimestamp`, which conforms itself.
public protocol CodecExtTimestampSource {
    var time: UInt64 { get }
    var zid: BoundedBytes<CodecExtTimestamp.ZidBound> { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecExtTimestamp: Equatable, Sendable, CodecExtTimestampSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `zid` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ZidBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 16
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var time: UInt64 = 0

    /// The bytes, as many as `zid_size` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var zid: BoundedBytes<CodecExtTimestamp.ZidBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(time: UInt64, zid: BoundedBytes<CodecExtTimestamp.ZidBound>) {
        self.time = time
        self.zid = zid
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecExtTimestamp {
        var reader = cursor
        let time = try reader.readVle(UInt64.self)
        let zid_size = try reader.readInteger(UInt8.self, .big)
        if zid_size > 16 {
            throw CodecError.lengthOutOfRange
        }
        let zid = try BoundedBytes<CodecExtTimestamp.ZidBound>(decoding: Array(try reader.readSlice(Int(clamping: zid_size))))
        cursor = reader
        return CodecExtTimestamp(time: time, zid: zid)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecExtTimestampSource` is written,
    /// with no copy into `CodecExtTimestamp`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecExtTimestampSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let zid_size_derived: UInt8 = UInt8(source.zid.byteCount)
        var frame: [UInt8] = []
        appendVle(source.time, to: &frame)
        appendInteger(zid_size_derived, to: &frame, .big)
        source.zid.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
