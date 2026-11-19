// SCE-GENERATED — DO NOT EDIT
// source-hash: c894bef24ddaa7280190ca7a64fef1e27cde508231dcd262e847a899318a6713
// source: codec_present_if_tail.scxml
// SCE-MAP: codec_present_if_tail.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecPresentIfTail` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecPresentIfTail.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecPresentIfTail`, which conforms itself.
public protocol CodecPresentIfTailSource {
    var flags: UInt8 { get }
    var payload: BoundedBytes<CodecPresentIfTail.PayloadBound>? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPresentIfTail: Equatable, Sendable, CodecPresentIfTailSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `payload` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum PayloadBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 64
    }

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_payload`).
    public var hasPayload: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes to the end of the frame, as many as the document's `max-size` admits: a frame
    /// with more is refused with `lengthOutOfRange`, and a value with more is no value of its type.
    public var payload: BoundedBytes<CodecPresentIfTail.PayloadBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, payload: BoundedBytes<CodecPresentIfTail.PayloadBound>?) {
        self.flags = flags
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPresentIfTail {
        var reader = cursor
        let flags = try reader.readInteger(UInt8.self, .big)
        var payload: BoundedBytes<CodecPresentIfTail.PayloadBound>? = nil
        if (flags & 0x1) != 0 {
            if reader.remaining > 64 {
                throw CodecError.lengthOutOfRange
            }
            payload = try BoundedBytes<CodecPresentIfTail.PayloadBound>(decoding: Array(try reader.readSlice(reader.remaining)))
        }
        cursor = reader
        return CodecPresentIfTail(flags: flags, payload: payload)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecPresentIfTailSource` is written,
    /// with no copy into `CodecPresentIfTail`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecPresentIfTailSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var flags_folded = source.flags
        flags_folded = (flags_folded & ~0x1) | ((source.payload != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(flags_folded, to: &frame, .big)
        if let held = source.payload {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
