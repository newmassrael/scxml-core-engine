// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_interest.scxml:73 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecZenohInterest` the gates `header.CURRENT || header.FUTURE` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
/// Each body and list is a source of its codec, so a host's type is presented with no copy.
public enum CodecZenohInterestHeaderCurrentOrFuture<Body_Source: CodecZenohInterestBodySource> {
    /// While header.CURRENT set, header.FUTURE clear.
    case current(body: Body_Source)    /// While header.CURRENT clear, header.FUTURE set.
    case future(body: Body_Source)    /// While header.CURRENT set, header.FUTURE set.
    case currentFuture(body: Body_Source)
    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .current: return 0
        case .future: return 1
        case .currentFuture: return 2
        }
    }
}

extension CodecZenohInterestHeaderCurrentOrFuture: Equatable where Body_Source: Equatable {}

extension CodecZenohInterestHeaderCurrentOrFuture: Sendable where Body_Source: Sendable {}

/// The elements a value of `CodecZenohInterest` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohInterest.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohInterest`, which conforms itself.
public protocol CodecZenohInterestSource {
    /// What `body` is presented as: a source of `CodecZenohInterestBody`, written by its encode.
    associatedtype Body_Source: CodecZenohInterestBodySource
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    var header: UInt8 { get }
    var id: UInt64 { get }
    var header_current_or_future: CodecZenohInterestHeaderCurrentOrFuture<Body_Source>? { get }
    var extensions: BoundedList<Extensions_Entry, CodecZenohInterest.ExtensionsBound>? { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohInterest: Equatable, Sendable, CodecZenohInterestSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = nil
    }

    /// The value presents `body` as a value of its own codec.
    public typealias Body_Source = CodecZenohInterestBody

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x19

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `CURRENT`).
    public var CURRENT: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `FUTURE`).
    public var FUTURE: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var id: UInt64 = 0

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohInterest.ExtensionsBound>? = nil

    /// The elements the gates `header.CURRENT || header.FUTURE` decide together, `nil` while they name no arm.
    public var header_current_or_future: CodecZenohInterestHeaderCurrentOrFuture<Body_Source>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, id: UInt64, header_current_or_future: CodecZenohInterestHeaderCurrentOrFuture<Body_Source>?, extensions: BoundedList<Extensions_Entry, CodecZenohInterest.ExtensionsBound>?) {
        self.header = header
        self.id = id
        self.header_current_or_future = header_current_or_future
        self.extensions = extensions
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohInterest {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let id = try reader.readVle(UInt64.self)
        var body: CodecZenohInterestBody? = nil
        if (header & 0x20) != 0 || (header & 0x40) != 0 {
            body = try CodecZenohInterestBody.decode(&reader)
        }
        var extensions: BoundedList<CodecZenohExtEntry, CodecZenohInterest.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            while true {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_more = entry.Z
                extensions_items.append(entry)
                if !extensions_more { break }
            }
            extensions = try BoundedList<CodecZenohExtEntry, CodecZenohInterest.ExtensionsBound>(decoding: extensions_items)
        }
        let header_current_or_future_group: CodecZenohInterestHeaderCurrentOrFuture<Body_Source>?
        if ((header >> 5) & 1) != 0 && ((header >> 6) & 1) == 0 {
            header_current_or_future_group = .current(body: body!)
        } else if ((header >> 5) & 1) == 0 && ((header >> 6) & 1) != 0 {
            header_current_or_future_group = .future(body: body!)
        } else if ((header >> 5) & 1) != 0 && ((header >> 6) & 1) != 0 {
            header_current_or_future_group = .currentFuture(body: body!)
        } else {
            header_current_or_future_group = nil
        }
        cursor = reader
        return CodecZenohInterest(header: header, id: id, header_current_or_future: header_current_or_future_group, extensions: extensions)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohInterestSource` is written,
    /// with no copy into `CodecZenohInterest`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohInterestSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var body_member: Source.Body_Source? = nil
        switch source.header_current_or_future {
        case .current(let body)?: body_member = body
        case .future(let body)?: body_member = body
        case .currentFuture(let body)?: body_member = body
        case nil: break
        }
        var header_folded = source.header
        header_folded = (header_folded & ~0x80) | ((source.extensions != nil) ? 0x80 : 0)
        header_folded = (header_folded & ~0x20) | ((source.header_current_or_future.map { [0, 2].contains($0.arm) } ?? false) ? 0x20 : 0)
        header_folded = (header_folded & ~0x40) | ((source.header_current_or_future.map { [1, 2].contains($0.arm) } ?? false) ? 0x40 : 0)
        var body_bytes: [UInt8]? = nil
        if let held = body_member {
            var piece = BytesSink()
            try CodecZenohInterestBody.encodeSource(held, into: &piece)
            body_bytes = piece.bytes
        }
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        appendVle(source.id, to: &frame)
        if let held = body_bytes { frame += held }
        if let held = source.extensions {
            var extensions_entries = BytesSink()
            var extensions_left = held.count
            for entry in held {
                extensions_left -= 1
                try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_left != 0))
            }
            frame += extensions_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
