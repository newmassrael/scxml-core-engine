// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_chain_has_envelope.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecChainHasEnvelope` the gates `extensions.has(0x2)` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
/// Each body and list is a source of its codec, so a host's type is presented with no copy.
public enum CodecChainHasEnvelopeExtensionsHas0x2<Extensions_Entry: CodecZenohExtEntrySource, Slices_Entry: CodecChainHasSliceSource> {
    /// While extensions.has(0x2) set.
    case has0x2(extensions: SceChain<Extensions_Entry, CodecChainHasEnvelope.ExtensionsBound, SceKindsOf<CodecChainHasEnvelope.ExtensionsKind2<Extensions_Entry>, SceNoKinds<Extensions_Entry>>, SceNoKinds<Extensions_Entry>>, slices: BoundedList<Slices_Entry, CodecChainHasEnvelope.SlicesBound>)    /// While extensions.has(0x2) clear.
    case notHas0x2(extensions: SceChain<Extensions_Entry, CodecChainHasEnvelope.ExtensionsBound, SceNoKinds<Extensions_Entry>, SceKindsOf<CodecChainHasEnvelope.ExtensionsKind2<Extensions_Entry>, SceNoKinds<Extensions_Entry>>>?, payload: BoundedBytes<CodecChainHasEnvelope.PayloadBound>)
    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .has0x2: return 0
        case .notHas0x2: return 1
        }
    }
}

extension CodecChainHasEnvelopeExtensionsHas0x2: Equatable where Extensions_Entry: Equatable, Slices_Entry: Equatable {}

extension CodecChainHasEnvelopeExtensionsHas0x2: Sendable where Extensions_Entry: Sendable, Slices_Entry: Sendable {}

/// The elements a value of `CodecChainHasEnvelope` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecChainHasEnvelope.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecChainHasEnvelope`, which conforms itself.
public protocol CodecChainHasEnvelopeSource {
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    /// What each entry of `slices` is presented as: a source of `CodecChainHasSlice`, written by its encode.
    associatedtype Slices_Entry: CodecChainHasSliceSource
    var header: UInt8 { get }
    var extensions_has_0x2: CodecChainHasEnvelopeExtensionsHas0x2<Extensions_Entry, Slices_Entry> { get }
}

/// The frame this codec lays on the wire: 6 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasEnvelope: Equatable, Sendable, CodecChainHasEnvelopeSource {
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

    /// Whether an entry of `extensions` is of the kind 0x2 a gate asks the chain for (docs/adr/0065
    /// §6 R6), read off any source of its codec: what the arms of the chain's kinds are held to.
    public enum ExtensionsKind2<Entry: CodecZenohExtEntrySource>: SceKind {
        public static func matches(_ entry: Entry) -> Bool {
            ((chainIdentifier(entry.header) >> 0) & 0xF) == 0x2
        }
    }

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The value presents each entry of `slices` as a value of its own codec.
    public typealias Slices_Entry = CodecChainHasSlice

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

    /// The elements the gates `extensions.has(0x2)` decide together.
    public var extensions_has_0x2: CodecChainHasEnvelopeExtensionsHas0x2<Extensions_Entry, Slices_Entry> = .notHas0x2(extensions: nil, payload: .fewest)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, extensions_has_0x2: CodecChainHasEnvelopeExtensionsHas0x2<Extensions_Entry, Slices_Entry>) {
        self.header = header
        self.extensions_has_0x2 = extensions_has_0x2
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasEnvelope {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        var extensions: BoundedList<CodecZenohExtEntry, CodecChainHasEnvelope.ExtensionsBound>? = nil
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
            extensions = try BoundedList<CodecZenohExtEntry, CodecChainHasEnvelope.ExtensionsBound>(decoding: extensions_items)
        }
        let _has_extensions_2 = extensions?.contains(where: { _e in chainIdentifier(_e.extId) == 2 }) ?? false
        var payload_len: UInt64? = nil
        if !_has_extensions_2 {
            payload_len = try reader.readVle(UInt64.self)
        }
        var payload: BoundedBytes<CodecChainHasEnvelope.PayloadBound>? = nil
        if !_has_extensions_2 {
            if (payload_len ?? 0) > 256 {
                throw CodecError.lengthOutOfRange
            }
            payload = try BoundedBytes<CodecChainHasEnvelope.PayloadBound>(decoding: Array(try reader.readSlice(Int(clamping: (payload_len ?? 0)))))
        }
        var slice_count: UInt32? = nil
        if _has_extensions_2 {
            slice_count = try reader.readVle(UInt32.self)
        }
        var slices: BoundedList<CodecChainHasSlice, CodecChainHasEnvelope.SlicesBound>? = nil
        if _has_extensions_2 {
            if (slice_count ?? 0) > 4 {
                throw CodecError.tooManyElements
            }
            var slices_items: [CodecChainHasSlice] = []
            for _ in 0..<Int(clamping: (slice_count ?? 0)) {
                slices_items.append(try CodecChainHasSlice.decode(&reader))
            }
            slices = try BoundedList<CodecChainHasSlice, CodecChainHasEnvelope.SlicesBound>(decoding: slices_items)
        }
        let extensions_has_0x2_group: CodecChainHasEnvelopeExtensionsHas0x2<Extensions_Entry, Slices_Entry>
        if _has_extensions_2 {
            extensions_has_0x2_group = .has0x2(extensions: try SceChain(decoding: extensions!), slices: slices!)
        } else {
            var extensions_chain: SceChain<Extensions_Entry, CodecChainHasEnvelope.ExtensionsBound, SceNoKinds<Extensions_Entry>, SceKindsOf<CodecChainHasEnvelope.ExtensionsKind2<Extensions_Entry>, SceNoKinds<Extensions_Entry>>>? = nil
            if let extensions_read = extensions { extensions_chain = try SceChain<Extensions_Entry, CodecChainHasEnvelope.ExtensionsBound, SceNoKinds<Extensions_Entry>, SceKindsOf<CodecChainHasEnvelope.ExtensionsKind2<Extensions_Entry>, SceNoKinds<Extensions_Entry>>>(decoding: extensions_read) }
            extensions_has_0x2_group = .notHas0x2(extensions: extensions_chain, payload: payload!)
        }
        cursor = reader
        return CodecChainHasEnvelope(header: header, extensions_has_0x2: extensions_has_0x2_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecChainHasEnvelopeSource` is written,
    /// with no copy into `CodecChainHasEnvelope`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecChainHasEnvelopeSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var extensions_member: [Source.Extensions_Entry]? = nil
        var slices_member: BoundedList<Source.Slices_Entry, CodecChainHasEnvelope.SlicesBound>? = nil
        var payload_member: BoundedBytes<CodecChainHasEnvelope.PayloadBound>? = nil
        switch source.extensions_has_0x2 {
        case .has0x2(let extensions, let slices): extensions_member = extensions.entries; slices_member = slices
        case .notHas0x2(let extensions, let payload): extensions_member = extensions?.entries; payload_member = payload
        }
        let payload_len_derived: UInt64 = payload_member != nil ? UInt64((payload_member?.byteCount ?? 0)) : 0
        let slice_count_derived: UInt32 = slices_member != nil ? UInt32((slices_member?.count ?? 0)) : 0
        var header_folded = source.header
        header_folded = (header_folded & ~0x80) | ((extensions_member != nil) ? 0x80 : 0)
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        if let held = extensions_member {
            var extensions_entries = BytesSink()
            var extensions_left = held.count
            for entry in held {
                extensions_left -= 1
                try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_left != 0))
            }
            frame += extensions_entries.bytes
        }
        if !([0].contains(source.extensions_has_0x2.arm)) {
            appendVle(payload_len_derived, to: &frame)
        }
        if let held = payload_member {
            held.append(to: &frame)
        }
        if [0].contains(source.extensions_has_0x2.arm) {
            appendVle(slice_count_derived, to: &frame)
        }
        if let held = slices_member {
            var slices_entries = BytesSink()
            for entry in held {
                try CodecChainHasSlice.encodeSource(entry, into: &slices_entries)
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
