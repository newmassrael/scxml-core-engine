// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_present_if_string.scxml:47 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecPresentIfString` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecPresentIfString.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecPresentIfString`, which conforms itself.
public protocol CodecPresentIfStringSource {
    var carrier: UInt8 { get }
    var text: BoundedText<CodecPresentIfString.TextBound>? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPresentIfString: Equatable, Sendable, CodecPresentIfStringSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `text` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum TextBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 32
    }

    /// The field at byte 0, big-endian.
    public var carrier: UInt8 = 0

    /// Bit 0 of `carrier` (flag `has_text`).
    public var hasText: Bool {
        get { (carrier & 0x1) != 0 }
        set { carrier = newValue ? (carrier | 0x1) : (carrier & ~0x1) }
    }

    /// On the wire only when `(carrier & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The text of as many UTF-8 bytes as `text_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var text: BoundedText<CodecPresentIfString.TextBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(carrier: UInt8, text: BoundedText<CodecPresentIfString.TextBound>?) {
        self.carrier = carrier
        self.text = text
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPresentIfString {
        var reader = cursor
        let carrier = try reader.readInteger(UInt8.self, .big)
        var text_len: UInt8? = nil
        if (carrier & 0x1) != 0 {
            text_len = try reader.readInteger(UInt8.self, .big)
        }
        var text: BoundedText<CodecPresentIfString.TextBound>? = nil
        if (carrier & 0x1) != 0 {
            if (text_len ?? 0) > 32 {
                throw CodecError.lengthOutOfRange
            }
            text = try BoundedText<CodecPresentIfString.TextBound>(decoding: try reader.readString(Int(clamping: (text_len ?? 0))))
        }
        cursor = reader
        return CodecPresentIfString(carrier: carrier, text: text)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecPresentIfStringSource` is written,
    /// with no copy into `CodecPresentIfString`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecPresentIfStringSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let text_len_derived: UInt8 = source.text != nil ? UInt8((source.text?.byteCount ?? 0)) : 0
        var carrier_folded = source.carrier
        carrier_folded = (carrier_folded & ~0x1) | ((source.text != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(carrier_folded, to: &frame, .big)
        if ((carrier_folded >> 0) & 1) != 0 {
            appendInteger(text_len_derived, to: &frame, .big)
        }
        if let held = source.text {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
