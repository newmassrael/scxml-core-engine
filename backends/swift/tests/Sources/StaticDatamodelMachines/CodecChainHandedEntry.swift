// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_chain_handed_entry.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainHandedEntry` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHandedEntry.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHandedEntry`, which conforms itself.
public protocol CodecChainHandedEntrySource {
    var header: UInt8 { get }
    var value: UInt8? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHandedEntry: Equatable, Sendable, CodecChainHandedEntrySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 4 bits of `header` from bit 0 (flag `ext_id`). A value
    /// wider than the flag is cut to it when written.
    public var extId: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0xF) }
        set { header = (header & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// Bit 6 of `header` (flag `wide`).
    public var wide: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(after_marker & 0x1) != 0 || (header & 0x40) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var value: UInt8? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, value: UInt8?) {
        self.header = header
        self.value = value
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, after_marker: UInt8) throws(CodecError) -> CodecChainHandedEntry {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var value: UInt8? = nil
        if (after_marker & 0x1) != 0 || (header & 0x40) != 0 {
            value = try reader.readInteger(UInt8.self, .big)
        }
        cursor = reader
        return CodecChainHandedEntry(header: header, value: value)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecChainHandedEntry` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `wide` flag of `header`, where a parent sets it.
        public var headerWide: Bool?
        /// The `Z` flag of `header`, where a parent sets it.
        public var headerZ: Bool?

        public init(headerWide: Bool? = nil, headerZ: Bool? = nil) {
            self.headerWide = headerWide
            self.headerZ = headerZ
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHandedEntrySource` is written,
    /// with no copy into `CodecChainHandedEntry`, and every other element of the frame is derived from
    /// them. A value the encoding cannot carry is refused before the first
    /// byte is written, with the `CodecError` the codec's totality names. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHandedEntrySource, Sink: SceSink>(_ source: Source, into sink: inout Sink, after_marker: UInt8, overrides: Overrides = Overrides()) throws(CodecError) {
        var header_folded = source.header
        if let set = overrides.headerWide { header_folded = set ? (header_folded | 0x40) : (header_folded & ~0x40) }
        if let set = overrides.headerZ { header_folded = set ? (header_folded | 0x80) : (header_folded & ~0x80) }
        if (after_marker != 0 || ((header_folded >> 6) & 1) != 0) != (source.value != nil) { throw CodecError.presentIfMismatch }
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = source.value {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink, after_marker: UInt8) throws(CodecError) {
        try Self.encodeSource(self, into: &sink, after_marker: after_marker)
    }
}
