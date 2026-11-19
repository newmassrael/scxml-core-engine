// SCE-GENERATED — DO NOT EDIT
// source-hash: 5ef603782381c6546797ec4c003c06995ebd02c4f56cbba8b20ca4f301fab154
// source: codec_present_if_checked.scxml
// SCE-MAP: codec_present_if_checked.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecPresentIfChecked` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecPresentIfChecked.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecPresentIfChecked`, which conforms itself.
public protocol CodecPresentIfCheckedSource {
    var flags: UInt8 { get }
    var seq: UInt16? { get }
    var trailer: UInt8? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPresentIfChecked: Equatable, Sendable, CodecPresentIfCheckedSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_seq`).
    public var hasSeq: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// Bit 1 of `flags` (flag `wants_trailer`).
    public var wantsTrailer: Bool {
        get { (flags & 0x2) != 0 }
        set { flags = newValue ? (flags | 0x2) : (flags & ~0x2) }
    }

    /// On the wire only when `(flags & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var seq: UInt16? = nil

    /// On the wire only when `(flags & 0x1) != 0 || (flags & 0x2) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 3, big-endian.
    public var trailer: UInt8? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, seq: UInt16?, trailer: UInt8?) {
        self.flags = flags
        self.seq = seq
        self.trailer = trailer
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPresentIfChecked {
        var reader = cursor
        let flags = try reader.readInteger(UInt8.self, .big)
        var seq: UInt16? = nil
        if (flags & 0x1) != 0 {
            seq = try reader.readInteger(UInt16.self, .big)
        }
        var trailer: UInt8? = nil
        if (flags & 0x1) != 0 || (flags & 0x2) != 0 {
            trailer = try reader.readInteger(UInt8.self, .big)
        }
        cursor = reader
        return CodecPresentIfChecked(flags: flags, seq: seq, trailer: trailer)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecPresentIfChecked` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `wants_trailer` flag of `flags`, where a parent sets it.
        public var flagsWantsTrailer: Bool?

        public init(flagsWantsTrailer: Bool? = nil) {
            self.flagsWantsTrailer = flagsWantsTrailer
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecPresentIfCheckedSource` is written,
    /// with no copy into `CodecPresentIfChecked`, and every other element of the frame is derived from
    /// them. A value the encoding cannot carry is refused before the first
    /// byte is written, with the `CodecError` the codec's totality names. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecPresentIfCheckedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var flags_folded = source.flags
        flags_folded = (flags_folded & ~0x1) | ((source.seq != nil) ? 0x1 : 0)
        if let set = overrides.flagsWantsTrailer { flags_folded = set ? (flags_folded | 0x2) : (flags_folded & ~0x2) }
        if (((flags_folded >> 0) & 1) != 0 || ((flags_folded >> 1) & 1) != 0) != (source.trailer != nil) { throw CodecError.presentIfMismatch }
        var frame: [UInt8] = []
        appendInteger(flags_folded, to: &frame, .big)
        if let held = source.seq {
            appendInteger(held, to: &frame, .big)
        }
        if let held = source.trailer {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
