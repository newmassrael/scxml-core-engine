// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_zenoh_interest_body.scxml:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohInterestBody` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohInterestBody.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohInterestBody`, which conforms itself.
public protocol CodecZenohInterestBodySource {
    /// What `keyexpr` is presented as: a source of `CodecZenohWireexpr`, written by its encode.
    associatedtype Keyexpr_Source: CodecZenohWireexprSource
    var header: UInt8 { get }
    var keyexpr: Keyexpr_Source? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohInterestBody: Equatable, Sendable, CodecZenohInterestBodySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The value presents `keyexpr` as a value of its own codec.
    public typealias Keyexpr_Source = CodecZenohWireexpr

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// Bit 0 of `header` (flag `keyexprs`).
    public var keyexprs: Bool {
        get { (header & 0x1) != 0 }
        set { header = newValue ? (header | 0x1) : (header & ~0x1) }
    }

    /// Bit 1 of `header` (flag `subscribers`).
    public var subscribers: Bool {
        get { (header & 0x2) != 0 }
        set { header = newValue ? (header | 0x2) : (header & ~0x2) }
    }

    /// Bit 2 of `header` (flag `queryables`).
    public var queryables: Bool {
        get { (header & 0x4) != 0 }
        set { header = newValue ? (header | 0x4) : (header & ~0x4) }
    }

    /// Bit 3 of `header` (flag `tokens`).
    public var tokens: Bool {
        get { (header & 0x8) != 0 }
        set { header = newValue ? (header | 0x8) : (header & ~0x8) }
    }

    /// Bit 4 of `header` (flag `restricted`).
    public var restricted: Bool {
        get { (header & 0x10) != 0 }
        set { header = newValue ? (header | 0x10) : (header & ~0x10) }
    }

    /// Bit 5 of `header` (flag `N`).
    public var N: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `M`).
    public var M: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `aggregate`).
    public var aggregate: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(header & 0x10) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A `CodecZenohWireexpr` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var keyexpr: CodecZenohWireexpr? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, keyexpr: Keyexpr_Source?) {
        self.header = header
        self.keyexpr = keyexpr
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohInterestBody {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var keyexpr: CodecZenohWireexpr? = nil
        if (header & 0x10) != 0 {
            keyexpr = try CodecZenohWireexpr.decode(&reader, N: UInt8(truncatingIfNeeded: (header >> 5) & 0x1))
        }
        cursor = reader
        return CodecZenohInterestBody(header: header, keyexpr: keyexpr)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecZenohInterestBody` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `keyexprs` flag of `header`, where a parent sets it.
        public var headerKeyexprs: Bool?
        /// The `subscribers` flag of `header`, where a parent sets it.
        public var headerSubscribers: Bool?
        /// The `queryables` flag of `header`, where a parent sets it.
        public var headerQueryables: Bool?
        /// The `tokens` flag of `header`, where a parent sets it.
        public var headerTokens: Bool?
        /// The `M` flag of `header`, where a parent sets it.
        public var headerM: Bool?
        /// The `aggregate` flag of `header`, where a parent sets it.
        public var headerAggregate: Bool?

        public init(headerKeyexprs: Bool? = nil, headerSubscribers: Bool? = nil, headerQueryables: Bool? = nil, headerTokens: Bool? = nil, headerM: Bool? = nil, headerAggregate: Bool? = nil) {
            self.headerKeyexprs = headerKeyexprs
            self.headerSubscribers = headerSubscribers
            self.headerQueryables = headerQueryables
            self.headerTokens = headerTokens
            self.headerM = headerM
            self.headerAggregate = headerAggregate
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohInterestBodySource` is written,
    /// with no copy into `CodecZenohInterestBody`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohInterestBodySource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var header_folded = source.header
        if source.keyexpr != nil { header_folded = (header_folded & ~0x20) | ((UInt8(truncatingIfNeeded: (source.keyexpr.map { Int(CodecZenohWireexpr.inputN($0)) } ?? 0)) & 0x1) << 5) }
        header_folded = (header_folded & ~0x10) | ((source.keyexpr != nil) ? 0x10 : 0)
        if let set = overrides.headerKeyexprs { header_folded = set ? (header_folded | 0x1) : (header_folded & ~0x1) }
        if let set = overrides.headerSubscribers { header_folded = set ? (header_folded | 0x2) : (header_folded & ~0x2) }
        if let set = overrides.headerQueryables { header_folded = set ? (header_folded | 0x4) : (header_folded & ~0x4) }
        if let set = overrides.headerTokens { header_folded = set ? (header_folded | 0x8) : (header_folded & ~0x8) }
        if let set = overrides.headerM { header_folded = set ? (header_folded | 0x40) : (header_folded & ~0x40) }
        if let set = overrides.headerAggregate { header_folded = set ? (header_folded | 0x80) : (header_folded & ~0x80) }
        var keyexpr_bytes: [UInt8]? = nil
        if let held = source.keyexpr {
            var piece = BytesSink()
            try CodecZenohWireexpr.encodeSource(held, into: &piece)
            keyexpr_bytes = piece.bytes
        }
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = keyexpr_bytes { frame += held }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
