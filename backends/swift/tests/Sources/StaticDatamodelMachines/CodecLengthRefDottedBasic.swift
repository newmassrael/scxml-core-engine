// SCE-GENERATED — DO NOT EDIT
// source-hash: fc64695a44b9210f57a6567030d85bf2a479334b65d23fdc6096eb5cb3c5c795
// source: codec_length_ref_dotted_basic.scxml
// SCE-MAP: codec_length_ref_dotted_basic.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecLengthRefDottedBasic` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecLengthRefDottedBasic.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecLengthRefDottedBasic`, which conforms itself.
public protocol CodecLengthRefDottedBasicSource {
    var carrier: UInt8 { get }
    var payload: BoundedBytes<CodecLengthRefDottedBasic.PayloadBound> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecLengthRefDottedBasic: Equatable, Sendable, CodecLengthRefDottedBasicSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `payload` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum PayloadBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 15
    }

    /// The field at byte 0, big-endian.
    public var carrier: UInt8 = 0

    /// The 4 bits of `carrier` from bit 0 (flag `hdr`). A value
    /// wider than the flag is cut to it when written.
    public var hdr: UInt8 {
        get { UInt8(truncatingIfNeeded: (carrier >> 0) & 0xF) }
        set { carrier = (carrier & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// The 4 bits of `carrier` from bit 4 (flag `payload_len`). A value
    /// wider than the flag is cut to it when written.
    public var payloadLen: UInt8 {
        get { UInt8(truncatingIfNeeded: (carrier >> 4) & 0xF) }
        set { carrier = (carrier & ~(0xF << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 4) }
    }

    /// The bytes, as many as `carrier.payload_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var payload: BoundedBytes<CodecLengthRefDottedBasic.PayloadBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(carrier: UInt8, payload: BoundedBytes<CodecLengthRefDottedBasic.PayloadBound>) {
        self.carrier = carrier
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecLengthRefDottedBasic {
        var reader = cursor
        let carrier = try reader.readInteger(UInt8.self, .big)
        let payload = try BoundedBytes<CodecLengthRefDottedBasic.PayloadBound>(decoding: Array(try reader.readSlice(Int(clamping: ((carrier >> 4) & 0xF)))))
        cursor = reader
        return CodecLengthRefDottedBasic(carrier: carrier, payload: payload)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecLengthRefDottedBasicSource` is written,
    /// with no copy into `CodecLengthRefDottedBasic`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecLengthRefDottedBasicSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var carrier_folded = source.carrier
        carrier_folded = (carrier_folded & ~0xF0) | ((UInt8(truncatingIfNeeded: source.payload.byteCount) & 0xF) << 4)
        var frame = [UInt8](repeating: 0, count: 1)
        storeInteger(carrier_folded, into: &frame, at: 0, .big)
        source.payload.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
