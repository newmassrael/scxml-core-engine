// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_repeat_basic.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecRepeatBasic` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecRepeatBasic.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecRepeatBasic`, which conforms itself.
public protocol CodecRepeatBasicSource {
    /// What each entry of `frags` is presented as: a source of `CodecRepeatElem`, written by its encode.
    associatedtype Frags_Entry: CodecRepeatElemSource
    var frags: BoundedList<Frags_Entry, CodecRepeatBasic.FragsBound> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatBasic: Equatable, Sendable, CodecRepeatBasicSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many entries `frags` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum FragsBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 32
    }

    /// The value presents each entry of `frags` as a value of its own codec.
    public typealias Frags_Entry = CodecRepeatElem

    /// As many entries as `num_frags` says, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, and a value
    /// with more is no value of its type; the encode derives the count.
    public var frags: BoundedList<CodecRepeatElem, CodecRepeatBasic.FragsBound> = .fewest(of: CodecRepeatElem())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(frags: BoundedList<Frags_Entry, CodecRepeatBasic.FragsBound>) {
        self.frags = frags
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecRepeatBasic {
        var reader = cursor
        let num_frags = try reader.readInteger(UInt8.self, .big)
        if num_frags > 32 {
            throw CodecError.tooManyElements
        }
        var frags_items: [CodecRepeatElem] = []
        for _ in 0..<Int(clamping: num_frags) {
            frags_items.append(try CodecRepeatElem.decode(&reader))
        }
        let frags = try BoundedList<CodecRepeatElem, CodecRepeatBasic.FragsBound>(decoding: frags_items)
        cursor = reader
        return CodecRepeatBasic(frags: frags)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecRepeatBasicSource` is written,
    /// with no copy into `CodecRepeatBasic`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecRepeatBasicSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let num_frags_derived: UInt8 = UInt8(source.frags.count)
        var frame: [UInt8] = []
        appendInteger(num_frags_derived, to: &frame, .big)
        var frags_entries = BytesSink()
        for entry in source.frags {
            try CodecRepeatElem.encodeSource(entry, into: &frags_entries)
        }
        frame += frags_entries.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
