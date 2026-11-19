// SCE-GENERATED — DO NOT EDIT
// source-hash: e32283ea1358db9b6bb8be6aea88a4126bfacf03a5d80ca1f39836cd73da88a5
// source: codec_scout_zid_body.scxml
// SCE-MAP: codec_scout_zid_body.scxml:35 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecScoutZidBody` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecScoutZidBody.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecScoutZidBody`, which conforms itself.
public protocol CodecScoutZidBodySource {
    var zid: BoundedBytes<CodecScoutZidBody.ZidBound> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecScoutZidBody: Equatable, Sendable, CodecScoutZidBodySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `zid` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ZidBound: SceByteBound {
        public static let least = 1
        public static let most: Int? = 16
    }

    /// The bytes, as many as `zid_len_m1` says, plus 1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var zid: BoundedBytes<CodecScoutZidBody.ZidBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(zid: BoundedBytes<CodecScoutZidBody.ZidBound>) {
        self.zid = zid
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecScoutZidBody {
        var reader = cursor
        let zid_len_m1 = try reader.readInteger(UInt8.self, .big)
        if zid_len_m1 > 15 {
            throw CodecError.lengthOutOfRange
        }
        let zid = try BoundedBytes<CodecScoutZidBody.ZidBound>(decoding: Array(try reader.readSlice(Int(clamping: zid_len_m1) &+ (1))))
        cursor = reader
        return CodecScoutZidBody(zid: zid)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecScoutZidBodySource` is written,
    /// with no copy into `CodecScoutZidBody`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecScoutZidBodySource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let zid_len_m1_derived: UInt8 = UInt8((source.zid.byteCount - 1))
        var frame = [UInt8](repeating: 0, count: 1)
        storeInteger(zid_len_m1_derived, into: &frame, at: 0, .big)
        source.zid.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
