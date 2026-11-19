// SCE-GENERATED — DO NOT EDIT
// source-hash: d09b375634699903d2b7cd2e3536677067ac6de208431b560409a4e3b5d1ee60
// source: codec_chain_prev_entry.scxml
// SCE-MAP: codec_chain_prev_entry.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecChainPrevEntry` the gates `after_marker` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
public enum CodecChainPrevEntryAfterMarker: Equatable, Sendable {
    /// While after_marker set.
    case afterMarker(long_value: UInt16)
    /// While after_marker clear.
    case notAfterMarker(short_value: UInt8)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .afterMarker: return 0
        case .notAfterMarker: return 1
        }
    }
}

/// The elements a value of `CodecChainPrevEntry` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainPrevEntry.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainPrevEntry`, which conforms itself.
public protocol CodecChainPrevEntrySource {
    var header: UInt8 { get }
    var after_marker: CodecChainPrevEntryAfterMarker { get }
    var tag: UInt8? { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainPrevEntry: Equatable, Sendable, CodecChainPrevEntrySource {
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

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(tagged & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 4, big-endian.
    public var tag: UInt8? = nil

    /// The elements the gates `after_marker` decide together.
    public var after_marker: CodecChainPrevEntryAfterMarker = .notAfterMarker(short_value: 0)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, after_marker: CodecChainPrevEntryAfterMarker, tag: UInt8?) {
        self.header = header
        self.after_marker = after_marker
        self.tag = tag
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, after_marker: UInt8, tagged: UInt8) throws(CodecError) -> CodecChainPrevEntry {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var short_value: UInt8? = nil
        if (after_marker & 0x1) == 0 {
            short_value = try reader.readInteger(UInt8.self, .big)
        }
        var long_value: UInt16? = nil
        if (after_marker & 0x1) != 0 {
            long_value = try reader.readInteger(UInt16.self, .big)
        }
        var tag: UInt8? = nil
        if (tagged & 0x1) != 0 {
            tag = try reader.readInteger(UInt8.self, .big)
        }
        let after_marker_group: CodecChainPrevEntryAfterMarker
        if (after_marker & 0x01) != 0 {
            after_marker_group = .afterMarker(long_value: long_value!)
        } else {
            after_marker_group = .notAfterMarker(short_value: short_value!)
        }
        cursor = reader
        return CodecChainPrevEntry(header: header, after_marker: after_marker_group, tag: tag)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecChainPrevEntry` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `Z` flag of `header`, where a parent sets it.
        public var headerZ: Bool?

        public init(headerZ: Bool? = nil) {
            self.headerZ = headerZ
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainPrevEntrySource` is written,
    /// with no copy into `CodecChainPrevEntry`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainPrevEntrySource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var long_value_member: UInt16? = nil
        var short_value_member: UInt8? = nil
        switch source.after_marker {
        case .afterMarker(let long_value): long_value_member = long_value
        case .notAfterMarker(let short_value): short_value_member = short_value
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
        if let held = source.tag {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `after_marker` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputAfterMarker<Source: CodecChainPrevEntrySource>(_ source: Source) -> UInt8 {
        [0].contains(source.after_marker.arm) ? 1 : 0
    }

    /// The flag input `tagged` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputTagged<Source: CodecChainPrevEntrySource>(_ source: Source) -> UInt8 {
        source.tag != nil ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
