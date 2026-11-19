// SCE-GENERATED — DO NOT EDIT
// source-hash: aa05f3e741c0f3571904b513bccf4ad394dd18c921630514a626e5342db52ad9
// source: codec_zenoh_frame.scxml
// SCE-MAP: codec_zenoh_frame.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohFrame` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohFrame.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohFrame`, which conforms itself.
public protocol CodecZenohFrameSource {
    var sn: UInt64 { get }
    var payload: BoundedBytes<CodecZenohFrame.PayloadBound> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohFrame: Equatable, Sendable, CodecZenohFrameSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `payload` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum PayloadBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 65536
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var sn: UInt64 = 0

    /// The bytes to the end of the frame, as many as the document's `max-size` admits: a frame
    /// with more is refused with `lengthOutOfRange`, and a value with more is no value of its type.
    public var payload: BoundedBytes<CodecZenohFrame.PayloadBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(sn: UInt64, payload: BoundedBytes<CodecZenohFrame.PayloadBound>) {
        self.sn = sn
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohFrame {
        var reader = cursor
        let sn = try reader.readVle(UInt64.self)
        if reader.remaining > 65536 {
            throw CodecError.lengthOutOfRange
        }
        let payload = try BoundedBytes<CodecZenohFrame.PayloadBound>(decoding: Array(try reader.readSlice(reader.remaining)))
        cursor = reader
        return CodecZenohFrame(sn: sn, payload: payload)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohFrameSource` is written,
    /// with no copy into `CodecZenohFrame`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohFrameSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        appendVle(source.sn, to: &frame)
        source.payload.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
