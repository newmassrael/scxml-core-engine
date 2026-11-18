// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_every_arm_says_near.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecEveryArmSaysNear` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecEveryArmSaysNear.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecEveryArmSaysNear`, which conforms itself.
public protocol CodecEveryArmSaysNearSource {
    var id: UInt8 { get }
    var suffix: BoundedBytes<CodecEveryArmSaysNear.SuffixBound>? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecEveryArmSaysNear: Equatable, Sendable, CodecEveryArmSaysNearSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// How many bytes `suffix` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum SuffixBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 8
    }

    /// The field at byte 0, big-endian.
    public var id: UInt8 = 0

    /// On the wire only when `(N & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `suffix_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    public var suffix: BoundedBytes<CodecEveryArmSaysNear.SuffixBound>? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(id: UInt8, suffix: BoundedBytes<CodecEveryArmSaysNear.SuffixBound>?) {
        self.id = id
        self.suffix = suffix
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, N: UInt8) throws(CodecError) -> CodecEveryArmSaysNear {
        var reader = cursor
        let id = try reader.readInteger(UInt8.self, .big)
        var suffix_len: UInt8? = nil
        if (N & 0x1) != 0 {
            suffix_len = try reader.readInteger(UInt8.self, .big)
        }
        var suffix: BoundedBytes<CodecEveryArmSaysNear.SuffixBound>? = nil
        if (N & 0x1) != 0 {
            if (suffix_len ?? 0) > 8 {
                throw CodecError.lengthOutOfRange
            }
            suffix = try BoundedBytes<CodecEveryArmSaysNear.SuffixBound>(decoding: Array(try reader.readSlice(Int(clamping: (suffix_len ?? 0)))))
        }
        cursor = reader
        return CodecEveryArmSaysNear(id: id, suffix: suffix)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecEveryArmSaysNearSource` is written,
    /// with no copy into `CodecEveryArmSaysNear`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecEveryArmSaysNearSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let N: UInt8 = source.suffix != nil ? 1 : 0
        let suffix_len_derived: UInt8 = source.suffix != nil ? UInt8((source.suffix?.byteCount ?? 0)) : 0
        var frame: [UInt8] = []
        appendInteger(source.id, to: &frame, .big)
        if N != 0 {
            appendInteger(suffix_len_derived, to: &frame, .big)
        }
        if let held = source.suffix {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `N` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputN<Source: CodecEveryArmSaysNearSource>(_ source: Source) -> UInt8 {
        source.suffix != nil ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
