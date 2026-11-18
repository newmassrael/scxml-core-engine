// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_query_value.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohQueryValue` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohQueryValue.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohQueryValue`, which conforms itself.
public protocol CodecZenohQueryValueSource {
    /// What each entry of `exts` is presented as: a source of `CodecZenohValueEntry`, written by its encode.
    associatedtype Exts_Entry: CodecZenohValueEntrySource
    var exts: Lawful<BoundedList<Exts_Entry, CodecZenohQueryValue.ExtsBound>, CodecZenohQueryValue.ExtsLaw<Exts_Entry>> { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohQueryValue: Equatable, Sendable, CodecZenohQueryValueSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 0

    /// How many entries `exts` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = nil
    }

    /// For the law of `exts` (docs/adr/0065 §6 R8): what each entry says of `after_shm`, read off any source of its codec.
    public enum ExtsAfterShmSaid<Entry: CodecZenohValueEntrySource>: SceSaid {
        public static func says(_ entry: Entry) -> Bool? {
            CodecZenohValueEntry.inputAfterShm(entry).map { $0 != 0 }
        }
    }

    /// For the law of `exts` (docs/adr/0065 §6 R8): whether an entry is of the kind 0x4, after which the next says `after_shm` set, read off any source of its codec.
    public enum ExtsAfterShmKind<Entry: CodecZenohValueEntrySource>: SceKind {
        public static func matches(_ entry: Entry) -> Bool {
            ((chainIdentifier(entry.header) >> 0) & 0xF) == 0x4
        }
    }

    /// The law the entries of `exts` say the inputs it hands them by (docs/adr/0065 §6 R8),
    /// over any source of their codec: its type holds every entry to it where a value of it is made.
    public typealias ExtsLaw<Entry: CodecZenohValueEntrySource> = SceAfterKey<ExtsAfterShmSaid<Entry>, ExtsAfterShmKind<Entry>>

    /// The value presents each entry of `exts` as a value of its own codec.
    public typealias Exts_Entry = CodecZenohValueEntry

    /// Entries of `CodecZenohValueEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var exts: Lawful<BoundedList<CodecZenohValueEntry, CodecZenohQueryValue.ExtsBound>, CodecZenohQueryValue.ExtsLaw<CodecZenohValueEntry>> = .init(defaultOf: .fewest(of: CodecZenohValueEntry()))

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(exts: Lawful<BoundedList<Exts_Entry, CodecZenohQueryValue.ExtsBound>, CodecZenohQueryValue.ExtsLaw<Exts_Entry>>) {
        self.exts = exts
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohQueryValue {
        var reader = cursor
        var _prev_exts_after_shm: UInt8 = 0
        var exts_items: [CodecZenohValueEntry] = []
        var exts_more = false
        while true {
            let entry = try CodecZenohValueEntry.decode(&reader, after_shm: _prev_exts_after_shm)
            exts_more = entry.Z
            exts_items.append(entry)
            _prev_exts_after_shm = (chainIdentifier(entry.extId) == 4) ? 1 : 0
            if !exts_more { break }
        }
        let exts = try Lawful<BoundedList<CodecZenohValueEntry, CodecZenohQueryValue.ExtsBound>, CodecZenohQueryValue.ExtsLaw<CodecZenohValueEntry>>(decoding: try BoundedList<CodecZenohValueEntry, CodecZenohQueryValue.ExtsBound>(decoding: exts_items), start: SceFixedStart())
        cursor = reader
        return CodecZenohQueryValue(exts: exts)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohQueryValueSource` is written,
    /// with no copy into `CodecZenohQueryValue`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohQueryValueSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var exts_entries = BytesSink()
        var exts_left = source.exts.count
        for entry in source.exts {
            exts_left -= 1
            try CodecZenohValueEntry.encodeSource(entry, into: &exts_entries, overrides: CodecZenohValueEntry.Overrides(headerZ: exts_left != 0))
        }
        frame += exts_entries.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
