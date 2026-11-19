// SCE-GENERATED — DO NOT EDIT
// source-hash: a548df1eb74b088f0f1f9753a2100130eecc77f681031714751b815f56498c8e
// source: codec_tlv_chain_present_if_basic.scxml
// SCE-MAP: codec_tlv_chain_present_if_basic.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecTlvChainPresentIfBasic` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecTlvChainPresentIfBasic.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecTlvChainPresentIfBasic`, which conforms itself.
public protocol CodecTlvChainPresentIfBasicSource {
    /// What each entry of `entries` is presented as: a source of `CodecTlvEntry`, written by its encode.
    associatedtype Entries_Entry: CodecTlvEntrySource
    var carrier: UInt8 { get }
    var entries: BoundedList<Entries_Entry, CodecTlvChainPresentIfBasic.EntriesBound>? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecTlvChainPresentIfBasic: Equatable, Sendable, CodecTlvChainPresentIfBasicSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many entries `entries` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum EntriesBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 4
    }

    /// The value presents each entry of `entries` as a value of its own codec.
    public typealias Entries_Entry = CodecTlvEntry

    /// The field at byte 0, big-endian.
    public var carrier: UInt8 = 0

    /// Bit 0 of `carrier` (flag `has_chain`).
    public var hasChain: Bool {
        get { (carrier & 0x1) != 0 }
        set { carrier = newValue ? (carrier | 0x1) : (carrier & ~0x1) }
    }

    /// On the wire only when `(carrier & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecTlvEntry`, one after another, at most 4: to the end of the frame. A frame
    /// with more is refused with `tooManyElements`, and a value with more is no value of its type.
    public var entries: BoundedList<CodecTlvEntry, CodecTlvChainPresentIfBasic.EntriesBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(carrier: UInt8, entries: BoundedList<Entries_Entry, CodecTlvChainPresentIfBasic.EntriesBound>?) {
        self.carrier = carrier
        self.entries = entries
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecTlvChainPresentIfBasic {
        var reader = cursor
        let carrier = try reader.readInteger(UInt8.self, .big)
        var entries: BoundedList<CodecTlvEntry, CodecTlvChainPresentIfBasic.EntriesBound>? = nil
        if (carrier & 0x1) != 0 {
            var entries_items: [CodecTlvEntry] = []
            for _ in 0..<4 {
                if reader.remaining == 0 { break }
                let entry = try CodecTlvEntry.decode(&reader)
                entries_items.append(entry)
            }
            if reader.remaining > 0 {
                throw CodecError.tooManyElements
            }
            entries = try BoundedList<CodecTlvEntry, CodecTlvChainPresentIfBasic.EntriesBound>(decoding: entries_items)
        }
        cursor = reader
        return CodecTlvChainPresentIfBasic(carrier: carrier, entries: entries)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecTlvChainPresentIfBasicSource` is written,
    /// with no copy into `CodecTlvChainPresentIfBasic`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecTlvChainPresentIfBasicSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var carrier_folded = source.carrier
        carrier_folded = (carrier_folded & ~0x1) | ((source.entries != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(carrier_folded, to: &frame, .big)
        if let held = source.entries {
            var entries_entries = BytesSink()
            for entry in held {
                try CodecTlvEntry.encodeSource(entry, into: &entries_entries)
            }
            frame += entries_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
