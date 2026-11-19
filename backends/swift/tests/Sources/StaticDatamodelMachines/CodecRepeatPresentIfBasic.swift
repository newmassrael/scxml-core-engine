// SCE-GENERATED — DO NOT EDIT
// source-hash: babf5a7c60cb177c5a8efc106a25a5dfe225aac2292a0b88286a187cb8250552
// source: codec_repeat_present_if_basic.scxml
// SCE-MAP: codec_repeat_present_if_basic.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecRepeatPresentIfBasic` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecRepeatPresentIfBasic.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecRepeatPresentIfBasic`, which conforms itself.
public protocol CodecRepeatPresentIfBasicSource {
    /// What each entry of `elems` is presented as: a source of `CodecRepeatElem`, written by its encode.
    associatedtype Elems_Entry: CodecRepeatElemSource
    var carrier: UInt8 { get }
    var elems: BoundedList<Elems_Entry, CodecRepeatPresentIfBasic.ElemsBound>? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatPresentIfBasic: Equatable, Sendable, CodecRepeatPresentIfBasicSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many entries `elems` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ElemsBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 32
    }

    /// The value presents each entry of `elems` as a value of its own codec.
    public typealias Elems_Entry = CodecRepeatElem

    /// The field at byte 0, big-endian.
    public var carrier: UInt8 = 0

    /// Bit 0 of `carrier` (flag `has_list`).
    public var hasList: Bool {
        get { (carrier & 0x1) != 0 }
        set { carrier = newValue ? (carrier | 0x1) : (carrier & ~0x1) }
    }

    /// On the wire only when `(carrier & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// As many entries as `num_elems` says, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, and a value
    /// with more is no value of its type; the encode derives the count.
    public var elems: BoundedList<CodecRepeatElem, CodecRepeatPresentIfBasic.ElemsBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(carrier: UInt8, elems: BoundedList<Elems_Entry, CodecRepeatPresentIfBasic.ElemsBound>?) {
        self.carrier = carrier
        self.elems = elems
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecRepeatPresentIfBasic {
        var reader = cursor
        let carrier = try reader.readInteger(UInt8.self, .big)
        var num_elems: UInt8? = nil
        if (carrier & 0x1) != 0 {
            num_elems = try reader.readInteger(UInt8.self, .big)
        }
        var elems: BoundedList<CodecRepeatElem, CodecRepeatPresentIfBasic.ElemsBound>? = nil
        if (carrier & 0x1) != 0 {
            if (num_elems ?? 0) > 32 {
                throw CodecError.tooManyElements
            }
            var elems_items: [CodecRepeatElem] = []
            for _ in 0..<Int(clamping: (num_elems ?? 0)) {
                elems_items.append(try CodecRepeatElem.decode(&reader))
            }
            elems = try BoundedList<CodecRepeatElem, CodecRepeatPresentIfBasic.ElemsBound>(decoding: elems_items)
        }
        cursor = reader
        return CodecRepeatPresentIfBasic(carrier: carrier, elems: elems)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecRepeatPresentIfBasicSource` is written,
    /// with no copy into `CodecRepeatPresentIfBasic`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecRepeatPresentIfBasicSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let num_elems_derived: UInt8 = source.elems != nil ? UInt8((source.elems?.count ?? 0)) : 0
        var carrier_folded = source.carrier
        carrier_folded = (carrier_folded & ~0x1) | ((source.elems != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(carrier_folded, to: &frame, .big)
        if ((carrier_folded >> 0) & 1) != 0 {
            appendInteger(num_elems_derived, to: &frame, .big)
        }
        if let held = source.elems {
            var elems_entries = BytesSink()
            for entry in held {
                try CodecRepeatElem.encodeSource(entry, into: &elems_entries)
            }
            frame += elems_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
