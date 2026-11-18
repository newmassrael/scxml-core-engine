// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_declare.scxml:49 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohDeclare` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohDeclare.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohDeclare`, which conforms itself.
public protocol CodecZenohDeclareSource {
    /// What `declaration` is presented as: a source of `CodecZenohDeclaration`, written by its encode.
    associatedtype Declaration_Source: CodecZenohDeclarationSource
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    var header: UInt8 { get }
    var interest_id: UInt32? { get }
    var extensions: BoundedList<Extensions_Entry, CodecZenohDeclare.ExtensionsBound>? { get }
    var declaration: Declaration_Source { get }
}

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohDeclare: Equatable, Sendable, CodecZenohDeclareSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecZenohDeclaration.frameLength

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = nil
    }

    /// The value presents `declaration` as a value of its own codec.
    public typealias Declaration_Source = CodecZenohDeclaration

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x1E

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `I`).
    public var I: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(header & 0x20) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var interest_id: UInt32? = nil

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohDeclare.ExtensionsBound>? = nil

    /// A `CodecZenohDeclaration` laid in place: it has no bytes of its own, the frame holds its fields
    /// where this field stands.
    public var declaration: CodecZenohDeclaration = CodecZenohDeclaration()

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, interest_id: UInt32?, extensions: BoundedList<Extensions_Entry, CodecZenohDeclare.ExtensionsBound>?, declaration: Declaration_Source) {
        self.header = header
        self.interest_id = interest_id
        self.extensions = extensions
        self.declaration = declaration
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohDeclare {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var interest_id: UInt32? = nil
        if (header & 0x20) != 0 {
            interest_id = try reader.readVle(UInt32.self)
        }
        var extensions: BoundedList<CodecZenohExtEntry, CodecZenohDeclare.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            while true {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_more = entry.Z
                extensions_items.append(entry)
                if !extensions_more { break }
            }
            extensions = try BoundedList<CodecZenohExtEntry, CodecZenohDeclare.ExtensionsBound>(decoding: extensions_items)
        }
        let declaration = try CodecZenohDeclaration.decode(&reader)
        cursor = reader
        return CodecZenohDeclare(header: header, interest_id: interest_id, extensions: extensions, declaration: declaration)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohDeclareSource` is written,
    /// with no copy into `CodecZenohDeclare`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohDeclareSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var header_folded = source.header
        header_folded = (header_folded & ~0x20) | ((source.interest_id != nil) ? 0x20 : 0)
        header_folded = (header_folded & ~0x80) | ((source.extensions != nil) ? 0x80 : 0)
        var declaration_piece = BytesSink()
        try CodecZenohDeclaration.encodeSource(source.declaration, into: &declaration_piece)
        let declaration_bytes = declaration_piece.bytes
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = source.interest_id {
            appendVle(held, to: &frame)
        }
        if let held = source.extensions {
            var extensions_entries = BytesSink()
            var extensions_left = held.count
            for entry in held {
                extensions_left -= 1
                try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_left != 0))
            }
            frame += extensions_entries.bytes
        }
        frame += declaration_bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
