// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_chain_has_marker.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecChainHasMarker` the gates `extensions.has(0x12)` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
/// Each body and list is a source of its codec, so a host's type is presented with no copy.
public enum CodecChainHasMarkerExtensionsHas0x12<Extensions_Entry: CodecZenohExtEntrySource, Slices_Entry: CodecChainHasMarkerSliceSource> {
    /// While extensions.has(0x12) set.
    case has0x12(extensions: ScePlaced<Extensions_Entry, CodecChainHasMarker.ExtensionsBound, CodecChainHasMarker.ExtensionsKind12<Extensions_Entry>>, slices: BoundedList<Slices_Entry, CodecChainHasMarker.SlicesBound>)
    /// While extensions.has(0x12) clear.
    case notHas0x12(extensions: SceChain<Extensions_Entry, CodecChainHasMarker.ExtensionsBound, SceNoKinds<Extensions_Entry>, SceKindsOf<CodecChainHasMarker.ExtensionsKind12<Extensions_Entry>, SceNoKinds<Extensions_Entry>>>?, payload: BoundedBytes<CodecChainHasMarker.PayloadBound>)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .has0x12: return 0
        case .notHas0x12: return 1
        }
    }
}

extension CodecChainHasMarkerExtensionsHas0x12: Equatable where Extensions_Entry: Equatable, Slices_Entry: Equatable {}

extension CodecChainHasMarkerExtensionsHas0x12: Sendable where Extensions_Entry: Sendable, Slices_Entry: Sendable {}

/// The elements a value of `CodecChainHasMarker` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHasMarker.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHasMarker`, which conforms itself.
public protocol CodecChainHasMarkerSource {
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    /// What each entry of `slices` is presented as: a source of `CodecChainHasMarkerSlice`, written by its encode.
    associatedtype Slices_Entry: CodecChainHasMarkerSliceSource
    var header: UInt8 { get }
    var extensions_has_0x12: CodecChainHasMarkerExtensionsHas0x12<Extensions_Entry, Slices_Entry> { get }
}

/// The frame this codec lays on the wire: 6 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasMarker: Equatable, Sendable, CodecChainHasMarkerSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = 4
    }

    /// How many bytes `payload` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum PayloadBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 256
    }

    /// How many entries `slices` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum SlicesBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = 4
    }

    /// Whether an entry of `extensions` is of the kind 0x12 a gate asks the chain for (docs/adr/0065
    /// §6 R6), read off any source of its codec: what the arms of the chain's kinds are held to.
    public enum ExtensionsKind12<Entry: CodecZenohExtEntrySource>: SceKind {
        public static func matches(_ entry: Entry) -> Bool {
            (chainIdentifier(entry.header) & 0x7F) == 0x12
        }
    }

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The value presents each entry of `slices` as a value of its own codec.
    public typealias Slices_Entry = CodecChainHasMarkerSlice

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 5 bits of `header` from bit 0 (flag `kind`). A value
    /// wider than the flag is cut to it when written.
    public var kind: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 7 of `header` (flag `E`).
    public var E: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// The elements the gates `extensions.has(0x12)` decide together.
    public var extensions_has_0x12: CodecChainHasMarkerExtensionsHas0x12<Extensions_Entry, Slices_Entry> = .notHas0x12(extensions: nil, payload: .fewest)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, extensions_has_0x12: CodecChainHasMarkerExtensionsHas0x12<Extensions_Entry, Slices_Entry>) {
        self.header = header
        self.extensions_has_0x12 = extensions_has_0x12
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasMarker {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var extensions: BoundedList<CodecZenohExtEntry, CodecChainHasMarker.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            for _ in 0..<4 {
                let entry = try CodecZenohExtEntry.decode(&reader)
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
            extensions = try BoundedList<CodecZenohExtEntry, CodecChainHasMarker.ExtensionsBound>(decoding: extensions_items)
        }
        let _has_extensions_18 = extensions?.contains(where: { _e in (chainIdentifier(_e.header) & 127) == 18 }) ?? false
        var payload_len: UInt64? = nil
        if !_has_extensions_18 {
            payload_len = try reader.readVle(UInt64.self)
        }
        var payload: BoundedBytes<CodecChainHasMarker.PayloadBound>? = nil
        if !_has_extensions_18 {
            if (payload_len ?? 0) > 256 {
                throw CodecError.lengthOutOfRange
            }
            payload = try BoundedBytes<CodecChainHasMarker.PayloadBound>(decoding: Array(try reader.readSlice(Int(clamping: (payload_len ?? 0)))))
        }
        var slice_count: UInt32? = nil
        if _has_extensions_18 {
            slice_count = try reader.readVle(UInt32.self)
        }
        var slices: BoundedList<CodecChainHasMarkerSlice, CodecChainHasMarker.SlicesBound>? = nil
        if _has_extensions_18 {
            if (slice_count ?? 0) > 4 {
                throw CodecError.tooManyElements
            }
            var slices_items: [CodecChainHasMarkerSlice] = []
            for _ in 0..<Int(clamping: (slice_count ?? 0)) {
                slices_items.append(try CodecChainHasMarkerSlice.decode(&reader))
            }
            slices = try BoundedList<CodecChainHasMarkerSlice, CodecChainHasMarker.SlicesBound>(decoding: slices_items)
        }
        let extensions_has_0x12_group: CodecChainHasMarkerExtensionsHas0x12<Extensions_Entry, Slices_Entry>
        if _has_extensions_18 {
            extensions_has_0x12_group = .has0x12(extensions: try ScePlaced(decoding: extensions!), slices: slices!)
        } else {
            var extensions_chain: SceChain<Extensions_Entry, CodecChainHasMarker.ExtensionsBound, SceNoKinds<Extensions_Entry>, SceKindsOf<CodecChainHasMarker.ExtensionsKind12<Extensions_Entry>, SceNoKinds<Extensions_Entry>>>? = nil
            if let extensions_read = extensions { extensions_chain = try SceChain<Extensions_Entry, CodecChainHasMarker.ExtensionsBound, SceNoKinds<Extensions_Entry>, SceKindsOf<CodecChainHasMarker.ExtensionsKind12<Extensions_Entry>, SceNoKinds<Extensions_Entry>>>(decoding: extensions_read) }
            extensions_has_0x12_group = .notHas0x12(extensions: extensions_chain, payload: payload!)
        }
        cursor = reader
        return CodecChainHasMarker(header: header, extensions_has_0x12: extensions_has_0x12_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHasMarkerSource` is written,
    /// with no copy into `CodecChainHasMarker`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHasMarkerSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var extensions_member: [Source.Extensions_Entry]? = nil
        var extensions_place: Int? = nil
        var slices_member: BoundedList<Source.Slices_Entry, CodecChainHasMarker.SlicesBound>? = nil
        var payload_member: BoundedBytes<CodecChainHasMarker.PayloadBound>? = nil
        switch source.extensions_has_0x12 {
        case .has0x12(let extensions, let slices): extensions_member = extensions.rest; extensions_place = extensions.place; slices_member = slices
        case .notHas0x12(let extensions, let payload): extensions_member = extensions?.entries; payload_member = payload
        }
        let payload_len_derived: UInt64 = payload_member != nil ? UInt64((payload_member?.byteCount ?? 0)) : 0
        let slice_count_derived: UInt32 = slices_member != nil ? UInt32((slices_member?.count ?? 0)) : 0
        var header_folded = source.header
        header_folded = (header_folded & ~0x80) | ((extensions_member != nil) ? 0x80 : 0)
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = extensions_member {
            var extensions_entries = BytesSink()
            let extensions_slots = held.count + (extensions_place != nil ? 1 : 0)
            var extensions_next = 0
            for extensions_at in 0..<extensions_slots {
                if extensions_at == extensions_place {
                    try CodecZenohExtEntry.encodeSource(CodecZenohExtEntry(header: 0x12, body: .codecZenohExtUnit(.init())), into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_at + 1 < extensions_slots))
                } else {
                    let entry = held[extensions_next]
                    extensions_next += 1
                    try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_at + 1 < extensions_slots))
                }
            }
            frame += extensions_entries.bytes
        }
        if !([0].contains(source.extensions_has_0x12.arm)) {
            appendVle(payload_len_derived, to: &frame)
        }
        if let held = payload_member {
            held.append(to: &frame)
        }
        if [0].contains(source.extensions_has_0x12.arm) {
            appendVle(slice_count_derived, to: &frame)
        }
        if let held = slices_member {
            var slices_entries = BytesSink()
            for entry in held {
                try CodecChainHasMarkerSlice.encodeSource(entry, into: &slices_entries)
            }
            frame += slices_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
