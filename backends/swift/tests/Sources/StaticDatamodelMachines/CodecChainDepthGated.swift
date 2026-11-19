// SCE-GENERATED — DO NOT EDIT
// source-hash: 99d59cb3e0b7e91553e82861edaa04855d2429c3bb763839dc3b14bd5c0468a7
// source: codec_chain_depth_gated.scxml
// SCE-MAP: codec_chain_depth_gated.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecChainDepthGated` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainDepthGated.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainDepthGated`, which conforms itself.
public protocol CodecChainDepthGatedSource {
    /// What each entry of `extensions` is presented as: a source of `CodecChainDepthEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecChainDepthEntrySource
    var header: UInt8 { get }
    var extensions: BoundedList<Extensions_Entry, CodecChainDepthGated.ExtensionsBound>? { get }
    var payload: BoundedBytes<CodecChainDepthGated.PayloadBound> { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainDepthGated: Equatable, Sendable, CodecChainDepthGatedSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = 8
    }

    /// How many bytes `payload` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum PayloadBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 16
    }

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecChainDepthEntry

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecChainDepthEntry`, one after another, at most 8: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, and a value with more is no value of its type.
    public var extensions: BoundedList<CodecChainDepthEntry, CodecChainDepthGated.ExtensionsBound>? = nil

    /// The bytes, as many as `payload_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var payload: BoundedBytes<CodecChainDepthGated.PayloadBound> = .fewest

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, extensions: BoundedList<Extensions_Entry, CodecChainDepthGated.ExtensionsBound>?, payload: BoundedBytes<CodecChainDepthGated.PayloadBound>) {
        self.header = header
        self.extensions = extensions
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainDepthGated {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var extensions: BoundedList<CodecChainDepthEntry, CodecChainDepthGated.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecChainDepthEntry] = []
            var extensions_more = false
            for _ in 0..<8 {
                let entry = try CodecChainDepthEntry.decode(&reader)
                extensions_more = entry.Z
                extensions_items.append(entry)
                if !extensions_more { break }
            }
            if extensions_more && reader.remaining == 0 {
                throw CodecError.needMoreBytes
            }
            if extensions_more {
                throw CodecError.tooManyElements
            }
            extensions = try BoundedList<CodecChainDepthEntry, CodecChainDepthGated.ExtensionsBound>(decoding: extensions_items)
        }
        let payload_len = try reader.readVle(UInt64.self)
        if payload_len > 16 {
            throw CodecError.lengthOutOfRange
        }
        let payload = try BoundedBytes<CodecChainDepthGated.PayloadBound>(decoding: Array(try reader.readSlice(Int(clamping: payload_len))))
        cursor = reader
        return CodecChainDepthGated(header: header, extensions: extensions, payload: payload)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainDepthGatedSource` is written,
    /// with no copy into `CodecChainDepthGated`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainDepthGatedSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let payload_len_derived: UInt64 = UInt64(source.payload.byteCount)
        var header_folded = source.header
        header_folded = (header_folded & ~0x80) | ((source.extensions != nil) ? 0x80 : 0)
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = source.extensions {
            var extensions_entries = BytesSink()
            var extensions_left = held.count
            for entry in held {
                extensions_left -= 1
                try CodecChainDepthEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecChainDepthEntry.Overrides(headerZ: extensions_left != 0))
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
