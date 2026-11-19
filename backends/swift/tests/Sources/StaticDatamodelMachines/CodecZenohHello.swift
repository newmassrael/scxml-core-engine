// SCE-GENERATED — DO NOT EDIT
// source-hash: 48efcea3688be905ad83348470d7945a4a8a462adbb4db310a2d1da5b24104da
// source: codec_zenoh_hello.scxml
// SCE-MAP: codec_zenoh_hello.scxml:41 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohHello` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohHello.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohHello`, which conforms itself.
public protocol CodecZenohHelloSource {
    /// What each entry of `locators` is presented as: a source of `CodecZenohLocator`, written by its encode.
    associatedtype Locators_Entry: CodecZenohLocatorSource
    var version: UInt8 { get }
    var cbyte: UInt8 { get }
    var zid: BoundedBytes<CodecZenohHello.ZidBound> { get }
    var locators: BoundedList<Locators_Entry, CodecZenohHello.LocatorsBound>? { get }
}

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohHello: Equatable, Sendable, CodecZenohHelloSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `zid` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ZidBound: SceByteBound {
        public static let least = 1
        public static let most: Int? = 16
    }

    /// How many entries `locators` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum LocatorsBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = nil
    }

    /// The value presents each entry of `locators` as a value of its own codec.
    public typealias Locators_Entry = CodecZenohLocator

    /// The field at byte 0, big-endian.
    public var version: UInt8 = 0

    /// The field at byte 1, big-endian.
    public var cbyte: UInt8 = 0

    /// The 2 bits of `cbyte` from bit 0 (flag `whatami`). A value
    /// wider than the flag is cut to it when written.
    public var whatami: UInt8 {
        get { UInt8(truncatingIfNeeded: (cbyte >> 0) & 0x3) }
        set { cbyte = (cbyte & ~(0x3 << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 0) }
    }

    /// The 4 bits of `cbyte` from bit 4 (flag `zid_len_m1`). A value
    /// wider than the flag is cut to it when written.
    public var zidLenM1: UInt8 {
        get { UInt8(truncatingIfNeeded: (cbyte >> 4) & 0xF) }
        set { cbyte = (cbyte & ~(0xF << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0xF) << 4) }
    }

    /// The bytes, as many as `cbyte.zid_len_m1` says, plus 1, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var zid: BoundedBytes<CodecZenohHello.ZidBound> = .fewest

    /// On the wire only when `(L & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// As many entries as `num_locators` says, each a `CodecZenohLocator`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, and a value
    /// with more is no value of its type; the encode derives the count.
    public var locators: BoundedList<CodecZenohLocator, CodecZenohHello.LocatorsBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, cbyte: UInt8, zid: BoundedBytes<CodecZenohHello.ZidBound>, locators: BoundedList<Locators_Entry, CodecZenohHello.LocatorsBound>?) {
        self.version = version
        self.cbyte = cbyte
        self.zid = zid
        self.locators = locators
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, L: UInt8) throws(CodecError) -> CodecZenohHello {
        var reader = cursor
        let version = try reader.readInteger(UInt8.self, .big)
        let cbyte = try reader.readInteger(UInt8.self, .big)
        let zid = try BoundedBytes<CodecZenohHello.ZidBound>(decoding: Array(try reader.readSlice(Int(clamping: ((cbyte >> 4) & 0xF)) &+ (1))))
        var num_locators: UInt64? = nil
        if (L & 0x1) != 0 {
            num_locators = try reader.readVle(UInt64.self)
        }
        var locators: BoundedList<CodecZenohLocator, CodecZenohHello.LocatorsBound>? = nil
        if (L & 0x1) != 0 {
            var locators_items: [CodecZenohLocator] = []
            for _ in 0..<Int(clamping: (num_locators ?? 0)) {
                locators_items.append(try CodecZenohLocator.decode(&reader))
            }
            locators = try BoundedList<CodecZenohLocator, CodecZenohHello.LocatorsBound>(decoding: locators_items)
        }
        cursor = reader
        return CodecZenohHello(version: version, cbyte: cbyte, zid: zid, locators: locators)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohHelloSource` is written,
    /// with no copy into `CodecZenohHello`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohHelloSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let L: UInt8 = source.locators != nil ? 1 : 0
        var cbyte_folded = source.cbyte
        cbyte_folded = (cbyte_folded & ~0xF0) | ((UInt8(truncatingIfNeeded: (source.zid.byteCount - 1)) & 0xF) << 4)
        let num_locators_derived: UInt64 = source.locators != nil ? UInt64((source.locators?.count ?? 0)) : 0
        var frame: [UInt8] = []
        appendInteger(source.version, to: &frame, .big)
        appendInteger(cbyte_folded, to: &frame, .big)
        source.zid.append(to: &frame)
        if L != 0 {
            appendVle(num_locators_derived, to: &frame)
        }
        if let held = source.locators {
            var locators_entries = BytesSink()
            for entry in held {
                try CodecZenohLocator.encodeSource(entry, into: &locators_entries)
            }
            frame += locators_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `L` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputL<Source: CodecZenohHelloSource>(_ source: Source) -> UInt8 {
        source.locators != nil ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
