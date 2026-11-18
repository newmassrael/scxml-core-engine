// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_repeat_unconditional_count.scxml:34 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecRepeatUnconditionalCount` the gates `options.H` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
/// Each body and list is a source of its codec, so a host's type is presented with no copy.
public enum CodecRepeatUnconditionalCountOptionsH<Links_Entry: CodecRepeatElemSource, Weights_Entry: CodecRepeatElemSource> {
    /// While options.H set.
    case h(links_weights: Aligned<BoundedList<Links_Entry, CodecRepeatUnconditionalCount.LinksBound>, BoundedList<Weights_Entry, CodecRepeatUnconditionalCount.WeightsBound>>)
    /// While options.H clear.
    case notH(links: BoundedList<Links_Entry, CodecRepeatUnconditionalCount.LinksBound>)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .h: return 0
        case .notH: return 1
        }
    }
}

extension CodecRepeatUnconditionalCountOptionsH: Equatable where Links_Entry: Equatable, Weights_Entry: Equatable {}

extension CodecRepeatUnconditionalCountOptionsH: Sendable where Links_Entry: Sendable, Weights_Entry: Sendable {}

/// The elements a value of `CodecRepeatUnconditionalCount` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecRepeatUnconditionalCount.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecRepeatUnconditionalCount`, which conforms itself.
public protocol CodecRepeatUnconditionalCountSource {
    /// What each entry of `links` is presented as: a source of `CodecRepeatElem`, written by its encode.
    associatedtype Links_Entry: CodecRepeatElemSource
    /// What each entry of `weights` is presented as: a source of `CodecRepeatElem`, written by its encode.
    associatedtype Weights_Entry: CodecRepeatElemSource
    var options: UInt8 { get }
    var options_h: CodecRepeatUnconditionalCountOptionsH<Links_Entry, Weights_Entry> { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatUnconditionalCount: Equatable, Sendable, CodecRepeatUnconditionalCountSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many entries `links` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum LinksBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 64
    }

    /// How many entries `weights` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum WeightsBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 64
    }

    /// The value presents each entry of `links` as a value of its own codec.
    public typealias Links_Entry = CodecRepeatElem

    /// The value presents each entry of `weights` as a value of its own codec.
    public typealias Weights_Entry = CodecRepeatElem

    /// The field at byte 0, big-endian.
    public var options: UInt8 = 0

    /// Bit 0 of `options` (flag `H`).
    public var H: Bool {
        get { (options & 0x1) != 0 }
        set { options = newValue ? (options | 0x1) : (options & ~0x1) }
    }

    /// The elements the gates `options.H` decide together.
    public var options_h: CodecRepeatUnconditionalCountOptionsH<Links_Entry, Weights_Entry> = .notH(links: .fewest(of: CodecRepeatElem()))

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(options: UInt8, options_h: CodecRepeatUnconditionalCountOptionsH<Links_Entry, Weights_Entry>) {
        self.options = options
        self.options_h = options_h
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecRepeatUnconditionalCount {
        var reader = cursor
        let options = try reader.readInteger(UInt8.self, .big)
        let links_len = try reader.readInteger(UInt8.self, .big)
        if links_len > 64 {
            throw CodecError.tooManyElements
        }
        var links_items: [CodecRepeatElem] = []
        for _ in 0..<Int(clamping: links_len) {
            links_items.append(try CodecRepeatElem.decode(&reader))
        }
        let links = try BoundedList<CodecRepeatElem, CodecRepeatUnconditionalCount.LinksBound>(decoding: links_items)
        var weights: BoundedList<CodecRepeatElem, CodecRepeatUnconditionalCount.WeightsBound>? = nil
        if (options & 0x1) != 0 {
            if links_len > 64 {
                throw CodecError.tooManyElements
            }
            var weights_items: [CodecRepeatElem] = []
            for _ in 0..<Int(clamping: links_len) {
                weights_items.append(try CodecRepeatElem.decode(&reader))
            }
            weights = try BoundedList<CodecRepeatElem, CodecRepeatUnconditionalCount.WeightsBound>(decoding: weights_items)
        }
        let options_h_group: CodecRepeatUnconditionalCountOptionsH<Links_Entry, Weights_Entry>
        if ((options >> 0) & 1) != 0 {
            options_h_group = .h(links_weights: try Aligned(decoding: links, weights!))
        } else {
            options_h_group = .notH(links: links)
        }
        cursor = reader
        return CodecRepeatUnconditionalCount(options: options, options_h: options_h_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecRepeatUnconditionalCountSource` is written,
    /// with no copy into `CodecRepeatUnconditionalCount`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecRepeatUnconditionalCountSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var links_member: BoundedList<Source.Links_Entry, CodecRepeatUnconditionalCount.LinksBound>? = nil
        var weights_member: BoundedList<Source.Weights_Entry, CodecRepeatUnconditionalCount.WeightsBound>? = nil
        switch source.options_h {
        case .h(let links_weights): links_member = links_weights.first; weights_member = links_weights.second
        case .notH(let links): links_member = links
        }
        let links_len_derived: UInt8 = UInt8((links_member?.count ?? 0))
        var options_folded = source.options
        options_folded = (options_folded & ~0x1) | ((weights_member != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(options_folded, to: &frame, .big)
        appendInteger(links_len_derived, to: &frame, .big)
        if let held = links_member {
            var links_entries = BytesSink()
            for entry in held {
                try CodecRepeatElem.encodeSource(entry, into: &links_entries)
            }
            frame += links_entries.bytes
        }
        if let held = weights_member {
            var weights_entries = BytesSink()
            for entry in held {
                try CodecRepeatElem.encodeSource(entry, into: &weights_entries)
            }
            frame += weights_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
