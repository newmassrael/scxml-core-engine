// SCE-GENERATED — DO NOT EDIT
// source-hash: 852c710804412f87a0591609f6d05b9c1c2651db926cc87bd42829ef229e01d1
// source: codec_chain_flag_entry.scxml
// SCE-MAP: codec_chain_flag_entry.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecChainFlagEntry` the gates `wide` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
public enum CodecChainFlagEntryWide: Equatable, Sendable {
    /// While wide set.
    case wide(long_value: UInt16)
    /// While wide clear.
    case notWide(short_value: UInt8)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .wide: return 0
        case .notWide: return 1
        }
    }
}

/// The elements a value of `CodecChainFlagEntry` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainFlagEntry.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainFlagEntry`, which conforms itself.
public protocol CodecChainFlagEntrySource {
    var header: UInt8 { get }
    var wide: CodecChainFlagEntryWide { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainFlagEntry: Equatable, Sendable, CodecChainFlagEntrySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 4 bits of `header` from bit 0 (flag `kind`). A value
    /// wider than the flag is cut to it when written.
    public var kind: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0xF) }
        set { header = (header & ~(0xF << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 0) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// The elements the gates `wide` decide together.
    public var wide: CodecChainFlagEntryWide = .notWide(short_value: 0)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, wide: CodecChainFlagEntryWide) {
        self.header = header
        self.wide = wide
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, wide: UInt8) throws(CodecError) -> CodecChainFlagEntry {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var short_value: UInt8? = nil
        if (wide & 0x1) == 0 {
            short_value = try reader.readInteger(UInt8.self, .big)
        }
        var long_value: UInt16? = nil
        if (wide & 0x1) != 0 {
            long_value = try reader.readInteger(UInt16.self, .big)
        }
        let wide_group: CodecChainFlagEntryWide
        if (wide & 0x01) != 0 {
            wide_group = .wide(long_value: long_value!)
        } else {
            wide_group = .notWide(short_value: short_value!)
        }
        cursor = reader
        return CodecChainFlagEntry(header: header, wide: wide_group)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecChainFlagEntry` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `Z` flag of `header`, where a parent sets it.
        public var headerZ: Bool?

        public init(headerZ: Bool? = nil) {
            self.headerZ = headerZ
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainFlagEntrySource` is written,
    /// with no copy into `CodecChainFlagEntry`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainFlagEntrySource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var long_value_member: UInt16? = nil
        var short_value_member: UInt8? = nil
        switch source.wide {
        case .wide(let long_value): long_value_member = long_value
        case .notWide(let short_value): short_value_member = short_value
        }
        var header_folded = source.header
        if let set = overrides.headerZ { header_folded = set ? (header_folded | 0x80) : (header_folded & ~0x80) }
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = short_value_member {
            appendInteger(held, to: &frame, .big)
        }
        if let held = long_value_member {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `wide` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputWide<Source: CodecChainFlagEntrySource>(_ source: Source) -> UInt8 {
        [0].contains(source.wide.arm) ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
