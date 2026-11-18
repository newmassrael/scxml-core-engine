// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_length_ref.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecLengthRef` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecLengthRef.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecLengthRef`, which conforms itself.
public protocol CodecLengthRefSource {
    var msgId: UInt8 { get }
    var payload: BoundedBytes<CodecLengthRef.PayloadBound> { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecLengthRef: Equatable, Sendable, CodecLengthRefSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `payload` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum PayloadBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 32
    }

    /// The field at byte 0, big-endian.
    public var msgId: UInt8 = 0

    /// The bytes, as many as `len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var payload: BoundedBytes<CodecLengthRef.PayloadBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(msgId: UInt8, payload: BoundedBytes<CodecLengthRef.PayloadBound>) {
        self.msgId = msgId
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecLengthRef {
        var reader = cursor
        let msgId = try reader.readInteger(UInt8.self, .big)
        let len = try reader.readInteger(UInt8.self, .big)
        if len > 32 {
            throw CodecError.lengthOutOfRange
        }
        let payload = try BoundedBytes<CodecLengthRef.PayloadBound>(decoding: Array(try reader.readSlice(Int(clamping: len))))
        cursor = reader
        return CodecLengthRef(msgId: msgId, payload: payload)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecLengthRefSource` is written,
    /// with no copy into `CodecLengthRef`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecLengthRefSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let len_derived: UInt8 = UInt8(source.payload.byteCount)
        var frame = [UInt8](repeating: 0, count: 2)
        storeInteger(source.msgId, into: &frame, at: 0, .big)
        storeInteger(len_derived, into: &frame, at: 1, .big)
        source.payload.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
