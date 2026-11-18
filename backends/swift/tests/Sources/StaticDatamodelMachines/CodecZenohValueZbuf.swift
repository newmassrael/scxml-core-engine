// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_zenoh_value_zbuf.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecZenohValueZbuf` the gates `after_shm` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
/// Each body and list is a source of its codec, so a host's type is presented with no copy.
public enum CodecZenohValueZbufAfterShm<Slices_Entry: CodecZenohValueSliceSource> {
    /// While after_shm set.
    case afterShm(encoding: UInt64, slices: BoundedList<Slices_Entry, CodecZenohValueZbuf.SlicesBound>, value_len: UInt64)
    /// While after_shm clear.
    case notAfterShm(value: BoundedBytes<CodecZenohValueZbuf.ValueBound>)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .afterShm: return 0
        case .notAfterShm: return 1
        }
    }
}

extension CodecZenohValueZbufAfterShm: Equatable where Slices_Entry: Equatable {}

extension CodecZenohValueZbufAfterShm: Sendable where Slices_Entry: Sendable {}

/// The elements a value of `CodecZenohValueZbuf` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohValueZbuf.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohValueZbuf`, which conforms itself.
public protocol CodecZenohValueZbufSource {
    /// What each entry of `slices` is presented as: a source of `CodecZenohValueSlice`, written by its encode.
    associatedtype Slices_Entry: CodecZenohValueSliceSource
    var after_shm: CodecZenohValueZbufAfterShm<Slices_Entry> { get }
}

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohValueZbuf: Equatable, Sendable, CodecZenohValueZbufSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `value` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum ValueBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 4294967295
    }

    /// How many entries `slices` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum SlicesBound: SceCountBound {
        public static let least = 0
        public static let most: Int? = nil
    }

    /// The value presents each entry of `slices` as a value of its own codec.
    public typealias Slices_Entry = CodecZenohValueSlice

    /// The elements the gates `after_shm` decide together.
    public var after_shm: CodecZenohValueZbufAfterShm<Slices_Entry> = .notAfterShm(value: .fewest)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(after_shm: CodecZenohValueZbufAfterShm<Slices_Entry>) {
        self.after_shm = after_shm
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, after_shm: UInt8) throws(CodecError) -> CodecZenohValueZbuf {
        var reader = cursor
        let value_len = try reader.readVle(UInt64.self)
        var value: BoundedBytes<CodecZenohValueZbuf.ValueBound>? = nil
        if (after_shm & 0x1) == 0 {
            if value_len > 4294967295 {
                throw CodecError.lengthOutOfRange
            }
            value = try BoundedBytes<CodecZenohValueZbuf.ValueBound>(decoding: Array(try reader.readSlice(Int(clamping: value_len))))
        }
        var encoding: UInt64? = nil
        if (after_shm & 0x1) != 0 {
            encoding = try reader.readVle(UInt64.self)
        }
        var slice_count: UInt64? = nil
        if (after_shm & 0x1) != 0 {
            slice_count = try reader.readVle(UInt64.self)
        }
        var slices: BoundedList<CodecZenohValueSlice, CodecZenohValueZbuf.SlicesBound>? = nil
        if (after_shm & 0x1) != 0 {
            var slices_items: [CodecZenohValueSlice] = []
            for _ in 0..<Int(clamping: (slice_count ?? 0)) {
                slices_items.append(try CodecZenohValueSlice.decode(&reader))
            }
            slices = try BoundedList<CodecZenohValueSlice, CodecZenohValueZbuf.SlicesBound>(decoding: slices_items)
        }
        let after_shm_group: CodecZenohValueZbufAfterShm<Slices_Entry>
        if (after_shm & 0x01) != 0 {
            after_shm_group = .afterShm(encoding: encoding!, slices: slices!, value_len: value_len)
        } else {
            after_shm_group = .notAfterShm(value: value!)
        }
        cursor = reader
        return CodecZenohValueZbuf(after_shm: after_shm_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohValueZbufSource` is written,
    /// with no copy into `CodecZenohValueZbuf`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohValueZbufSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var encoding_member: UInt64? = nil
        var slices_member: BoundedList<Source.Slices_Entry, CodecZenohValueZbuf.SlicesBound>? = nil
        var value_len_member: UInt64? = nil
        var value_member: BoundedBytes<CodecZenohValueZbuf.ValueBound>? = nil
        switch source.after_shm {
        case .afterShm(let encoding, let slices, let value_len): encoding_member = encoding; slices_member = slices; value_len_member = value_len
        case .notAfterShm(let value): value_member = value
        }
        let after_shm: UInt8 = [0].contains(source.after_shm.arm) ? 1 : 0
        let value_len_derived: UInt64 = value_member != nil ? UInt64((value_member?.byteCount ?? 0)) : (value_len_member ?? 0)
        let slice_count_derived: UInt64 = slices_member != nil ? UInt64((slices_member?.count ?? 0)) : 0
        var frame: [UInt8] = []
        appendVle(value_len_derived, to: &frame)
        if let held = value_member {
            held.append(to: &frame)
        }
        if let held = encoding_member {
            appendVle(held, to: &frame)
        }
        if after_shm != 0 {
            appendVle(slice_count_derived, to: &frame)
        }
        if let held = slices_member {
            var slices_entries = BytesSink()
            for entry in held {
                try CodecZenohValueSlice.encodeSource(entry, into: &slices_entries)
            }
            frame += slices_entries.bytes
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `after_shm` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputAfterShm<Source: CodecZenohValueZbufSource>(_ source: Source) -> UInt8 {
        [0].contains(source.after_shm.arm) ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
