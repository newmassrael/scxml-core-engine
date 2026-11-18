// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_declared_bounds_gated.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecDeclaredBoundsGated` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecDeclaredBoundsGated.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecDeclaredBoundsGated`, which conforms itself.
public protocol CodecDeclaredBoundsGatedSource {
    var flags: UInt8 { get }
    var name: BoundedText<CodecDeclaredBoundsGated.NameBound>? { get }
    var rest: BoundedBytes<CodecDeclaredBoundsGated.RestBound> { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecDeclaredBoundsGated: Equatable, Sendable, CodecDeclaredBoundsGatedSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `name` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum NameBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 3
    }

    /// How many bytes `rest` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum RestBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 2
    }

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `G`).
    public var G: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The text of as many UTF-8 bytes as `name_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var name: BoundedText<CodecDeclaredBoundsGated.NameBound>? = nil

    /// The bytes to the end of the frame, as many as the document's `max-size` admits: a frame
    /// with more is refused with `lengthOutOfRange`, and a value with more is no value of its type.
    public var rest: BoundedBytes<CodecDeclaredBoundsGated.RestBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, name: BoundedText<CodecDeclaredBoundsGated.NameBound>?, rest: BoundedBytes<CodecDeclaredBoundsGated.RestBound>) {
        self.flags = flags
        self.name = name
        self.rest = rest
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecDeclaredBoundsGated {
        var reader = cursor
        let flags = try reader.readInteger(UInt8.self, .big)
        var name_len: UInt8? = nil
        if (flags & 0x1) != 0 {
            name_len = try reader.readInteger(UInt8.self, .big)
        }
        var name: BoundedText<CodecDeclaredBoundsGated.NameBound>? = nil
        if (flags & 0x1) != 0 {
            if (name_len ?? 0) > 3 {
                throw CodecError.lengthOutOfRange
            }
            name = try BoundedText<CodecDeclaredBoundsGated.NameBound>(decoding: try reader.readString(Int(clamping: (name_len ?? 0))))
        }
        if reader.remaining > 2 {
            throw CodecError.lengthOutOfRange
        }
        let rest = try BoundedBytes<CodecDeclaredBoundsGated.RestBound>(decoding: Array(try reader.readSlice(reader.remaining)))
        cursor = reader
        return CodecDeclaredBoundsGated(flags: flags, name: name, rest: rest)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecDeclaredBoundsGatedSource` is written,
    /// with no copy into `CodecDeclaredBoundsGated`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecDeclaredBoundsGatedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let name_len_derived: UInt8 = source.name != nil ? UInt8((source.name?.byteCount ?? 0)) : 0
        var flags_folded = source.flags
        flags_folded = (flags_folded & ~0x1) | ((source.name != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(flags_folded, to: &frame, .big)
        if ((flags_folded >> 0) & 1) != 0 {
            appendInteger(name_len_derived, to: &frame, .big)
        }
        if let held = source.name {
            held.append(to: &frame)
        }
        source.rest.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
