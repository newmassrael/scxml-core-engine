// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_err.scxml:57 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohErr` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohErr.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohErr`, which conforms itself.
public protocol CodecZenohErrSource {
    /// What `encoding` is presented as: a source of `CodecZenohEncoding`, written by its encode.
    associatedtype Encoding_Source: CodecZenohEncodingSource
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    var header: UInt8 { get }
    var encoding: Encoding_Source? { get }
    var extensions: BoundedList<Extensions_Entry, CodecZenohErr.ExtensionsBound>? { get }
    var payload: BoundedBytes<CodecZenohErr.PayloadBound> { get }
}

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohErr: Equatable, Sendable, CodecZenohErrSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = nil
    }

    /// How many bytes `payload` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum PayloadBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 4294967295
    }

    /// The value presents `encoding` as a value of its own codec.
    public typealias Encoding_Source = CodecZenohEncoding

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x5

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `X`).
    public var X: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `E`).
    public var E: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(header & 0x40) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A `CodecZenohEncoding` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var encoding: CodecZenohEncoding? = nil

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohErr.ExtensionsBound>? = nil

    /// The bytes, as many as `payload_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var payload: BoundedBytes<CodecZenohErr.PayloadBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, encoding: Encoding_Source?, extensions: BoundedList<Extensions_Entry, CodecZenohErr.ExtensionsBound>?, payload: BoundedBytes<CodecZenohErr.PayloadBound>) {
        self.header = header
        self.encoding = encoding
        self.extensions = extensions
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohErr {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var encoding: CodecZenohEncoding? = nil
        if (header & 0x40) != 0 {
            encoding = try CodecZenohEncoding.decode(&reader)
        }
        var extensions: BoundedList<CodecZenohExtEntry, CodecZenohErr.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            while true {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_more = entry.Z
                extensions_items.append(entry)
                if !extensions_more { break }
            }
            extensions = try BoundedList<CodecZenohExtEntry, CodecZenohErr.ExtensionsBound>(decoding: extensions_items)
        }
        let payload_len = try reader.readVle(UInt64.self)
        if payload_len > 4294967295 {
            throw CodecError.lengthOutOfRange
        }
        let payload = try BoundedBytes<CodecZenohErr.PayloadBound>(decoding: Array(try reader.readSlice(Int(clamping: payload_len))))
        cursor = reader
        return CodecZenohErr(header: header, encoding: encoding, extensions: extensions, payload: payload)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecZenohErr` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `X` flag of `header`, where a parent sets it.
        public var headerX: Bool?

        public init(headerX: Bool? = nil) {
            self.headerX = headerX
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohErrSource` is written,
    /// with no copy into `CodecZenohErr`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohErrSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        let payload_len_derived: UInt64 = UInt64(source.payload.byteCount)
        var header_folded = source.header
        header_folded = (header_folded & ~0x40) | ((source.encoding != nil) ? 0x40 : 0)
        header_folded = (header_folded & ~0x80) | ((source.extensions != nil) ? 0x80 : 0)
        if let set = overrides.headerX { header_folded = set ? (header_folded | 0x20) : (header_folded & ~0x20) }
        var encoding_bytes: [UInt8]? = nil
        if let held = source.encoding {
            var piece = BytesSink()
            try CodecZenohEncoding.encodeSource(held, into: &piece)
            encoding_bytes = piece.bytes
        }
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = encoding_bytes { frame += held }
        if let held = source.extensions {
            var extensions_entries = BytesSink()
            var extensions_left = held.count
            for entry in held {
                extensions_left -= 1
                try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_left != 0))
            }
            frame += extensions_entries.bytes
        }
        appendVle(payload_len_derived, to: &frame)
        source.payload.append(to: &frame)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
