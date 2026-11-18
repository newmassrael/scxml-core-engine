// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_present_if_vle.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecPresentIfVle` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecPresentIfVle.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecPresentIfVle`, which conforms itself.
public protocol CodecPresentIfVleSource {
    var flags: UInt8 { get }
    var optional_id: UInt64? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPresentIfVle: Equatable, Sendable, CodecPresentIfVleSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_id`).
    public var hasId: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var optional_id: UInt64? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, optional_id: UInt64?) {
        self.flags = flags
        self.optional_id = optional_id
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPresentIfVle {
        var reader = cursor
        let flags = try reader.readInteger(UInt8.self, .big)
        var optional_id: UInt64? = nil
        if (flags & 0x1) != 0 {
            optional_id = try reader.readVle(UInt64.self)
        }
        cursor = reader
        return CodecPresentIfVle(flags: flags, optional_id: optional_id)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecPresentIfVleSource` is written,
    /// with no copy into `CodecPresentIfVle`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecPresentIfVleSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var flags_folded = source.flags
        flags_folded = (flags_folded & ~0x1) | ((source.optional_id != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(flags_folded, to: &frame, .big)
        if let held = source.optional_id {
            appendVle(held, to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
