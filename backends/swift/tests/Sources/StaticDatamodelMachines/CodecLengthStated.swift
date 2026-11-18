// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_length_stated.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecLengthStated` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecLengthStated.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecLengthStated`, which conforms itself.
public protocol CodecLengthStatedSource {
    var flags: UInt8 { get }
    var span_len: UInt8? { get }
    var alpha: BoundedBytes<CodecLengthStated.AlphaBound>? { get }
    var beta: BoundedBytes<CodecLengthStated.BetaBound>? { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecLengthStated: Equatable, Sendable, CodecLengthStatedSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `alpha` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum AlphaBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 255
    }

    /// How many bytes `beta` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum BetaBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 255
    }

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_alpha`).
    public var hasAlpha: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// Bit 1 of `flags` (flag `has_beta`).
    public var hasBeta: Bool {
        get { (flags & 0x2) != 0 }
        set { flags = newValue ? (flags | 0x2) : (flags & ~0x2) }
    }

    /// The field at byte 1, big-endian.
    public var span_len: UInt8? = nil

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `span_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var alpha: BoundedBytes<CodecLengthStated.AlphaBound>? = nil

    /// On the wire only when `(flags & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `span_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var beta: BoundedBytes<CodecLengthStated.BetaBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, span_len: UInt8?, alpha: BoundedBytes<CodecLengthStated.AlphaBound>?, beta: BoundedBytes<CodecLengthStated.BetaBound>?) {
        self.flags = flags
        self.span_len = span_len
        self.alpha = alpha
        self.beta = beta
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecLengthStated {
        var reader = cursor
        let flags = try reader.readInteger(UInt8.self, .big)
        let span_len = try reader.readInteger(UInt8.self, .big)
        var alpha: BoundedBytes<CodecLengthStated.AlphaBound>? = nil
        if (flags & 0x1) != 0 {
            alpha = try BoundedBytes<CodecLengthStated.AlphaBound>(decoding: Array(try reader.readSlice(Int(clamping: span_len))))
        }
        var beta: BoundedBytes<CodecLengthStated.BetaBound>? = nil
        if (flags & 0x2) != 0 {
            beta = try BoundedBytes<CodecLengthStated.BetaBound>(decoding: Array(try reader.readSlice(Int(clamping: span_len))))
        }
        cursor = reader
        return CodecLengthStated(flags: flags, span_len: (alpha == nil && beta == nil) ? span_len : nil, alpha: alpha, beta: beta)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecLengthStatedSource` is written,
    /// with no copy into `CodecLengthStated`, and every other element of the frame is derived from
    /// them. A value the encoding cannot carry is refused before the first
    /// byte is written, with the `CodecError` the codec's totality names. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecLengthStatedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var span_len_derived_first: Int? = nil
        if source.alpha != nil {
            span_len_derived_first = (source.alpha?.byteCount ?? 0)
        } else if source.beta != nil {
            span_len_derived_first = (source.beta?.byteCount ?? 0)
        }
        if source.beta != nil && (source.beta?.byteCount ?? 0) != span_len_derived_first { throw CodecError.inconsistentSource }
        let span_len_derived: UInt8 = span_len_derived_first != nil ? UInt8((span_len_derived_first ?? 0)) : (source.span_len ?? 0)
        if span_len_derived_first != nil, let stated = source.span_len, stated != span_len_derived { throw CodecError.inconsistentSource }
        var flags_folded = source.flags
        flags_folded = (flags_folded & ~0x1) | ((source.alpha != nil) ? 0x1 : 0)
        flags_folded = (flags_folded & ~0x2) | ((source.beta != nil) ? 0x2 : 0)
        var frame: [UInt8] = []
        appendInteger(flags_folded, to: &frame, .big)
        appendInteger(span_len_derived, to: &frame, .big)
        if let held = source.alpha {
            held.append(to: &frame)
        }
        if let held = source.beta {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
